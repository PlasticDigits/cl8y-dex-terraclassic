#!/bin/bash
# Initialize LocalTerra chain with pre-funded accounts
#
# This script sets up:
# - Chain configuration
# - Test accounts with funds
# - Validator
# - Fast block times
# - Enabled APIs
# - Disabled oracle slashing (no oracle feeder on local chain)

set -e

TERRA_HOME="${TERRA_HOME:-$HOME/.terra}"
CHAIN_ID="${CHAIN_ID:-localterra}"
MONIKER="${MONIKER:-localterra}"

# Test account mnemonic (DO NOT USE IN PRODUCTION)
TEST_MNEMONIC="notice oak worry limit wrap speak medal online prefer cluster roof addict wrist behave treat actual wasp year salad speed social layer crew genius"

echo "=== Initializing LocalTerra ==="
echo "Chain ID: $CHAIN_ID"
echo "Moniker: $MONIKER"
echo "Home: $TERRA_HOME"

# Initialize chain
echo "[1/7] Initializing chain..."
terrad init "$MONIKER" --chain-id "$CHAIN_ID" --home "$TERRA_HOME"

# Import test account
echo "[2/7] Importing test account..."
echo "$TEST_MNEMONIC" | terrad keys add test1 --recover --keyring-backend test --home "$TERRA_HOME"

# Get test account address
TEST_ADDRESS=$(terrad keys show test1 -a --keyring-backend test --home "$TERRA_HOME")
echo "Test account: $TEST_ADDRESS"

# Add genesis account with funds
echo "[3/7] Adding genesis account with funds..."
terrad add-genesis-account "$TEST_ADDRESS" \
    1000000000000uluna,\
10000000000000uusd,\
10000000000000ukrw,\
10000000000000usdr,\
10000000000000ueur,\
10000000000000ugbp,\
10000000000000ujpy,\
10000000000000ucny \
    --keyring-backend test \
    --home "$TERRA_HOME"

# Create validator genesis transaction
echo "[4/7] Creating validator..."
terrad gentx test1 10000000uluna \
    --chain-id "$CHAIN_ID" \
    --keyring-backend test \
    --home "$TERRA_HOME"

# Collect genesis transactions
echo "[5/7] Collecting genesis transactions..."
terrad collect-gentxs --home "$TERRA_HOME"

# Configure for local development
echo "[6/7] Configuring for local development..."

CONFIG_TOML="$TERRA_HOME/config/config.toml"
APP_TOML="$TERRA_HOME/config/app.toml"

# Fast block times (200ms instead of 5s)
sed -i 's/timeout_propose = "3s"/timeout_propose = "200ms"/g' "$CONFIG_TOML"
sed -i 's/timeout_propose_delta = "500ms"/timeout_propose_delta = "100ms"/g' "$CONFIG_TOML"
sed -i 's/timeout_prevote = "1s"/timeout_prevote = "100ms"/g' "$CONFIG_TOML"
sed -i 's/timeout_prevote_delta = "500ms"/timeout_prevote_delta = "100ms"/g' "$CONFIG_TOML"
sed -i 's/timeout_precommit = "1s"/timeout_precommit = "100ms"/g' "$CONFIG_TOML"
sed -i 's/timeout_precommit_delta = "500ms"/timeout_precommit_delta = "100ms"/g' "$CONFIG_TOML"
sed -i 's/timeout_commit = "5s"/timeout_commit = "200ms"/g' "$CONFIG_TOML"

# Bind RPC to all interfaces
sed -i 's/laddr = "tcp:\/\/127.0.0.1:26657"/laddr = "tcp:\/\/0.0.0.0:26657"/g' "$CONFIG_TOML"

# Enable CORS for development
sed -i 's/cors_allowed_origins = \[\]/cors_allowed_origins = ["*"]/g' "$CONFIG_TOML"

# Enable REST API
sed -i 's/enable = false/enable = true/g' "$APP_TOML"
sed -i 's/address = "tcp:\/\/localhost:1317"/address = "tcp:\/\/0.0.0.0:1317"/g' "$APP_TOML"
sed -i 's/enabled-unsafe-cors = false/enabled-unsafe-cors = true/g' "$APP_TOML"

# Enable gRPC
sed -i 's/address = "localhost:9090"/address = "0.0.0.0:9090"/g' "$APP_TOML"

# Set minimum gas price to 0 for easier testing
sed -i 's/minimum-gas-prices = ""/minimum-gas-prices = "0uluna"/g' "$APP_TOML"

# Disable oracle slashing — LocalTerra runs a single validator with no oracle
# feeder, so without this patch the validator gets jailed at block 100800
# (the default slash_window) and the chain permanently stops producing blocks.
echo "[7/7] Patching genesis: disabling oracle slashing..."
GENESIS="$TERRA_HOME/config/genesis.json"
jq '.app_state.oracle.params.slash_fraction = "0.000000000000000000"
  | .app_state.oracle.params.min_valid_per_window = "0.000000000000000000"
  | .app_state.oracle.params.whitelist = []' "$GENESIS" > "${GENESIS}.tmp" && mv "${GENESIS}.tmp" "$GENESIS"

echo ""
echo "=== LocalTerra Initialized ==="
echo ""
echo "Test Account:"
echo "  Address:  $TEST_ADDRESS"
echo "  Mnemonic: $TEST_MNEMONIC"
echo ""
echo "Balances:"
echo "  1,000,000 LUNC (1000000000000 uluna)"
echo "  10,000,000 USTC (10000000000000 uusd)"
echo "  + other stablecoins"
echo ""
echo "Endpoints (after start):"
echo "  RPC:  http://localhost:26657"
echo "  LCD:  http://localhost:1317"
echo "  gRPC: localhost:9090"
echo ""
