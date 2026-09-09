#!/usr/bin/env bash
# Rebalance cLUNC/cUSTC to the indexer LUNC+USTC oracles, deepen v2 LP in USD
# rungs ($200 → $500 → $2k → $5k → $10k, $5k each side at the end), send LP to
# the CMM treasury, then holder-burn leftover cLUNC/cUSTC on the ops wallet.
#
# Flow (each rung):
#   1. Fetch indexer oracles, cross-check live CEX (CoinGecko/Binance/KuCoin/MEXC).
#   2. Pool-only swap until human cUSTC-per-cLUNC is within 0.1% of LUNC_USD/USTC_USD.
#   3. Mint (primary minter) or wrap native to cover the TVL delta.
#   4. provide_liquidity with receiver = CMM.
#   5. Re-query and rebalance again before the next rung.
#   6. After the last rung: transfer any leftover LP to CMM; burn leftover wrap CW20.
#      Never burns LP, LP tokens, CMM holdings, native uluna, or other CW20s.
#
# Usage:
#   DRY_RUN=1 ./scripts/rebalance-mint-clunc-custc-lp.sh
#   CLUNC_LP_YES=1 ./scripts/rebalance-mint-clunc-custc-lp.sh
#   CLUNC_LP_BURN_ONLY=1 CLUNC_LP_YES=1 ./scripts/rebalance-mint-clunc-custc-lp.sh
#   CLUNC_LP_MINT_MODE=wrap  # treasury WrapDeposit instead of unbacked mint
#
# Unlock once (non-interactive):
#   read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
#
# Never commit TERRAD_HOST_KEYRING_PASS.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/clunc-custc-lp-defaults.sh
source "$SCRIPT_DIR/lib/clunc-custc-lp-defaults.sh"
# shellcheck source=lib/terrad-host.sh
source "$SCRIPT_DIR/lib/terrad-host.sh"

TERRAD_HOST_CHAIN_ID="${TERRAD_HOST_CHAIN_ID:-columbus-5}"
TERRAD_HOST_NODE="${TERRAD_HOST_NODE:-https://terra-classic-rpc.publicnode.com:443}"
LCD_URL="${CLUNC_LP_LCD_URL%/}"
MATH_PY="$SCRIPT_DIR/lib/clunc-custc-lp-math.py"
INDEXER="${CLUNC_LP_INDEXER%/}"
FACTORY="${CLUNC_LP_FACTORY}"
CLUNC="${CLUNC_LP_CLUNC}"
CUSTC="${CLUNC_LP_CUSTC}"
PAIR="${CLUNC_LP_PAIR}"
LP_PIN="${CLUNC_LP_LP_TOKEN}"
TREASURY="${CLUNC_LP_TREASURY}"
WRAP_MAPPER="${CLUNC_LP_WRAP_MAPPER}"
MINTER_KEY="${CLUNC_LP_MINTER_KEY}"
MINTER_ADDR="${CLUNC_LP_MINTER_ADDR}"
ADMIN_KEY="${CLUNC_LP_ADMIN_KEY}"
ADMIN_ADDR="${CLUNC_LP_ADMIN_ADDR}"
RUNGS="${CLUNC_LP_RUNGS}"
TOLERANCE="${CLUNC_LP_PRICE_TOLERANCE}"
CROSS_TOL="${CLUNC_LP_CROSS_CHECK_TOLERANCE}"
MAX_AGE="${CLUNC_LP_ORACLE_MAX_AGE_SEC}"
MINT_MODE="${CLUNC_LP_MINT_MODE}"
SWAP_MAX_SPREAD="${CLUNC_LP_SWAP_MAX_SPREAD:-0.20}"
SWAP_MAX_ITERS="${CLUNC_LP_SWAP_MAX_ITERS:-3}"
PROVIDE_SLIP="${CLUNC_LP_PROVIDE_SLIPPAGE:-0.005}"
BUFFER_BPS="${CLUNC_LP_MINT_BUFFER_BPS:-50}"
LCD_TIMEOUT="${CLUNC_LP_LCD_TIMEOUT:-25}"
GAS_RESERVE_ULUNA="${CLUNC_LP_GAS_RESERVE_ULUNA:-2000000000}"
HTTP_UA="cl8y-dex-ops/clunc-custc-lp (+https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic)"

die() { echo "ERROR: $*" >&2; exit 1; }

usage() {
  sed -n '2,24p' "$0" | sed 's/^# \{0,1\}//'
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

case "$MINT_MODE" in
  minter|wrap) ;;
  *) die "CLUNC_LP_MINT_MODE must be minter or wrap (got $MINT_MODE)" ;;
esac

lcd_b64() {
  if [[ "$(uname)" == Darwin ]]; then
    printf '%s' "$1" | base64 | tr -d '\n'
  else
    printf '%s' "$1" | base64 -w0
  fi
}

lcd_smart() {
  local contract="$1" msg="$2" raw
  raw="$(curl -sS --connect-timeout 10 --max-time "$LCD_TIMEOUT" -H "User-Agent: $HTTP_UA" \
    "${LCD_URL}/cosmwasm/wasm/v1/contract/${contract}/smart/$(lcd_b64 "$msg")")"
  jq -e '(.data != null) and ((.code // 0) == 0)' >/dev/null <<<"$raw" \
    || die "LCD smart query failed for $contract: $msg"
  if [[ "$(jq -r '.data | type' <<<"$raw")" == "string" ]]; then
    jq -r '.data | @base64d | fromjson' <<<"$raw"
  else
    jq '.data' <<<"$raw"
  fi
}

lcd_bank() {
  local addr="$1" denom="$2"
  curl -sS --connect-timeout 10 --max-time "$LCD_TIMEOUT" -H "User-Agent: $HTTP_UA" \
    "${LCD_URL}/cosmos/bank/v1beta1/balances/${addr}/by_denom?denom=${denom}" \
    | jq -r '.balance.amount // "0"'
}

cw20_balance() {
  lcd_smart "$1" "$(jq -nc --arg a "$2" '{balance:{address:$a}}')" | jq -r '.balance // "0"'
}

token_decimals() {
  lcd_smart "$1" '{"token_info":{}}' | jq -r '.decimals'
}

token_symbol() {
  lcd_smart "$1" '{"token_info":{}}' | jq -r '.symbol'
}

is_minter() {
  local token="$1" who="$2" primary extras
  primary="$(lcd_smart "$token" '{"minter":{}}' | jq -r '.minter // empty')"
  extras="$(lcd_smart "$token" '{"minters":{}}' | jq -r '[.minters[]?] | join(" ")' 2>/dev/null || echo "")"
  [[ "$primary" == "$who" || " $extras " == *" $who "* ]]
}

pool_json() { lcd_smart "$1" '{"pool":{}}'; }

asset_amount_for() {
  local pool="$1" token="$2"
  jq -r --arg t "$token" '
    .assets[]
    | select(.info.token.contract_addr == $t)
    | .amount
  ' <<<"$pool"
}

prompt_keyring_pass() {
  if [[ -n "${TERRAD_HOST_KEYRING_PASS:-}" ]]; then
    return 0
  fi
  if [[ ! -t 0 ]]; then
    die "TERRAD_HOST_KEYRING_PASS unset and stdin is not a TTY. Unlock once:
  read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS"
  fi
  read -rs -p "terrad keyring passphrase: " TERRAD_HOST_KEYRING_PASS
  echo
  export TERRAD_HOST_KEYRING_PASS
  [[ -n "$TERRAD_HOST_KEYRING_PASS" ]] || die "empty passphrase"
}

broadcast_from() {
  local key="$1" expected="$2" label="$3"
  shift 3
  local out tx_hash
  echo "  → $label" >&2
  TERRAD_HOST_KEY="$key"
  TERRAD_HOST_EXPECTED_ADDR="$expected"
  out="$(terrad_host_tx "$@")"
  tx_hash="$(printf '%s' "$out" | jq -r '.txhash // empty')"
  [[ -n "$tx_hash" ]] || {
    echo "ERROR: no txhash from: $label" >&2
    printf '%s\n' "$out" >&2
    exit 1
  }
  echo "    tx: $tx_hash" >&2
  terrad_host_wait_tx_inclusion "$tx_hash"
  printf '%s' "$tx_hash"
}

broadcast_admin() {
  local label="$1"
  shift
  broadcast_from "$ADMIN_KEY" "$ADMIN_ADDR" "$label" "$@"
}

broadcast_minter() {
  local label="$1"
  shift
  broadcast_from "$MINTER_KEY" "$MINTER_ADDR" "$label" "$@"
}

mint_cw20() {
  local sym="$1" token="$2" amount="$3"
  if [[ "$amount" == "0" ]]; then
    echo "  skip mint $sym (already funded)"
    return 0
  fi
  local msg
  msg="$(jq -nc --arg r "$ADMIN_ADDR" --arg a "$amount" '{mint:{recipient:$r,amount:$a}}')"
  if [[ "$MINTER_ADDR" == "$ADMIN_ADDR" ]]; then
    broadcast_admin "mint $sym $amount → $ADMIN_ADDR" wasm execute "$token" "$msg" >/dev/null
  else
    broadcast_minter "mint $sym $amount → $ADMIN_ADDR" wasm execute "$token" "$msg" >/dev/null
  fi
}

wrap_native() {
  local denom="$1" amount="$2" label="$3"
  if [[ "$amount" == "0" ]]; then
    echo "  skip wrap $denom (already funded)"
    return 0
  fi
  broadcast_admin "wrap $label $amount $denom" \
    wasm execute "$TREASURY" '{"wrap_deposit":{}}' --amount "${amount}${denom}" >/dev/null
}

cw20_send_swap() {
  local token="$1" amount="$2"
  local hook b64 msg
  hook="$(jq -nc --arg s "$SWAP_MAX_SPREAD" \
    '{swap:{belief_price:null,max_spread:$s,to:null,deadline:null,hybrid:null,trader:null}}')"
  if [[ "$(uname)" == Darwin ]]; then
    b64="$(printf '%s' "$hook" | base64 | tr -d '\n')"
  else
    b64="$(printf '%s' "$hook" | base64 -w0)"
  fi
  msg="$(jq -nc --arg c "$PAIR" --arg a "$amount" --arg m "$b64" \
    '{send:{contract:$c,amount:$a,msg:$m}}')"
  broadcast_admin "swap $amount on cLUNC/cUSTC (pool-only)" wasm execute "$token" "$msg"
}

provide_to_treasury() {
  local a0="$1" a1="$2"
  local allow0 allow1 provide
  allow0="$(jq -nc --arg s "$PAIR" --arg a "$a0" \
    '{increase_allowance:{spender:$s,amount:$a,expires:{never:{}}}}')"
  allow1="$(jq -nc --arg s "$PAIR" --arg a "$a1" \
    '{increase_allowance:{spender:$s,amount:$a,expires:{never:{}}}}')"
  provide="$(jq -nc --arg a "$CLUNC" --arg b "$CUSTC" --arg aa "$a0" --arg bb "$a1" \
    --arg recv "$TREASURY" --arg slip "$PROVIDE_SLIP" \
    '{provide_liquidity:{assets:[
      {info:{token:{contract_addr:$a}},amount:$aa},
      {info:{token:{contract_addr:$b}},amount:$bb}
    ],slippage_tolerance:$slip,receiver:$recv,deadline:null}}')"
  broadcast_admin "increase_allowance cLUNC" wasm execute "$CLUNC" "$allow0" >/dev/null
  broadcast_admin "increase_allowance cUSTC" wasm execute "$CUSTC" "$allow1" >/dev/null
  broadcast_admin "provide_liquidity cLUNC/cUSTC → treasury" wasm execute "$PAIR" "$provide"
}

assert_not_protected_token() {
  local token="$1" what="$2"
  [[ "$token" == "$LP_TOKEN" || "$token" == "$LP_PIN" ]] \
    && die "refusing to $what LP token $token"
  [[ "$token" == "$TREASURY" ]] && die "refusing to $what CMM treasury $token"
  [[ "$token" == "$PAIR" ]] && die "refusing to $what pair $token"
}

burn_if_any() {
  local sym="$1" token="$2" amount="$3"
  assert_not_protected_token "$token" "burn"
  if [[ "$amount" == "0" ]]; then
    echo "  skip burn $sym (balance 0)"
    return 0
  fi
  local msg
  msg="$(jq -nc --arg a "$amount" '{burn:{amount:$a}}')"
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "  [DRY_RUN] burn $sym $amount on $token"
    return 0
  fi
  broadcast_admin "burn $sym $amount" wasm execute "$token" "$msg" >/dev/null
}

transfer_lp_if_any() {
  local amount="$1"
  if [[ "$amount" == "0" ]]; then
    echo "  admin LP already 0"
    return 0
  fi
  local msg
  msg="$(jq -nc --arg r "$TREASURY" --arg a "$amount" '{transfer:{recipient:$r,amount:$a}}')"
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "  [DRY_RUN] transfer LP $amount → $TREASURY"
    return 0
  fi
  broadcast_admin "transfer leftover LP $amount → CMM" wasm execute "$LP_TOKEN" "$msg" >/dev/null
}

rel_err_ok() {
  python3 - "$1" "$2" "$TOLERANCE" <<'PY'
import sys
from decimal import Decimal
cur, tgt, tol = map(Decimal, sys.argv[1:])
sys.exit(0 if abs(cur - tgt) / tgt <= tol else 1)
PY
}

human_px() {
  python3 - "$1" "$2" "$3" "$4" <<'PY'
import sys
from decimal import Decimal
r0, r1, d0, d1 = sys.argv[1:]
print((Decimal(r1) / (Decimal(10) ** int(d1))) / (Decimal(r0) / (Decimal(10) ** int(d0))))
PY
}

gt() {
  python3 -c "import sys; sys.exit(0 if int('$1') > int('$2') else 1)"
}

ge() {
  python3 -c "import sys; sys.exit(0 if int('$1') >= int('$2') else 1)"
}

fetch_oracles() {
  python3 "$MATH_PY" --fetch-oracles <<EOF
{
  "mode": "fetch-oracles",
  "indexer": "$INDEXER",
  "max_age_sec": $MAX_AGE,
  "cross_tolerance": "$CROSS_TOL",
  "skip_cross_check": $([[ "${CLUNC_LP_SKIP_CROSS_CHECK:-0}" == "1" ]] && echo true || echo false),
  "clunc_addr": "$CLUNC",
  "custc_addr": "$CUSTC"
}
EOF
}

fetch_oracles_ok() {
  local i json errs tries="${CLUNC_LP_ORACLE_RETRIES:-4}"
  for ((i = 1; i <= tries; i++)); do
    json="$(fetch_oracles)" || true
    if jq -e '.ok == true and .lunc_usd != null and .ustc_usd != null' >/dev/null 2>&1 <<<"$json"; then
      printf '%s' "$json"
      return 0
    fi
    errs="$(jq -c '.errors // []' <<<"$json" 2>/dev/null || echo "$json")"
    echo "  oracle fetch attempt $i/$tries failed: $errs" >&2
    if [[ "$i" -lt "$tries" ]]; then
      sleep $((i * 3))
    fi
  done
  echo "$json" >&2
  return 1
}

plan_step() {
  local target_usd="$1"
  python3 "$MATH_PY" <<EOF
{
  "mode": "plan-step",
  "lunc_usd": "$LUNC_USD",
  "ustc_usd": "$USTC_USD",
  "target_usd": "$target_usd",
  "tolerance": "$TOLERANCE",
  "fee_bps": $FEE_BPS,
  "buffer_bps": $BUFFER_BPS,
  "fee_wrap_bps": $FEE_WRAP_BPS,
  "dec_clunc": $DEC_CLUNC,
  "dec_custc": $DEC_CUSTC,
  "r0": "$R0",
  "r1": "$R1",
  "bal_clunc": "$BAL_CLUNC",
  "bal_custc": "$BAL_CUSTC"
}
EOF
}

refresh_pool() {
  POOL="$(pool_json "$PAIR")"
  R0="$(asset_amount_for "$POOL" "$CLUNC")"
  R1="$(asset_amount_for "$POOL" "$CUSTC")"
  [[ -n "$R0" && -n "$R1" ]] || die "failed to read cLUNC/cUSTC reserves"
  CUR_PX="$(human_px "$R0" "$R1" "$DEC_CLUNC" "$DEC_CUSTC")"
}

refresh_balances() {
  BAL_CLUNC="$(cw20_balance "$CLUNC" "$ADMIN_ADDR")"
  BAL_CUSTC="$(cw20_balance "$CUSTC" "$ADMIN_ADDR")"
}

fund_inventory() {
  local token="$1" amount="$2" have need native
  if [[ "$amount" == "0" ]]; then
    return 0
  fi
  if [[ "$token" == "clunc" ]]; then
    have="$(cw20_balance "$CLUNC" "$ADMIN_ADDR")"
    if python3 -c "import sys; sys.exit(0 if int('$have') < int('$amount') else 1)"; then
      need="$(python3 -c "print(int('$amount') - int('$have'))")"
      echo "  topping up cLUNC $need (have $have)"
      if [[ "$MINT_MODE" == "wrap" ]]; then
        native="$(python3 -c "print((int('$need') * 10000 + (10000 - $FEE_WRAP_BPS) - 1) // (10000 - $FEE_WRAP_BPS))")"
        wrap_native uluna "$native" cLUNC
      else
        mint_cw20 cLUNC "$CLUNC" "$need"
      fi
    fi
  elif [[ "$token" == "custc" ]]; then
    have="$(cw20_balance "$CUSTC" "$ADMIN_ADDR")"
    if python3 -c "import sys; sys.exit(0 if int('$have') < int('$amount') else 1)"; then
      need="$(python3 -c "print(int('$amount') - int('$have'))")"
      echo "  topping up cUSTC $need (have $have)"
      if [[ "$MINT_MODE" == "wrap" ]]; then
        native="$(python3 -c "print((int('$need') * 10000 + (10000 - $FEE_WRAP_BPS) - 1) // (10000 - $FEE_WRAP_BPS))")"
        wrap_native uusd "$native" cUSTC
      else
        mint_cw20 cUSTC "$CUSTC" "$need"
      fi
    fi
  else
    die "unknown token $token"
  fi
}

execute_rebalance_swaps() {
  local i token amt live
  for ((i = 1; i <= SWAP_MAX_ITERS; i++)); do
    refresh_pool
    if rel_err_ok "$CUR_PX" "$TARGET_PX"; then
      echo "  within $TOLERANCE of target at $CUR_PX (iter $i, no swap)"
      return 0
    fi
    refresh_balances
    live="$(plan_step "$CURRENT_TARGET_USD")"
    token="$(jq -r '.swap.offer_token // empty' <<<"$live")"
    amt="$(jq -r '.swap.offer_amount' <<<"$live")"
    [[ "$(jq -r '.swap.needed' <<<"$live")" == "true" && -n "$token" && "$amt" != "0" ]] \
      || die "price $CUR_PX off peg but planner found no swap"
    echo "  iter $i/$SWAP_MAX_ITERS live $R0 / $R1  offer $token $amt"
    echo "    projected $(jq -r '.swap.projected_price' <<<"$live")  rel $(jq -r '.swap.rel_error' <<<"$live")"
    fund_inventory "$token" "$amt"
    if [[ "$token" == "clunc" ]]; then
      cw20_send_swap "$CLUNC" "$amt" >/dev/null
    else
      cw20_send_swap "$CUSTC" "$amt" >/dev/null
    fi
  done
  refresh_pool
  rel_err_ok "$CUR_PX" "$TARGET_PX" \
    || die "cLUNC/cUSTC price $CUR_PX not within $TOLERANCE of $TARGET_PX after $SWAP_MAX_ITERS swaps"
}

burn_admin_wrap() {
  local fail=0
  [[ "$ADMIN_ADDR" != "$TREASURY" ]] || die "refusing to burn on CMM treasury"
  refresh_balances
  echo "[burn] leftover cLUNC/cUSTC on $ADMIN_ADDR (not LP, not CMM, not native)"
  echo "  before cLUNC=$BAL_CLUNC cUSTC=$BAL_CUSTC"
  burn_if_any cLUNC "$CLUNC" "$BAL_CLUNC"
  burn_if_any cUSTC "$CUSTC" "$BAL_CUSTC"
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    return 0
  fi
  refresh_balances
  ADM_LP="$(cw20_balance "$LP_TOKEN" "$ADMIN_ADDR")"
  if [[ "$BAL_CLUNC" == "0" && "$BAL_CUSTC" == "0" ]]; then
    echo "  PASS admin cLUNC/cUSTC = 0"
  else
    echo "  FAIL leftover wrap remains cLUNC=$BAL_CLUNC cUSTC=$BAL_CUSTC" >&2
    fail=1
  fi
  if [[ "$ADM_LP" == "0" ]]; then
    echo "  PASS admin LP = 0"
  else
    echo "  FAIL admin still holds LP $ADM_LP (burn skipped)" >&2
    fail=1
  fi
  return "$fail"
}

echo "=============================================="
echo "Rebalance + mint cLUNC/cUSTC LP → CMM"
echo "=============================================="
echo "Pair:       $PAIR"
echo "Treasury:   $TREASURY"
echo "Admin key:  $ADMIN_KEY ($ADMIN_ADDR)"
echo "Minter key: $MINTER_KEY ($MINTER_ADDR)"
echo "Rungs USD:  $RUNGS"
echo "Tolerance:  $TOLERANCE"
echo "Mint mode:  $MINT_MODE"
echo "BURN_ONLY:  ${CLUNC_LP_BURN_ONLY:-0}"
echo "DRY_RUN:    ${DRY_RUN:-0}"
echo ""

command -v jq >/dev/null || die "jq required"
command -v python3 >/dev/null || die "python3 required"
command -v curl >/dev/null || die "curl required"
[[ -f "$MATH_PY" ]] || die "missing $MATH_PY"
python3 "$MATH_PY" --self-test
if [[ "${DRY_RUN:-0}" != "1" ]]; then
  command -v terrad >/dev/null || die "terrad required on PATH"
fi

[[ "$ADMIN_ADDR" != "$TREASURY" ]] || die "ops wallet must not be the CMM treasury"

echo "[preflight] tokens + pair"
SYM_CLUNC="$(token_symbol "$CLUNC")"
SYM_CUSTC="$(token_symbol "$CUSTC")"
DEC_CLUNC="$(token_decimals "$CLUNC")"
DEC_CUSTC="$(token_decimals "$CUSTC")"
echo "  cLUNC symbol=$SYM_CLUNC decimals=$DEC_CLUNC"
echo "  cUSTC symbol=$SYM_CUSTC decimals=$DEC_CUSTC"
[[ "$(printf '%s' "$SYM_CLUNC" | tr '[:upper:]' '[:lower:]')" == "clunc" ]] || die "cLUNC symbol $SYM_CLUNC"
[[ "$(printf '%s' "$SYM_CUSTC" | tr '[:upper:]' '[:lower:]')" == "custc" ]] || die "cUSTC symbol $SYM_CUSTC"
[[ "$DEC_CLUNC" == "6" && "$DEC_CUSTC" == "6" ]] \
  || die "expected 6/6 decimals (got $DEC_CLUNC/$DEC_CUSTC)"

PAIR_INFO="$(lcd_smart "$PAIR" '{"pair":{}}')"
LP_TOKEN="$(jq -r '.liquidity_token // empty' <<<"$PAIR_INFO")"
[[ -n "$LP_TOKEN" ]] || die "missing LP token"
[[ "$LP_TOKEN" == "$LP_PIN" ]] || die "LP token $LP_TOKEN != pin $LP_PIN"
GOT0="$(jq -r '.asset_infos[0].token.contract_addr' <<<"$PAIR_INFO")"
GOT1="$(jq -r '.asset_infos[1].token.contract_addr' <<<"$PAIR_INFO")"
[[ "$GOT0" == "$CLUNC" && "$GOT1" == "$CUSTC" ]] \
  || die "pair asset order $GOT0 / $GOT1 != cLUNC / cUSTC"

FEE_BPS="$(lcd_smart "$PAIR" '{"get_fee_config":{}}' | jq -r '.fee_config.fee_bps // .fee_bps')"
MAPPER_CFG="$(lcd_smart "$WRAP_MAPPER" '{"config":{}}')"
[[ "$(jq -r '.paused' <<<"$MAPPER_CFG")" != "true" ]] || die "wrap-mapper is paused"
FEE_WRAP_BPS="$(jq -r '.fee_wrap_bps // .fee_bps' <<<"$MAPPER_CFG")"
[[ -n "$FEE_WRAP_BPS" && "$FEE_WRAP_BPS" != "null" ]] || die "wrap fee missing"

if [[ "$MINT_MODE" == "minter" ]]; then
  is_minter "$CLUNC" "$MINTER_ADDR" || die "cLUNC minter is not $MINTER_ADDR"
  is_minter "$CUSTC" "$MINTER_ADDR" || die "cUSTC minter is not $MINTER_ADDR"
  echo "  WARN unbacked minter mint increases wrap CW20 supply without treasury native (W1)."
  echo "       Prefer CLUNC_LP_MINT_MODE=wrap when the ops wallet holds native LUNC/USTC."
fi

refresh_pool
refresh_balances
TRE_LP0="$(cw20_balance "$LP_TOKEN" "$TREASURY")"
ADM_LP0="$(cw20_balance "$LP_TOKEN" "$ADMIN_ADDR")"
echo "  reserves cLUNC=$R0 cUSTC=$R1  fee_bps=$FEE_BPS  wrap_fee_bps=$FEE_WRAP_BPS"
echo "  admin wrap cLUNC=$BAL_CLUNC cUSTC=$BAL_CUSTC  LP=$ADM_LP0"
echo "  treasury LP=$TRE_LP0"

echo ""
echo "[oracle] indexer + live CEX"
ORACLE_JSON="$(fetch_oracles_ok)" || {
  echo "$ORACLE_JSON" >&2
  die "oracle fetch / cross-check failed (set CLUNC_LP_SKIP_CROSS_CHECK=1 to bypass live CEX)"
}
echo "$ORACLE_JSON" | jq '{
  ok, lunc_usd, ustc_usd, target_custc_per_clunc,
  live_median, errors, warnings,
  live
}'
LUNC_USD="$(jq -r '.lunc_usd' <<<"$ORACLE_JSON")"
USTC_USD="$(jq -r '.ustc_usd' <<<"$ORACLE_JSON")"
TARGET_PX="$(jq -r '.target_custc_per_clunc' <<<"$ORACLE_JSON")"
[[ -n "$LUNC_USD" && -n "$USTC_USD" && "$LUNC_USD" != "null" ]] || die "empty oracle"

if [[ "${DRY_RUN:-0}" != "1" ]]; then
  prompt_keyring_pass
  TERRAD_HOST_KEY="$ADMIN_KEY"
  TERRAD_HOST_EXPECTED_ADDR="$ADMIN_ADDR"
  GOT_ADMIN="$(terrad_host_key_address)" || exit 1
  [[ "$GOT_ADMIN" == "$ADMIN_ADDR" ]] || die "admin key $ADMIN_KEY is $GOT_ADMIN (want $ADMIN_ADDR)"
  if [[ "$MINT_MODE" == "minter" && "$MINTER_ADDR" != "$ADMIN_ADDR" ]]; then
    TERRAD_HOST_KEY="$MINTER_KEY"
    TERRAD_HOST_EXPECTED_ADDR="$MINTER_ADDR"
    GOT_MINTER="$(terrad_host_key_address)" || exit 1
    [[ "$GOT_MINTER" == "$MINTER_ADDR" ]] || die "minter key $MINTER_KEY is $GOT_MINTER (want $MINTER_ADDR)"
  fi
else
  echo "  [DRY_RUN] skipping key unlock; assuming admin $ADMIN_ADDR"
fi

if [[ "${CLUNC_LP_BURN_ONLY:-0}" == "1" ]]; then
  if [[ "${DRY_RUN:-0}" != "1" && "${CLUNC_LP_YES:-0}" != "1" ]]; then
    if [[ ! -t 0 ]]; then
      die "refusing live burn without TTY; set CLUNC_LP_YES=1"
    fi
    read -r -p "Broadcast holder burn of leftover cLUNC/cUSTC on $ADMIN_ADDR ($TERRAD_HOST_CHAIN_ID)? [y/N] " ans
    [[ "$ans" == "y" || "$ans" == "Y" ]] || die "aborted"
  fi
  echo ""
  if ! burn_admin_wrap; then
    die "leftover wrap burn failed"
  fi
  echo "OK — admin leftover cLUNC/cUSTC burned."
  exit 0
fi

echo ""
echo "[plan] all rungs against current pool"
PLAN="$(python3 "$MATH_PY" <<EOF
{
  "mode": "plan-rungs",
  "lunc_usd": "$LUNC_USD",
  "ustc_usd": "$USTC_USD",
  "tolerance": "$TOLERANCE",
  "fee_bps": $FEE_BPS,
  "buffer_bps": $BUFFER_BPS,
  "fee_wrap_bps": $FEE_WRAP_BPS,
  "dec_clunc": $DEC_CLUNC,
  "dec_custc": $DEC_CUSTC,
  "r0": "$R0",
  "r1": "$R1",
  "bal_clunc": "$BAL_CLUNC",
  "bal_custc": "$BAL_CUSTC",
  "rungs": "$RUNGS"
}
EOF
)"
echo "$PLAN" | jq '{
  rungs,
  totals,
  final,
  steps: [.steps[] | {target_usd, add_usd, already_on_peg, swap, lp, mint, wrap_native}]
}'

if [[ "$MINT_MODE" == "wrap" ]]; then
  NEED_ULUNA="$(jq -r '.totals.uluna' <<<"$PLAN")"
  NEED_UUSD="$(jq -r '.totals.uusd' <<<"$PLAN")"
  HAVE_ULUNA="$(lcd_bank "$ADMIN_ADDR" uluna)"
  HAVE_UUSD="$(lcd_bank "$ADMIN_ADDR" uusd)"
  echo "  wrap native need uluna=$NEED_ULUNA (have $HAVE_ULUNA, gas reserve $GAS_RESERVE_ULUNA)"
  echo "  wrap native need uusd=$NEED_UUSD (have $HAVE_UUSD)"
  python3 -c "import sys; sys.exit(0 if int('$HAVE_ULUNA') >= int('$NEED_ULUNA') + int('$GAS_RESERVE_ULUNA') else 1)" \
    || die "admin uluna short for wrap + gas"
  python3 -c "import sys; sys.exit(0 if int('$HAVE_UUSD') >= int('$NEED_UUSD') else 1)" \
    || die "admin uusd short for wrap"
fi

if [[ "${DRY_RUN:-0}" == "1" ]]; then
  echo ""
  echo "DRY_RUN complete (no txs). Live run re-queries oracles + reserves on each rung."
  echo "Re-run without DRY_RUN=1 to broadcast."
  exit 0
fi

if [[ "${CLUNC_LP_YES:-0}" != "1" ]]; then
  if [[ ! -t 0 ]]; then
    die "refusing live broadcast without TTY; set CLUNC_LP_YES=1"
  fi
  read -r -p "Broadcast mint/wrap / rebalance / LP-to-treasury / leftover burn on $TERRAD_HOST_CHAIN_ID? [y/N] " ans
  [[ "$ans" == "y" || "$ans" == "Y" ]] || die "aborted"
fi

IFS=',' read -r -a RUNG_ARR <<<"$RUNGS"
RUNG_N=${#RUNG_ARR[@]}
FAIL=0

for idx in "${!RUNG_ARR[@]}"; do
  CURRENT_TARGET_USD="${RUNG_ARR[$idx]// /}"
  echo ""
  echo "========== rung $((idx + 1))/$RUNG_N  target \$${CURRENT_TARGET_USD} =========="
  ORACLE_JSON="$(fetch_oracles_ok)" || die "oracle refresh failed on rung $CURRENT_TARGET_USD"
  LUNC_USD="$(jq -r '.lunc_usd' <<<"$ORACLE_JSON")"
  USTC_USD="$(jq -r '.ustc_usd' <<<"$ORACLE_JSON")"
  TARGET_PX="$(jq -r '.target_custc_per_clunc' <<<"$ORACLE_JSON")"
  echo "  oracle LUNC/USD=$LUNC_USD USTC/USD=$USTC_USD  target cUSTC/cLUNC=$TARGET_PX"

  echo "[rebalance] before provide"
  execute_rebalance_swaps

  refresh_pool
  refresh_balances
  LIVE="$(plan_step "$CURRENT_TARGET_USD")"
  ADD_USD="$(jq -r '.add_usd' <<<"$LIVE")"
  LP0="$(jq -r '.lp.clunc' <<<"$LIVE")"
  LP1="$(jq -r '.lp.custc' <<<"$LIVE")"
  echo "  post-swap TVL \$$(jq -r '.post_swap_tvl_usd' <<<"$LIVE")  add \$$ADD_USD"
  echo "  LP legs raw cLUNC=$LP0 cUSTC=$LP1"

  if [[ "$LP0" == "0" && "$LP1" == "0" ]]; then
    echo "  skip provide (already at or above \$${CURRENT_TARGET_USD})"
  else
    fund_inventory clunc "$LP0"
    fund_inventory custc "$LP1"
    refresh_pool
    if ! rel_err_ok "$CUR_PX" "$TARGET_PX"; then
      echo "  peg drifted during mint; rebalance again"
      execute_rebalance_swaps
    fi
    refresh_pool
    refresh_balances
    LIVE="$(plan_step "$CURRENT_TARGET_USD")"
    LP0="$(jq -r '.lp.clunc' <<<"$LIVE")"
    LP1="$(jq -r '.lp.custc' <<<"$LIVE")"
    if [[ "$LP0" == "0" && "$LP1" == "0" ]]; then
      echo "  skip provide after re-plan (already at target)"
    else
      fund_inventory clunc "$LP0"
      fund_inventory custc "$LP1"
      refresh_balances
      ge "$BAL_CLUNC" "$LP0" && ge "$BAL_CUSTC" "$LP1" \
        || die "admin inventory short for LP (cLUNC $BAL_CLUNC need $LP0; cUSTC $BAL_CUSTC need $LP1)"
      TRE_BEFORE="$(cw20_balance "$LP_TOKEN" "$TREASURY")"
      provide_to_treasury "$LP0" "$LP1" >/dev/null
      TRE_AFTER="$(cw20_balance "$LP_TOKEN" "$TREASURY")"
      gt "$TRE_AFTER" "$TRE_BEFORE" \
        || die "treasury LP did not increase ($TRE_BEFORE → $TRE_AFTER)"
      echo "  PASS treasury LP $TRE_BEFORE → $TRE_AFTER"
    fi
  fi

  echo "[rebalance] after provide"
  execute_rebalance_swaps
  refresh_pool
  echo "  live price $CUR_PX  target $TARGET_PX  reserves $R0 / $R1"
done

echo ""
echo "[lp sweep] leftover LP on admin → CMM"
ADM_LP="$(cw20_balance "$LP_TOKEN" "$ADMIN_ADDR")"
transfer_lp_if_any "$ADM_LP"
ADM_LP="$(cw20_balance "$LP_TOKEN" "$ADMIN_ADDR")"
TRE_LP1="$(cw20_balance "$LP_TOKEN" "$TREASURY")"
if [[ "$ADM_LP" == "0" ]]; then
  echo "  PASS admin holds no LP"
else
  echo "  FAIL admin still holds LP $ADM_LP" >&2
  FAIL=1
fi
gt "$TRE_LP1" "$TRE_LP0" || {
  echo "  FAIL treasury LP did not increase overall ($TRE_LP0 → $TRE_LP1)" >&2
  FAIL=1
}

echo ""
echo "[verify] final TVL vs \$10000 (\$5k each side)"
refresh_pool
FINAL="$(python3 - "$R0" "$R1" "$DEC_CLUNC" "$DEC_CUSTC" "$LUNC_USD" "$USTC_USD" "$TARGET_PX" "$TOLERANCE" <<'PY'
import json, sys
from decimal import Decimal
r0, r1, d0, d1, lunc, ustc, target, tol = sys.argv[1:]
r0, r1, d0, d1 = int(r0), int(r1), int(d0), int(d1)
lunc, ustc, target, tol = map(Decimal, (lunc, ustc, target, tol))
h0 = Decimal(r0) / (Decimal(10) ** d0)
h1 = Decimal(r1) / (Decimal(10) ** d1)
px = h1 / h0
tvl = h0 * lunc + h1 * ustc
side0 = h0 * lunc
side1 = h1 * ustc
print(json.dumps({
  "price": str(px),
  "tvl_usd": str(tvl),
  "clunc_usd": str(side0),
  "custc_usd": str(side1),
  "price_ok": abs(px - target) / target <= tol,
  "tvl_ok": tvl >= Decimal("9800"),
  "sides_ok": abs(side0 - Decimal(5000)) <= Decimal(250) and abs(side1 - Decimal(5000)) <= Decimal(250),
}))
PY
)"
echo "$FINAL" | jq .
[[ "$(jq -r '.price_ok' <<<"$FINAL")" == "true" ]] || {
  echo "  FAIL final price off peg" >&2
  FAIL=1
}
[[ "$(jq -r '.tvl_ok' <<<"$FINAL")" == "true" ]] || {
  echo "  FAIL final TVL below \$9800" >&2
  FAIL=1
}
[[ "$(jq -r '.sides_ok' <<<"$FINAL")" == "true" ]] || {
  echo "  FAIL sides not within \$250 of \$5k" >&2
  FAIL=1
}

if [[ "$FAIL" -ne 0 ]]; then
  die "post-checks failed (leftover wrap not burned)"
fi

echo ""
if ! burn_admin_wrap; then
  die "post-checks passed but leftover wrap burn failed"
fi
echo "OK — cLUNC/cUSTC LP sent to CMM (rungs $RUNGS); leftover cLUNC/cUSTC burned."
