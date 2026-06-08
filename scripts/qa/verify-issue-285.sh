#!/usr/bin/env bash
# Verification for GitLab #285 — limit-order lifecycle parser must not trust forgeable
# `contract_address` (no underscore) for emitter scoping; only runtime `_contract_address`.
#
# Layers:
#   1. Parser unit tests (forged fill/cancel/placement + genuine dual-key regression)
#   2. Integration: limit_order_parked_lifecycle (_contract_address fixtures)
#   3. Live LocalTerra: hybrid swap → on-chain limit_order_fill → indexer limit-fills API
#
# Refs: indexer/src/indexer/parser.rs, docs/indexer-invariants.md,
#       skills/AGENTS_INDEXER_LIFECYCLE_EMITTER_SCOPING.md
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/lib/lcd-smart-query.sh
source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
# shellcheck source=scripts/lib/localterra-host-curl.sh
source "$REPO_ROOT/scripts/lib/localterra-host-curl.sh"
# shellcheck source=scripts/lib/e2e-terrad-tx.sh
source "$REPO_ROOT/scripts/lib/e2e-terrad-tx.sh"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

run_step() {
  local label="$1"
  shift
  echo ""
  echo "[$label]"
  if "$@"; then
    ok "$label"
  else
    bad "$label"
  fi
}

read_env_var() { sed -n "s/^${2}=//p" "$1" 2>/dev/null | head -1; }

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #285 — lifecycle emitter scoping (_contract_address only)"
echo "════════════════════════════════════════════════════════════════"

run_step "parser forged-attack + regression unit tests" \
  bash -c 'cd indexer && cargo test --lib forged_contract_address -- --quiet && \
    cargo test --lib genuine_fill_with_both_contract_address -- --quiet'

run_step "integration limit_order_parked_lifecycle (_contract_address fixtures)" \
  bash -c 'cd indexer && cargo test --test limit_order_parked_lifecycle -j 1 -- --test-threads=1 --quiet'

CONTAINER_NAME="$(localterra_container_id "$REPO_ROOT")"
IDX_ENV="$REPO_ROOT/indexer/.env"
FE_ENV="$REPO_ROOT/frontend-dapp/.env.local"

if [[ -z "$CONTAINER_NAME" ]]; then
  echo ""
  echo "[live] SKIP — localterra not running."
  RESULTS+=("SKIP  live stack (localterra not running)")
elif [[ ! -f "$IDX_ENV" || ! -f "$FE_ENV" ]]; then
  echo ""
  echo "[live] SKIP — deploy env missing (run make deploy-local)."
  RESULTS+=("SKIP  live stack (deploy env files missing)")
else
  LCD="$(read_env_var "$IDX_ENV" LCD_URLS)"; LCD="${LCD%%,*}"
  API_PORT="$(read_env_var "$IDX_ENV" API_PORT)"
  LCD="${LCD:-http://127.0.0.1:1317}"; LCD="${LCD%/}"
  API_PORT="${API_PORT:-3001}"
  INDEXER="http://127.0.0.1:${API_PORT}"

  terrad_tx() { e2e_terrad_tx "$CONTAINER_NAME" "$@"; }
  wait_tx() { sleep 4; }

  b64_query() {
    echo -n "$1" | base64 -w0 2>/dev/null || echo -n "$1" | base64
  }

  decode_smart_payload() {
    local raw="$1"
    local data_type
    data_type=$(echo "$raw" | jq -r '.data | type')
    if [[ "$data_type" == "string" ]]; then
      echo "$raw" | jq -r '.data | @base64d | fromjson'
    else
      echo "$raw" | jq '.data'
    fi
  }

  tx_hash_from_json() {
    sed -n '/^{/,$p' | tail -1 | jq -r '.txhash // .tx_response.txhash // empty'
  }

  query_tx_lcd() {
    local txhash="$1"
    local attempts=0
    local max="${VERIFY285_TX_QUERY_ATTEMPTS:-20}"
    while (( attempts < max )); do
      local json
      json="$(localterra_lcd_curl "$LCD" "/cosmos/tx/v1beta1/txs/${txhash}" 2>/dev/null || true)"
      if [[ -n "$json" ]] && echo "$json" | jq -e '.tx_response.txhash // .txhash' >/dev/null 2>&1; then
        echo "$json"
        return 0
      fi
      sleep 2
      attempts=$((attempts + 1))
    done
    return 1
  }

  tx_has_wasm_action() {
    local txjson="$1"
    local action="$2"
    echo "$txjson" | jq -e --arg act "$action" '
      [(.tx_response.logs[]?.events[]?, .tx_response.events[]?)
       | select(.type == "wasm" or .type == "wasm-wasm")
       | .attributes[]
       | select(.key == "action" and .value == $act)] | length > 0
    ' >/dev/null
  }

  tx_wasm_attr_before_action() {
    # Last attribute value for key before the first action=want in document order within wasm events.
    local txjson="$1"
    local want_action="$2"
    local want_key="$3"
    echo "$txjson" | jq -r --arg act "$want_action" --arg k "$want_key" '
      [(.tx_response.logs[]?.events[]?, .tx_response.events[]?)
       | select(.type == "wasm" or .type == "wasm-wasm")
       | . as $ev
       | ($ev.attributes | to_entries | map(select(.value.key == "action" and .value.value == $act)) | .[0].key // null) as $idx
       | if $idx == null then empty
         else [ range(0; $idx) as $i | $ev.attributes[$i] | select(.key == $k) | .value ] | last // empty
         end] | .[0] // empty
    '
  }

  echo ""
  echo "[live] Starting indexer + hybrid book seed..."
  if ! bash "$REPO_ROOT/scripts/e2e-start-indexer.sh"; then
    bad "live: e2e-start-indexer.sh"
  else
    ok "live: indexer ready at ${INDEXER}"
  fi

  if bash "$REPO_ROOT/scripts/e2e-provision-dev-wallet.sh" >/dev/null 2>&1; then
    ok "live: dev wallet provisioned"
  else
    bad "live: e2e-provision-dev-wallet.sh"
  fi

  if bash "$REPO_ROOT/scripts/e2e-seed-hybrid-book.sh"; then
    ok "live: hybrid book seed (resting bid)"
  else
    bad "live: e2e-seed-hybrid-book.sh"
  fi

  RAW_PAIRS="$(lcd_smart_query_raw "$LCD" "$(read_env_var "$FE_ENV" VITE_FACTORY_ADDRESS)" '{"pairs":{"start_after":null,"limit":60}}')"
  PAIRS_DOC="$(decode_smart_payload "$RAW_PAIRS")"
  PAIR_ADDR=""
  TOKEN0=""
  TOKEN1=""
  while IFS= read -r row; do
    [[ -n "$row" ]] || continue
    PAIR_ADDR="$(echo "$row" | jq -r '.contract_addr')"
    T0="$(echo "$row" | jq -r '.asset_infos[0].token.contract_addr // empty')"
    T1="$(echo "$row" | jq -r '.asset_infos[1].token.contract_addr // empty')"
    if [[ "$T0" =~ ^terra1 && "$T1" =~ ^terra1 ]]; then
      TOKEN0="$T0"
      TOKEN1="$T1"
      break
    fi
  done < <(echo "$PAIRS_DOC" | jq -c '.pairs[]')

  if [[ -z "$PAIR_ADDR" || -z "$TOKEN0" ]]; then
    bad "live: no dual-CW20 pair on factory"
  else
    ok "live: using pair ${PAIR_ADDR:0:18}… token0 ${TOKEN0:0:12}…"

  BOOK_INPUT="${VERIFY285_BOOK_INPUT:-1000000}"
  HYBRID_JSON="$(jq -nc --arg amt "$BOOK_INPUT" \
    '{pool_input:"0",book_input:$amt,max_maker_fills:8,book_start_hint:null}')"
  SWAP_HOOK="$(jq -nc --argjson hybrid "$HYBRID_JSON" \
    '{swap:{belief_price:null,min_return:"1",max_spread:"0.5",to:null,deadline:null,hybrid:$hybrid,trader:null}}' \
    | base64 -w0 2>/dev/null || jq -nc --argjson hybrid "$HYBRID_JSON" \
    '{swap:{belief_price:null,min_return:"1",max_spread:"0.5",to:null,deadline:null,hybrid:$hybrid,trader:null}}' | base64)"
  SEND_MSG="$(jq -nc --arg pair "$PAIR_ADDR" --arg amt "$BOOK_INPUT" --arg hook "$SWAP_HOOK" \
    '{send:{contract:$pair,amount:$amt,msg:$hook}}')"

  echo ""
  echo "[live] Broadcasting hybrid swap (book leg fills resting bid)..."
  SWAP_TX="$(terrad_tx wasm execute "$TOKEN0" "$SEND_MSG" | tx_hash_from_json)" || SWAP_TX=""
  wait_tx
  if [[ -z "$SWAP_TX" ]]; then
    bad "live: hybrid swap broadcast (no txhash)"
  else
    ok "live: hybrid swap tx ${SWAP_TX:0:16}…"
  fi

  TX_JSON=""
  if TX_JSON="$(query_tx_lcd "$SWAP_TX")"; then
    ok "live: LCD returned tx JSON"
  else
    bad "live: LCD tx query timed out for ${SWAP_TX}"
  fi

  if [[ -n "$TX_JSON" ]]; then
    if tx_has_wasm_action "$TX_JSON" "limit_order_fill"; then
      ok "live: on-chain tx includes wasm action=limit_order_fill"
    else
      bad "live: tx missing limit_order_fill (hybrid book leg did not fill)"
    fi

    EMITTER="$(tx_wasm_attr_before_action "$TX_JSON" "limit_order_fill" "_contract_address")"
    CONV_ADDR="$(echo "$TX_JSON" | jq -r '
      [(.tx_response.logs[]?.events[]?, .tx_response.events[]?)
       | select(.type == "wasm" or .type == "wasm-wasm")
       | .attributes[]
       | select(.key == "contract_address") | .value] | last // empty
    ')"
    if [[ "$EMITTER" == "$PAIR_ADDR" ]]; then
      ok "live: limit_order_fill scoped by _contract_address=${PAIR_ADDR:0:18}… (pair)"
    else
      bad "live: _contract_address before fill action=$EMITTER expected=$PAIR_ADDR"
    fi
    if [[ -n "$CONV_ADDR" && "$CONV_ADDR" == "$PAIR_ADDR" ]]; then
      ok "live: genuine tx also carries pair convenience contract_address (same value)"
    elif [[ -n "$CONV_ADDR" ]]; then
      echo "    note: contract_address=$CONV_ADDR (convenience attr; not used for scoping)"
    fi
  fi

  echo ""
  echo "[live] Polling indexer GET /api/v1/pairs/{pair}/limit-fills for tx ${SWAP_TX:0:16}…"
  FOUND=0
  for i in $(seq 1 90); do
    FILLS="$(curl -sf "${INDEXER}/api/v1/pairs/${PAIR_ADDR}/limit-fills?limit=50" 2>/dev/null || echo '[]')"
    if echo "$FILLS" | jq -e --arg tx "$SWAP_TX" --arg pair "$PAIR_ADDR" \
      '[.[] | select(.tx_hash == $tx and .pair_address == $pair)] | length > 0' >/dev/null 2>&1; then
      FOUND=1
      break
    fi
    sleep 2
  done
  if [[ "$FOUND" -eq 1 ]]; then
    ok "live: indexer limit-fills row for tx attributed to pair ${PAIR_ADDR:0:18}…"
  else
    bad "live: indexer did not list limit fill for hybrid swap tx within 180s"
    echo "    last response: $(echo "$FILLS" | jq -c 'if type == "array" then length else . end' 2>/dev/null || echo ERR)" >&2
  fi
  fi
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: ${PASS} passed, ${FAIL} failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"
[[ "$FAIL" -eq 0 ]]
