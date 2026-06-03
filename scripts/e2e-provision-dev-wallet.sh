#!/usr/bin/env bash
# Idempotent CW20 top-up for the Playwright simulated dev wallet on LocalTerra.
# Requires: docker localterra container (same as scripts/deploy-dex-local.sh),
#           frontend-dapp/.env.local with VITE_FACTORY_ADDRESS and VITE_TERRA_LCD_URL.
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
ENV_LOCAL="$REPO_ROOT/frontend-dapp/.env.local"
DEV_ADDR="terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v"
MIN_RAW_BALANCE="${E2E_DEV_MIN_CW20_U128:-1000000000000}"
MINT_TOPUP="${E2E_DEV_CW20_MINT_TOPUP:-10000000000000000}"

if [[ ! -f "$ENV_LOCAL" ]]; then
  echo "e2e-provision: missing $ENV_LOCAL (run scripts/deploy-dex-local.sh first)." >&2
  exit 1
fi

# shellcheck disable=SC1090
set -a
# Export VITE_* lines only; ignore comments and blanks.
while IFS= read -r line || [[ -n "$line" ]]; do
  [[ "$line" =~ ^[[:space:]]*# ]] && continue
  [[ "$line" =~ ^VITE_[A-Z0-9_]+= ]] || continue
  key="${line%%=*}"
  val="${line#*=}"
  export "$key=$val"
done <"$ENV_LOCAL"
set +a

if [[ -z "${VITE_FACTORY_ADDRESS:-}" ]]; then
  echo "e2e-provision: VITE_FACTORY_ADDRESS not set in .env.local." >&2
  exit 1
fi

LCD="${VITE_TERRA_LCD_URL:-http://localhost:1317}"
LCD="${LCD%/}"

CONTAINER="$(docker compose -f "$REPO_ROOT/docker-compose.yml" ps -q localterra 2>/dev/null | head -1 || true)"
if [[ -z "$CONTAINER" ]]; then
  echo "e2e-provision: localterra container not running; start it with docker compose up -d localterra." >&2
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

decode_pairs_payload() {
  local raw="$1"
  local data_type
  data_type=$(echo "$raw" | jq -r '.data | type')
  if [[ "$data_type" == "string" ]]; then
    echo "$raw" | jq -r '.data | @base64d | fromjson'
  else
    echo "$raw" | jq '.data'
  fi
}

RAW_PAIRS="$(lcd_smart_query_raw "$LCD" "$VITE_FACTORY_ADDRESS" '{"pairs":{"start_after":null,"limit":60}}')"
PAIRS_DOC="$(decode_pairs_payload "$RAW_PAIRS")"

mapfile -t TOKEN_ADDRS < <(echo "$PAIRS_DOC" | jq -r '.pairs[] | .asset_infos[] | .token.contract_addr? // empty' | sort -u)

if [[ ${#TOKEN_ADDRS[@]} -eq 0 ]]; then
  echo "e2e-provision: factory returned no CW20 token addresses; check deployment." >&2
  exit 1
fi

for TOKEN in "${TOKEN_ADDRS[@]}"; do
  [[ -n "$TOKEN" ]] || continue
  # Wrapped natives mint via wrap-mapper only (GitLab #201 wrap-pair seed).
  if [[ -n "${VITE_LUNC_C_TOKEN_ADDRESS:-}" && "$TOKEN" == "$VITE_LUNC_C_TOKEN_ADDRESS" ]]; then
    continue
  fi
  if [[ -n "${VITE_USTC_C_TOKEN_ADDRESS:-}" && "$TOKEN" == "$VITE_USTC_C_TOKEN_ADDRESS" ]]; then
    continue
  fi
  MIN_FOR_TOKEN="$MIN_RAW_BALANCE"
  if [[ -n "${VITE_CL8Y_TOKEN_ADDRESS:-}" && "$TOKEN" == "$VITE_CL8Y_TOKEN_ADDRESS" ]]; then
    MIN_FOR_TOKEN="${E2E_DEV_MIN_CL8Y_U128:-1000000000000000000}"
  fi
  RAW_BAL="$(lcd_smart_query_raw "$LCD" "$TOKEN" "{\"balance\":{\"address\":\"$DEV_ADDR\"}}")"
  BAL="$(decode_pairs_payload "$RAW_BAL" | jq -r '.balance // "0"')"
  if [[ "$BAL" =~ ^[0-9]+$ ]] && ((10#$BAL >= 10#$MIN_FOR_TOKEN)); then
    continue
  fi
  echo "e2e-provision: minting $MINT_TOPUP units to dev wallet on $TOKEN (balance was $BAL)."
  terrad_tx wasm execute "$TOKEN" "{\"mint\":{\"recipient\":\"$DEV_ADDR\",\"amount\":\"$MINT_TOPUP\"}}" >/dev/null
  sleep 2
done

echo "e2e-provision: CW20 balances for factory tokens are at least $MIN_RAW_BALANCE (raw units) where minting is allowed."
