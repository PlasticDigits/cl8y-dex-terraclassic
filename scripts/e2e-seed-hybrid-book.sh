#!/usr/bin/env bash
# Idempotent resting bid + ask on the first dual-CW20 factory pair for hybrid Playwright E2E.
# - Bid: hybrid when paying token0 (see limit_order_tests::bid_and_hybrid_swap_partially_fills_book).
# - Ask: hybrid when paying token1 on multihop hop 1 (see ask_and_hybrid_swap_partially_fills_book; GitLab #422).
# Requires: docker localterra, frontend-dapp/.env.local (same as scripts/e2e-provision-dev-wallet.sh).
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
ENV_LOCAL="$REPO_ROOT/frontend-dapp/.env.local"
BID_ESCROW_RAW="${E2E_HYBRID_SEED_BID_ESCROW:-50000000}"
BID_PRICE="${E2E_HYBRID_SEED_BID_PRICE:-1}"
ASK_ESCROW_RAW="${E2E_HYBRID_SEED_ASK_ESCROW:-50000000}"
ASK_PRICE="${E2E_HYBRID_SEED_ASK_PRICE:-1}"

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

# shellcheck source=scripts/lib/localterra-host-curl.sh
source "$REPO_ROOT/scripts/lib/localterra-host-curl.sh"
CONTAINER="$(localterra_container_id "$REPO_ROOT")"
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

seeded_any=0

RAW_BID_HEAD="$(lcd_smart_query_raw "$LCD" "$PAIR_ADDR" '{"order_book_head":{"side":"bid"}}')"
BID_HEAD_ID="$(decode_smart_payload "$RAW_BID_HEAD" | order_book_head_id_from_payload)"
if [[ -n "$BID_HEAD_ID" && "$BID_HEAD_ID" != "null" ]]; then
  echo "e2e-seed-hybrid-book: bid book already has head order $BID_HEAD_ID on $PAIR_ADDR; skipping bid."
else
  PLACE_HOOK="$(jq -nc --arg price "$BID_PRICE" --arg amount "$BID_ESCROW_RAW" \
    '{place_limit_order_batch:{side:"bid",orders:[{price:$price,amount:$amount,max_adjust_steps:64}]}}')"
  PLACE_HOOK_B64="$(echo -n "$PLACE_HOOK" | base64 -w0 2>/dev/null || echo -n "$PLACE_HOOK" | base64)"
  SEND_MSG="$(jq -nc --arg pair "$PAIR_ADDR" --arg amt "$BID_ESCROW_RAW" --arg hook "$PLACE_HOOK_B64" \
    '{send:{contract:$pair,amount:$amt,msg:$hook}}')"
  echo "e2e-seed-hybrid-book: placing bid on $PAIR_ADDR (escrow token1 $TOKEN1, amount $BID_ESCROW_RAW, price $BID_PRICE)."
  terrad_tx wasm execute "$TOKEN1" "$SEND_MSG" >/dev/null
  seeded_any=1
fi

RAW_ASK_HEAD="$(lcd_smart_query_raw "$LCD" "$PAIR_ADDR" '{"order_book_head":{"side":"ask"}}')"
ASK_HEAD_ID="$(decode_smart_payload "$RAW_ASK_HEAD" | order_book_head_id_from_payload)"
if [[ -n "$ASK_HEAD_ID" && "$ASK_HEAD_ID" != "null" ]]; then
  echo "e2e-seed-hybrid-book: ask book already has head order $ASK_HEAD_ID on $PAIR_ADDR; skipping ask."
else
  PLACE_HOOK="$(jq -nc --arg price "$ASK_PRICE" --arg amount "$ASK_ESCROW_RAW" \
    '{place_limit_order_batch:{side:"ask",orders:[{price:$price,amount:$amount,max_adjust_steps:64}]}}')"
  PLACE_HOOK_B64="$(echo -n "$PLACE_HOOK" | base64 -w0 2>/dev/null || echo -n "$PLACE_HOOK" | base64)"
  SEND_MSG="$(jq -nc --arg pair "$PAIR_ADDR" --arg amt "$ASK_ESCROW_RAW" --arg hook "$PLACE_HOOK_B64" \
    '{send:{contract:$pair,amount:$amt,msg:$hook}}')"
  echo "e2e-seed-hybrid-book: placing ask on $PAIR_ADDR (escrow token0 $TOKEN0, amount $ASK_ESCROW_RAW, price $ASK_PRICE)."
  terrad_tx wasm execute "$TOKEN0" "$SEND_MSG" >/dev/null
  seeded_any=1
fi

if [[ "$seeded_any" -eq 1 ]]; then
  sleep 2
fi
echo "e2e-seed-hybrid-book: bid + ask books ready for hybrid E2E (#193 direct, #422 multihop)."
