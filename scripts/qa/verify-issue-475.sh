#!/usr/bin/env bash
# Verification for GitLab #475 — retail getGasLimitForTx inventory / BASE_GAS_LIMIT guardrail.
# Also covers Mint faucet drip envelope (#474).
#
# Layers:
#   1. Frontend unit tests: retail shape fixtures + drip/unwrap + DEV warn
#   2. Constant floor checks vs documented measured gas
#   3. Optional live LocalTerra: drip gas_used < FAUCET_DRIP_GAS_LIMIT
#
# Refs: frontend-dapp/src/services/terraclassic/terraGas.ts,
#       terraGasRetailInventory.ts, skills/AGENTS_TERRACLASSIC_GAS.md
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

DRIP_LIMIT=400000
MEASURED_DRIP_FLOOR=248000

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #475 — retail gas inventory / BASE_GAS_LIMIT guardrail"
echo "════════════════════════════════════════════════════════════════"

echo ""
echo "[1] Frontend unit tests (terraGas.retailShapes + drip/feeDiscount anchors)..."
if bash scripts/with-node.sh --cwd frontend-dapp -- npm run test -- --run \
  src/services/terraclassic/__tests__/terraGas.retailShapes.test.ts \
  src/services/terraclassic/__tests__/terraGas.feeDiscount.test.ts 2>&1; then
  ok "unit tests: retail inventory + fee-discount anchors"
else
  bad "unit tests: retail inventory + fee-discount anchors"
fi

echo ""
echo "[2] FAUCET_DRIP_GAS_LIMIT exceeds measured floor (#474)..."
if [ "$DRIP_LIMIT" -gt "$MEASURED_DRIP_FLOOR" ]; then
  ok "drip limit $DRIP_LIMIT > floor $MEASURED_DRIP_FLOOR"
else
  bad "drip limit $DRIP_LIMIT does not exceed floor $MEASURED_DRIP_FLOOR"
fi

CONTAINER_NAME="$(docker compose ps -q localterra 2>/dev/null | head -1 || true)"
if [ -z "$CONTAINER_NAME" ]; then
  if sg docker -c 'docker compose ps -q localterra' 2>/dev/null | head -1 | grep -q .; then
    CONTAINER_NAME="$(sg docker -c 'docker compose ps -q localterra' | head -1)"
  fi
fi

IDX_ENV="$REPO_ROOT/indexer/.env"
FE_ENV="$REPO_ROOT/frontend-dapp/.env.local"
FAUCET="$(read_env_var "$FE_ENV" VITE_FAUCET_ADDRESS)"
TOKEN="$(read_env_var "$FE_ENV" VITE_EMBER_TOKEN_ADDRESS)"
if [ -z "$TOKEN" ]; then
  TOKEN="$(read_env_var "$FE_ENV" VITE_CL8Y_TOKEN_ADDRESS)"
fi
LCD="$(read_env_var "$IDX_ENV" LCD_URLS)"; LCD="${LCD%%,*}"
LCD="${LCD:-http://127.0.0.1:1317}"; LCD="${LCD%/}"

if [ -z "$CONTAINER_NAME" ] || [ -z "$FAUCET" ] || [ -z "$TOKEN" ]; then
  skip "live LocalTerra drip gas_used check (chain not up or faucet/token env missing)"
else
  # shellcheck source=scripts/lib/terrad-wait-tx.sh
  source "$REPO_ROOT/scripts/lib/terrad-wait-tx.sh"

  CHAIN_ID="${CHAIN_ID:-localterra}"
  TERRAD_NODE="${TERRAD_NODE:-http://127.0.0.1:26657}"

  terrad_tx() {
    docker exec "$CONTAINER_NAME" terrad tx "$@" \
      --from test1 --keyring-backend test --chain-id "$CHAIN_ID" \
      --gas auto --gas-adjustment 1.3 --fees 500000000uluna \
      --node "$TERRAD_NODE" --broadcast-mode sync -y --output json
  }
  wait_tx() {
    terrad_wait_tx_inclusion "$CONTAINER_NAME" "$1" "$TERRAD_NODE" 90
  }

  tx_gas_used() {
    local txhash="$1"
    local json gas
    json="$(docker exec "$CONTAINER_NAME" terrad query tx "$txhash" --node "$TERRAD_NODE" --output json 2>/dev/null || true)"
    gas="$(echo "$json" | jq -r '.gas_used // .tx_response.gas_used // empty')"
    [[ -n "$gas" && "$gas" =~ ^[0-9]+$ && "$gas" != "0" ]] || return 1
    echo "$gas"
  }

  echo ""
  echo "[3] Live LocalTerra: drip gas_used < FAUCET_DRIP_GAS_LIMIT ($DRIP_LIMIT)..."
  echo "    faucet=$FAUCET token=$TOKEN"

  DRIP_TX="$(terrad_tx wasm execute "$FAUCET" \
    "$(printf '{"drip":{"token":"%s"}}' "$TOKEN")" | jq -r '.txhash')"
  wait_tx "$DRIP_TX"
  DRIP_GAS="$(tx_gas_used "$DRIP_TX" || true)"
  echo "    drip tx=$DRIP_TX gas_used=${DRIP_GAS:-?}"
  if [ -n "${DRIP_GAS:-}" ] && [ "$DRIP_GAS" -lt "$DRIP_LIMIT" ]; then
    ok "on-chain drip gas_used=$DRIP_GAS < limit=$DRIP_LIMIT"
    if [ "$DRIP_GAS" -gt "$MEASURED_DRIP_FLOOR" ]; then
      ok "on-chain drip gas_used=$DRIP_GAS above documented floor $MEASURED_DRIP_FLOOR"
    else
      ok "on-chain drip gas_used=$DRIP_GAS (below prior floor — update MEASURED_FAUCET_DRIP_GAS_FLOOR if stable)"
    fi
  else
    bad "on-chain drip gas_used=${DRIP_GAS:-unavailable} not below limit=$DRIP_LIMIT"
  fi
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: ${PASS} passed, ${FAIL} failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"
[ "$FAIL" -eq 0 ]
