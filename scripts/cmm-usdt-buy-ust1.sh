#!/usr/bin/env bash
# Spend USDT held by the CMM treasury to buy UST1 with the indexer
# best-execution solver every 20 minutes, sending the UST1 back to CMM, until
# the CMM USDT CW20 balance is down to a floor (default 500 USDT).
#
# Flow (each tick):
#   1. Read the CMM USDT CW20 balance. Stop when it is <= the floor.
#   2. Pull min(per-tick budget, balance - floor) USDT from CMM to the admin
#      hot wallet with Treasury InstantWithdrawCw20. The admin must be the
#      governance-registered USDT spender (run --setup-spender once).
#   3. GET /api/v1/route/solve/best USDT->UST1 and validate the router ops.
#   4. Execute the validated router ops with a slippage floor.
#   5. CW20-transfer the received UST1 to CMM and verify CMM UST1 increased.
#   6. Wait 20 minutes after the tick (success or failure), then repeat.
#
# Usage:
#   ./scripts/cmm-usdt-buy-ust1.sh --status
#   DRY_RUN=1 ./scripts/cmm-usdt-buy-ust1.sh
#   CMM_USDT_BUY_YES=1 ./scripts/cmm-usdt-buy-ust1.sh
#   CMM_USDT_BUY_LOOP=0 CMM_USDT_BUY_YES=1 ./scripts/cmm-usdt-buy-ust1.sh
#   CMM_USDT_BUY_YES=1 ./scripts/cmm-usdt-buy-ust1.sh --setup-spender
#
# Unlock once (or let the script prompt on the TTY):
#   read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
#
# Never commit TERRAD_HOST_KEYRING_PASS.
set -euo pipefail
# Keep query/build failures fatal inside command substitutions as well.
shopt -s inherit_errexit

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/cmm-usdt-buy-ust1-defaults.sh
source "$SCRIPT_DIR/lib/cmm-usdt-buy-ust1-defaults.sh"
# Hybrid solver hops can sim low vs execute; 1.6 matches the sibling buybacks.
TERRAD_HOST_GAS_ADJUSTMENT="${TERRAD_HOST_GAS_ADJUSTMENT:-1.6}"
# Independent Columbus-5 RPC for this drip; explicit operator overrides win.
TERRAD_HOST_NODE="${TERRAD_HOST_NODE:-https://rpc.terra-classic.hexxagon.io:443}"
# shellcheck source=lib/terrad-host.sh
source "$SCRIPT_DIR/lib/terrad-host.sh"

TERRAD_HOST_CHAIN_ID="${TERRAD_HOST_CHAIN_ID:-columbus-5}"

LCD_URL="${TERRAD_HOST_LCD_URL:-${CMM_USDT_BUY_LCD_URL%/}}"
INDEXER="${CMM_USDT_BUY_INDEXER%/}"
HOOK_PY="$SCRIPT_DIR/lib/ust1-clunc-buyback-math.py"
MSIG_KEY="${CMM_USDT_BUY_MSIG_KEY}"
SIGNER1="${CMM_USDT_BUY_SIGNER1}"
SIGNER2="${CMM_USDT_BUY_SIGNER2}"
MSIG_ADDR="${CMM_USDT_BUY_MSIG_ADDR}"
ADMIN_KEY="${CMM_USDT_BUY_ADMIN_KEY}"
ADMIN_ADDR="${CMM_USDT_BUY_ADMIN_ADDR}"
SPENDER="${CMM_USDT_BUY_SPENDER}"
TREASURY="${CMM_USDT_BUY_TREASURY}"
ROUTER="${CMM_USDT_BUY_ROUTER}"
UST1="${CMM_USDT_BUY_UST1}"
USDT="${CMM_USDT_BUY_USDT}"
FACTORY="${CMM_USDT_BUY_FACTORY}"
PER_TICK_USDT="${CMM_USDT_BUY_USDT_PER_TICK}"
FLOOR_USDT="${CMM_USDT_BUY_FLOOR_USDT}"
INTERVAL_SEC="${CMM_USDT_BUY_INTERVAL_SEC}"
SOURCE="${CMM_USDT_BUY_SOURCE}"
DEST="${CMM_USDT_BUY_DEST}"
SLIP="${CMM_USDT_BUY_SLIPPAGE_PERCENT}"
MAX_SPREAD="${CMM_USDT_BUY_MAX_SPREAD}"
PULL_LIMIT_USDT="${CMM_USDT_BUY_PULL_LIMIT_24H_USDT}"
LCD_TIMEOUT="${CMM_USDT_BUY_LCD_TIMEOUT:-25}"
QUOTE_TIMEOUT="${CMM_USDT_BUY_QUOTE_TIMEOUT:-60}"
STATE_FILE="${CMM_USDT_BUY_STATE_FILE:-$HOME/.cl8y-dex/cmm-usdt-buy-ust1-state.json}"
HTTP_UA="cl8y-dex-ops/cmm-usdt-buy-ust1 (+https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic)"

die() { echo "ERROR: $*" >&2; exit 1; }

usage() {
  sed -n '2,26p' "$0" | sed 's/^# \{0,1\}//'
}

case "${1:-}" in
  -h|--help) usage; exit 0 ;;
esac

case "$SOURCE" in cmm|wallet) ;; *) die "CMM_USDT_BUY_SOURCE must be cmm or wallet (got $SOURCE)" ;; esac
case "$DEST" in cmm|hold|burn) ;; *) die "CMM_USDT_BUY_DEST must be cmm, hold, or burn (got $DEST)" ;; esac

# Live default: keep running until the CMM USDT floor is reached. Dry-run is one tick.
if [[ -z "${CMM_USDT_BUY_LOOP:-}" ]]; then
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    CMM_USDT_BUY_LOOP=0
  else
    CMM_USDT_BUY_LOOP=1
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

raw_human() {
  python3 -c "from decimal import Decimal; print(Decimal('$1') / (Decimal(10) ** $2))"
}

human_raw() {
  python3 -c "from decimal import Decimal, ROUND_DOWN; print(int((Decimal('$1') * (Decimal(10) ** $2)).to_integral_value(rounding=ROUND_DOWN)))"
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

spender_remaining() {
  lcd_smart "$TREASURY" "$(jq -nc --arg t "$USDT" --arg s "$SPENDER" \
    '{cw20_spender_limit:{token:$t,spender:$s}}')" | jq -r '.remaining // "0"'
}

spender_registered() {
  local raw
  raw="$(lcd_smart_try "$TREASURY" '{"cw20_spenders":{}}')" || return 1
  jq -e --arg t "$USDT" --arg s "$SPENDER" \
    'any(.spenders[]?; (.token == $t) and (.spender == $s))' >/dev/null <<<"$raw"
}

# Sequential pair hybrid_simulation (same as the sibling buybacks). Prints JSON array.
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

broadcast_admin() {
  terrad_host_attest || return 1
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

# 2-of-3 governance tx (treasury Config.governance is the multisig).
broadcast_msig() {
  terrad_host_attest || return 1
  local label="$1" contract="$2" msg="$3"
  local workdir unsigned sig1 sig2 signed out tx_hash code
  echo "  → msig $label" >&2
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "    [DRY_RUN] wasm execute $contract $msg" >&2
    echo "DRY_RUN_TX"
    return 0
  fi
  workdir="$(mktemp -d "${TMPDIR:-/tmp}/cmm-usdt-buy-msig.XXXXXX")"
  unsigned="$workdir/unsigned.json"
  sig1="$workdir/sig1.json"
  sig2="$workdir/sig2.json"
  signed="$workdir/signed.json"
  # shellcheck disable=SC2046
  terrad_host_exec_retry tx wasm execute "$contract" "$msg" \
    --from "$MSIG_KEY" \
    --generate-only \
    $(terrad_host_common_flags) \
    $(terrad_host_gas_flags) \
    $(terrad_host_fee_flags) \
    --output json >"$unsigned"
  jq -e '.body.messages | length > 0' "$unsigned" >/dev/null \
    || die "unsigned governance tx invalid ($label)"
  # shellcheck disable=SC2046
  terrad_host_exec_retry tx sign "$unsigned" \
    --from "$SIGNER1" --multisig "$MSIG_KEY" --sign-mode amino-json \
    $(terrad_host_common_flags) --output json >"$sig1"
  # shellcheck disable=SC2046
  terrad_host_exec_retry tx sign "$unsigned" \
    --from "$SIGNER2" --multisig "$MSIG_KEY" --sign-mode amino-json \
    $(terrad_host_common_flags) --output json >"$sig2"
  # shellcheck disable=SC2046
  terrad_host_exec_retry tx multisign "$unsigned" "$MSIG_KEY" "$sig1" "$sig2" \
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

load_state() {
  if [[ -f "$STATE_FILE" ]]; then
    jq -c '.' "$STATE_FILE"
  else
    echo '{"bought_raw":"0","spent_raw":"0","rounds":0}'
  fi
}

save_state() {
  local bought_raw="$1" spent_raw="$2" rounds="$3"
  mkdir -p "$(dirname "$STATE_FILE")"
  jq -nc --arg b "$bought_raw" --arg s "$spent_raw" --argjson n "$rounds" \
    '{bought_raw:$b,spent_raw:$s,rounds:$n}' >"$STATE_FILE"
}

quote_best() {
  local amount="$1" url raw body code st i
  local attempts="${CMM_USDT_BUY_QUOTE_RETRIES:-4}"
  local delay="${CMM_USDT_BUY_QUOTE_RETRY_DELAY_SEC:-3}"
  url="${INDEXER}/api/v1/route/solve/best?token_in=${USDT}&token_out=${UST1}&amount_in=${amount}&trader=${ADMIN_ADDR}&sender=${ADMIN_ADDR}"
  echo "  GET $url" >&2
  raw=""
  for ((i = 1; i <= attempts; i++)); do
    set +e
    raw="$(curl -sS --connect-timeout 10 --max-time "$QUOTE_TIMEOUT" -H "User-Agent: $HTTP_UA" -w $'\n%{http_code}' "$url" 2>/dev/null)"
    st=$?
    set -e
    code="${raw##*$'\n'}"
    body="${raw%$'\n'*}"
    if [[ "$st" == "0" && "$code" == "200" ]]; then
      raw="$body"
      break
    fi
    if [[ "$i" -eq "$attempts" ]]; then
      die "indexer quote request failed (HTTP ${code:-?}, curl ${st}) after $attempts attempts"
    fi
    if [[ "$code" =~ ^(429|500|502|503|504)$ || "$st" != "0" ]]; then
      echo "  [quote] transient HTTP ${code:-?} (attempt $i/$attempts), retry in ${delay}s" >&2
      sleep "$delay"
      delay=$((delay + 3))
      continue
    fi
    die "indexer quote request failed (HTTP ${code:-?})"
  done
  if ! jq -e '.router_operations | type == "array" and length > 0' >/dev/null <<<"$raw"; then
    echo "$raw" | jq -c '.' >&2 || true
    die "indexer returned no router_operations"
  fi
  jq -e '.estimated_amount_out != null and (.estimated_amount_out | tonumber) > 0' >/dev/null <<<"$raw" \
    || die "indexer quote missing estimated_amount_out"
  jq -e --arg offer "$USDT" --arg ask "$UST1" '
    .router_operations[0].terra_swap.offer_asset_info.token.contract_addr == $offer and
    .router_operations[-1].terra_swap.ask_asset_info.token.contract_addr == $ask
  ' >/dev/null <<<"$raw" || die "indexer quote token endpoints do not match requested swap"
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
  local got
  got="$(terrad_host_key_address)" || exit 1
  [[ "$got" == "$ADMIN_ADDR" ]] || die "admin key $ADMIN_KEY is $got (want $ADMIN_ADDR)"
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

print_header() {
  echo "=============================================="
  echo "CMM USDT → UST1 best-solver drip"
  echo "=============================================="
  echo "Router:      $ROUTER"
  echo "RPC:         $TERRAD_HOST_NODE"
  echo "CMM:         $TREASURY"
  echo "Admin key:   $ADMIN_KEY ($ADMIN_ADDR)"
  echo "Spender:     $SPENDER"
  echo "Source:      $SOURCE  (per tick $(printf '%s' "$PER_TICK_USDT") USDT)"
  echo "Floor:       $FLOOR_USDT USDT (stop when CMM USDT <= floor)"
  echo "Dest:        $DEST"
  echo "Interval:    ${INTERVAL_SEC}s after each tick  LOOP=${CMM_USDT_BUY_LOOP}"
  echo "Slippage:    ${SLIP}%"
  echo "DRY_RUN:     ${DRY_RUN:-0}"
  echo "State:       $STATE_FILE"
}

stop_if_floor() {
  local cmm_usdt="$1"
  echo "  CMM USDT=$cmm_usdt ($(raw_human "$cmm_usdt" "$DEC_USDT") USDT)"
  if python3 -c "import sys; sys.exit(0 if int('$cmm_usdt') <= int('$FLOOR_RAW') else 1)"; then
    echo "OK — CMM USDT floor reached ($(raw_human "$cmm_usdt" "$DEC_USDT") <= $FLOOR_USDT)."
    return 10
  fi
  return 0
}

pull_from_treasury() {
  local need="$1" msg
  msg="$(jq -nc --arg r "$ADMIN_ADDR" --arg t "$USDT" --arg a "$need" \
    '{instant_withdraw_cw20:{recipient:$r,token:$t,amount:$a}}')"
  broadcast_admin "InstantWithdrawCw20 $need USDT CMM → admin" \
    wasm execute "$TREASURY" "$msg" >/dev/null
}

one_tick() {
  terrad_host_attest || return 1
  local cmm_usdt tick_raw excess have need quote est min_recv hops hop_returns
  local hook_json hook_b64 send_msg before after delta left xfer cmm_ust1_before cmm_ust1_after
  local remaining
  local state bought_raw spent_raw rounds

  echo ""
  echo "[balance]"
  cmm_usdt="$(cw20_balance "$USDT" "$TREASURY")"
  stop_if_floor "$cmm_usdt" || return $?

  excess="$(python3 -c "x=int('$cmm_usdt')-int('$FLOOR_RAW'); print(x if x>0 else 0)")"
  tick_raw="$PER_TICK_RAW"
  if python3 -c "import sys; sys.exit(0 if int('$tick_raw') > int('$excess') else 1)"; then
    tick_raw="$excess"
  fi
  echo "  tick=$(raw_human "$tick_raw" "$DEC_USDT") USDT raw=$tick_raw (excess=$(raw_human "$excess" "$DEC_USDT"))"
  [[ "$tick_raw" != "0" ]] || { echo "  nothing to spend above floor"; return 0; }

  echo "[fund]"
  have="$(cw20_balance "$USDT" "$ADMIN_ADDR")"
  if [[ "$SOURCE" == "wallet" ]]; then
    need="0"
  else
    need="$(python3 -c "x=int('$tick_raw')-int('$have'); print(x if x>0 else 0)")"
  fi
  echo "  admin USDT=$(raw_human "$have" "$DEC_USDT") need=$(raw_human "$need" "$DEC_USDT")"
  if [[ "$need" != "0" ]]; then
    if [[ "${DRY_RUN:-0}" == "1" ]]; then
      echo "  [DRY_RUN] would InstantWithdrawCw20 $need USDT from CMM to admin"
    else
      spender_registered || die "admin $SPENDER is not the registered USDT spender on CMM; run --setup-spender (DEX 2-of-3)"
      remaining="$(spender_remaining)"
      echo "  spender 24h remaining=$(raw_human "$remaining" "$DEC_USDT") USDT"
      if python3 -c "import sys; sys.exit(0 if int('$need') > int('$remaining') else 1)"; then
        need="$remaining"
      fi
      [[ "$need" != "0" ]] || die "USDT spender 24h pull limit exhausted; re-run --setup-spender to raise it"
      echo "  pull=$(raw_human "$need" "$DEC_USDT") USDT (clamped to remaining)"
      pull_from_treasury "$need"
    fi
  fi

  echo "[quote] $tick_raw USDT → UST1 via /route/solve/best"
  quote="$(quote_best "$tick_raw")"
  hops="$(jq -r '.hops | length' <<<"$quote")"
  est="$(jq -r '.estimated_amount_out' <<<"$quote")"
  echo "  hops=$hops kind=$(jq -r '.quote_kind // empty' <<<"$quote") out=$(raw_human "$est" "$DEC_UST1") UST1 solver=$(jq -r '.solver_version // empty' <<<"$quote")"
  echo "  path=$(jq -r '[.hops[]?.pair] | join(" → ")' <<<"$quote")"
  min_recv="$(python3 "$HOOK_PY" min-receive --out "$est" --slip "$SLIP")"
  [[ "$min_recv" != "0" ]] || die "min_receive is 0 (quote too small for slippage $SLIP%)"
  echo "  min_receive=$(raw_human "$min_recv" "$DEC_UST1") UST1 (${SLIP}% floor of $est)"

  TERRAD_HOST_IDENTITY_CONTRACTS+=" $(jq -r '[(.hops[]?.pair // empty), (.router_operations[]?.terra_swap.offer_asset_info.token.contract_addr // empty), (.router_operations[]?.terra_swap.ask_asset_info.token.contract_addr // empty)] | unique | join(" ")' <<<"$quote")"
  terrad_host_attest || return 1
  echo "[hop-sim] pair hybrid_simulation for #334 book min_return"
  hop_returns="$(sim_hop_returns "$quote" "$tick_raw")"
  echo "  hop_returns=$hop_returns"

  before="$(cw20_balance "$UST1" "$ADMIN_ADDR")"
  local qfile
  qfile="$(mktemp "${TMPDIR:-/tmp}/cmm-usdt-buy-quote.XXXXXX.json")"
  printf '%s' "$quote" >"$qfile"
  hook_json="$(python3 "$HOOK_PY" send-msg \
    --quote-file "$qfile" \
    --amount "$tick_raw" \
    --router "$ROUTER" \
    --max-spread "$MAX_SPREAD" \
    --min-receive "$min_recv" \
    --slip "$SLIP" \
    --hop-returns "$hop_returns")"
  rm -f "$qfile"
  hook_b64="$(lcd_b64 "$hook_json")"
  send_msg="$(jq -nc --arg c "$ROUTER" --arg a "$tick_raw" --arg m "$hook_b64" \
    '{send:{contract:$c,amount:$a,msg:$m}}')"
  echo "  hop min_return=$(jq -c '[.execute_swap_operations.operations[] | .terra_swap.min_return // null]' <<<"$hook_json")"

  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "  [DRY_RUN] would swap $tick_raw USDT via router ($hops hops, min $(raw_human "$min_recv" "$DEC_UST1") UST1)"
    case "$DEST" in
      cmm) echo "  [DRY_RUN] would transfer received UST1 to CMM" ;;
      burn) echo "  [DRY_RUN] would burn received UST1" ;;
      hold) echo "  [DRY_RUN] would leave received UST1 on admin" ;;
    esac
    return 0
  fi

  have="$(cw20_balance "$USDT" "$ADMIN_ADDR")"
  python3 -c "import sys; sys.exit(0 if int('$have') >= int('$tick_raw') else 1)" \
    || die "admin USDT $have < tick $tick_raw"

  broadcast_admin "solver swap $tick_raw USDT → UST1 ($hops hops)" \
    wasm execute "$USDT" "$send_msg" >/dev/null

  after="$(cw20_balance "$UST1" "$ADMIN_ADDR")"
  delta="$(python3 "$HOOK_PY" delta --before "$before" --after "$after")"
  [[ "$delta" != "0" ]] || die "swap succeeded but admin UST1 did not increase (before=$before after=$after)"
  echo "  received UST1=$(raw_human "$delta" "$DEC_UST1")"

  case "$DEST" in
    cmm)
      cmm_ust1_before="$(cw20_balance "$UST1" "$TREASURY")"
      xfer="$(jq -nc --arg r "$TREASURY" --arg a "$delta" '{transfer:{recipient:$r,amount:$a}}')"
      [[ "$ADMIN_ADDR" != "$TREASURY" ]] || die "refusing to transfer from CMM to itself"
      broadcast_admin "transfer UST1 $delta → CMM" wasm execute "$UST1" "$xfer" >/dev/null
      cmm_ust1_after="$(cw20_balance "$UST1" "$TREASURY")"
      python3 -c "import sys; sys.exit(0 if int('$cmm_ust1_after') > int('$cmm_ust1_before') else 1)" \
        || die "CMM UST1 did not increase ($cmm_ust1_before → $cmm_ust1_after)"
      left="$(cw20_balance "$UST1" "$ADMIN_ADDR")"
      python3 -c "import sys; sys.exit(0 if int('$left') == int('$before') else 1)" \
        || die "admin UST1 after CMM transfer=$left (want pre-swap $before)"
      echo "  CMM UST1 $(raw_human "$cmm_ust1_before" "$DEC_UST1") → $(raw_human "$cmm_ust1_after" "$DEC_UST1")"
      ;;
    burn)
      [[ "$ADMIN_ADDR" != "$TREASURY" ]] || die "refusing to burn on CMM treasury"
      local burn
      burn="$(jq -nc --arg a "$delta" '{burn:{amount:$a}}')"
      broadcast_admin "burn UST1 $delta" wasm execute "$UST1" "$burn" >/dev/null
      left="$(cw20_balance "$UST1" "$ADMIN_ADDR")"
      python3 -c "import sys; sys.exit(0 if int('$left') == int('$before') else 1)" \
        || die "admin UST1 after burn=$left (want pre-swap $before)"
      echo "  burned UST1=$delta"
      ;;
    hold)
      echo "  holding UST1=$delta on admin (not CMM, not burned)"
      ;;
  esac

  state="$(load_state)"
  bought_raw="$(jq -r '.bought_raw // "0"' <<<"$state")"
  spent_raw="$(jq -r '.spent_raw // "0"' <<<"$state")"
  rounds="$(jq -r '.rounds // 0' <<<"$state")"
  bought_raw="$(python3 -c "print(int('$bought_raw') + int('$delta'))")"
  spent_raw="$(python3 -c "print(int('$spent_raw') + int('$tick_raw'))")"
  rounds=$((rounds + 1))
  save_state "$bought_raw" "$spent_raw" "$rounds"
  echo "  campaign bought=$(raw_human "$bought_raw" "$DEC_UST1") UST1 spent=$(raw_human "$spent_raw" "$DEC_USDT") USDT rounds=$rounds"

  cmm_usdt="$(cw20_balance "$USDT" "$TREASURY")"
  stop_if_floor "$cmm_usdt" || return $?
  return 0
}

setup_spender() {
  print_header
  local limit_raw
  limit_raw="$(human_raw "$PULL_LIMIT_USDT" "$DEC_USDT")"
  echo "[setup] register CMM USDT spender $SPENDER (24h limit $(printf '%s' "$PULL_LIMIT_USDT") USDT)"
  local msg
  msg="$(jq -nc --arg t "$USDT" --arg s "$SPENDER" --arg l "$limit_raw" \
    '{set_cw20_spender:{token:$t,spender:$s,limit_24h:$l}}')"
  broadcast_msig "register USDT spender $SPENDER" "$TREASURY" "$msg" >/dev/null
  spender_registered || die "spender registration did not take"
  echo "  registered; remaining=$(raw_human "$(spender_remaining)" "$DEC_USDT") USDT"
}

show_status() {
  print_header
  echo ""
  echo "CMM USDT=$(raw_human "$(cw20_balance "$USDT" "$TREASURY")" "$DEC_USDT")"
  echo "CMM UST1=$(raw_human "$(cw20_balance "$UST1" "$TREASURY")" "$DEC_UST1")"
  echo "admin USDT=$(raw_human "$(cw20_balance "$USDT" "$ADMIN_ADDR")" "$DEC_USDT")"
  if spender_registered; then
    echo "spender registered: yes (remaining $(raw_human "$(spender_remaining)" "$DEC_USDT") USDT/24h)"
  else
    echo "spender registered: no (run --setup-spender with CMM_USDT_BUY_YES=1)"
  fi
}

# ---------------------------------------------------------------------------
python3 "$HOOK_PY" self-test >/dev/null

print_header

echo "[preflight] tokens + spender"
terrad_host_attest || die "chain/protocol trust preflight failed"
DEC_USDT="$(token_decimals "$USDT")"
DEC_UST1="$(token_decimals "$UST1")"
echo "  USDT symbol=$(token_symbol "$USDT") decimals=$DEC_USDT"
echo "  UST1 symbol=$(token_symbol "$UST1") decimals=$DEC_UST1"
PER_TICK_RAW="$(human_raw "$PER_TICK_USDT" "$DEC_USDT")"
FLOOR_RAW="$(human_raw "$FLOOR_USDT" "$DEC_USDT")"
[[ "$PER_TICK_RAW" != "0" ]] || die "CMM_USDT_BUY_USDT_PER_TICK rounds to 0 raw"
[[ "$ADMIN_ADDR" != "$TREASURY" ]] || die "admin must not be CMM treasury"

if [[ "${1:-}" == "--setup-spender" ]]; then
  ensure_unlocked
  if [[ "${DRY_RUN:-0}" != "1" && "${CMM_USDT_BUY_YES:-0}" != "1" ]]; then
    if [[ ! -t 0 ]]; then
      die "refusing live broadcast without TTY; set CMM_USDT_BUY_YES=1"
    fi
    read -r -p "Register $SPENDER as the CMM USDT spender on $TERRAD_HOST_CHAIN_ID? [y/N] " ans
    [[ "$ans" == "y" || "$ans" == "Y" ]] || die "aborted"
  fi
  setup_spender
  exit 0
fi

if [[ "${1:-}" == "--status" ]]; then
  show_status
  exit 0
fi

ensure_unlocked

if [[ "${DRY_RUN:-0}" != "1" && "${CMM_USDT_BUY_YES:-0}" != "1" ]]; then
  if [[ ! -t 0 ]]; then
    die "refusing live broadcast without TTY; set CMM_USDT_BUY_YES=1"
  fi
  read -r -p "Broadcast USDT pull / solver swap / $DEST on $TERRAD_HOST_CHAIN_ID? [y/N] " ans
  [[ "$ans" == "y" || "$ans" == "Y" ]] || die "aborted"
fi

if [[ "${CMM_USDT_BUY_LOOP}" == "1" ]]; then
  trap 'echo; echo "stopped."; exit 0' INT TERM
  while true; do
    set +e
    # Subshell so a tick failure does not kill the loop.
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
( set -euo pipefail; one_tick )
st=$?
set -e
if [[ "$st" == "10" || "$st" == "0" ]]; then
  exit 0
fi
exit "$st"
