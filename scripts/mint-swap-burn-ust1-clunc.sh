#!/usr/bin/env bash
# Hourly mint 50 UST1 → indexer best-solver swap to cLUNC → holder-burn until
# CMM bank uluna × LUNC oracle is $2500.
#
# Flow (each tick):
#   1. Price CMM bank uluna at the indexer LUNC oracle (cLUNC CW20 is not counted).
#   2. Stop when that USD >= UST1_CLUNC_TARGET_USD (default 2500).
#   3. 2-of-3 extra-minter mint of 50 UST1 → admin (top-up only).
#   4. GET /api/v1/route/solve/best UST1→cLUNC and execute router ops.
#   5. Holder-burn the cLUNC received. Does not unwrap (that would drain CMM
#      wrap custody). The tick does not raise bank uluna; DEX premium vs wrap
#      is meant to pull third-party WrapDeposit (native LUNC into CMM).
#   6. Live default: sleep 1h and repeat until CMM bank uluna USD hits target.
#
# Usage:
#   DRY_RUN=1 ./scripts/mint-swap-burn-ust1-clunc.sh
#   UST1_CLUNC_YES=1 ./scripts/mint-swap-burn-ust1-clunc.sh
#   UST1_CLUNC_LOOP=0 UST1_CLUNC_YES=1 ./scripts/mint-swap-burn-ust1-clunc.sh
#
# Unlock once (non-interactive):
#   read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
#
# Never commit TERRAD_HOST_KEYRING_PASS.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/ust1-clunc-buyback-defaults.sh
source "$SCRIPT_DIR/lib/ust1-clunc-buyback-defaults.sh"
# shellcheck source=lib/terrad-host.sh
source "$SCRIPT_DIR/lib/terrad-host.sh"

TERRAD_HOST_CHAIN_ID="${TERRAD_HOST_CHAIN_ID:-columbus-5}"
TERRAD_HOST_NODE="${TERRAD_HOST_NODE:-https://terra-classic-rpc.publicnode.com:443}"
# Hybrid solver hops can sim low vs execute; 1.6 matches this workload better than 1.4.
TERRAD_HOST_GAS_ADJUSTMENT="${TERRAD_HOST_GAS_ADJUSTMENT:-1.6}"

LCD_URL="${UST1_CLUNC_LCD_URL%/}"
INDEXER="${UST1_CLUNC_INDEXER%/}"
MATH_PY="$SCRIPT_DIR/lib/ust1-clunc-buyback-math.py"
MSIG_KEY="${UST1_CLUNC_MSIG_KEY}"
SIGNER1="${UST1_CLUNC_SIGNER1}"
SIGNER2="${UST1_CLUNC_SIGNER2}"
MSIG_ADDR="${UST1_CLUNC_MSIG_ADDR}"
ADMIN_KEY="${UST1_CLUNC_ADMIN_KEY}"
ADMIN_ADDR="${UST1_CLUNC_ADMIN_ADDR}"
TREASURY="${UST1_CLUNC_TREASURY}"
ROUTER="${UST1_CLUNC_ROUTER}"
UST1="${UST1_CLUNC_UST1}"
CLUNC="${UST1_CLUNC_CLUNC}"
MINT_HUMAN="${UST1_CLUNC_MINT_HUMAN}"
TARGET_USD="${UST1_CLUNC_TARGET_USD}"
INTERVAL_SEC="${UST1_CLUNC_INTERVAL_SEC}"
DEST="${UST1_CLUNC_DEST}"
SLIP="${UST1_CLUNC_SLIPPAGE_PERCENT}"
MAX_SPREAD="${UST1_CLUNC_MAX_SPREAD}"
LCD_TIMEOUT="${UST1_CLUNC_LCD_TIMEOUT:-25}"
QUOTE_TIMEOUT="${UST1_CLUNC_QUOTE_TIMEOUT:-60}"
STATE_FILE="${UST1_CLUNC_STATE_FILE:-$HOME/.cl8y-dex/ust1-clunc-buyback-state.json}"
HTTP_UA="cl8y-dex-ops/ust1-clunc-buyback (+https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic)"

die() { echo "ERROR: $*" >&2; exit 1; }

usage() {
  sed -n '2,26p' "$0" | sed 's/^# \{0,1\}//'
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

case "$DEST" in
  burn|cmm) ;;
  *) die "UST1_CLUNC_DEST must be burn or cmm (got $DEST)" ;;
esac

# Live default: keep running until CMM bank uluna USD >= target. Dry-run is one tick.
if [[ -z "${UST1_CLUNC_LOOP:-}" ]]; then
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    UST1_CLUNC_LOOP=0
  else
    UST1_CLUNC_LOOP=1
  fi
fi

lcd_b64() {
  if [[ "$(uname)" == Darwin ]]; then
    printf '%s' "$1" | base64 | tr -d '\n'
  else
    printf '%s' "$1" | base64 -w0
  fi
}

lcd_smart_try() {
  local contract="$1" msg="$2" raw
  raw="$(curl -sS --connect-timeout 10 --max-time "$LCD_TIMEOUT" -H "User-Agent: $HTTP_UA" \
    "${LCD_URL}/cosmwasm/wasm/v1/contract/${contract}/smart/$(lcd_b64 "$msg")" 2>/dev/null || true)"
  if ! jq -e '(.data != null) and ((.code // 0) == 0)' >/dev/null <<<"$raw" 2>/dev/null; then
    return 1
  fi
  if [[ "$(jq -r '.data | type' <<<"$raw")" == "string" ]]; then
    jq -r '.data | @base64d | fromjson' <<<"$raw"
  else
    jq '.data' <<<"$raw"
  fi
}

# Sequential pair hybrid_simulation (same as dApp #334 hop floors). Prints JSON string array.
sim_hop_returns() {
  local quote_json="$1" current="$2"
  local n i pair offer_info hybrid book msg sim ret
  n="$(jq -r '.router_operations | length' <<<"$quote_json")"
  local -a rets=()
  for ((i = 0; i < n; i++)); do
    pair="$(jq -r ".hops[$i].pair // empty" <<<"$quote_json")"
    offer_info="$(jq -c ".router_operations[$i].terra_swap.offer_asset_info" <<<"$quote_json")"
    hybrid="$(jq -c ".router_operations[$i].terra_swap.hybrid // null" <<<"$quote_json")"
    book="$(jq -r 'if type=="object" then (.book_input // "0") else "0" end' <<<"$hybrid")"
    if [[ "$hybrid" == "null" ]]; then
      hybrid="$(jq -nc --arg a "$current" '{pool_input:$a,book_input:"0",max_maker_fills:1,book_start_hint:null}')"
    fi
    msg="$(jq -nc --argjson info "$offer_info" --arg amt "$current" --argjson hy "$hybrid" --arg t "$ADMIN_ADDR" \
      '{hybrid_simulation:{offer_asset:{info:$info,amount:$amt},hybrid:$hy,trader:$t,sender:$t}}')"
    ret="0"
    if [[ -n "$pair" ]] && sim="$(lcd_smart_try "$pair" "$msg")"; then
      ret="$(jq -r '.return_amount // "0"' <<<"$sim")"
    fi
    if [[ -z "$ret" || "$ret" == "null" ]]; then
      ret="0"
    fi
    echo "  hop $((i + 1)) pair=$pair book=$book sim_return=$ret" >&2
    rets+=("$ret")
    if [[ "$ret" != "0" ]]; then
      current="$ret"
    fi
  done
  python3 -c 'import json,sys; print(json.dumps(sys.argv[1:]))' "${rets[@]}"
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

fetch_lunc_usd() {
  if [[ -n "${UST1_CLUNC_LUNC_USD:-}" ]]; then
    printf '%s' "$UST1_CLUNC_LUNC_USD"
    return 0
  fi
  curl -sS --connect-timeout 10 --max-time 20 -H "User-Agent: $HTTP_UA" \
    "$UST1_CLUNC_INDEXER_ORACLE" | jq -er '.price_usd // empty'
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

broadcast_msig() {
  local label="$1" contract="$2" msg="$3"
  local workdir unsigned sig1 sig2 signed out tx_hash code
  echo "  → msig $label" >&2
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "    [DRY_RUN] wasm execute $contract $msg" >&2
    echo "DRY_RUN_TX"
    return 0
  fi
  workdir="$(mktemp -d "${TMPDIR:-/tmp}/ust1-clunc-msig.XXXXXX")"
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
  local amount="$1"
  if [[ "$amount" == "0" ]]; then
    echo "  skip mint (admin already funded)"
    return 0
  fi
  local msg
  msg="$(jq -nc --arg r "$ADMIN_ADDR" --arg a "$amount" '{mint:{recipient:$r,amount:$a}}')"
  broadcast_msig "mint UST1 $amount → $ADMIN_ADDR" "$UST1" "$msg" >/dev/null
}

load_state() {
  if [[ -f "$STATE_FILE" ]]; then
    jq -c '.' "$STATE_FILE"
  else
    echo '{"burned_raw":"0","burned_usd":"0","rounds":0}'
  fi
}

save_state() {
  local burned_raw="$1" burned_usd="$2" rounds="$3"
  mkdir -p "$(dirname "$STATE_FILE")"
  jq -nc --arg r "$burned_raw" --arg u "$burned_usd" --argjson n "$rounds" \
    '{burned_raw:$r,burned_usd:$u,rounds:$n}' >"$STATE_FILE"
}

quote_best() {
  local amount="$1" url raw
  url="${INDEXER}/api/v1/route/solve/best?token_in=${UST1}&token_out=${CLUNC}&amount_in=${amount}&trader=${ADMIN_ADDR}&sender=${ADMIN_ADDR}"
  echo "  GET $url" >&2
  raw="$(curl -sS --connect-timeout 10 --max-time "$QUOTE_TIMEOUT" -H "User-Agent: $HTTP_UA" "$url")" \
    || die "indexer quote request failed"
  if ! jq -e '.router_operations | type == "array" and length > 0' >/dev/null <<<"$raw"; then
    echo "$raw" | jq -c '.' >&2 || true
    die "indexer returned no router_operations"
  fi
  jq -e '.estimated_amount_out != null and (.estimated_amount_out | tonumber) > 0' >/dev/null <<<"$raw" \
    || die "indexer quote missing estimated_amount_out"
  printf '%s' "$raw"
}

ensure_unlocked() {
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "  [DRY_RUN] skipping key unlock; assuming admin $ADMIN_ADDR"
    return 0
  fi
  prompt_keyring_pass
  TERRAD_HOST_KEY="$ADMIN_KEY"
  TERRAD_HOST_EXPECTED_ADDR="$ADMIN_ADDR"
  GOT_ADMIN="$(terrad_host_key_address)" || exit 1
  [[ "$GOT_ADMIN" == "$ADMIN_ADDR" ]] || die "admin key $ADMIN_KEY is $GOT_ADMIN (want $ADMIN_ADDR)"
}

print_header() {
  echo "=============================================="
  echo "UST1 → cLUNC best-solver buyback → burn"
  echo "=============================================="
  echo "Router:     $ROUTER"
  echo "Treasury:   $TREASURY"
  echo "Admin key:  $ADMIN_KEY ($ADMIN_ADDR)"
  echo "Multisig:   $MSIG_KEY ($MSIG_ADDR)"
  echo "Mint/tick:  $MINT_HUMAN UST1"
  echo "Target USD: $TARGET_USD  (CMM bank uluna × LUNC oracle)"
  echo "Dest:       $DEST"
  echo "Interval:   ${INTERVAL_SEC}s  LOOP=${UST1_CLUNC_LOOP}"
  echo "Slippage:   ${SLIP}%"
  echo "DRY_RUN:    ${DRY_RUN:-0}"
  echo "State:      $STATE_FILE"
}

refresh_cmm_uluna() {
  local price="$1"
  CMM_ULUNA="$(lcd_bank "$TREASURY" "uluna")"
  CMM_ULUNA_USD="$(python3 "$MATH_PY" usd --raw "$CMM_ULUNA" --decimals 6 --price "$price")"
}

stop_if_target() {
  local remain
  remain="$(python3 "$MATH_PY" remaining --current "$CMM_ULUNA_USD" --target "$TARGET_USD")"
  echo "  CMM bank uluna=$CMM_ULUNA  USD=\$${CMM_ULUNA_USD}  (uluna × LUNC oracle)"
  echo "  remain \$${remain}  target=\$${TARGET_USD}"
  if python3 "$MATH_PY" progress --current "$CMM_ULUNA_USD" --target "$TARGET_USD"; then
    echo "OK — CMM bank uluna target reached (\$${CMM_ULUNA_USD} >= \$${TARGET_USD})."
    return 10
  fi
  return 0
}

one_tick() {
  local lunc_usd mint_raw have need quote est min_recv hops hook_json hook_b64 send_msg
  local before after delta hop_returns
  local state burned_raw burned_usd rounds

  echo ""
  echo "[oracle]"
  lunc_usd="$(fetch_lunc_usd)" || die "LUNC USD oracle failed"
  echo "  LUNC/USD=$lunc_usd"

  state="$(load_state)"
  burned_raw="$(jq -r '.burned_raw // "0"' <<<"$state")"
  burned_usd="$(jq -r '.burned_usd // "0"' <<<"$state")"
  rounds="$(jq -r '.rounds // 0' <<<"$state")"

  refresh_cmm_uluna "$lunc_usd"
  echo "[progress]"
  echo "  burned this campaign raw=$burned_raw  usd=\$${burned_usd}  rounds=$rounds  (telemetry only)"
  stop_if_target || return $?

  mint_raw="$(python3 "$MATH_PY" mint-raw --human "$MINT_HUMAN" --decimals "$DEC_UST1")"
  have="$(cw20_balance "$UST1" "$ADMIN_ADDR")"
  if python3 -c "import sys; sys.exit(0 if int('$have') < int('$mint_raw') else 1)"; then
    need="$(python3 -c "print(int('$mint_raw') - int('$have'))")"
  else
    need="0"
  fi
  echo "[mint] have UST1=$have need=$need (tick=$mint_raw)"
  mint_if_needed "$need"
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "  [DRY_RUN] assuming mint credited; quoting $mint_raw"
  else
    have="$(cw20_balance "$UST1" "$ADMIN_ADDR")"
    python3 -c "import sys; sys.exit(0 if int('$have') >= int('$mint_raw') else 1)" \
      || die "admin UST1 $have < mint size $mint_raw"
  fi

  echo "[quote] $mint_raw UST1 → cLUNC via /route/solve/best"
  quote="$(quote_best "$mint_raw")"
  hops="$(jq -r '.hops | length' <<<"$quote")"
  est="$(jq -r '.estimated_amount_out' <<<"$quote")"
  echo "  hops=$hops kind=$(jq -r '.quote_kind // empty' <<<"$quote") out=$est solver=$(jq -r '.solver_version // empty' <<<"$quote")"
  echo "  path=$(jq -r '[.hops[]?.pair] | join(" → ")' <<<"$quote")"
  min_recv="$(python3 "$MATH_PY" min-receive --out "$est" --slip "$SLIP")"
  [[ "$min_recv" != "0" ]] || die "min_receive is 0 (quote too small for slippage $SLIP%)"
  echo "  min_receive=$min_recv (${SLIP}% floor of $est)"

  echo "[hop-sim] pair hybrid_simulation for #334 book min_return"
  hop_returns="$(sim_hop_returns "$quote" "$mint_raw")"
  echo "  hop_returns=$hop_returns"

  before="$(cw20_balance "$CLUNC" "$ADMIN_ADDR")"
  local qfile
  qfile="$(mktemp "${TMPDIR:-/tmp}/ust1-clunc-quote.XXXXXX.json")"
  printf '%s' "$quote" >"$qfile"
  hook_json="$(python3 "$MATH_PY" send-msg \
    --quote-file "$qfile" \
    --amount "$mint_raw" \
    --router "$ROUTER" \
    --max-spread "$MAX_SPREAD" \
    --min-receive "$min_recv" \
    --slip "$SLIP" \
    --hop-returns "$hop_returns")"
  rm -f "$qfile"
  hook_b64="$(lcd_b64 "$hook_json")"
  send_msg="$(jq -nc --arg c "$ROUTER" --arg a "$mint_raw" --arg m "$hook_b64" \
    '{send:{contract:$c,amount:$a,msg:$m}}')"
  echo "  hop min_return=$(jq -c '[.execute_swap_operations.operations[].terra_swap.min_return // null]' <<<"$hook_json")"

  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "  [DRY_RUN] would swap $mint_raw UST1 via router ($hops hops, min $min_recv cLUNC)"
    if [[ "$DEST" == "cmm" ]]; then
      echo "  [DRY_RUN] would transfer received cLUNC to CMM"
    else
      echo "  [DRY_RUN] would burn received cLUNC"
    fi
    return 0
  fi

  broadcast_admin "solver swap $mint_raw UST1 → cLUNC ($hops hops)" \
    wasm execute "$UST1" "$send_msg" >/dev/null

  after="$(cw20_balance "$CLUNC" "$ADMIN_ADDR")"
  delta="$(python3 "$MATH_PY" delta --before "$before" --after "$after")"
  [[ "$delta" != "0" ]] || die "swap succeeded but admin cLUNC did not increase (before=$before after=$after)"
  local delta_usd
  delta_usd="$(python3 "$MATH_PY" usd --raw "$delta" --decimals "$DEC_CLUNC" --price "$lunc_usd")"
  echo "  received cLUNC=$delta (~\$${delta_usd})"

  if [[ "$DEST" == "cmm" ]]; then
    local xfer cmm_clunc_before cmm_clunc_after
    cmm_clunc_before="$(cw20_balance "$CLUNC" "$TREASURY")"
    xfer="$(jq -nc --arg r "$TREASURY" --arg a "$delta" '{transfer:{recipient:$r,amount:$a}}')"
    [[ "$ADMIN_ADDR" != "$TREASURY" ]] || die "refusing to transfer from CMM to itself"
    broadcast_admin "transfer cLUNC $delta → CMM" wasm execute "$CLUNC" "$xfer" >/dev/null
    cmm_clunc_after="$(cw20_balance "$CLUNC" "$TREASURY")"
    python3 -c "import sys; sys.exit(0 if int('$cmm_clunc_after') > int('$cmm_clunc_before') else 1)" \
      || die "CMM cLUNC did not increase ($cmm_clunc_before → $cmm_clunc_after)"
    echo "  CMM cLUNC $cmm_clunc_before → $cmm_clunc_after (stop is still bank uluna only)"
  else
    [[ "$ADMIN_ADDR" != "$TREASURY" ]] || die "refusing to burn on CMM treasury"
    [[ "$CLUNC" != "$UST1" ]] || die "refusing to burn UST1 as cLUNC"
    local burn
    burn="$(jq -nc --arg a "$delta" '{burn:{amount:$a}}')"
    broadcast_admin "burn cLUNC $delta" wasm execute "$CLUNC" "$burn" >/dev/null
    local left
    left="$(cw20_balance "$CLUNC" "$ADMIN_ADDR")"
    python3 -c "import sys; sys.exit(0 if int('$left') == int('$before') else 1)" \
      || die "admin cLUNC after burn=$left (want pre-swap $before)"
    burned_raw="$(python3 -c "print(int('$burned_raw') + int('$delta'))")"
    burned_usd="$(python3 -c "from decimal import Decimal; print(Decimal('$burned_usd') + Decimal('$delta_usd'))")"
    rounds=$((rounds + 1))
    save_state "$burned_raw" "$burned_usd" "$rounds"
    echo "  campaign burned_usd=\$${burned_usd} rounds=$rounds"
  fi
  refresh_cmm_uluna "$lunc_usd"
  stop_if_target || return $?
  return 0
}

python3 "$MATH_PY" self-test >/dev/null

print_header
echo ""
echo "[preflight] tokens + extra minter $MSIG_ADDR"
DEC_UST1="$(token_decimals "$UST1")"
DEC_CLUNC="$(token_decimals "$CLUNC")"
echo "  UST1 symbol=$(token_symbol "$UST1") decimals=$DEC_UST1"
echo "  cLUNC symbol=$(token_symbol "$CLUNC") decimals=$DEC_CLUNC"
is_minter "$UST1" "$MSIG_ADDR" || die "DEX 2-of-3 $MSIG_ADDR is not an extra minter of UST1"
[[ "$ADMIN_ADDR" != "$TREASURY" ]] || die "admin must not be CMM treasury"

ensure_unlocked

if [[ "${DRY_RUN:-0}" != "1" && "${UST1_CLUNC_YES:-0}" != "1" ]]; then
  if [[ ! -t 0 ]]; then
    die "refusing live broadcast without TTY; set UST1_CLUNC_YES=1"
  fi
  read -r -p "Broadcast mint / solver swap / $DEST on $TERRAD_HOST_CHAIN_ID? [y/N] " ans
  [[ "$ans" == "y" || "$ans" == "Y" ]] || die "aborted"
fi

if [[ "${UST1_CLUNC_LOOP}" == "1" ]]; then
  trap 'echo; echo "stopped."; exit 0' INT TERM
  while true; do
    set +e
    # Subshell so a tick `die` does not kill the hourly loop.
    ( set -euo pipefail; one_tick )
    st=$?
    set -e
    if [[ "$st" == "10" ]]; then
      exit 0
    fi
    if [[ "$st" != "0" ]]; then
      echo "WARN: tick failed (exit $st); sleeping ${INTERVAL_SEC}s then retry" >&2
    else
      echo "sleep ${INTERVAL_SEC}s until next tick"
    fi
    sleep "$INTERVAL_SEC"
  done
fi

set +e
one_tick
st=$?
set -e
if [[ "$st" == "10" || "$st" == "0" ]]; then
  exit 0
fi
exit "$st"
