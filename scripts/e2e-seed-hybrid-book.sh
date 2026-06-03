#!/usr/bin/env bash
# Idempotent resting bid on the first dual-CW20 factory pair for hybrid-swap Playwright E2E.
# Hybrid swaps paying token0 with a book leg consume bid-side liquidity (see limit_order_tests::bid_and_hybrid_swap_partially_fills_book).
# Requires: docker localterra, frontend-dapp/.env.local (same as scripts/e2e-provision-dev-wallet.sh).
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
ENV_LOCAL="$REPO_ROOT/frontend-dapp/.env.local"
BID_ESCROW_RAW="${E2E_HYBRID_SEED_BID_ESCROW:-50000000}"
BID_PRICE="${E2E_HYBRID_SEED_BID_PRICE:-1}"

if [[ ! -f "$ENV_LOCAL" ]]; then
  echo "e2e-seed-hybrid-book: missing $ENV_LOCAL (run scripts/deploy-dex-local.sh first)." >&2
  exit 1
fi

# shellcheck disable=SC1090
set -a
while IFS= read -r line || [[ -n "$line" ]]; do
  [[ "$line" =~ ^[[:space:]]*# ]] && continue
  [[ "$line" =~ ^VITE_[A-Z0-9_]+= ]] || continue
  key="${line%%=*}"
  val="${line#*=}"
  export "$key=$val"
done <"$ENV_LOCAL"
set +a

if [[ -z "${VITE_FACTORY_ADDRESS:-}" ]]; then
  echo "e2e-seed-hybrid-book: VITE_FACTORY_ADDRESS not set in .env.local." >&2
  exit 1
fi

LCD="${VITE_TERRA_LCD_URL:-http://localhost:1317}"
LCD="${LCD%/}"

CONTAINER="$(docker compose -f "$REPO_ROOT/docker-compose.yml" ps -q localterra 2>/dev/null | head -1 || true)"
if [[ -z "$CONTAINER" ]]; then
  echo "e2e-seed-hybrid-book: localterra container not running." >&2
  exit 1
fi

# shellcheck source=scripts/lib/e2e-terrad-tx.sh
source "$REPO_ROOT/scripts/lib/e2e-terrad-tx.sh"
# shellcheck source=scripts/lib/lcd-smart-query.sh
source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"

terrad_tx() {
  e2e_terrad_tx "$CONTAINER" "$@"
}

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

# Pair `OrderBookHead` returns `Option<u64>` — LCD payload is a bare number, not `{ head_order_id }`.
order_book_head_id_from_payload() {
  jq -r 'if type == "number" then tostring elif type == "object" then (.head_order_id // empty | tostring) else empty end'
}

RAW_PAIRS="$(lcd_smart_query_raw "$LCD" "$VITE_FACTORY_ADDRESS" '{"pairs":{"start_after":null,"limit":60}}')"
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
  echo "e2e-seed-hybrid-book: no dual-CW20 pair on factory first page; redeploy with scripts/deploy-dex-local.sh." >&2
  exit 1
fi

RAW_HEAD="$(lcd_smart_query_raw "$LCD" "$PAIR_ADDR" '{"order_book_head":{"side":"bid"}}')"
HEAD_ID="$(decode_smart_payload "$RAW_HEAD" | order_book_head_id_from_payload)"
if [[ -n "$HEAD_ID" && "$HEAD_ID" != "null" ]]; then
  echo "e2e-seed-hybrid-book: bid book already has head order $HEAD_ID on $PAIR_ADDR; skipping."
  exit 0
fi

# Retail single placement uses batch with one rung (GitLab #206 — `place_limit_order` hook removed).
PLACE_HOOK="$(jq -nc --arg price "$BID_PRICE" --arg amount "$BID_ESCROW_RAW" \
  '{place_limit_order_batch:{side:"bid",orders:[{price:$price,amount:$amount,max_adjust_steps:64}]}}')"
PLACE_HOOK_B64="$(echo -n "$PLACE_HOOK" | base64 -w0 2>/dev/null || echo -n "$PLACE_HOOK" | base64)"
SEND_MSG="$(jq -nc --arg pair "$PAIR_ADDR" --arg amt "$BID_ESCROW_RAW" --arg hook "$PLACE_HOOK_B64" \
  '{send:{contract:$pair,amount:$amt,msg:$hook}}')"

echo "e2e-seed-hybrid-book: placing bid on $PAIR_ADDR (escrow token1 $TOKEN1, amount $BID_ESCROW_RAW, price $BID_PRICE)."
terrad_tx wasm execute "$TOKEN1" "$SEND_MSG" >/dev/null
sleep 2
echo "e2e-seed-hybrid-book: resting bid seeded for hybrid E2E."
