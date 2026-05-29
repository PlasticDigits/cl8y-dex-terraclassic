#!/usr/bin/env bash
# Resolve E2E_TRADE_PAIR for indexer-outage Playwright (first dual-CW20 factory pair).
# Prefers .qa-deploy-stamp pair_address; falls back to LCD factory query.
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
STAMP="$REPO_ROOT/.qa-deploy-stamp"
ENV_LOCAL="$REPO_ROOT/frontend-dapp/.env.local"

if [[ -f "$STAMP" ]]; then
  # shellcheck disable=SC1090
  source "$STAMP"
  if [[ -n "${pair_address:-}" ]]; then
    echo "$pair_address"
    exit 0
  fi
fi

if [[ ! -f "$ENV_LOCAL" ]]; then
  echo "e2e-trade-pair: missing $ENV_LOCAL (run scripts/deploy-dex-local.sh first)." >&2
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
  echo "e2e-trade-pair: VITE_FACTORY_ADDRESS not set in .env.local." >&2
  exit 1
fi

LCD="${VITE_TERRA_LCD_URL:-http://localhost:1317}"
LCD="${LCD%/}"

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

Q_PAIRS="$(b64_query '{"pairs":{"start_after":null,"limit":60}}')"
RAW_PAIRS="$(curl -sf "$LCD/cosmwasm/wasm/v1/contract/$VITE_FACTORY_ADDRESS/smart/$Q_PAIRS")"
PAIRS_DOC="$(decode_smart_payload "$RAW_PAIRS")"

PAIR_ADDR=""
while IFS= read -r row; do
  [[ -n "$row" ]] || continue
  addr="$(echo "$row" | jq -r '.contract_addr')"
  t0="$(echo "$row" | jq -r '.asset_infos[0].token.contract_addr // empty')"
  t1="$(echo "$row" | jq -r '.asset_infos[1].token.contract_addr // empty')"
  if [[ "$t0" =~ ^terra1 && "$t1" =~ ^terra1 ]]; then
    PAIR_ADDR="$addr"
    break
  fi
done < <(echo "$PAIRS_DOC" | jq -c '.pairs[]')

if [[ -z "$PAIR_ADDR" ]]; then
  echo "e2e-trade-pair: no dual-CW20 pair on factory." >&2
  exit 1
fi

echo "$PAIR_ADDR"
