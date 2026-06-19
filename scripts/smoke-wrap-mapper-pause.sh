#!/usr/bin/env bash
# LocalTerra smoke: wrap-mapper pause/unpause cycle (SEC-B06 / GitLab #396).
#
# Proves on a live LocalTerra deploy that:
#   1. wrap-mapper SetPaused(true) blocks treasury WrapDeposit with a clear "paused" error
#   2. The same pause blocks CW20 Send → unwrap with a clear "paused" error
#   3. SetPaused(false) restores wrap and unwrap execution
#
# Requires: curl, jq, localterra container, frontend-dapp/.env.local (make deploy-local, full seed).
#
# Env (optional):
#   TERRA_LCD_URL          REST base (from .env.local when sourced via smoke-wrap-env.sh)
#   WRAP_MAPPER_ADDR       Wrap-mapper contract (VITE_WRAP_MAPPER_ADDRESS)
#   TREASURY_ADDR          Treasury contract (VITE_TREASURY_ADDRESS)
#   LUNC_C_ADDR            LUNC-C CW20 (VITE_LUNC_C_TOKEN_ADDRESS)
#   SMOKE_WRAP_AMOUNT      uluna wrap probe amount (default 1000000 = 1 LUNC)
#   SMOKE_UNWRAP_AMOUNT    LUNC-C unwrap probe amount (default 500000)
#
# Example:
#   make smoke-wrap-mapper-pause
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$REPO_ROOT"

# shellcheck source=scripts/lib/smoke-wrap-env.sh
source "$REPO_ROOT/scripts/lib/smoke-wrap-env.sh"
# shellcheck source=scripts/lib/lcd-smart-query.sh
source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
# shellcheck source=scripts/lib/e2e-terrad-tx.sh
source "$REPO_ROOT/scripts/lib/e2e-terrad-tx.sh"
# shellcheck source=scripts/lib/terrad-wait-tx.sh
source "$REPO_ROOT/scripts/lib/terrad-wait-tx.sh"

TEST_ADDRESS="${TEST_ADDRESS:-terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v}"
CHAIN_ID="${CHAIN_ID:-localterra}"
TERRAD_NODE="${TERRAD_NODE:-http://127.0.0.1:26657}"
CONTAINER_NAME="$(docker compose ps -q localterra 2>/dev/null | head -1)"

WRAP_AMOUNT="${SMOKE_WRAP_AMOUNT:-1000000}"
UNWRAP_AMOUNT="${SMOKE_UNWRAP_AMOUNT:-500000}"

if [ -z "$CONTAINER_NAME" ]; then
  echo "ERROR: localterra container not running (make start)." >&2
  exit 1
fi

terrad_tx() {
  e2e_terrad_tx "$CONTAINER_NAME" "$@"
}

tx_hash_from_json() {
  sed -n '/^{/,$p' | tail -1 | jq -r '.txhash // .tx_response.txhash // empty'
}

query_tx_json() {
  local txhash="$1"
  local attempts=0
  local max=15
  local json=""
  while [ "$attempts" -lt "$max" ]; do
    json="$(docker exec "$CONTAINER_NAME" terrad query tx "$txhash" --node "$TERRAD_NODE" --output json 2>/dev/null || true)"
    if [ -n "$json" ] && echo "$json" | jq -e '.txhash // .tx_response.txhash // .hash' >/dev/null 2>&1; then
      echo "$json"
      return 0
    fi
    sleep 2
    attempts=$((attempts + 1))
  done
  return 1
}

tx_code() {
  local json="$1"
  echo "$json" | jq -r '.code // .tx_response.code // "0"'
}

tx_raw_log() {
  local json="$1"
  echo "$json" | jq -r '.raw_log // .tx_response.raw_log // ""'
}

wrap_mapper_paused() {
  lcd_decode_smart_data "$(lcd_smart_query_raw "$TERRA_LCD_URL" "$WRAP_MAPPER_ADDR" '{"config":{}}')" \
    | jq -r '.paused'
}

cw20_balance() {
  local addr="$1"
  lcd_decode_smart_data "$(lcd_smart_query_raw "$TERRA_LCD_URL" "$LUNC_C_ADDR" "{\"balance\":{\"address\":\"$addr\"}}")" \
    | jq -r '.balance // "0"'
}

unwrap_hook_b64() {
  if [[ "$(uname)" == Darwin ]]; then
    printf '%s' '{"unwrap":{"recipient":null}}' | base64 | tr -d '\n'
  else
    printf '%s' '{"unwrap":{"recipient":null}}' | base64 -w0
  fi
}

broadcast_tx() {
  terrad_tx "$@"
}

expect_tx_success() {
  local label="$1"
  shift
  local out tx_hash json code
  out="$(broadcast_tx "$@" 2>&1)" || {
    echo "ERROR: $label — broadcast failed: $out" >&2
    return 1
  }
  tx_hash="$(printf '%s' "$out" | tx_hash_from_json)"
  if ! json="$(query_tx_json "$tx_hash")"; then
    echo "ERROR: $label — could not query tx $tx_hash" >&2
    return 1
  fi
  code="$(tx_code "$json")"
  if [ "$code" = "0" ]; then
    echo "  OK: $label (tx $tx_hash)"
    return 0
  fi
  echo "ERROR: $label — tx $tx_hash failed code=$code: $(tx_raw_log "$json")" >&2
  return 1
}

expect_tx_rejected_paused() {
  local label="$1"
  shift
  local out tx_hash json code log
  if ! out="$(broadcast_tx "$@" 2>&1)"; then
    if echo "$out" | grep -Eiq 'paused|contract execution is disabled'; then
      echo "  OK: $label — rejected at broadcast with paused error"
      return 0
    fi
    echo "ERROR: $label — broadcast failed without paused error: $out" >&2
    return 1
  fi
  tx_hash="$(printf '%s' "$out" | tx_hash_from_json)"
  if ! json="$(query_tx_json "$tx_hash")"; then
    echo "ERROR: $label — could not query tx $tx_hash" >&2
    return 1
  fi
  code="$(tx_code "$json")"
  log="$(tx_raw_log "$json")"
  if [ "$code" != "0" ] && echo "$log" | grep -Eiq 'paused|contract execution is disabled'; then
    echo "  OK: $label — on-chain rejection (code=$code): $(echo "$log" | head -c 120)"
    return 0
  fi
  echo "ERROR: $label — expected paused rejection, got code=$code log=$log" >&2
  return 1
}

set_wrap_mapper_paused() {
  local paused="$1"
  expect_tx_success "set_paused paused=$paused" \
    wasm execute "$WRAP_MAPPER_ADDR" "{\"set_paused\":{\"paused\":$paused}}"
}

echo "== Wrap-mapper pause smoke (SEC-B06) =="
echo "  wrap-mapper: $WRAP_MAPPER_ADDR"
echo "  treasury:    $TREASURY_ADDR"
echo "  LUNC-C:      $LUNC_C_ADDR"

if [ "$(wrap_mapper_paused)" = "true" ]; then
  echo "== Pre-clean: unpause wrap-mapper (idempotent start) =="
  set_wrap_mapper_paused false
fi

bal="$(cw20_balance "$TEST_ADDRESS")"
if [ "${bal:-0}" -lt "$UNWRAP_AMOUNT" ]; then
  echo "== Seed: wrap ${WRAP_AMOUNT}uluna for unwrap probe balance =="
  expect_tx_success "seed wrap_deposit" \
    wasm execute "$TREASURY_ADDR" '{"wrap_deposit":{}}' --amount "${WRAP_AMOUNT}uluna"
fi

echo "== Pause wrap-mapper =="
set_wrap_mapper_paused true
if [ "$(wrap_mapper_paused)" != "true" ]; then
  echo "ERROR: wrap-mapper config.paused is not true after SetPaused" >&2
  exit 1
fi

echo "== Assert wrap blocked while paused =="
expect_tx_rejected_paused "wrap_deposit while paused" \
  wasm execute "$TREASURY_ADDR" '{"wrap_deposit":{}}' --amount "${WRAP_AMOUNT}uluna"

UNWRAP_B64="$(unwrap_hook_b64)"
echo "== Assert unwrap blocked while paused =="
expect_tx_rejected_paused "unwrap send while paused" \
  wasm execute "$LUNC_C_ADDR" \
  "{\"send\":{\"contract\":\"$WRAP_MAPPER_ADDR\",\"amount\":\"$UNWRAP_AMOUNT\",\"msg\":\"$UNWRAP_B64\"}}"

echo "== Unpause wrap-mapper =="
set_wrap_mapper_paused false
if [ "$(wrap_mapper_paused)" != "false" ]; then
  echo "ERROR: wrap-mapper config.paused is not false after unpause" >&2
  exit 1
fi

echo "== Assert wrap restored after unpause =="
expect_tx_success "wrap_deposit after unpause" \
  wasm execute "$TREASURY_ADDR" '{"wrap_deposit":{}}' --amount "${WRAP_AMOUNT}uluna"

echo "== Assert unwrap restored after unpause =="
expect_tx_success "unwrap send after unpause" \
  wasm execute "$LUNC_C_ADDR" \
  "{\"send\":{\"contract\":\"$WRAP_MAPPER_ADDR\",\"amount\":\"$UNWRAP_AMOUNT\",\"msg\":\"$UNWRAP_B64\"}}"

echo "OK: smoke-wrap-mapper-pause passed (wrap/unwrap rejected under pause; restored after unpause)."
