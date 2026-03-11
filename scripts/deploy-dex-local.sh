#!/bin/bash
set -e

CHAIN_ID="localterra"
NODE="http://localhost:26657"
LCD="http://localhost:1317"
TEST_ADDRESS="terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v"
CONTAINER_NAME="cl8y-dex-terraclassic-localterra-1"
ARTIFACTS_DIR="$(cd "$(dirname "$0")/../smartcontracts/artifacts" && pwd)"

terrad_tx() {
    docker exec "$CONTAINER_NAME" terrad tx "$@" \
        --from test1 \
        --keyring-backend test \
        --chain-id "$CHAIN_ID" \
        --gas auto \
        --gas-adjustment 1.3 \
        --fees 500000000uluna \
        --node "$NODE" \
        --broadcast-mode sync \
        -y --output json
}

terrad_query() {
    docker exec "$CONTAINER_NAME" terrad query "$@" \
        --node "$NODE" \
        --output json
}

get_code_id() {
    local TX_HASH="$1"
    sleep 3
    local RESULT
    RESULT=$(terrad_query tx "$TX_HASH")
    echo "$RESULT" | jq -r '.logs[0].events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value'
}

get_contract_address() {
    local TX_HASH="$1"
    sleep 3
    local RESULT
    RESULT=$(terrad_query tx "$TX_HASH")
    echo "$RESULT" | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value'
}

echo "=============================================="
echo "CL8Y DEX - Local Deployment"
echo "=============================================="

echo ""
echo "[1/16] Waiting for LocalTerra to be ready..."
for i in $(seq 1 60); do
    if curl -sf "$NODE/status" > /dev/null 2>&1; then
        echo "LocalTerra is ready!"
        break
    fi
    if [ "$i" -eq 60 ]; then
        echo "ERROR: LocalTerra did not start within 60 seconds."
        echo "Make sure it's running: docker compose up -d"
        exit 1
    fi
    echo "  Waiting... ($i/60)"
    sleep 2
done

echo ""
echo "[2/16] Copying wasm artifacts into container..."
if [ ! -d "$ARTIFACTS_DIR" ]; then
    echo "ERROR: artifacts/ directory not found at $ARTIFACTS_DIR"
    echo "Run 'make build-optimized' first."
    exit 1
fi

docker cp "$ARTIFACTS_DIR/." "$CONTAINER_NAME:/tmp/artifacts/"
echo "Artifacts copied."

echo ""
echo "[3/16] Uploading CW20 Mintable wasm..."
CW20_WASM="/tmp/artifacts/cw20_mintable.wasm"
if ! docker exec "$CONTAINER_NAME" test -f "$CW20_WASM"; then
    CW20_WASM="/tmp/artifacts/cw20_base.wasm"
    if ! docker exec "$CONTAINER_NAME" test -f "$CW20_WASM"; then
        echo "ERROR: Neither cw20_mintable.wasm nor cw20_base.wasm found in artifacts."
        exit 1
    fi
fi
TX_HASH=$(terrad_tx wasm store "$CW20_WASM" | jq -r '.txhash')
echo "  TX: $TX_HASH"
CW20_CODE_ID=$(get_code_id "$TX_HASH")
echo "  CW20 Code ID: $CW20_CODE_ID"

echo ""
echo "[4/16] Uploading cl8y_dex_factory.wasm..."
TX_HASH=$(terrad_tx wasm store /tmp/artifacts/cl8y_dex_factory.wasm | jq -r '.txhash')
echo "  TX: $TX_HASH"
FACTORY_CODE_ID=$(get_code_id "$TX_HASH")
echo "  Factory Code ID: $FACTORY_CODE_ID"

echo ""
echo "[5/16] Uploading cl8y_dex_pair.wasm..."
TX_HASH=$(terrad_tx wasm store /tmp/artifacts/cl8y_dex_pair.wasm | jq -r '.txhash')
echo "  TX: $TX_HASH"
PAIR_CODE_ID=$(get_code_id "$TX_HASH")
echo "  Pair Code ID: $PAIR_CODE_ID"

echo ""
echo "[6/16] Uploading cl8y_dex_router.wasm..."
TX_HASH=$(terrad_tx wasm store /tmp/artifacts/cl8y_dex_router.wasm | jq -r '.txhash')
echo "  TX: $TX_HASH"
ROUTER_CODE_ID=$(get_code_id "$TX_HASH")
echo "  Router Code ID: $ROUTER_CODE_ID"

echo ""
echo "[7/16] Instantiating Factory..."
FACTORY_INIT_MSG=$(cat <<EOF
{
  "governance": "$TEST_ADDRESS",
  "treasury": "$TEST_ADDRESS",
  "default_fee_bps": 180,
  "pair_code_id": $PAIR_CODE_ID,
  "lp_token_code_id": $CW20_CODE_ID,
  "whitelisted_code_ids": [$CW20_CODE_ID]
}
EOF
)
TX_HASH=$(terrad_tx wasm instantiate "$FACTORY_CODE_ID" "$FACTORY_INIT_MSG" \
    --label "cl8y-dex-factory" \
    --admin "$TEST_ADDRESS" | jq -r '.txhash')
echo "  TX: $TX_HASH"
FACTORY_ADDRESS=$(get_contract_address "$TX_HASH")
echo "  Factory Address: $FACTORY_ADDRESS"

echo ""
echo "[8/16] Instantiating Router..."
ROUTER_INIT_MSG="{\"factory\": \"$FACTORY_ADDRESS\"}"
TX_HASH=$(terrad_tx wasm instantiate "$ROUTER_CODE_ID" "$ROUTER_INIT_MSG" \
    --label "cl8y-dex-router" \
    --admin "$TEST_ADDRESS" | jq -r '.txhash')
echo "  TX: $TX_HASH"
ROUTER_ADDRESS=$(get_contract_address "$TX_HASH")
echo "  Router Address: $ROUTER_ADDRESS"

echo ""
echo "[9/16] Instantiating Test Token A (TSTA)..."
TOKEN_A_INIT_MSG=$(cat <<EOF
{
  "name": "Token A",
  "symbol": "TSTA",
  "decimals": 6,
  "initial_balances": [
    {
      "address": "$TEST_ADDRESS",
      "amount": "1000000000000"
    }
  ],
  "mint": {
    "minter": "$TEST_ADDRESS"
  }
}
EOF
)
TX_HASH=$(terrad_tx wasm instantiate "$CW20_CODE_ID" "$TOKEN_A_INIT_MSG" \
    --label "test-token-a" \
    --admin "$TEST_ADDRESS" | jq -r '.txhash')
echo "  TX: $TX_HASH"
TOKEN_A_ADDRESS=$(get_contract_address "$TX_HASH")
echo "  Token A Address: $TOKEN_A_ADDRESS"

echo ""
echo "[10/16] Instantiating Test Token B (TSTB)..."
TOKEN_B_INIT_MSG=$(cat <<EOF
{
  "name": "Token B",
  "symbol": "TSTB",
  "decimals": 6,
  "initial_balances": [
    {
      "address": "$TEST_ADDRESS",
      "amount": "1000000000000"
    }
  ],
  "mint": {
    "minter": "$TEST_ADDRESS"
  }
}
EOF
)
TX_HASH=$(terrad_tx wasm instantiate "$CW20_CODE_ID" "$TOKEN_B_INIT_MSG" \
    --label "test-token-b" \
    --admin "$TEST_ADDRESS" | jq -r '.txhash')
echo "  TX: $TX_HASH"
TOKEN_B_ADDRESS=$(get_contract_address "$TX_HASH")
echo "  Token B Address: $TOKEN_B_ADDRESS"

echo ""
echo "[11/16] Creating test pair (TSTA/TSTB) via Factory..."
CREATE_PAIR_MSG=$(cat <<EOF
{
  "create_pair": {
    "asset_infos": [
      { "token": { "contract_addr": "$TOKEN_A_ADDRESS" } },
      { "token": { "contract_addr": "$TOKEN_B_ADDRESS" } }
    ]
  }
}
EOF
)
TX_HASH=$(terrad_tx wasm execute "$FACTORY_ADDRESS" "$CREATE_PAIR_MSG" | jq -r '.txhash')
echo "  TX: $TX_HASH"
sleep 3
PAIR_RESULT=$(terrad_query tx "$TX_HASH")
PAIR_ADDRESS=$(echo "$PAIR_RESULT" | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value' | head -1)
echo "  Pair Address: $PAIR_ADDRESS"

echo ""
echo "[12/16] Uploading cl8y_dex_fee_discount.wasm..."
TX_HASH=$(terrad_tx wasm store /tmp/artifacts/cl8y_dex_fee_discount.wasm | jq -r '.txhash')
echo "  TX: $TX_HASH"
FEE_DISCOUNT_CODE_ID=$(get_code_id "$TX_HASH")
echo "  Fee Discount Code ID: $FEE_DISCOUNT_CODE_ID"

echo ""
echo "[13/16] Instantiating Fee Discount contract..."
FEE_DISCOUNT_INIT_MSG=$(cat <<EOF
{
  "governance": "$TEST_ADDRESS",
  "cl8y_token": "$TOKEN_A_ADDRESS"
}
EOF
)
TX_HASH=$(terrad_tx wasm instantiate "$FEE_DISCOUNT_CODE_ID" "$FEE_DISCOUNT_INIT_MSG" \
    --label "cl8y-dex-fee-discount" \
    --admin "$TEST_ADDRESS" | jq -r '.txhash')
echo "  TX: $TX_HASH"
FEE_DISCOUNT_ADDRESS=$(get_contract_address "$TX_HASH")
echo "  Fee Discount Address: $FEE_DISCOUNT_ADDRESS"

echo ""
echo "[14/16] Adding fee discount tiers..."
for TIER_DATA in \
  '{"add_tier":{"tier_id":0,"min_cl8y_balance":"0","discount_bps":10000,"governance_only":true}}' \
  '{"add_tier":{"tier_id":1,"min_cl8y_balance":"1000000000000000000","discount_bps":1000,"governance_only":false}}' \
  '{"add_tier":{"tier_id":2,"min_cl8y_balance":"50000000000000000000","discount_bps":2500,"governance_only":false}}' \
  '{"add_tier":{"tier_id":3,"min_cl8y_balance":"200000000000000000000","discount_bps":3500,"governance_only":false}}' \
  '{"add_tier":{"tier_id":4,"min_cl8y_balance":"1000000000000000000000","discount_bps":5000,"governance_only":false}}' \
  '{"add_tier":{"tier_id":5,"min_cl8y_balance":"15000000000000000000000","discount_bps":8000,"governance_only":false}}' \
  '{"add_tier":{"tier_id":255,"min_cl8y_balance":"0","discount_bps":0,"governance_only":true}}'
do
  TX_HASH=$(terrad_tx wasm execute "$FEE_DISCOUNT_ADDRESS" "$TIER_DATA" | jq -r '.txhash')
  echo "  Added tier: $TX_HASH"
  sleep 2
done
echo "  All tiers added."

echo ""
echo "[15/16] Adding trusted router and setting discount registry..."
TX_HASH=$(terrad_tx wasm execute "$FEE_DISCOUNT_ADDRESS" \
  "{\"add_trusted_router\":{\"router\":\"$ROUTER_ADDRESS\"}}" | jq -r '.txhash')
echo "  Added trusted router: $TX_HASH"
sleep 3

TX_HASH=$(terrad_tx wasm execute "$FACTORY_ADDRESS" \
  "{\"set_discount_registry\":{\"pair\":\"$PAIR_ADDRESS\",\"registry\":\"$FEE_DISCOUNT_ADDRESS\"}}" | jq -r '.txhash')
echo "  Set discount registry on pair: $TX_HASH"
sleep 3

echo ""
echo "[16/16] Deployment complete!"
echo ""
echo "=============================================="
echo "  Contract Addresses"
echo "=============================================="
echo "  Factory:       $FACTORY_ADDRESS"
echo "  Router:        $ROUTER_ADDRESS"
echo "  Fee Discount:  $FEE_DISCOUNT_ADDRESS"
echo "  Token A:       $TOKEN_A_ADDRESS"
echo "  Token B:       $TOKEN_B_ADDRESS"
echo "  Pair (A/B):    $PAIR_ADDRESS"
echo "=============================================="
echo ""
echo "Update frontend-dapp/.env:"
echo "  VITE_FACTORY_ADDRESS=$FACTORY_ADDRESS"
echo "  VITE_ROUTER_ADDRESS=$ROUTER_ADDRESS"
echo "  VITE_FEE_DISCOUNT_ADDRESS=$FEE_DISCOUNT_ADDRESS"
echo "  VITE_NETWORK=local"
echo "  VITE_TERRA_LCD_URL=$LCD"
echo "  VITE_TERRA_RPC_URL=$NODE"
echo ""
echo "Test address: $TEST_ADDRESS"
echo "  Has 1M TSTA and 1M TSTB"
