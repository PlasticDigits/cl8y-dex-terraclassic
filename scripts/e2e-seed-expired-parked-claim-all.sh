#!/usr/bin/env bash
# Place ≥2 expired bids, wait for chain expiry, hybrid-swap park them for dev wallet (GitLab #259).
# Requires: docker localterra, frontend-dapp/.env.local, indexer running for Playwright poll.
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
ENV_LOCAL="$REPO_ROOT/frontend-dapp/.env.local"
DEV_ADDR="terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v"
BID_ESCROW_RAW="${E2E_EXPIRED_PARK_BID_ESCROW:-10000}"
BID_PRICE_HIGH="${E2E_EXPIRED_PARK_BID_PRICE_HIGH:-2}"
BID_PRICE_LOW="${E2E_EXPIRED_PARK_BID_PRICE_LOW:-1.99}"
EXPIRY_LEAD_SEC="${E2E_EXPIRED_PARK_EXPIRY_LEAD_SEC:-45}"
EXPIRY_WAIT_MAX_SEC="${E2E_EXPIRED_PARK_EXPIRY_WAIT_MAX_SEC:-120}"
HYBRID_BOOK_INPUT="${E2E_EXPIRED_PARK_HYBRID_BOOK_INPUT:-5000}"

if [[ ! -f "$ENV_LOCAL" ]]; then
  echo "e2e-seed-expired-parked-claim-all: missing $ENV_LOCAL (run scripts/deploy-dex-local.sh first)." >&2
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
  echo "e2e-seed-expired-parked-claim-all: VITE_FACTORY_ADDRESS not set in .env.local." >&2
  exit 1
fi

LCD="${VITE_TERRA_LCD_URL:-http://localhost:1317}"
LCD="${LCD%/}"

CONTAINER="$(docker compose -f "$REPO_ROOT/docker-compose.yml" ps -q localterra 2>/dev/null | head -1 || true)"
if [[ -z "$CONTAINER" ]]; then
  echo "e2e-seed-expired-parked-claim-all: localterra container not running." >&2
  exit 1
fi

# shellcheck source=scripts/lib/e2e-terrad-tx.sh
source "$REPO_ROOT/scripts/lib/e2e-terrad-tx.sh"

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

latest_block_unix() {
  local iso
  iso="$(curl -sf "$LCD/cosmos/base/tendermint/v1beta1/blocks/latest" | jq -r '.block.header.time')"
  date -u -d "$iso" +%s 2>/dev/null || date -u -j -f "%Y-%m-%dT%H:%M:%S" "${iso%%.*}" +%s
}

Q_PAIRS="$(b64_query '{"pairs":{"start_after":null,"limit":60}}')"
RAW_PAIRS="$(curl -sf "$LCD/cosmwasm/wasm/v1/contract/$VITE_FACTORY_ADDRESS/smart/$Q_PAIRS")"
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

if [[ -z "$PAIR_ADDR" || -z "$TOKEN0" || -z "$TOKEN1" ]]; then
  echo "e2e-seed-expired-parked-claim-all: no dual-CW20 pair on factory; redeploy." >&2
  exit 1
fi

Q_PAUSED="$(b64_query '{"is_paused":{}}')"
RAW_PAUSED="$(curl -sf "$LCD/cosmwasm/wasm/v1/contract/$PAIR_ADDR/smart/$Q_PAUSED")"
if [[ "$(decode_smart_payload "$RAW_PAUSED" | jq -r '.paused // false')" == "true" ]]; then
  echo "e2e-seed-expired-parked-claim-all: pair $PAIR_ADDR is paused (L6)." >&2
  exit 1
fi

NOW_SEC="$(latest_block_unix)"
EXPIRES=$((NOW_SEC + EXPIRY_LEAD_SEC))

TOTAL_ESCROW_RAW=$((BID_ESCROW_RAW * 2))

PLACE_HOOK="$(jq -nc \
  --argjson exp "$EXPIRES" \
  --arg high "$BID_PRICE_HIGH" \
  --arg low "$BID_PRICE_LOW" \
  --arg amt "$BID_ESCROW_RAW" \
  '{place_limit_order_batch:{side:"bid",orders:[
    {price:$high,amount:$amt,max_adjust_steps:64,expires_at:$exp},
    {price:$low,amount:$amt,max_adjust_steps:64,expires_at:$exp}
  ]}}')"
PLACE_HOOK_B64="$(echo -n "$PLACE_HOOK" | base64 -w0 2>/dev/null || echo -n "$PLACE_HOOK" | base64)"
SEND_PLACE="$(jq -nc --arg pair "$PAIR_ADDR" --arg amt "$TOTAL_ESCROW_RAW" --arg hook "$PLACE_HOOK_B64" \
  '{send:{contract:$pair,amount:$amt,msg:$hook}}')"

echo "e2e-seed-expired-parked-claim-all: placing 2 expired bids on $PAIR_ADDR (expires_at=$EXPIRES)."
terrad_tx wasm execute "$TOKEN1" "$SEND_PLACE" >/dev/null
sleep 2

echo "e2e-seed-expired-parked-claim-all: waiting for block_time >= $EXPIRES (max ${EXPIRY_WAIT_MAX_SEC}s)."
waited=0
while (( waited < EXPIRY_WAIT_MAX_SEC )); do
  NOW_SEC="$(latest_block_unix)"
  if (( NOW_SEC >= EXPIRES )); then
    break
  fi
  sleep 2
  waited=$((waited + 2))
done
if (( NOW_SEC < EXPIRES )); then
  echo "e2e-seed-expired-parked-claim-all: chain time did not reach expires_at within ${EXPIRY_WAIT_MAX_SEC}s." >&2
  exit 1
fi

# Pure-book hybrid requires belief_price or min_return on execute (GitLab #334 / L9).
SWAP_HOOK="$(jq -nc --arg book "$HYBRID_BOOK_INPUT" \
  '{swap:{belief_price:null,max_spread:"1",min_return:"1",to:null,deadline:null,hybrid:{pool_input:"0",book_input:$book,max_maker_fills:8,book_start_hint:null},trader:null}}')"
SWAP_HOOK_B64="$(echo -n "$SWAP_HOOK" | base64 -w0 2>/dev/null || echo -n "$SWAP_HOOK" | base64)"
SEND_SWAP="$(jq -nc --arg pair "$PAIR_ADDR" --arg amt "$HYBRID_BOOK_INPUT" --arg hook "$SWAP_HOOK_B64" \
  '{send:{contract:$pair,amount:$amt,msg:$hook}}')"

echo "e2e-seed-expired-parked-claim-all: hybrid swap to park expired bids (book_input=$HYBRID_BOOK_INPUT)."
terrad_tx wasm execute "$TOKEN0" "$SEND_SWAP" >/dev/null
sleep 3

echo "e2e-seed-expired-parked-claim-all: seeded pair=$PAIR_ADDR owner=$DEV_ADDR expires_at=$EXPIRES"
