#!/bin/bash
set -e

NETWORK="${1:-testnet}"
OWNER="${2}"

if [ -z "$OWNER" ]; then
  echo "Usage: ./scripts/deploy.sh <network> <owner_address>"
  echo "  network: testnet | mainnet"
  echo "  owner_address: terra1... address that will own the factory and router"
  exit 1
fi

case "$NETWORK" in
  testnet)
    CHAIN_ID="rebel-2"
    NODE="https://terra-classic-rpc.publicnode.com:443"
    ;;
  mainnet)
    CHAIN_ID="columbus-5"
    NODE="https://terra-classic-rpc.publicnode.com:443"
    ;;
  *)
    echo "Unknown network: $NETWORK (use testnet or mainnet)"
    exit 1
    ;;
esac

ARTIFACTS_DIR="$(dirname "$0")/../artifacts"

echo "=============================================="
echo "CL8Y DEX Deployment - $NETWORK ($CHAIN_ID)"
echo "=============================================="
echo ""
echo "Owner: $OWNER"
echo "Node:  $NODE"
echo ""

if [ ! -d "$ARTIFACTS_DIR" ]; then
  echo "ERROR: artifacts/ directory not found."
  echo "Run 'make build-optimized' first to generate optimized wasm files."
  exit 1
fi

echo "Step 1: Upload contracts"
echo "========================"
echo ""
echo "Upload CW20 Base (LP token):"
echo "  terrad tx wasm store $ARTIFACTS_DIR/cw20_base.wasm \\"
echo "    --from wallet \\"
echo "    --chain-id $CHAIN_ID \\"
echo "    --node $NODE \\"
echo "    --gas auto --gas-adjustment 1.3 \\"
echo "    --fees 5000000uluna \\"
echo "    --broadcast-mode sync -y"
echo ""
echo "Upload Factory:"
echo "  terrad tx wasm store $ARTIFACTS_DIR/cl8y_dex_factory.wasm \\"
echo "    --from wallet \\"
echo "    --chain-id $CHAIN_ID \\"
echo "    --node $NODE \\"
echo "    --gas auto --gas-adjustment 1.3 \\"
echo "    --fees 5000000uluna \\"
echo "    --broadcast-mode sync -y"
echo ""
echo "Upload Pair:"
echo "  terrad tx wasm store $ARTIFACTS_DIR/cl8y_dex_pair.wasm \\"
echo "    --from wallet \\"
echo "    --chain-id $CHAIN_ID \\"
echo "    --node $NODE \\"
echo "    --gas auto --gas-adjustment 1.3 \\"
echo "    --fees 5000000uluna \\"
echo "    --broadcast-mode sync -y"
echo ""
echo "Upload Router:"
echo "  terrad tx wasm store $ARTIFACTS_DIR/cl8y_dex_router.wasm \\"
echo "    --from wallet \\"
echo "    --chain-id $CHAIN_ID \\"
echo "    --node $NODE \\"
echo "    --gas auto --gas-adjustment 1.3 \\"
echo "    --fees 5000000uluna \\"
echo "    --broadcast-mode sync -y"
echo ""

echo "Step 2: Query code IDs"
echo "======================"
echo ""
echo "After each upload, query the tx to get the code_id:"
echo "  terrad query tx <TX_HASH> --node $NODE --output json | jq '.logs[0].events[] | select(.type==\"store_code\") | .attributes[] | select(.key==\"code_id\") | .value'"
echo ""

echo "Step 3: Instantiate Factory"
echo "==========================="
echo ""
echo "Replace <CW20_CODE_ID>, <PAIR_CODE_ID> with actual code IDs:"
echo ""
echo "  terrad tx wasm instantiate <FACTORY_CODE_ID> '{\"governance\": \"$OWNER\", \"treasury\": \"$OWNER\", \"default_fee_bps\": 180, \"pair_code_id\": <PAIR_CODE_ID>, \"lp_token_code_id\": <CW20_CODE_ID>, \"whitelisted_code_ids\": [<CW20_CODE_ID>]}' \\"
echo "    --label \"cl8y-dex-factory\" \\"
echo "    --admin $OWNER \\"
echo "    --from wallet \\"
echo "    --chain-id $CHAIN_ID \\"
echo "    --node $NODE \\"
echo "    --gas auto --gas-adjustment 1.3 \\"
echo "    --fees 2000000uluna \\"
echo "    --broadcast-mode sync -y"
echo ""

echo "Step 4: Instantiate Router"
echo "=========================="
echo ""
echo "Replace <FACTORY_ADDRESS> with the factory contract address:"
echo ""
echo "  terrad tx wasm instantiate <ROUTER_CODE_ID> '{\"factory\": \"<FACTORY_ADDRESS>\"}' \\"
echo "    --label \"cl8y-dex-router\" \\"
echo "    --admin $OWNER \\"
echo "    --from wallet \\"
echo "    --chain-id $CHAIN_ID \\"
echo "    --node $NODE \\"
echo "    --gas auto --gas-adjustment 1.3 \\"
echo "    --fees 2000000uluna \\"
echo "    --broadcast-mode sync -y"
echo ""

echo "Step 5: Verify deployment"
echo "========================="
echo ""
echo "Query factory config:"
echo "  terrad query wasm contract-state smart <FACTORY_ADDRESS> '{\"config\": {}}' --node $NODE --output json"
echo ""
echo "Query router config:"
echo "  terrad query wasm contract-state smart <ROUTER_ADDRESS> '{\"config\": {}}' --node $NODE --output json"
echo ""
echo "=============================================="
echo "Update frontend-dapp/.env with:"
echo "  VITE_FACTORY_ADDRESS=<FACTORY_ADDRESS>"
echo "  VITE_ROUTER_ADDRESS=<ROUTER_ADDRESS>"
echo "  VITE_NETWORK=$NETWORK"
echo "=============================================="
