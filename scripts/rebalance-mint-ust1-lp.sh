#!/usr/bin/env bash
# Rebalance UST1/cUSTC to the USTC oracle, mint $1k LP on UST1/cUSTC and UST1/USTR,
# and send both LP tokens to the CMM treasury.
#
# Flow:
#   1. Prompt terrad keyring passphrase once (or use TERRAD_HOST_KEYRING_PASS).
#   2. 2-of-3 extra-minter mint of UST1 / cUSTC / USTR → admin wallet.
#   3. Admin pool-only swap on UST1/cUSTC until quote/base is within 0.1% of 1/USTC-USD.
#      Does not swap UST1/USTR.
#   4. provide_liquidity on both pairs with receiver = CMM treasury.
#   5. Re-query price + treasury LP balances and fail if checks do not pass.
#
# Usage:
#   DRY_RUN=1 ./scripts/rebalance-mint-ust1-lp.sh
#   UST1_LP_YES=1 ./scripts/rebalance-mint-ust1-lp.sh
#
# Unlock once (non-interactive):
#   read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
#
# Never commit TERRAD_HOST_KEYRING_PASS.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=lib/ust1-lp-rebalance-defaults.sh
source "$SCRIPT_DIR/lib/ust1-lp-rebalance-defaults.sh"
# shellcheck source=lib/terrad-host.sh
source "$SCRIPT_DIR/lib/terrad-host.sh"

TERRAD_HOST_CHAIN_ID="${TERRAD_HOST_CHAIN_ID:-columbus-5}"
TERRAD_HOST_NODE="${TERRAD_HOST_NODE:-https://terra-classic-rpc.publicnode.com:443}"
LCD_URL="${UST1_LP_LCD_URL%/}"
MATH_PY="$SCRIPT_DIR/lib/ust1-lp-rebalance-math.py"
MSIG_KEY="${UST1_LP_MSIG_KEY}"
SIGNER1="${UST1_LP_SIGNER1}"
SIGNER2="${UST1_LP_SIGNER2}"
MSIG_ADDR="${UST1_LP_MSIG_ADDR}"
ADMIN_KEY="${UST1_LP_ADMIN_KEY}"
ADMIN_ADDR="${UST1_LP_ADMIN_ADDR}"
TREASURY="${UST1_LP_TREASURY}"
FACTORY="${UST1_LP_FACTORY}"
UST1="${UST1_LP_UST1}"
CUSTC="${UST1_LP_CUSTC}"
USTR="${UST1_LP_USTR}"
PAIR_CUSTC="${UST1_LP_PAIR_CUSTC}"
PAIR_USTR="${UST1_LP_PAIR_USTR}"
USD_EACH="${UST1_LP_USD_EACH}"
TOLERANCE="${UST1_LP_PRICE_TOLERANCE}"
USTR_PER="${UST1_LP_USTR_PER_USTC}"
SWAP_MAX_SPREAD="${UST1_LP_SWAP_MAX_SPREAD:-0.20}"
PROVIDE_SLIP="${UST1_LP_PROVIDE_SLIPPAGE:-0.005}"
BUFFER_BPS="${UST1_LP_MINT_BUFFER_BPS:-50}"
LCD_TIMEOUT="${UST1_LP_LCD_TIMEOUT:-25}"

die() { echo "ERROR: $*" >&2; exit 1; }

usage() {
  sed -n '2,22p' "$0" | sed 's/^# \{0,1\}//'
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

lcd_b64() {
  if [[ "$(uname)" == Darwin ]]; then
    printf '%s' "$1" | base64 | tr -d '\n'
  else
    printf '%s' "$1" | base64 -w0
  fi
}

lcd_smart() {
  local contract="$1" msg="$2" raw
  raw="$(curl -sS --connect-timeout 10 --max-time "$LCD_TIMEOUT" \
    "${LCD_URL}/cosmwasm/wasm/v1/contract/${contract}/smart/$(lcd_b64 "$msg")")"
  jq -e '(.data != null) and ((.code // 0) == 0)' >/dev/null <<<"$raw" \
    || die "LCD smart query failed for $contract: $msg"
  if [[ "$(jq -r '.data | type' <<<"$raw")" == "string" ]]; then
    jq -r '.data | @base64d | fromjson' <<<"$raw"
  else
    jq '.data' <<<"$raw"
  fi
}

cw20_balance() {
  lcd_smart "$1" "$(jq -nc --arg a "$2" '{balance:{address:$a}}')" | jq -r '.balance // "0"'
}

token_decimals() {
  lcd_smart "$1" '{"token_info":{}}' | jq -r '.decimals'
}

is_minter() {
  local token="$1" who="$2" primary extras
  primary="$(lcd_smart "$token" '{"minter":{}}' | jq -r '.minter // empty')"
  extras="$(lcd_smart "$token" '{"minters":{}}' | jq -r '[.minters[]?] | join(" ")')"
  [[ "$primary" == "$who" || " $extras " == *" $who "* ]]
}

pool_json() { lcd_smart "$1" '{"pool":{}}'; }

pair_lp_token() {
  lcd_smart "$1" '{"pair":{}}' | jq -r '.liquidity_token // empty'
}

asset_amount_for() {
  local pool="$1" token="$2"
  jq -r --arg t "$token" '
    .assets[]
    | select(.info.token.contract_addr == $t)
    | .amount
  ' <<<"$pool"
}

fetch_ustc_usd() {
  if [[ -n "${UST1_LP_USTC_USD:-}" ]]; then
    printf '%s' "$UST1_LP_USTC_USD"
    return 0
  fi
  curl -sS --connect-timeout 10 --max-time 20 "$UST1_LP_INDEXER_ORACLE" \
    | jq -er '.price_usd // empty'
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

broadcast_admin() {
  local label="$1"
  shift
  local out tx_hash
  echo "  → $label" >&2
  TERRAD_HOST_KEY="$ADMIN_KEY"
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

broadcast_msig() {
  local label="$1" contract="$2" msg="$3"
  local workdir unsigned sig1 sig2 signed out tx_hash code
  echo "  → msig $label" >&2
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "    [DRY_RUN] wasm execute $contract $msg" >&2
    echo "DRY_RUN_TX"
    return 0
  fi
  workdir="$(mktemp -d "${TMPDIR:-/tmp}/ust1-lp-msig.XXXXXX")"
  unsigned="$workdir/unsigned.json"
  sig1="$workdir/sig1.json"
  sig2="$workdir/sig2.json"
  signed="$workdir/signed.json"
  # shellcheck disable=SC2046
  terrad_host_exec tx wasm execute "$contract" "$msg" \
    --from "$MSIG_KEY" \
    --generate-only \
    $(terrad_host_common_flags) \
    $(terrad_host_gas_flags) \
    $(terrad_host_fee_flags) \
    --output json >"$unsigned"
  jq -e '.body.messages | length > 0' "$unsigned" >/dev/null \
    || die "unsigned mint tx invalid ($label)"
  # shellcheck disable=SC2046
  terrad_host_exec tx sign "$unsigned" \
    --from "$SIGNER1" --multisig "$MSIG_KEY" --sign-mode amino-json \
    $(terrad_host_common_flags) --output json >"$sig1"
  # shellcheck disable=SC2046
  terrad_host_exec tx sign "$unsigned" \
    --from "$SIGNER2" --multisig "$MSIG_KEY" --sign-mode amino-json \
    $(terrad_host_common_flags) --output json >"$sig2"
  # shellcheck disable=SC2046
  terrad_host_exec tx multisign "$unsigned" "$MSIG_KEY" "$sig1" "$sig2" \
    $(terrad_host_common_flags) --output json >"$signed"
  # shellcheck disable=SC2046
  out="$(terrad_host_exec tx broadcast "$signed" \
    $(terrad_host_common_flags) \
    --broadcast-mode "${TERRAD_HOST_BROADCAST_MODE:-sync}" \
    -y --output json)"
  rm -rf "$workdir"
  tx_hash="$(printf '%s' "$out" | jq -r '.txhash // empty')"
  code="$(printf '%s' "$out" | jq -r '.code // 0')"
  [[ -n "$tx_hash" && "$code" == "0" ]] || {
    echo "ERROR: msig broadcast failed ($label)" >&2
    printf '%s\n' "$out" >&2
    exit 1
  }
  echo "    tx: $tx_hash" >&2
  terrad_host_wait_tx_inclusion "$tx_hash"
  printf '%s' "$tx_hash"
}

mint_if_needed() {
  local sym="$1" token="$2" amount="$3"
  if [[ "$amount" == "0" ]]; then
    echo "  skip mint $sym (admin already funded)"
    return 0
  fi
  local msg
  msg="$(jq -nc --arg r "$ADMIN_ADDR" --arg a "$amount" '{mint:{recipient:$r,amount:$a}}')"
  broadcast_msig "mint $sym $amount → $ADMIN_ADDR" "$token" "$msg" >/dev/null
}

cw20_send_swap() {
  local token="$1" pair="$2" amount="$3"
  local hook b64 msg
  hook="$(jq -nc --arg s "$SWAP_MAX_SPREAD" \
    '{swap:{belief_price:null,max_spread:$s,to:null,deadline:null,hybrid:null,trader:null}}')"
  if [[ "$(uname)" == Darwin ]]; then
    b64="$(printf '%s' "$hook" | base64 | tr -d '\n')"
  else
    b64="$(printf '%s' "$hook" | base64 -w0)"
  fi
  msg="$(jq -nc --arg c "$pair" --arg a "$amount" --arg m "$b64" \
    '{send:{contract:$c,amount:$a,msg:$m}}')"
  broadcast_admin "swap $amount on UST1/cUSTC (pool-only)" wasm execute "$token" "$msg"
}

provide_to_treasury() {
  local label="$1" pair="$2" t0="$3" a0="$4" t1="$5" a1="$6"
  local allow0 allow1 provide
  allow0="$(jq -nc --arg s "$pair" --arg a "$a0" \
    '{increase_allowance:{spender:$s,amount:$a,expires:{never:{}}}}')"
  allow1="$(jq -nc --arg s "$pair" --arg a "$a1" \
    '{increase_allowance:{spender:$s,amount:$a,expires:{never:{}}}}')"
  provide="$(jq -nc --arg a "$t0" --arg b "$t1" --arg aa "$a0" --arg bb "$a1" \
    --arg recv "$TREASURY" --arg slip "$PROVIDE_SLIP" \
    '{provide_liquidity:{assets:[
      {info:{token:{contract_addr:$a}},amount:$aa},
      {info:{token:{contract_addr:$b}},amount:$bb}
    ],slippage_tolerance:$slip,receiver:$recv,deadline:null}}')"
  broadcast_admin "increase_allowance $label token0" wasm execute "$t0" "$allow0" >/dev/null
  broadcast_admin "increase_allowance $label token1" wasm execute "$t1" "$allow1" >/dev/null
  broadcast_admin "provide_liquidity $label → treasury" wasm execute "$pair" "$provide"
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

echo "=============================================="
echo "Rebalance + mint UST1 LP → CMM"
echo "=============================================="
echo "Factory:   $FACTORY"
echo "Treasury:  $TREASURY"
echo "Admin key: $ADMIN_KEY"
echo "Multisig:  $MSIG_KEY ($MSIG_ADDR)"
echo "USD each:  $USD_EACH"
echo "Tolerance: $TOLERANCE"
echo "DRY_RUN:   ${DRY_RUN:-0}"
echo ""

command -v jq >/dev/null || die "jq required"
command -v python3 >/dev/null || die "python3 required"
command -v curl >/dev/null || die "curl required"
[[ -f "$MATH_PY" ]] || die "missing $MATH_PY"
python3 "$MATH_PY" --self-test
if [[ "${DRY_RUN:-0}" != "1" ]]; then
  command -v terrad >/dev/null || die "terrad required on PATH"
fi

USTC_USD="$(fetch_ustc_usd)" || die "oracle USTC/USD unavailable (set UST1_LP_USTC_USD)"
[[ -n "$USTC_USD" ]] || die "empty USTC/USD"
echo "[oracle] USTC/USD=$USTC_USD  (USTR = ${USTR_PER} × USTC)"

echo "[preflight] tokens + extra minter $MSIG_ADDR"
for tok_sym in "$UST1:UST1" "$CUSTC:cUSTC" "$USTR:USTR"; do
  tok="${tok_sym%%:*}"
  sym="${tok_sym##*:}"
  info="$(lcd_smart "$tok" '{"token_info":{}}')"
  got="$(jq -r '.symbol' <<<"$info")"
  echo "  $sym symbol=$got decimals=$(jq -r '.decimals' <<<"$info")"
  [[ "$(printf '%s' "$got" | tr '[:upper:]' '[:lower:]')" == "$(printf '%s' "$sym" | tr '[:upper:]' '[:lower:]')" ]] \
    || die "$tok symbol $got != $sym"
  is_minter "$tok" "$MSIG_ADDR" || die "$sym minter/minters does not include DEX 2-of-3 $MSIG_ADDR"
done

DEC_UST1="$(token_decimals "$UST1")"
DEC_CUSTC="$(token_decimals "$CUSTC")"
DEC_USTR="$(token_decimals "$USTR")"

POOL_C="$(pool_json "$PAIR_CUSTC")"
POOL_U="$(pool_json "$PAIR_USTR")"
R0="$(asset_amount_for "$POOL_C" "$UST1")"
R1="$(asset_amount_for "$POOL_C" "$CUSTC")"
U0="$(asset_amount_for "$POOL_U" "$UST1")"
U1="$(asset_amount_for "$POOL_U" "$USTR")"
[[ -n "$R0" && -n "$R1" && -n "$U0" && -n "$U1" ]] || die "failed to read pool reserves"
FEE_BPS="$(lcd_smart "$PAIR_CUSTC" '{"get_fee_config":{}}' | jq -r '.fee_config.fee_bps // .fee_bps')"
LP_CUSTC="$(pair_lp_token "$PAIR_CUSTC")"
LP_USTR="$(pair_lp_token "$PAIR_USTR")"
[[ -n "$LP_CUSTC" && -n "$LP_USTR" ]] || die "missing LP token addresses"

echo "  UST1/cUSTC $PAIR_CUSTC  reserves $R0 / $R1  fee_bps=$FEE_BPS"
echo "  UST1/USTR  $PAIR_USTR  reserves $U0 / $U1"
echo "  LP tokens  $LP_CUSTC  |  $LP_USTR"

if [[ "${DRY_RUN:-0}" != "1" ]]; then
  prompt_keyring_pass
  TERRAD_HOST_KEY="$ADMIN_KEY"
  TERRAD_HOST_EXPECTED_ADDR="$ADMIN_ADDR"
  GOT_ADMIN="$(terrad_host_key_address)" || exit 1
  [[ "$GOT_ADMIN" == "$ADMIN_ADDR" ]] || die "admin key $ADMIN_KEY is $GOT_ADMIN (want $ADMIN_ADDR)"
else
  echo "  [DRY_RUN] skipping key unlock; assuming admin $ADMIN_ADDR"
fi

BAL_UST1="$(cw20_balance "$UST1" "$ADMIN_ADDR")"
BAL_CUSTC="$(cw20_balance "$CUSTC" "$ADMIN_ADDR")"
BAL_USTR="$(cw20_balance "$USTR" "$ADMIN_ADDR")"
TRE_LP_C0="$(cw20_balance "$LP_CUSTC" "$TREASURY")"
TRE_LP_U0="$(cw20_balance "$LP_USTR" "$TREASURY")"
echo "  admin balances UST1=$BAL_UST1 cUSTC=$BAL_CUSTC USTR=$BAL_USTR"
echo "  treasury LP    UST1-CUST=$TRE_LP_C0 UST1-USTR=$TRE_LP_U0"

PLAN="$(python3 "$MATH_PY" <<EOF
{
  "ustc_usd": "$USTC_USD",
  "ustr_per_ustc": "$USTR_PER",
  "usd_each": "$USD_EACH",
  "tolerance": "$TOLERANCE",
  "fee_bps": $FEE_BPS,
  "buffer_bps": $BUFFER_BPS,
  "dec_ust1": $DEC_UST1,
  "dec_custc": $DEC_CUSTC,
  "dec_ustr": $DEC_USTR,
  "custc_r0": "$R0",
  "custc_r1": "$R1",
  "ustr_r0": "$U0",
  "ustr_r1": "$U1",
  "bal_ust1": "$BAL_UST1",
  "bal_custc": "$BAL_CUSTC",
  "bal_ustr": "$BAL_USTR"
}
EOF
)"

echo ""
echo "[plan]"
jq '{
  ustc_usd,
  target_custc_per_ust1,
  current_custc_per_ust1,
  current_custc_rel_error,
  already_on_peg,
  swap,
  lp_custc,
  lp_ustr,
  mint
}' <<<"$PLAN"

TARGET="$(jq -r '.target_custc_per_ust1' <<<"$PLAN")"
MINT_UST1="$(jq -r '.mint.ust1' <<<"$PLAN")"
MINT_CUSTC="$(jq -r '.mint.custc' <<<"$PLAN")"
MINT_USTR="$(jq -r '.mint.ustr' <<<"$PLAN")"
SWAP_NEEDED="$(jq -r '.swap.needed' <<<"$PLAN")"
SWAP_TOKEN="$(jq -r '.swap.offer_token // empty' <<<"$PLAN")"
SWAP_AMT="$(jq -r '.swap.offer_amount' <<<"$PLAN")"

if [[ "${DRY_RUN:-0}" == "1" ]]; then
  echo ""
  echo "DRY_RUN complete (no txs). Re-run without DRY_RUN=1 to broadcast."
  exit 0
fi

if [[ "${UST1_LP_YES:-0}" != "1" ]]; then
  if [[ ! -t 0 ]]; then
    die "refusing live broadcast without TTY; set UST1_LP_YES=1"
  fi
  read -r -p "Broadcast mint / UST1-cUSTC swap / LP-to-treasury on $TERRAD_HOST_CHAIN_ID? [y/N] " ans
  [[ "$ans" == "y" || "$ans" == "Y" ]] || die "aborted"
fi

echo ""
echo "[mint] 2-of-3 → $ADMIN_ADDR"
mint_if_needed UST1 "$UST1" "$MINT_UST1"
mint_if_needed cUSTC "$CUSTC" "$MINT_CUSTC"
mint_if_needed USTR "$USTR" "$MINT_USTR"

BAL_UST1="$(cw20_balance "$UST1" "$ADMIN_ADDR")"
BAL_CUSTC="$(cw20_balance "$CUSTC" "$ADMIN_ADDR")"
BAL_USTR="$(cw20_balance "$USTR" "$ADMIN_ADDR")"
echo "  post-mint admin UST1=$BAL_UST1 cUSTC=$BAL_CUSTC USTR=$BAL_USTR"

echo ""
echo "[rebalance] UST1/cUSTC only"
SWAP_TX=""
if [[ "$SWAP_NEEDED" == "true" ]]; then
  if [[ "$SWAP_TOKEN" == "ust1" ]]; then
    [[ "$(python3 -c "print(int('$BAL_UST1') >= int('$SWAP_AMT'))")" == "True" ]] \
      || die "admin UST1 $BAL_UST1 < swap $SWAP_AMT"
    SWAP_TX="$(cw20_send_swap "$UST1" "$PAIR_CUSTC" "$SWAP_AMT")"
  elif [[ "$SWAP_TOKEN" == "custc" ]]; then
    [[ "$(python3 -c "print(int('$BAL_CUSTC') >= int('$SWAP_AMT'))")" == "True" ]] \
      || die "admin cUSTC $BAL_CUSTC < swap $SWAP_AMT"
    SWAP_TX="$(cw20_send_swap "$CUSTC" "$PAIR_CUSTC" "$SWAP_AMT")"
  else
    die "unknown swap token $SWAP_TOKEN"
  fi
else
  echo "  already within $TOLERANCE of target; skipping swap"
fi

POOL_C="$(pool_json "$PAIR_CUSTC")"
R0="$(asset_amount_for "$POOL_C" "$UST1")"
R1="$(asset_amount_for "$POOL_C" "$CUSTC")"
CUR_PX="$(human_px "$R0" "$R1" "$DEC_UST1" "$DEC_CUSTC")"
echo "  post-swap price $CUR_PX  target $TARGET"
rel_err_ok "$CUR_PX" "$TARGET" || die "UST1/cUSTC price $CUR_PX not within $TOLERANCE of $TARGET"

# Re-size LP against live reserves + live admin balances.
BAL_UST1="$(cw20_balance "$UST1" "$ADMIN_ADDR")"
BAL_CUSTC="$(cw20_balance "$CUSTC" "$ADMIN_ADDR")"
BAL_USTR="$(cw20_balance "$USTR" "$ADMIN_ADDR")"
POOL_U="$(pool_json "$PAIR_USTR")"
U0="$(asset_amount_for "$POOL_U" "$UST1")"
U1="$(asset_amount_for "$POOL_U" "$USTR")"
LIVE="$(python3 "$MATH_PY" <<EOF
{
  "ustc_usd": "$USTC_USD",
  "ustr_per_ustc": "$USTR_PER",
  "usd_each": "$USD_EACH",
  "tolerance": "$TOLERANCE",
  "fee_bps": $FEE_BPS,
  "buffer_bps": 0,
  "dec_ust1": $DEC_UST1,
  "dec_custc": $DEC_CUSTC,
  "dec_ustr": $DEC_USTR,
  "custc_r0": "$R0",
  "custc_r1": "$R1",
  "ustr_r0": "$U0",
  "ustr_r1": "$U1",
  "bal_ust1": "$BAL_UST1",
  "bal_custc": "$BAL_CUSTC",
  "bal_ustr": "$BAL_USTR"
}
EOF
)"
LP_C_UST1="$(jq -r '.lp_custc.ust1' <<<"$LIVE")"
LP_C_CUSTC="$(jq -r '.lp_custc.custc' <<<"$LIVE")"
LP_U_UST1="$(jq -r '.lp_ustr.ust1' <<<"$LIVE")"
LP_U_USTR="$(jq -r '.lp_ustr.ustr' <<<"$LIVE")"
NEED_UST1="$(python3 -c "print(int('$LP_C_UST1') + int('$LP_U_UST1'))")"
[[ "$(python3 -c "print(int('$BAL_UST1') >= int('$NEED_UST1') and int('$BAL_CUSTC') >= int('$LP_C_CUSTC') and int('$BAL_USTR') >= int('$LP_U_USTR'))")" == "True" ]] \
  || die "admin inventory short for LP (UST1 $BAL_UST1 need $NEED_UST1; cUSTC $BAL_CUSTC need $LP_C_CUSTC; USTR $BAL_USTR need $LP_U_USTR)"

echo ""
echo "[provide] $USD_EACH USD each → $TREASURY"
TX_LP_C="$(provide_to_treasury "UST1/cUSTC" "$PAIR_CUSTC" "$UST1" "$LP_C_UST1" "$CUSTC" "$LP_C_CUSTC")"
TX_LP_U="$(provide_to_treasury "UST1/USTR" "$PAIR_USTR" "$UST1" "$LP_U_UST1" "$USTR" "$LP_U_USTR")"

echo ""
echo "[verify]"
FAIL=0
POOL_C="$(pool_json "$PAIR_CUSTC")"
R0="$(asset_amount_for "$POOL_C" "$UST1")"
R1="$(asset_amount_for "$POOL_C" "$CUSTC")"
CUR_PX="$(human_px "$R0" "$R1" "$DEC_UST1" "$DEC_CUSTC")"
if rel_err_ok "$CUR_PX" "$TARGET"; then
  echo "  PASS price $CUR_PX within $TOLERANCE of $TARGET"
else
  echo "  FAIL price $CUR_PX vs target $TARGET (tol $TOLERANCE)" >&2
  FAIL=1
fi

TRE_LP_C1="$(cw20_balance "$LP_CUSTC" "$TREASURY")"
TRE_LP_U1="$(cw20_balance "$LP_USTR" "$TREASURY")"
ADM_LP_C="$(cw20_balance "$LP_CUSTC" "$ADMIN_ADDR")"
ADM_LP_U="$(cw20_balance "$LP_USTR" "$ADMIN_ADDR")"

if python3 -c "import sys; sys.exit(0 if int('$TRE_LP_C1') > int('$TRE_LP_C0') else 1)"; then
  echo "  PASS treasury UST1-CUST-LP $TRE_LP_C0 → $TRE_LP_C1"
else
  echo "  FAIL treasury UST1-CUST-LP did not increase ($TRE_LP_C0 → $TRE_LP_C1)" >&2
  FAIL=1
fi
if python3 -c "import sys; sys.exit(0 if int('$TRE_LP_U1') > int('$TRE_LP_U0') else 1)"; then
  echo "  PASS treasury UST1-USTR-LP $TRE_LP_U0 → $TRE_LP_U1"
else
  echo "  FAIL treasury UST1-USTR-LP did not increase ($TRE_LP_U0 → $TRE_LP_U1)" >&2
  FAIL=1
fi
if [[ "$ADM_LP_C" == "0" && "$ADM_LP_U" == "0" ]]; then
  echo "  PASS admin holds no leftover LP"
else
  echo "  FAIL admin still holds LP CUST=$ADM_LP_C USTR=$ADM_LP_U" >&2
  FAIL=1
fi

echo ""
echo "swap_tx=${SWAP_TX:-skipped}"
echo "lp_custc_tx=$TX_LP_C"
echo "lp_ustr_tx=$TX_LP_U"
echo "treasury=$TREASURY"

if [[ "$FAIL" -ne 0 ]]; then
  die "post-checks failed"
fi
echo "OK — rebalance + \$1k LP on both pairs sent to CMM."
