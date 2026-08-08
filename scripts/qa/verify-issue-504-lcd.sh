#!/usr/bin/env bash
# LocalTerra LCD smoke for GitLab #504 — place dust rung, hybrid fill, assert reason=DustFilled.
# Invoked by verify-issue-504.sh when VERIFY504_LCD=1.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/lib/lcd-smart-query.sh
source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
# shellcheck source=scripts/lib/e2e-terrad-tx.sh
source "$REPO_ROOT/scripts/lib/e2e-terrad-tx.sh"

if ! make has-localterra >/dev/null 2>&1; then
  echo "ERROR: LocalTerra not up" >&2
  exit 1
fi

CONTAINER_NAME="$(sg docker -c 'docker compose ps -q localterra' 2>/dev/null | head -1)"
if [[ -z "$CONTAINER_NAME" ]]; then
  CONTAINER_NAME="$(docker compose ps -q localterra 2>/dev/null | head -1)"
fi
[[ -n "$CONTAINER_NAME" ]] || { echo "ERROR: no localterra container" >&2; exit 1; }

read_env_var() { sed -n "s/^${2}=//p" "$1" 2>/dev/null | head -1; }

IDX_ENV="$REPO_ROOT/indexer/.env"
FE_ENV="$REPO_ROOT/frontend-dapp/.env.local"
FACTORY="$(read_env_var "$IDX_ENV" FACTORY_ADDRESS)"
LCD="$(read_env_var "$IDX_ENV" LCD_URLS)"; LCD="${LCD%%,*}"
LCD="${LCD:-http://127.0.0.1:1317}"; LCD="${LCD%/}"

if [[ -z "$FACTORY" && -f "$FE_ENV" ]]; then
  FACTORY="$(read_env_var "$FE_ENV" VITE_FACTORY_ADDRESS)"
  LCD="$(read_env_var "$FE_ENV" VITE_TERRA_LCD_URL)"
  LCD="${LCD:-http://127.0.0.1:1317}"; LCD="${LCD%/}"
fi
LCD="${LCD//localhost/127.0.0.1}"

[[ -n "$FACTORY" ]] || { echo "ERROR: FACTORY missing — run make deploy-local" >&2; exit 1; }

terrad_tx() { e2e_terrad_tx "$CONTAINER_NAME" "$@"; }

tx_hash_from_json() {
  # terrad may prefix "gas estimate: N" on stdout; take the last JSON object only.
  local raw
  raw="$(cat)"
  echo "$raw" | sed -n '/^{/,$p' | tail -1 | jq -r '.txhash // .tx_response.txhash // empty' 2>/dev/null \
    || echo "$raw" | tr '\n' ' ' | grep -oE '\{.*\}' | tail -1 | jq -r '.txhash // .tx_response.txhash // empty' 2>/dev/null \
    || true
}

query_tx_json() {
  local txhash="$1"
  local attempts=0
  local max=20
  local json=""
  [[ -n "$txhash" ]] || return 1
  while (( attempts < max )); do
    json="$(docker exec "$CONTAINER_NAME" terrad query tx "$txhash" --node http://127.0.0.1:26657 --output json 2>/dev/null || true)"
    if [[ -n "$json" ]] && echo "$json" | jq -e '.txhash // .tx_response.txhash // .hash' >/dev/null 2>&1; then
      echo "$json"
      return 0
    fi
    sleep 2
    attempts=$((attempts + 1))
  done
  return 1
}

tx_wasm_attr() {
  local txhash="$1"
  local key="$2"
  local json
  json="$(query_tx_json "$txhash")" || return 1
  echo "$json" | jq -r --arg k "$key" \
    '[(.events // .logs[]?.events // [])[] | select(.type | test("wasm")) | .attributes[]? | select(.key == $k) | .value] | last // empty'
}

pairs_doc="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$FACTORY" '{"pairs":{"start_after":null,"limit":30}}')")"
PAIR=""
TOKEN0=""
TOKEN1=""
while IFS= read -r row; do
  t0="$(echo "$row" | jq -r '.asset_infos[0].token.contract_addr // empty')"
  t1="$(echo "$row" | jq -r '.asset_infos[1].token.contract_addr // empty')"
  if [[ "$t0" =~ ^terra1 && "$t1" =~ ^terra1 ]]; then
    PAIR="$(echo "$row" | jq -r '.contract_addr')"
    TOKEN0="$t0"
    TOKEN1="$t1"
    break
  fi
done < <(echo "$pairs_doc" | jq -c '.pairs[]? // empty')

[[ -n "$PAIR" ]] || { echo "ERROR: no dual-CW20 pair on factory" >&2; exit 1; }
echo "  pair=$PAIR token0=$TOKEN0 token1=$TOKEN1"

if ! lcd_smart_query_ok "$LCD" "$PAIR" '{"expired_limit_refund":{"order_id":1}}'; then
  echo "ERROR: expired_limit_refund query rejected (stale wasm?)" >&2
  exit 1
fi
echo "  expired_limit_refund variant OK"

# Mirror limit_order_tests::match_dust_flush_bid_hybrid_then_maker_claims
# price=1.05, fill=94380952 -> cost=99099999, escrow=99100000 -> dust remaining=1
FILL=94380952
ESCROW=99100000
PRICE=1.05

place_hook="$(jq -nc --arg price "$PRICE" --arg amt "$ESCROW" \
  '{place_limit_order_batch:{side:"bid",orders:[{price:$price,amount:$amt,max_adjust_steps:32}]}}')"
place_tx="$(terrad_tx wasm execute "$TOKEN1" "$(jq -nc \
  --arg pair "$PAIR" --arg amt "$ESCROW" \
  --arg hook "$(echo -n "$place_hook" | base64 -w0 2>/dev/null || echo -n "$place_hook" | base64)" \
  '{send:{contract:$pair,amount:$amt,msg:$hook}}')")"
place_hash="$(echo "$place_tx" | tx_hash_from_json)"
ORDER_ID="$(tx_wasm_attr "$place_hash" order_id || true)"
[[ -n "$ORDER_ID" && "$ORDER_ID" != "null" ]] || {
  echo "ERROR: place bid failed (tx=$place_hash)" >&2
  echo "$place_tx" | tail -c 800 >&2
  exit 1
}
echo "  placed order_id=$ORDER_ID"

swap_hook="$(jq -nc --argjson book "$FILL" \
  '{swap:{belief_price:null,max_spread:"1",min_return:"1",to:null,deadline:null,hybrid:{pool_input:"0",book_input:($book|tostring),max_maker_fills:8,book_start_hint:null},trader:null}}')"
swap_tx="$(terrad_tx wasm execute "$TOKEN0" "$(jq -nc \
  --arg pair "$PAIR" --arg amt "$FILL" \
  --arg hook "$(echo -n "$swap_hook" | base64 -w0 2>/dev/null || echo -n "$swap_hook" | base64)" \
  '{send:{contract:$pair,amount:$amt,msg:$hook}}')")"
swap_hash="$(echo "$swap_tx" | tx_hash_from_json)"
reason_attr="$(tx_wasm_attr "$swap_hash" reason || true)"
force_attr="$(tx_wasm_attr "$swap_hash" force_expired || true)"
echo "  swap_tx=$swap_hash reason=$reason_attr force_expired=$force_attr"

[[ "$reason_attr" == "dust_filled" ]] || {
  echo "ERROR: expected wasm reason=dust_filled, got '$reason_attr'" >&2
  exit 1
}
[[ "$force_attr" == "true" ]] || {
  echo "ERROR: expected force_expired=true, got '$force_attr'" >&2
  exit 1
}

refund_raw="$(lcd_smart_query_raw "$LCD" "$PAIR" "$(jq -nc --argjson id "$ORDER_ID" '{expired_limit_refund:{order_id:$id}}')")"
refund="$(lcd_decode_smart_data "$refund_raw")"
lcd_reason="$(echo "$refund" | jq -r '.reason // empty')"
lcd_rem="$(echo "$refund" | jq -r '.remaining // empty')"
lcd_exp="$(echo "$refund" | jq -r '.expires_at // empty')"
echo "  LCD refund=$refund"

# cw_serde emits snake_case on the wire (same as wasm attr); not PascalCase.
[[ "$lcd_reason" == "dust_filled" ]] || {
  echo "ERROR: LCD reason want dust_filled got '$lcd_reason' (must not be expired)" >&2
  exit 1
}
[[ "$lcd_rem" == "1" ]] || {
  echo "ERROR: LCD remaining want 1 got '$lcd_rem'" >&2
  exit 1
}
if [[ -n "$lcd_exp" && "$lcd_exp" != "null" ]]; then
  echo "ERROR: DustFilled must clear expires_at, got '$lcd_exp'" >&2
  exit 1
fi

echo "  bot reconcile: reason=dust_filled => book as near-complete fill (not unfilled expiry)"
exit 0
