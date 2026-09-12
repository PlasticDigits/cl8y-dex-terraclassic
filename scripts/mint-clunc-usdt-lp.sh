#!/usr/bin/env bash
# Create a new columbus-5 cLUNC/USDT pair and seed $1k + $1k.
#
#   1. Fetch indexer LUNC/USD (cLUNC is 1:1 wrap).
#   2. Mint $1k of cLUNC to cl8y2_admin (primary minter). Never mint USDT.
#   3. Factory CreatePair if missing (100 LUNC fee; inherits discount registry).
#   4. provide_liquidity $1k cLUNC + $1k USDT (USDT already on cl8y2_admin).
#
# USDT is 18 decimals. All amount compares go through Python — bash -lt overflows.
#
# Usage:
#   DRY_RUN=1 ./scripts/mint-clunc-usdt-lp.sh
#   CLUNC_USDT_YES=1 ./scripts/mint-clunc-usdt-lp.sh
#   CLUNC_USDT_USD_EACH=1000                 # default, each side
#   CLUNC_USDT_LP_RECEIVER=$ADMIN_ADDR         # keep LP on ops wallet (default CMM)
#
# Unlock once (non-interactive):
#   read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
#
# Never commit TERRAD_HOST_KEYRING_PASS.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=lib/clunc-usdt-lp-defaults.sh
source "$SCRIPT_DIR/lib/clunc-usdt-lp-defaults.sh"
# shellcheck source=lib/terrad-host.sh
source "$SCRIPT_DIR/lib/terrad-host.sh"

TERRAD_HOST_CHAIN_ID="${TERRAD_HOST_CHAIN_ID:-columbus-5}"
TERRAD_HOST_NODE="${TERRAD_HOST_NODE:-https://terra-classic-rpc.publicnode.com:443}"
TERRAD_HOST_KEY="${CLUNC_USDT_ADMIN_KEY}"
TERRAD_HOST_EXPECTED_ADDR="${CLUNC_USDT_ADMIN_ADDR}"

LCD_URL="${CLUNC_USDT_LCD_URL%/}"
MATH_PY="$SCRIPT_DIR/lib/clunc-usdt-lp-math.py"
INDEXER_ORACLE="${CLUNC_USDT_INDEXER_ORACLE}"
FACTORY="${CLUNC_USDT_FACTORY}"
CLUNC="${CLUNC_USDT_CLUNC}"
USDT="${CLUNC_USDT_USDT}"
ADMIN_KEY="${CLUNC_USDT_ADMIN_KEY}"
ADMIN_ADDR="${CLUNC_USDT_ADMIN_ADDR}"
USD_EACH="${CLUNC_USDT_USD_EACH}"
USDT_USD="${CLUNC_USDT_USDT_USD}"
MAX_AGE="${CLUNC_USDT_ORACLE_MAX_AGE_SEC}"
MOVE_TOL="${CLUNC_USDT_PRICE_MOVE_TOLERANCE}"
EXPECTED_CODE="${CLUNC_USDT_EXPECTED_CW20_CODE_ID}"
GAS_RESERVE="${CLUNC_USDT_GAS_RESERVE_ULUNA}"
LP_RECEIVER="${CLUNC_USDT_LP_RECEIVER}"
LCD_TIMEOUT="${CLUNC_USDT_LCD_TIMEOUT:-25}"
HTTP_UA="cl8y-dex-ops/clunc-usdt-lp (+https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic)"
DEPLOY_DIR="$REPO_ROOT/${CLUNC_USDT_DEPLOY_DIR_REL}"
ENV_OUT="${CLUNC_USDT_ENV_OUT:-$DEPLOY_DIR/addresses.env}"

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

lcd_get() {
  curl -sS --connect-timeout 10 --max-time "$LCD_TIMEOUT" -H "User-Agent: $HTTP_UA" "$1"
}

lcd_smart_raw() {
  local contract="$1" msg="$2"
  lcd_get "${LCD_URL}/cosmwasm/wasm/v1/contract/${contract}/smart/$(lcd_b64 "$msg")"
}

lcd_smart() {
  local contract="$1" msg="$2" raw
  raw="$(lcd_smart_raw "$contract" "$msg")"
  jq -e '(.data != null) and ((.code // 0) == 0)' >/dev/null <<<"$raw" \
    || die "LCD smart query failed for $contract: $msg — $raw"
  if [[ "$(jq -r '.data | type' <<<"$raw")" == "string" ]]; then
    jq -r '.data | @base64d | fromjson' <<<"$raw"
  else
    jq '.data' <<<"$raw"
  fi
}

lcd_bank() {
  local addr="$1" denom="$2"
  lcd_get "${LCD_URL}/cosmos/bank/v1beta1/balances/${addr}/by_denom?denom=${denom}" \
    | jq -r '.balance.amount // "0"'
}

contract_code_id() {
  lcd_get "${LCD_URL}/cosmwasm/wasm/v1/contract/${1}" | jq -r '.contract_info.code_id // empty'
}

cw20_balance() {
  lcd_smart "$1" "$(jq -nc --arg a "$2" '{balance:{address:$a}}')" | jq -r '.balance // "0"'
}

token_info() { lcd_smart "$1" '{"token_info":{}}'; }

is_minter() {
  local token="$1" who="$2" primary extras raw
  primary="$(lcd_smart "$token" '{"minter":{}}' | jq -r '.minter // empty')"
  extras=""
  set +e
  raw="$(lcd_smart_raw "$token" '{"minters":{}}' 2>/dev/null)"
  set -e
  if echo "$raw" | jq -e '.data' >/dev/null 2>&1; then
    if [[ "$(jq -r '.data | type' <<<"$raw")" == "string" ]]; then
      extras="$(jq -r '.data | @base64d | fromjson | [.minters[]?] | join(" ")' <<<"$raw" 2>/dev/null || echo "")"
    else
      extras="$(jq -r '[.data.minters[]?] | join(" ")' <<<"$raw" 2>/dev/null || echo "")"
    fi
  fi
  [[ "$primary" == "$who" || " $extras " == *" $who "* ]]
}

code_whitelisted() {
  local code_id="$1"
  lcd_smart "$FACTORY" "$(jq -nc --argjson id "$code_id" '{is_code_id_whitelisted:{code_id:$id}}')" \
    | jq -r '.whitelisted // false'
}

factory_pair_fee() {
  lcd_smart "$FACTORY" '{"config":{}}' | jq -r '.pair_creation_fee_uluna // "0"'
}

factory_pair_addr() {
  local a="$1" b="$2" raw data
  local q
  q="$(jq -nc --arg a "$a" --arg b "$b" \
    '{pair:{asset_infos:[{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')"
  set +e
  raw="$(lcd_smart_raw "$FACTORY" "$q" 2>/dev/null)"
  set -e
  if echo "$raw" | grep -qi 'pair not found'; then
    printf ''
    return 0
  fi
  if echo "$raw" | jq -e '.data' >/dev/null 2>&1; then
    if [[ "$(jq -r '.data | type' <<<"$raw")" == "string" ]]; then
      data="$(jq -r '.data | @base64d | fromjson' <<<"$raw")"
    else
      data="$(jq '.data' <<<"$raw")"
    fi
    jq -r '.pair.contract_addr // .contract_addr // .pair.pair.contract_addr // empty' <<<"$data"
    return 0
  fi
  printf ''
}

pair_pool() { lcd_smart "$1" '{"pool":{}}'; }

pair_total_share() {
  pair_pool "$1" | jq -r '.total_share // "0"'
}

pair_lp_token() {
  lcd_smart "$1" '{"pair":{}}' | jq -r '.liquidity_token // .liquidity_token.address // empty'
}

py_ge() {
  python3 -c "import sys; sys.exit(0 if int(sys.argv[1]) >= int(sys.argv[2]) else 1)" "$1" "$2"
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
  TERRAD_HOST_EXPECTED_ADDR="$ADMIN_ADDR"
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

fetch_lunc_oracle() {
  local raw parsed tries="${CLUNC_USDT_ORACLE_RETRIES:-4}" i
  if [[ -n "${CLUNC_USDT_LUNC_USD:-}" ]]; then
    python3 "$MATH_PY" --parse-oracle --max-age 999999999 <<EOF
{"ticker":"lunc","price_usd":"${CLUNC_USDT_LUNC_USD}","sources":[{"source":"average","fetched_at":"$(date -u +%Y-%m-%dT%H:%M:%S+00:00)"}]}
EOF
    return 0
  fi
  for ((i = 1; i <= tries; i++)); do
    raw="$(curl -sS --connect-timeout 10 --max-time 20 -H "User-Agent: $HTTP_UA" "$INDEXER_ORACLE" || true)"
    parsed="$(printf '%s' "$raw" | python3 "$MATH_PY" --parse-oracle --max-age "$MAX_AGE" || true)"
    if jq -e '.ok == true' >/dev/null 2>&1 <<<"$parsed"; then
      printf '%s' "$parsed"
      return 0
    fi
    echo "  oracle fetch attempt $i/$tries failed: $(jq -c '.errors // []' <<<"$parsed" 2>/dev/null || echo "$parsed")" >&2
    if [[ "$i" -lt "$tries" ]]; then
      sleep $((i * 3))
    fi
  done
  echo "$parsed" >&2
  return 1
}

size_legs() {
  local lunc_usd="$1" bal_clunc="$2" bal_usdt="$3"
  python3 "$MATH_PY" <<EOF
{
  "mode": "size",
  "usd_each": "$USD_EACH",
  "lunc_usd": "$lunc_usd",
  "usdt_usd": "$USDT_USD",
  "dec_clunc": $DEC_CLUNC,
  "dec_usdt": $DEC_USDT,
  "bal_clunc": "$bal_clunc",
  "bal_usdt": "$bal_usdt"
}
EOF
}

echo "=============================================="
echo "cLUNC/USDT new pair — \$${USD_EACH}+\$${USD_EACH} seed"
echo "=============================================="
echo "Factory:   $FACTORY"
echo "cLUNC:     $CLUNC"
echo "USDT:      $USDT"
echo "Admin:     $ADMIN_KEY ($ADMIN_ADDR)"
echo "LP recv:   $LP_RECEIVER"
echo "USDT/\$:    $USDT_USD"
echo "DRY_RUN:   ${DRY_RUN:-0}"
echo ""

echo "[preflight] token_info + code IDs + whitelist + minter"
CLUNC_INFO="$(token_info "$CLUNC")"
USDT_INFO="$(token_info "$USDT")"
CLUNC_SYM="$(jq -r '.symbol' <<<"$CLUNC_INFO")"
USDT_SYM="$(jq -r '.symbol' <<<"$USDT_INFO")"
DEC_CLUNC="$(jq -r '.decimals' <<<"$CLUNC_INFO")"
DEC_USDT="$(jq -r '.decimals' <<<"$USDT_INFO")"
CLUNC_CODE="$(contract_code_id "$CLUNC")"
USDT_CODE="$(contract_code_id "$USDT")"
echo "  cLUNC symbol=$CLUNC_SYM decimals=$DEC_CLUNC code_id=$CLUNC_CODE"
echo "  USDT  symbol=$USDT_SYM decimals=$DEC_USDT code_id=$USDT_CODE"
[[ "$CLUNC_SYM" == "cLUNC" ]] || die "cLUNC on-chain symbol=$CLUNC_SYM"
[[ "$USDT_SYM" == "USDT" ]] || die "USDT on-chain symbol=$USDT_SYM"
[[ "$DEC_CLUNC" == "6" ]] || die "cLUNC decimals=$DEC_CLUNC (want 6)"
[[ "$DEC_USDT" == "18" ]] || die "USDT decimals=$DEC_USDT (want 18)"
[[ "$CLUNC_CODE" == "$EXPECTED_CODE" ]] || die "cLUNC code_id=$CLUNC_CODE expected $EXPECTED_CODE"
[[ "$USDT_CODE" == "$EXPECTED_CODE" ]] || die "USDT code_id=$USDT_CODE expected $EXPECTED_CODE"
[[ "$(code_whitelisted "$CLUNC_CODE")" == "true" ]] || die "cLUNC code_id $CLUNC_CODE not factory-whitelisted"
[[ "$(code_whitelisted "$USDT_CODE")" == "true" ]] || die "USDT code_id $USDT_CODE not factory-whitelisted"
is_minter "$CLUNC" "$ADMIN_ADDR" || die "$ADMIN_ADDR is not a cLUNC minter (cannot mint wrap)"
if is_minter "$USDT" "$ADMIN_ADDR"; then
  die "refusing to run: $ADMIN_ADDR is a USDT minter — this script must never mint USDT"
fi
echo "  cLUNC minter OK; USDT minter is not admin (will not mint USDT)"

FEE_ULUNA="$(factory_pair_fee)"
echo "  pair_creation_fee_uluna=$FEE_ULUNA"

EXISTING="$(factory_pair_addr "$CLUNC" "$USDT" || true)"
if [[ -z "$EXISTING" ]]; then
  EXISTING="$(factory_pair_addr "$USDT" "$CLUNC" || true)"
fi
if [[ -n "$EXISTING" ]]; then
  SHARE="$(pair_total_share "$EXISTING")"
  echo "  existing factory pair: $EXISTING (total_share=$SHARE)"
  if [[ "$SHARE" != "0" && -n "$SHARE" ]]; then
    die "cLUNC/USDT already has liquidity (pair $EXISTING share=$SHARE). This script seeds a new empty pool only."
  fi
else
  echo "  no factory pair yet for cLUNC/USDT"
fi

if [[ "${DRY_RUN:-0}" != "1" ]]; then
  prompt_keyring_pass
  GOT="$(terrad_host_key_address)" || exit 1
  [[ "$GOT" == "$ADMIN_ADDR" ]] || die "key $ADMIN_KEY is $GOT (want $ADMIN_ADDR)"
else
  echo "  [DRY_RUN] skipping key unlock; assuming $ADMIN_ADDR"
fi

HAVE_ULUNA="$(lcd_bank "$ADMIN_ADDR" uluna)"
BAL_CLUNC="$(cw20_balance "$CLUNC" "$ADMIN_ADDR")"
BAL_USDT="$(cw20_balance "$USDT" "$ADMIN_ADDR")"
echo "  balances: uluna=$HAVE_ULUNA cLUNC=$BAL_CLUNC USDT=$BAL_USDT"

echo ""
echo "[oracle] $INDEXER_ORACLE"
ORACLE_JSON="$(fetch_lunc_oracle)" || die "LUNC/USD oracle failed"
LUNC_USD="$(jq -r '.lunc_usd' <<<"$ORACLE_JSON")"
echo "  LUNC/USD=$LUNC_USD  age=$(jq -r '.age_sec // "n/a"' <<<"$ORACLE_JSON")s"

PLAN="$(size_legs "$LUNC_USD" "$BAL_CLUNC" "$BAL_USDT")"
echo "$PLAN" | jq '{ok, usd_each, tvl_usd, lunc_usd, clunc_human, usdt_human, clunc_raw, usdt_raw, mint_clunc, usdt_short, errors}'
jq -e '.ok == true' >/dev/null <<<"$PLAN" || die "size failed: $(jq -c '.errors' <<<"$PLAN")"

CLUNC_RAW="$(jq -r '.clunc_raw' <<<"$PLAN")"
USDT_RAW="$(jq -r '.usdt_raw' <<<"$PLAN")"
MINT_CLUNC="$(jq -r '.mint_clunc' <<<"$PLAN")"

NEED_ULUNA="$FEE_ULUNA"
if [[ -n "$EXISTING" ]]; then
  NEED_ULUNA="0"
fi
py_ge "$HAVE_ULUNA" "$(python3 -c "print(int('$NEED_ULUNA') + int('$GAS_RESERVE'))")" \
  || die "admin uluna $HAVE_ULUNA < pair fee $NEED_ULUNA + gas reserve $GAS_RESERVE"

echo ""
echo "[plan]"
echo "  1. mint $MINT_CLUNC raw cLUNC (\$$USD_EACH @ $LUNC_USD) → $ADMIN_ADDR"
echo "  2. create_pair cLUNC+USDT on $FACTORY (fee ${FEE_ULUNA}uluna) if missing"
echo "  3. increase_allowance ×2 + provide_liquidity"
echo "     cLUNC $CLUNC_RAW  +  USDT $USDT_RAW"
echo "     receiver=$LP_RECEIVER"
echo "  4. write $ENV_OUT"
echo ""

if [[ "${DRY_RUN:-0}" == "1" ]]; then
  echo "DRY_RUN complete (no txs). Re-run without DRY_RUN=1 to broadcast."
  exit 0
fi

if [[ "${CLUNC_USDT_YES:-0}" != "1" ]]; then
  if [[ ! -t 0 ]]; then
    die "refusing live broadcast without TTY; set CLUNC_USDT_YES=1"
  fi
  read -r -p "Broadcast mint + create_pair + \$${USD_EACH}+\$${USD_EACH} cLUNC/USDT LP on $TERRAD_HOST_CHAIN_ID? [y/N] " ans
  [[ "$ans" == "y" || "$ans" == "Y" ]] || die "aborted"
fi

echo ""
echo "[oracle refresh before mint]"
ORACLE_JSON="$(fetch_lunc_oracle)" || die "LUNC/USD oracle refresh failed"
LUNC_USD_NEW="$(jq -r '.lunc_usd' <<<"$ORACLE_JSON")"
MOVED="$(python3 "$MATH_PY" <<EOF
{"mode":"price-moved","old":"$LUNC_USD","new":"$LUNC_USD_NEW","tolerance":"$MOVE_TOL"}
EOF
)"
if jq -e '.moved == true' >/dev/null <<<"$MOVED"; then
  die "LUNC/USD moved $LUNC_USD → $LUNC_USD_NEW (tolerance $MOVE_TOL). Re-run DRY_RUN."
fi
LUNC_USD="$LUNC_USD_NEW"
BAL_CLUNC="$(cw20_balance "$CLUNC" "$ADMIN_ADDR")"
BAL_USDT="$(cw20_balance "$USDT" "$ADMIN_ADDR")"
PLAN="$(size_legs "$LUNC_USD" "$BAL_CLUNC" "$BAL_USDT")"
jq -e '.ok == true' >/dev/null <<<"$PLAN" || die "size failed after refresh: $(jq -c '.errors' <<<"$PLAN")"
CLUNC_RAW="$(jq -r '.clunc_raw' <<<"$PLAN")"
USDT_RAW="$(jq -r '.usdt_raw' <<<"$PLAN")"
MINT_CLUNC="$(jq -r '.mint_clunc' <<<"$PLAN")"
echo "  LUNC/USD=$LUNC_USD mint_clunc=$MINT_CLUNC provide $CLUNC_RAW / $USDT_RAW"

CREATE_TX=""
SEED_TX=""
MINT_TX=""

if [[ "$MINT_CLUNC" != "0" ]]; then
  MINT_MSG="$(jq -nc --arg r "$ADMIN_ADDR" --arg a "$MINT_CLUNC" '{mint:{recipient:$r,amount:$a}}')"
  MINT_TX="$(broadcast_admin "mint cLUNC $MINT_CLUNC → $ADMIN_ADDR" wasm execute "$CLUNC" "$MINT_MSG")"
  BAL_CLUNC="$(cw20_balance "$CLUNC" "$ADMIN_ADDR")"
  py_ge "$BAL_CLUNC" "$CLUNC_RAW" || die "cLUNC balance $BAL_CLUNC < seed $CLUNC_RAW after mint"
else
  echo "  skip mint cLUNC (already funded)"
fi

PAIR_ADDR="$EXISTING"
if [[ -z "$PAIR_ADDR" ]]; then
  CREATE_MSG="$(jq -nc --arg a "$CLUNC" --arg b "$USDT" \
    '{create_pair:{asset_infos:[{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')"
  CREATE_TX="$(broadcast_admin "create_pair cLUNC/USDT" \
    wasm execute "$FACTORY" "$CREATE_MSG" --amount "${FEE_ULUNA}uluna")"
  sleep 3
  PAIR_ADDR="$(factory_pair_addr "$CLUNC" "$USDT")"
  if [[ -z "$PAIR_ADDR" ]]; then
    PAIR_ADDR="$(factory_pair_addr "$USDT" "$CLUNC")"
  fi
  [[ -n "$PAIR_ADDR" ]] || die "create_pair succeeded but factory Pair query empty (tx $CREATE_TX)"
  echo "  pair: $PAIR_ADDR"
else
  echo "  reusing empty pair $PAIR_ADDR"
fi

SHARE="$(pair_total_share "$PAIR_ADDR")"
if [[ "$SHARE" != "0" && -n "$SHARE" ]]; then
  die "pair $PAIR_ADDR already has total_share=$SHARE; refusing to reprice"
fi

ALLOW_A="$(jq -nc --arg s "$PAIR_ADDR" --arg a "$CLUNC_RAW" \
  '{increase_allowance:{spender:$s,amount:$a,expires:{never:{}}}}')"
ALLOW_B="$(jq -nc --arg s "$PAIR_ADDR" --arg a "$USDT_RAW" \
  '{increase_allowance:{spender:$s,amount:$a,expires:{never:{}}}}')"
broadcast_admin "increase_allowance cLUNC" wasm execute "$CLUNC" "$ALLOW_A" >/dev/null
broadcast_admin "increase_allowance USDT" wasm execute "$USDT" "$ALLOW_B" >/dev/null

PROVIDE="$(jq -nc --arg a "$CLUNC" --arg b "$USDT" --arg aa "$CLUNC_RAW" --arg bb "$USDT_RAW" \
  --arg recv "$LP_RECEIVER" \
  '{provide_liquidity:{assets:[
    {info:{token:{contract_addr:$a}},amount:$aa},
    {info:{token:{contract_addr:$b}},amount:$bb}
  ],slippage_tolerance:null,receiver:$recv,deadline:null}}')"
SEED_TX="$(broadcast_admin "provide_liquidity cLUNC/USDT → $LP_RECEIVER" wasm execute "$PAIR_ADDR" "$PROVIDE")"

SHARE="$(pair_total_share "$PAIR_ADDR")"
[[ "$SHARE" != "0" && -n "$SHARE" ]] || die "provide_liquidity left total_share=$SHARE"
LP_TOKEN="$(pair_lp_token "$PAIR_ADDR")"
echo "  pool total_share=$SHARE lp_token=$LP_TOKEN"

mkdir -p "$DEPLOY_DIR"
{
  echo "# cLUNC/USDT pair — written by mint-clunc-usdt-lp.sh $(date -u +%Y-%m-%dT%H:%MZ)"
  echo "NETWORK=mainnet"
  echo "CHAIN_ID=columbus-5"
  echo "FACTORY_ADDRESS=$FACTORY"
  echo "CLUNC_TOKEN_ADDRESS=$CLUNC"
  echo "USDT_TOKEN_ADDRESS=$USDT"
  echo "PAIR_ADDRESS=$PAIR_ADDR"
  echo "LP_TOKEN_ADDRESS=$LP_TOKEN"
  echo "LP_RECEIVER=$LP_RECEIVER"
  echo "USD_EACH=$USD_EACH"
  echo "LUNC_USD=$LUNC_USD"
  echo "CLUNC_RAW=$CLUNC_RAW"
  echo "USDT_RAW=$USDT_RAW"
  echo "MINT_TX=${MINT_TX:-}"
  echo "CREATE_TX=${CREATE_TX:-}"
  echo "SEED_TX=${SEED_TX:-}"
} >"$ENV_OUT"

echo ""
echo "Done."
echo "  PAIR_ADDRESS=$PAIR_ADDR"
echo "  env: $ENV_OUT"
echo "  Confirm indexer discovery on dex.cl8y.com /pool (factory provenance)."
