#!/usr/bin/env bash
# Idempotent LUNC-C/USTC-C factory pairs so native LUNC/USTC appear in swap token lists (GitLab #201 wrap E2E).
# Requires: localterra container, frontend-dapp/.env.local with factory + wrap token addresses.
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
ENV_LOCAL="$REPO_ROOT/frontend-dapp/.env.local"
TEST_ADDRESS="terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v"
LIQ_RAW="${E2E_WRAP_PAIR_LIQ_U128:-100000000000}"

if [[ ! -f "$ENV_LOCAL" ]]; then
  echo "e2e-seed-wrap-pairs: missing $ENV_LOCAL (run scripts/deploy-dex-local.sh first)." >&2
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

for req in VITE_FACTORY_ADDRESS VITE_LUNC_C_TOKEN_ADDRESS VITE_USTC_C_TOKEN_ADDRESS VITE_TREASURY_ADDRESS; do
  if [[ -z "${!req:-}" ]]; then
    echo "e2e-seed-wrap-pairs: $req not set in .env.local." >&2
    exit 1
  fi
done

CONTAINER="$(docker compose -f "$REPO_ROOT/docker-compose.yml" ps -q localterra 2>/dev/null | head -1 || true)"
if [[ -z "$CONTAINER" ]]; then
  echo "e2e-seed-wrap-pairs: localterra container not running." >&2
  exit 1
fi

# shellcheck source=scripts/lib/e2e-terrad-tx.sh
source "$REPO_ROOT/scripts/lib/e2e-terrad-tx.sh"

terrad_tx() {
  e2e_terrad_tx "$CONTAINER" "$@"
}

terrad_query() {
  docker exec "$CONTAINER" terrad query "$@" \
    --node http://127.0.0.1:26657 \
    --output json
}

pair_addr_from_tx() {
  local tx_hash="$1"
  sleep 3
  terrad_query tx "$tx_hash" | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value' | head -1
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

LCD="${VITE_TERRA_LCD_URL:-http://localhost:1317}"
LCD="${LCD%/}"

factory_has_pair_with_token() {
  local token="$1"
  local q pairs_doc
  q="$(b64_query '{"pairs":{"start_after":null,"limit":120}}')"
  pairs_doc="$(decode_smart_payload "$(curl -sf "$LCD/cosmwasm/wasm/v1/contract/$VITE_FACTORY_ADDRESS/smart/$q")")"
  echo "$pairs_doc" | jq -e --arg t "$token" '
    [.pairs[]? | select(
      (.asset_infos[0].token.contract_addr // "") == $t
      or (.asset_infos[1].token.contract_addr // "") == $t
    )] | length > 0
  ' >/dev/null
}

cw20_balance() {
  local token="$1"
  local q
  q="$(b64_query "{\"balance\":{\"address\":\"$TEST_ADDRESS\"}}")"
  curl -sf "$LCD/cosmwasm/wasm/v1/contract/$token/smart/$q" | jq -r '.data.balance // "0"'
}

fund_wrap_token_via_treasury() {
  local wrap_addr="$1"
  local native_denom="$2"
  local bal need topup
  bal="$(cw20_balance "$wrap_addr")"
  need="$LIQ_RAW"
  if [[ "$bal" -ge "$need" ]]; then
    return 0
  fi
  topup=$((need * 2))
  echo "e2e-seed-wrap-pairs: topping up $wrap_addr via treasury wrap (${topup} ${native_denom})..."
  terrad_tx wasm execute "$VITE_TREASURY_ADDRESS" '{"wrap_deposit":{}}' --amount "${topup}${native_denom}" >/dev/null
  sleep 2
}

# First factory pair (EMBER/CORAL) for stable wrap-pair partners.
Q_PAIRS="$(b64_query '{"pairs":{"start_after":null,"limit":5}}')"
RAW_PAIRS="$(curl -sf "$LCD/cosmwasm/wasm/v1/contract/$VITE_FACTORY_ADDRESS/smart/$Q_PAIRS")"
PAIRS_DOC="$(decode_smart_payload "$RAW_PAIRS")"
EMBER_ADDR="$(echo "$PAIRS_DOC" | jq -r '.pairs[0].asset_infos[0].token.contract_addr // empty')"
CORAL_ADDR="$(echo "$PAIRS_DOC" | jq -r '.pairs[0].asset_infos[1].token.contract_addr // empty')"
if [[ -z "$EMBER_ADDR" || -z "$CORAL_ADDR" ]]; then
  echo "e2e-seed-wrap-pairs: could not resolve EMBER/CORAL from factory pairs." >&2
  exit 1
fi

create_wrap_pair() {
  local wrap_addr="$1"
  local wrap_sym="$2"
  local partner_addr="$3"
  local partner_sym="$4"

  if factory_has_pair_with_token "$wrap_addr"; then
    echo "e2e-seed-wrap-pairs: pair including $wrap_sym already exists; skipping."
    return 0
  fi

  if [[ "$wrap_sym" == "LUNC-C" ]]; then
    fund_wrap_token_via_treasury "$wrap_addr" "uluna"
  else
    fund_wrap_token_via_treasury "$wrap_addr" "uusd"
  fi

  echo "e2e-seed-wrap-pairs: creating $wrap_sym/$partner_sym pair..."
  local create_msg pair_addr tx_hash
  create_msg="{\"create_pair\":{\"asset_infos\":[{\"token\":{\"contract_addr\":\"$wrap_addr\"}},{\"token\":{\"contract_addr\":\"$partner_addr\"}}]}}"
  if ! tx_hash="$(terrad_tx wasm execute "$VITE_FACTORY_ADDRESS" "$create_msg" | jq -r '.txhash')"; then
    if factory_has_pair_with_token "$wrap_addr"; then
      echo "e2e-seed-wrap-pairs: $wrap_sym/$partner_sym already on factory; skipping create."
      return 0
    fi
    echo "e2e-seed-wrap-pairs: create_pair failed for $wrap_sym/$partner_sym." >&2
    exit 1
  fi
  pair_addr="$(pair_addr_from_tx "$tx_hash")"
  if [[ -z "$pair_addr" ]]; then
    if factory_has_pair_with_token "$wrap_addr"; then
      echo "e2e-seed-wrap-pairs: $wrap_sym/$partner_sym pair exists (tx $tx_hash); skipping."
      return 0
    fi
    echo "e2e-seed-wrap-pairs: failed to resolve pair address for $wrap_sym/$partner_sym (tx $tx_hash)." >&2
    exit 1
  fi

  terrad_tx wasm execute "$wrap_addr" \
    "{\"increase_allowance\":{\"spender\":\"$pair_addr\",\"amount\":\"$LIQ_RAW\",\"expires\":{\"never\":{}}}}" >/dev/null
  sleep 2
  terrad_tx wasm execute "$partner_addr" \
    "{\"increase_allowance\":{\"spender\":\"$pair_addr\",\"amount\":\"$LIQ_RAW\",\"expires\":{\"never\":{}}}}" >/dev/null
  sleep 2
  local provide_msg
  provide_msg="{\"provide_liquidity\":{\"assets\":[{\"info\":{\"token\":{\"contract_addr\":\"$wrap_addr\"}},\"amount\":\"$LIQ_RAW\"},{\"info\":{\"token\":{\"contract_addr\":\"$partner_addr\"}},\"amount\":\"$LIQ_RAW\"}],\"slippage_tolerance\":null,\"receiver\":null,\"deadline\":null}}"
  terrad_tx wasm execute "$pair_addr" "$provide_msg" >/dev/null
  echo "e2e-seed-wrap-pairs: $wrap_sym/$partner_sym pair $pair_addr seeded."
}

create_wrap_pair "$VITE_LUNC_C_TOKEN_ADDRESS" "LUNC-C" "$EMBER_ADDR" "EMBER"
create_wrap_pair "$VITE_USTC_C_TOKEN_ADDRESS" "USTC-C" "$CORAL_ADDR" "CORAL"

API_PORT="${API_PORT:-3001}"
if [[ -n "${VITE_INDEXER_URL:-}" ]]; then
  INDEXER_API="${VITE_INDEXER_URL%/}"
else
  INDEXER_API="http://127.0.0.1:${API_PORT}"
fi

echo "e2e-seed-wrap-pairs: waiting for indexer to list LUNC-C pair (pool page / wrap-pool E2E)..."
for i in $(seq 1 90); do
  if curl -sf "${INDEXER_API}/api/v1/pairs?limit=200" | jq -e --arg lc "$VITE_LUNC_C_TOKEN_ADDRESS" \
    '[.items[] | select(.asset_0.contract_addr == $lc or .asset_1.contract_addr == $lc)] | length > 0' >/dev/null; then
    echo "e2e-seed-wrap-pairs: indexer has LUNC-C pair."
    exit 0
  fi
  sleep 2
done
echo "e2e-seed-wrap-pairs: WARN: indexer did not list LUNC-C pair within 180s; wrap-pool specs may fail until indexer catches up." >&2
