#!/usr/bin/env bash
# Hourly mint $200 of cUSTC → indexer best-solver swap to UST1 until CMM vFDUSD
# CW20 × Venus redeem × CEX FDUSD/USD is $500.
#
# Flow (each tick):
#   1. Price CMM vFDUSD at Venus fdusd_per_vfdusd × indexer FDUSD/USD.
#   2. Stop when that USD >= CUSTC_UST1_TARGET_USD (default 500).
#   3. 2-of-3 extra-minter mint of $200 cUSTC (USTC oracle) → admin (top-up only).
#   4. GET /api/v1/route/solve/best cUSTC→UST1 and execute router ops.
#   5. Default dest transfers received UST1 to CMM. Does not unwrap (cUSTC is
#      wrap) and does not window-redeem (that would drain CMM vFDUSD). The tick
#      does not raise vFDUSD; DEX UST1 premium vs the window is meant to pull
#      third-party vFDUSD deposits into CMM.
#   6. Live default: sleep 1h and repeat until CMM vFDUSD USD hits target.
#
# Usage:
#   DRY_RUN=1 ./scripts/mint-swap-custc-ust1.sh
#   CUSTC_UST1_YES=1 ./scripts/mint-swap-custc-ust1.sh
#   CUSTC_UST1_LOOP=0 CUSTC_UST1_YES=1 ./scripts/mint-swap-custc-ust1.sh
#
# Unlock once (non-interactive):
#   read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
#
# Never commit TERRAD_HOST_KEYRING_PASS.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/custc-ust1-buyback-defaults.sh
source "$SCRIPT_DIR/lib/custc-ust1-buyback-defaults.sh"
# shellcheck source=lib/terrad-host.sh
source "$SCRIPT_DIR/lib/terrad-host.sh"

TERRAD_HOST_CHAIN_ID="${TERRAD_HOST_CHAIN_ID:-columbus-5}"
TERRAD_HOST_NODE="${TERRAD_HOST_NODE:-https://terra-classic-rpc.publicnode.com:443}"
# Hybrid solver hops can sim low vs execute; 1.6 matches this workload better than 1.4.
TERRAD_HOST_GAS_ADJUSTMENT="${TERRAD_HOST_GAS_ADJUSTMENT:-1.6}"

LCD_URL="${CUSTC_UST1_LCD_URL%/}"
INDEXER="${CUSTC_UST1_INDEXER%/}"
MATH_PY="$SCRIPT_DIR/lib/custc-ust1-buyback-math.py"
HOOK_PY="$SCRIPT_DIR/lib/ust1-clunc-buyback-math.py"
MSIG_KEY="${CUSTC_UST1_MSIG_KEY}"
SIGNER1="${CUSTC_UST1_SIGNER1}"
SIGNER2="${CUSTC_UST1_SIGNER2}"
MSIG_ADDR="${CUSTC_UST1_MSIG_ADDR}"
ADMIN_KEY="${CUSTC_UST1_ADMIN_KEY}"
ADMIN_ADDR="${CUSTC_UST1_ADMIN_ADDR}"
TREASURY="${CUSTC_UST1_TREASURY}"
ROUTER="${CUSTC_UST1_ROUTER}"
UST1="${CUSTC_UST1_UST1}"
CUSTC="${CUSTC_UST1_CUSTC}"
VFDUSD="${CUSTC_UST1_VFDUSD}"
MINT_USD="${CUSTC_UST1_MINT_USD}"
TARGET_USD="${CUSTC_UST1_TARGET_USD}"
INTERVAL_SEC="${CUSTC_UST1_INTERVAL_SEC}"
DEST="${CUSTC_UST1_DEST}"
SLIP="${CUSTC_UST1_SLIPPAGE_PERCENT}"
MAX_SPREAD="${CUSTC_UST1_MAX_SPREAD}"
LCD_TIMEOUT="${CUSTC_UST1_LCD_TIMEOUT:-25}"
QUOTE_TIMEOUT="${CUSTC_UST1_QUOTE_TIMEOUT:-60}"
STATE_FILE="${CUSTC_UST1_STATE_FILE:-$HOME/.cl8y-dex/custc-ust1-buyback-state.json}"
HTTP_UA="cl8y-dex-ops/custc-ust1-buyback (+https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic)"

die() { echo "ERROR: $*" >&2; exit 1; }

usage() {
  sed -n '2,28p' "$0" | sed 's/^# \{0,1\}//'
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

case "$DEST" in
  cmm|hold|burn) ;;
  *) die "CUSTC_UST1_DEST must be cmm, hold, or burn (got $DEST)" ;;
esac

# Live default: keep running until CMM vFDUSD USD >= target. Dry-run is one tick.
if [[ -z "${CUSTC_UST1_LOOP:-}" ]]; then
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    CUSTC_UST1_LOOP=0
  else
    CUSTC_UST1_LOOP=1
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

token_supply() {
  lcd_smart "$1" '{"token_info":{}}' | jq -r '.total_supply // "0"'
}

is_minter() {
  local token="$1" who="$2" primary extras
  primary="$(lcd_smart "$token" '{"minter":{}}' | jq -r '.minter // empty')"
  extras="$(lcd_smart "$token" '{"minters":{}}' | jq -r '[.minters[]?] | join(" ")' 2>/dev/null || echo "")"
  [[ "$primary" == "$who" || " $extras " == *" $who "* ]]
}

fetch_ustc_usd() {
  if [[ -n "${CUSTC_UST1_USTC_USD:-}" ]]; then
    printf '%s' "$CUSTC_UST1_USTC_USD"
    return 0
  fi
  curl -sS --connect-timeout 10 --max-time 20 -H "User-Agent: $HTTP_UA" \
    "$CUSTC_UST1_INDEXER_USTC" | jq -er '.price_usd // empty'
}

# Prints: fdusd_usd \t venus_rate  (or dies). Pin CUSTC_UST1_VFDUSD_USD to skip both.
fetch_vfdusd_marks() {
  local raw fdusd venus
  if [[ -n "${CUSTC_UST1_VFDUSD_USD:-}" ]]; then
    # Direct USD per 1 human vFDUSD (already Venus × FDUSD). Venus=1 dummy for the CLI.
    printf '%s\t%s' "$CUSTC_UST1_VFDUSD_USD" "1"
    return 0
  fi
  raw="$(curl -sS --connect-timeout 10 --max-time 20 -H "User-Agent: $HTTP_UA" \
    "$CUSTC_UST1_INDEXER_VFDUSD")" || die "vFDUSD oracle request failed"
  fdusd="${CUSTC_UST1_FDUSD_USD:-}"
  venus="${CUSTC_UST1_FDUSD_PER_VFDUSD:-}"
  if [[ -z "$fdusd" ]]; then
    fdusd="$(jq -er '.price_usd // empty' <<<"$raw")" || die "vFDUSD oracle missing price_usd"
  fi
  if [[ -z "$venus" ]]; then
    venus="$(jq -er '.venus.fdusd_per_vfdusd // empty' <<<"$raw" 2>/dev/null || true)"
  fi
  if [[ -z "$venus" || "$venus" == "null" ]]; then
    venus="$(curl -sS --connect-timeout 10 --max-time 20 -H "User-Agent: $HTTP_UA" \
      "${INDEXER}/api/v1/oracle/price/vfdusd/venus" | jq -er '.fdusd_per_vfdusd // empty')" \
      || die "Venus fdusd_per_vfdusd missing (do not assume 1 vFDUSD = 1 FDUSD)"
  fi
  printf '%s\t%s' "$fdusd" "$venus"
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
  workdir="$(mktemp -d "${TMPDIR:-/tmp}/custc-ust1-msig.XXXXXX")"
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
  broadcast_msig "mint cUSTC $amount → $ADMIN_ADDR" "$CUSTC" "$msg" >/dev/null
}

load_state() {
  if [[ -f "$STATE_FILE" ]]; then
    jq -c '.' "$STATE_FILE"
  else
    echo '{"swapped_raw":"0","swapped_usd":"0","rounds":0}'
  fi
}

save_state() {
  local swapped_raw="$1" swapped_usd="$2" rounds="$3"
  mkdir -p "$(dirname "$STATE_FILE")"
  jq -nc --arg r "$swapped_raw" --arg u "$swapped_usd" --argjson n "$rounds" \
    '{swapped_raw:$r,swapped_usd:$u,rounds:$n}' >"$STATE_FILE"
}

quote_best() {
  local amount="$1" url raw
  url="${INDEXER}/api/v1/route/solve/best?token_in=${CUSTC}&token_out=${UST1}&amount_in=${amount}&trader=${ADMIN_ADDR}&sender=${ADMIN_ADDR}"
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
  echo "cUSTC → UST1 best-solver buyback"
  echo "=============================================="
  echo "Router:     $ROUTER"
  echo "Treasury:   $TREASURY"
  echo "Admin key:  $ADMIN_KEY ($ADMIN_ADDR)"
  echo "Multisig:   $MSIG_KEY ($MSIG_ADDR)"
  echo "Mint/tick:  \$${MINT_USD} cUSTC (USTC oracle)"
  echo "Target USD: $TARGET_USD  (CMM vFDUSD × Venus × FDUSD/USD)"
  echo "Dest:       $DEST"
  echo "Interval:   ${INTERVAL_SEC}s  LOOP=${CUSTC_UST1_LOOP}"
  echo "Slippage:   ${SLIP}%"
  echo "DRY_RUN:    ${DRY_RUN:-0}"
  echo "State:      $STATE_FILE"
}

refresh_cmm_vfdusd() {
  local fdusd_usd="$1" venus="$2"
  CMM_VFDUSD="$(cw20_balance "$VFDUSD" "$TREASURY")"
  if [[ -n "${CUSTC_UST1_VFDUSD_USD:-}" ]]; then
    CMM_VFDUSD_USD="$(python3 "$MATH_PY" usd --raw "$CMM_VFDUSD" --decimals "$DEC_VFDUSD" --price "$CUSTC_UST1_VFDUSD_USD")"
    VFDUSD_PX="$CUSTC_UST1_VFDUSD_USD"
  else
    CMM_VFDUSD_USD="$(python3 "$MATH_PY" vfdusd-usd --raw "$CMM_VFDUSD" --decimals "$DEC_VFDUSD" \
      --fdusd-usd "$fdusd_usd" --venus "$venus")"
    VFDUSD_PX="$(python3 "$MATH_PY" vfdusd-px --fdusd-usd "$fdusd_usd" --venus "$venus")"
  fi
}

stop_if_target() {
  local remain
  remain="$(python3 "$MATH_PY" remaining --current "$CMM_VFDUSD_USD" --target "$TARGET_USD")"
  echo "  CMM vFDUSD=$CMM_VFDUSD  USD=\$${CMM_VFDUSD_USD}  (× Venus × FDUSD/USD; px=\$${VFDUSD_PX})"
  echo "  remain \$${remain}  target=\$${TARGET_USD}"
  if python3 "$MATH_PY" progress --current "$CMM_VFDUSD_USD" --target "$TARGET_USD"; then
    echo "OK — CMM vFDUSD target reached (\$${CMM_VFDUSD_USD} >= \$${TARGET_USD})."
    return 10
  fi
  return 0
}

warn_wrap_solvency() {
  local supply backing mint_raw
  mint_raw="$1"
  supply="$(token_supply "$CUSTC")"
  backing="$(lcd_bank "$TREASURY" "uusd")"
  echo "  wrap solvency probe: cUSTC supply=$supply  CMM uusd=$backing  this mint=$mint_raw"
  python3 -c "import sys; sys.exit(0 if int('$supply') + int('$mint_raw') > int('$backing' or 0) else 1)" \
    && echo "  WARN: extra-minting cUSTC is unbacked wrap (supply+mint > CMM uusd). Same as CLUNC_LP_MINT_MODE=minter." >&2 \
    || true
}

one_tick() {
  local ustc_usd fdusd_usd venus mint_raw have need quote est min_recv hops hook_json hook_b64 send_msg
  local before after delta hop_returns marks
  local state swapped_raw swapped_usd rounds

  echo ""
  echo "[oracle]"
  ustc_usd="$(fetch_ustc_usd)" || die "USTC USD oracle failed"
  echo "  USTC/USD=$ustc_usd"
  marks="$(fetch_vfdusd_marks)" || die "vFDUSD marks failed"
  fdusd_usd="${marks%%$'\t'*}"
  venus="${marks#*$'\t'}"
  if [[ -n "${CUSTC_UST1_VFDUSD_USD:-}" ]]; then
    echo "  vFDUSD/USD=$CUSTC_UST1_VFDUSD_USD (pinned; skip Venus × FDUSD)"
  else
    echo "  FDUSD/USD=$fdusd_usd  Venus fdusd_per_vfdusd=$venus"
  fi

  state="$(load_state)"
  swapped_raw="$(jq -r '.swapped_raw // "0"' <<<"$state")"
  swapped_usd="$(jq -r '.swapped_usd // "0"' <<<"$state")"
  rounds="$(jq -r '.rounds // 0' <<<"$state")"

  refresh_cmm_vfdusd "$fdusd_usd" "$venus"
  echo "[progress]"
  echo "  swapped this campaign raw=$swapped_raw  usd=\$${swapped_usd}  rounds=$rounds  (telemetry only)"
  stop_if_target || return $?

  mint_raw="$(python3 "$MATH_PY" mint-raw-usd --usd "$MINT_USD" --price "$ustc_usd" --decimals "$DEC_CUSTC")"
  echo "[mint] \$${MINT_USD} → $mint_raw cUSTC raw"
  warn_wrap_solvency "$mint_raw"
  have="$(cw20_balance "$CUSTC" "$ADMIN_ADDR")"
  if python3 -c "import sys; sys.exit(0 if int('$have') < int('$mint_raw') else 1)"; then
    need="$(python3 -c "print(int('$mint_raw') - int('$have'))")"
  else
    need="0"
  fi
  echo "  have cUSTC=$have need=$need (tick=$mint_raw)"
  mint_if_needed "$need"
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "  [DRY_RUN] assuming mint credited; quoting $mint_raw"
  else
    have="$(cw20_balance "$CUSTC" "$ADMIN_ADDR")"
    python3 -c "import sys; sys.exit(0 if int('$have') >= int('$mint_raw') else 1)" \
      || die "admin cUSTC $have < mint size $mint_raw"
  fi

  echo "[quote] $mint_raw cUSTC → UST1 via /route/solve/best"
  quote="$(quote_best "$mint_raw")"
  hops="$(jq -r '.hops | length' <<<"$quote")"
  est="$(jq -r '.estimated_amount_out' <<<"$quote")"
  echo "  hops=$hops kind=$(jq -r '.quote_kind // empty' <<<"$quote") out=$est solver=$(jq -r '.solver_version // empty' <<<"$quote")"
  echo "  path=$(jq -r '[.hops[]?.pair] | join(" → ")' <<<"$quote")"
  min_recv="$(python3 "$HOOK_PY" min-receive --out "$est" --slip "$SLIP")"
  [[ "$min_recv" != "0" ]] || die "min_receive is 0 (quote too small for slippage $SLIP%)"
  echo "  min_receive=$min_recv (${SLIP}% floor of $est)"

  echo "[hop-sim] pair hybrid_simulation for #334 book min_return"
  hop_returns="$(sim_hop_returns "$quote" "$mint_raw")"
  echo "  hop_returns=$hop_returns"

  before="$(cw20_balance "$UST1" "$ADMIN_ADDR")"
  local qfile
  qfile="$(mktemp "${TMPDIR:-/tmp}/custc-ust1-quote.XXXXXX.json")"
  printf '%s' "$quote" >"$qfile"
  hook_json="$(python3 "$HOOK_PY" send-msg \
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
    echo "  [DRY_RUN] would swap $mint_raw cUSTC via router ($hops hops, min $min_recv UST1)"
    case "$DEST" in
      cmm) echo "  [DRY_RUN] would transfer received UST1 to CMM" ;;
      burn) echo "  [DRY_RUN] would burn received UST1" ;;
      hold) echo "  [DRY_RUN] would leave received UST1 on admin" ;;
    esac
    return 0
  fi

  broadcast_admin "solver swap $mint_raw cUSTC → UST1 ($hops hops)" \
    wasm execute "$CUSTC" "$send_msg" >/dev/null

  after="$(cw20_balance "$UST1" "$ADMIN_ADDR")"
  delta="$(python3 "$HOOK_PY" delta --before "$before" --after "$after")"
  [[ "$delta" != "0" ]] || die "swap succeeded but admin UST1 did not increase (before=$before after=$after)"
  local delta_usd
  delta_usd="$(python3 "$MATH_PY" usd --raw "$delta" --decimals "$DEC_UST1" --price "1")"
  echo "  received UST1=$delta (~\$${delta_usd} at \$1)"

  case "$DEST" in
    cmm)
      local xfer cmm_ust1_before cmm_ust1_after
      cmm_ust1_before="$(cw20_balance "$UST1" "$TREASURY")"
      xfer="$(jq -nc --arg r "$TREASURY" --arg a "$delta" '{transfer:{recipient:$r,amount:$a}}')"
      [[ "$ADMIN_ADDR" != "$TREASURY" ]] || die "refusing to transfer from CMM to itself"
      broadcast_admin "transfer UST1 $delta → CMM" wasm execute "$UST1" "$xfer" >/dev/null
      cmm_ust1_after="$(cw20_balance "$UST1" "$TREASURY")"
      python3 -c "import sys; sys.exit(0 if int('$cmm_ust1_after') > int('$cmm_ust1_before') else 1)" \
        || die "CMM UST1 did not increase ($cmm_ust1_before → $cmm_ust1_after)"
      local left
      left="$(cw20_balance "$UST1" "$ADMIN_ADDR")"
      python3 -c "import sys; sys.exit(0 if int('$left') == int('$before') else 1)" \
        || die "admin UST1 after CMM transfer=$left (want pre-swap $before)"
      echo "  CMM UST1 $cmm_ust1_before → $cmm_ust1_after (stop is still vFDUSD only)"
      ;;
    burn)
      [[ "$ADMIN_ADDR" != "$TREASURY" ]] || die "refusing to burn on CMM treasury"
      [[ "$UST1" != "$CUSTC" ]] || die "refusing to burn cUSTC as UST1"
      local burn
      burn="$(jq -nc --arg a "$delta" '{burn:{amount:$a}}')"
      broadcast_admin "burn UST1 $delta" wasm execute "$UST1" "$burn" >/dev/null
      local left
      left="$(cw20_balance "$UST1" "$ADMIN_ADDR")"
      python3 -c "import sys; sys.exit(0 if int('$left') == int('$before') else 1)" \
        || die "admin UST1 after burn=$left (want pre-swap $before)"
      echo "  burned UST1=$delta"
      ;;
    hold)
      echo "  holding UST1=$delta on admin (not CMM, not burned)"
      ;;
  esac

  swapped_raw="$(python3 -c "print(int('$swapped_raw') + int('$delta'))")"
  swapped_usd="$(python3 -c "from decimal import Decimal; print(Decimal('$swapped_usd') + Decimal('$delta_usd'))")"
  rounds=$((rounds + 1))
  save_state "$swapped_raw" "$swapped_usd" "$rounds"
  echo "  campaign swapped_usd=\$${swapped_usd} rounds=$rounds"

  refresh_cmm_vfdusd "$fdusd_usd" "$venus"
  stop_if_target || return $?
  return 0
}

python3 "$MATH_PY" self-test >/dev/null
python3 "$HOOK_PY" self-test >/dev/null

print_header
echo ""
echo "[preflight] tokens + extra minter $MSIG_ADDR"
DEC_CUSTC="$(token_decimals "$CUSTC")"
DEC_UST1="$(token_decimals "$UST1")"
DEC_VFDUSD="$(token_decimals "$VFDUSD")"
echo "  cUSTC symbol=$(token_symbol "$CUSTC") decimals=$DEC_CUSTC"
echo "  UST1 symbol=$(token_symbol "$UST1") decimals=$DEC_UST1"
echo "  vFDUSD symbol=$(token_symbol "$VFDUSD") decimals=$DEC_VFDUSD"
is_minter "$CUSTC" "$MSIG_ADDR" || die "DEX 2-of-3 $MSIG_ADDR is not an extra minter of cUSTC"
[[ "$ADMIN_ADDR" != "$TREASURY" ]] || die "admin must not be CMM treasury"

ensure_unlocked

if [[ "${DRY_RUN:-0}" != "1" && "${CUSTC_UST1_YES:-0}" != "1" ]]; then
  if [[ ! -t 0 ]]; then
    die "refusing live broadcast without TTY; set CUSTC_UST1_YES=1"
  fi
  read -r -p "Broadcast mint / solver swap / $DEST on $TERRAD_HOST_CHAIN_ID? [y/N] " ans
  [[ "$ans" == "y" || "$ans" == "Y" ]] || die "aborted"
fi

if [[ "${CUSTC_UST1_LOOP}" == "1" ]]; then
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
