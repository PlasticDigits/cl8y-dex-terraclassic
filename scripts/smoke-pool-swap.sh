#!/usr/bin/env bash
# Post-deploy **pool-only** sanity checks against a pair (no hybrid / limit-book leg).
#
# Read-only path (default): LCD `Pool` query and optional `Simulation` query.
# Requires: curl, jq. Optional: terrad for the same queries if REST layout differs on your node.
#
# Env:
#   TERRA_LCD_URL   REST base (default http://127.0.0.1:1317)
#   PAIR_ADDR       Pair contract address (required)
#   OFFER_TOKEN     CW20 address of asset offered (optional; if set, runs Simulation)
#   OFFER_AMOUNT    Amount as string integer (default 1000)
#
# Example (LocalTerra after deploy):
#   export PAIR_ADDR=terra1...
#   export OFFER_TOKEN=terra1...
#   ./scripts/smoke-pool-swap.sh
set -euo pipefail

LCD="${TERRA_LCD_URL:-http://127.0.0.1:1317}"
LCD="${LCD%/}"

PAIR="${PAIR_ADDR:-}"
if [[ -z "$PAIR" ]]; then
  echo "ERROR: set PAIR_ADDR to the pair contract." >&2
  exit 1
fi

smart_query_b64() {
  local msg="$1"
  if [[ "$(uname)" == Darwin ]]; then
    printf '%s' "$msg" | base64 | tr -d '\n'
  else
    printf '%s' "$msg" | base64 -w0
  fi
}

lcd_smart() {
  local contract="$1"
  local msg="$2"
  local b64
  b64="$(smart_query_b64 "$msg")"
  curl -sf "${LCD}/cosmwasm/wasm/v1/contract/${contract}/smart/${b64}"
}

echo "== Pool query (${PAIR}) =="
POOL_JSON="$(lcd_smart "$PAIR" '{"pool":{}}')"
echo "$POOL_JSON" | jq .

OFFER_TOKEN="${OFFER_TOKEN:-}"
OFFER_AMOUNT="${OFFER_AMOUNT:-1000}"
if [[ -n "$OFFER_TOKEN" ]]; then
  echo "== Simulation (pool-only; offer CW20 ${OFFER_TOKEN}, amount ${OFFER_AMOUNT}) =="
  SIM_MSG="$(jq -nc \
    --arg addr "$OFFER_TOKEN" \
    --arg amt "$OFFER_AMOUNT" \
    '{simulation:{offer_asset:{info:{token:{contract_addr:$addr}},amount:$amt}}}')"
  lcd_smart "$PAIR" "$SIM_MSG" | jq .
else
  echo "== Skipping Simulation (set OFFER_TOKEN to run). =="
fi

echo "OK: smoke-pool-swap read-only checks passed."
