#!/usr/bin/env bash
# Verification for GitLab #384 — fee-discount register/deregister gas limits (FT-3 / FT-4 UI).
#
# Layers:
#   1. Frontend unit tests: getGasLimitForTx maps register/deregister above measured LocalTerra gas
#   2. Optional live LocalTerra: on-chain gas_used < dApp gas_wanted for register + deregister
#
# Refs: frontend-dapp/src/services/terraclassic/terraGas.ts,
#       skills/AGENTS_TERRACLASSIC_GAS.md, skills/AGENTS_FEE_DISCOUNT_TIERS.md
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }
skip() { RESULTS+=("SKIP  $1"); echo "  [SKIP] $1"; }

read_env_var() {
  [ -f "$1" ] || return 0
  sed -n "s/^${2}=//p" "$1" 2>/dev/null | head -1
}

REGISTER_LIMIT=300000
DEREGISTER_LIMIT=250000
MEASURED_REGISTER=204438
MEASURED_DEREGISTER=160932

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #384 — fee-discount register/deregister gas limits"
echo "════════════════════════════════════════════════════════════════"

echo ""
echo "[1] Frontend unit tests (terraGas.feeDiscount + transactions register/deregister)..."
if bash scripts/with-node.sh --cwd frontend-dapp -- npm run test -- --run \
  src/services/terraclassic/__tests__/terraGas.feeDiscount.test.ts \
  src/services/terraclassic/__tests__/transactions.test.ts -t 'register|deregister|REGISTER|DEREGISTER' 2>&1; then
  ok "unit tests: getGasLimitForTx register/deregister"
else
  bad "unit tests: getGasLimitForTx register/deregister"
fi

echo ""
echo "[2] Constants exceed measured LocalTerra gas (#384)..."
if [ "$REGISTER_LIMIT" -gt "$MEASURED_REGISTER" ] && [ "$DEREGISTER_LIMIT" -gt "$MEASURED_DEREGISTER" ]; then
  ok "limits exceed measured gas (register $REGISTER_LIMIT > $MEASURED_REGISTER, deregister $DEREGISTER_LIMIT > $MEASURED_DEREGISTER)"
else
  bad "gas limit constants do not exceed measured on-chain consumption"
fi

CONTAINER_NAME="$(docker compose ps -q localterra 2>/dev/null | head -1 || true)"
if [ -z "$CONTAINER_NAME" ]; then
  if sg docker -c 'docker compose ps -q localterra' 2>/dev/null | head -1 | grep -q .; then
    CONTAINER_NAME="$(sg docker -c 'docker compose ps -q localterra' | head -1)"
  fi
fi

IDX_ENV="$REPO_ROOT/indexer/.env"
FE_ENV="$REPO_ROOT/frontend-dapp/.env.local"
FEE_DISCOUNT="$(read_env_var "$IDX_ENV" FEE_DISCOUNT_ADDRESS)"
CL8Y="$(read_env_var "$FE_ENV" VITE_CL8Y_TOKEN_ADDRESS)"
LCD="$(read_env_var "$IDX_ENV" LCD_URLS)"; LCD="${LCD%%,*}"
LCD="${LCD:-http://127.0.0.1:1317}"; LCD="${LCD%/}"

if [ -z "$CONTAINER_NAME" ] || [ -z "$FEE_DISCOUNT" ] || [ -z "$CL8Y" ]; then
  skip "live LocalTerra gas_used check (chain not up or deploy env missing)"
else
  # shellcheck source=scripts/lib/lcd-smart-query.sh
  source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
  # shellcheck source=scripts/lib/terrad-wait-tx.sh
  source "$REPO_ROOT/scripts/lib/terrad-wait-tx.sh"

  CHAIN_ID="${CHAIN_ID:-localterra}"
  TERRAD_NODE="${TERRAD_NODE:-http://127.0.0.1:26657}"
  TEST_ADDRESS="${TEST_ADDRESS:-terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v}"

  terrad_tx() {
    docker exec "$CONTAINER_NAME" terrad tx "$@" \
      --from test1 --keyring-backend test --chain-id "$CHAIN_ID" \
      --gas auto --gas-adjustment 1.3 --fees 500000000uluna \
      --node "$TERRAD_NODE" --broadcast-mode sync -y --output json
  }
  wait_tx() {
    terrad_wait_tx_inclusion "$CONTAINER_NAME" "$1" "$TERRAD_NODE" 90
  }
  TIER1_MIN="1000000000000000000"

  tx_gas_used() {
    local txhash="$1"
    local json gas
    json="$(docker exec "$CONTAINER_NAME" terrad query tx "$txhash" --node "$TERRAD_NODE" --output json 2>/dev/null || true)"
    gas="$(echo "$json" | jq -r '.gas_used // .tx_response.gas_used // empty')"
    [[ -n "$gas" && "$gas" =~ ^[0-9]+$ && "$gas" != "0" ]] || return 1
    echo "$gas"
  }

  echo ""
  echo "[3] Live LocalTerra: register gas_used < REGISTER_FEE_DISCOUNT_GAS_LIMIT ($REGISTER_LIMIT)..."

  BAL_RAW="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$CL8Y" \
    "$(printf '{"balance":{"address":"%s"}}' "$TEST_ADDRESS")")")"
  BAL="$(echo "$BAL_RAW" | jq -r '.balance // "0"')"
  if [ "$(echo "$BAL >= $TIER1_MIN" | bc)" != "1" ]; then
    MINT_TX="$(terrad_tx wasm execute "$CL8Y" \
      "$(printf '{"mint":{"recipient":"%s","amount":"10000000000000000000000"}}' "$TEST_ADDRESS")" | jq -r '.txhash')"
    wait_tx "$MINT_TX"
  fi

  REG_BEFORE="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$FEE_DISCOUNT" \
    "$(printf '{"get_registration":{"trader":"%s"}}' "$TEST_ADDRESS")")")"
  if [ "$(echo "$REG_BEFORE" | jq -r '.registered // false')" = "true" ]; then
    tier_before="$(echo "$REG_BEFORE" | jq -r '.tier_id // 255')"
    if [ "$tier_before" != "0" ] && [ "$tier_before" != "255" ]; then
      CLEAN_TX="$(terrad_tx wasm execute "$FEE_DISCOUNT" '{"deregister":{}}' | jq -r '.txhash')"
      wait_tx "$CLEAN_TX"
    fi
  fi

  REG_TX="$(terrad_tx wasm execute "$FEE_DISCOUNT" '{"register":{"tier_id":1}}' | jq -r '.txhash')"
  wait_tx "$REG_TX"
  REG_GAS="$(tx_gas_used "$REG_TX" || true)"
  echo "    register tx=$REG_TX gas_used=${REG_GAS:-?}"
  if [ -n "${REG_GAS:-}" ] && [ "$REG_GAS" -lt "$REGISTER_LIMIT" ]; then
    ok "on-chain register gas_used=$REG_GAS < limit=$REGISTER_LIMIT"
  else
    bad "on-chain register gas_used=${REG_GAS:-unavailable} not below limit=$REGISTER_LIMIT"
  fi

  echo ""
  echo "[4] Live LocalTerra: deregister gas_used < DEREGISTER_FEE_DISCOUNT_GAS_LIMIT ($DEREGISTER_LIMIT)..."
  DEREG_TX="$(terrad_tx wasm execute "$FEE_DISCOUNT" '{"deregister":{}}' | jq -r '.txhash')"
  wait_tx "$DEREG_TX"
  DEREG_GAS="$(tx_gas_used "$DEREG_TX" || true)"
  echo "    deregister tx=$DEREG_TX gas_used=${DEREG_GAS:-?}"
  if [ -n "${DEREG_GAS:-}" ] && [ "$DEREG_GAS" -lt "$DEREGISTER_LIMIT" ]; then
    ok "on-chain deregister gas_used=$DEREG_GAS < limit=$DEREGISTER_LIMIT"
  else
    bad "on-chain deregister gas_used=${DEREG_GAS:-unavailable} not below limit=$DEREGISTER_LIMIT"
  fi
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: ${PASS} passed, ${FAIL} failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"
[ "$FAIL" -eq 0 ]
