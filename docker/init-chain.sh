#!/bin/bash
# Initialize LocalTerra chain with pre-funded accounts (Terra Classic terrad v4 / SDK 0.53).
#
# This script sets up:
# - Chain configuration (bond_denom / gov min_deposit aligned to uluna)
# - Test accounts with funds
# - Validator
# - Fast block times
# - Enabled APIs
# - Disabled oracle slashing (no oracle feeder on local chain)
#
# GitLab #292 — keep in sync with ghcr.io/plasticdigits/localterra-cl8y image init when bumping digest.

set -e

TERRA_HOME="${TERRA_HOME:-$HOME/.terra}"
CHAIN_ID="${CHAIN_ID:-localterra}"
MONIKER="${MONIKER:-localterra}"
DENOM="${DENOM:-uluna}"

# Test account mnemonic (DO NOT USE IN PRODUCTION)
TEST_MNEMONIC="notice oak worry limit wrap speak medal online prefer cluster roof addict wrist behave treat actual wasp year salad speed social layer crew genius"

echo "=== Initializing LocalTerra ==="
echo "Chain ID: $CHAIN_ID"
echo "Moniker: $MONIKER"
echo "Home: $TERRA_HOME"

update_genesis() {
    local genesis="$TERRA_HOME/config/genesis.json"
    jq "$1" "$genesis" > "${genesis}.tmp" && mv "${genesis}.tmp" "$genesis"
}

# Initialize chain
echo "[1/8] Initializing chain..."
terrad init "$MONIKER" --chain-id "$CHAIN_ID" --home "$TERRA_HOME"

# Import test account
echo "[2/8] Importing test account..."
echo "$TEST_MNEMONIC" | terrad keys add test1 --recover --keyring-backend test --home "$TERRA_HOME"

# Get test account address
TEST_ADDRESS=$(terrad keys show test1 -a --keyring-backend test --home "$TERRA_HOME")
echo "Test account: $TEST_ADDRESS"

# Add genesis account with funds
echo "[3/8] Adding genesis account with funds..."
terrad add-genesis-account "$TEST_ADDRESS" \
    10000000000000uluna,\
100000000000000uusd,\
100000000000000ukrw,\
100000000000000usdr,\
100000000000000ueur,\
100000000000000ugbp,\
100000000000000ujpy,\
100000000000000ucny \
    --keyring-backend test \
    --home "$TERRA_HOME"

# SDK 0.53 defaults bond_denom to "stake"; align genesis with Terra Classic denoms
echo "[4/8] Configuring genesis parameters..."
update_genesis '.app_state["mint"]["params"]["mint_denom"]="'$DENOM'"'
update_genesis '.app_state["gov"]["deposit_params"]["min_deposit"]=[{"denom":"'$DENOM'","amount": "1000000"}]'
update_genesis '.app_state["gov"]["params"]["min_deposit"]=[{"denom":"'$DENOM'","amount": "1000000"}]'
update_genesis '.app_state["gov"]["params"]["voting_period"]="30s"'
update_genesis '.app_state["gov"]["voting_params"]["voting_period"]="30s"'
if jq -e '.app_state["gov"]["params"]["expedited_voting_period"]' "$TERRA_HOME/config/genesis.json" > /dev/null 2>&1; then
    update_genesis '.app_state["gov"]["params"]["expedited_voting_period"]="4s"'
fi
update_genesis '.app_state["crisis"]["constant_fee"]={"denom":"'$DENOM'","amount":"1000"}'
update_genesis '.app_state["staking"]["params"]["bond_denom"]="'$DENOM'"'

# Create validator genesis transaction
echo "[5/8] Creating validator..."
terrad gentx test1 10000000uluna \
    --commission-rate=0.01 \
    --commission-max-rate=0.02 \
    --chain-id "$CHAIN_ID" \
    --keyring-backend test \
    --home "$TERRA_HOME"

# Collect genesis transactions
echo "[6/8] Collecting genesis transactions..."
terrad collect-gentxs --home "$TERRA_HOME"

# NOTE: terrad v4 (cosmos-sdk 0.53) `validate-genesis` SIGSEGVs (nil deref in x/genutil
# ValidateAndGetGenTx) on any genesis containing collected gentxs — which this one always
# does here. The old `|| echo WARNING` swallowed that panic, leaving a dead safety net.
# Validate JSON well-formedness + that gentxs were actually collected instead.
GENESIS_FILE="$TERRA_HOME/config/genesis.json"
if jq -e '.app_state.genutil.gen_txs | length > 0' "$GENESIS_FILE" > /dev/null 2>&1; then
    echo "Genesis OK: $(jq -r '.app_state.genutil.gen_txs | length' "$GENESIS_FILE") gentx(s) collected."
else
    echo "ERROR: genesis.json is malformed or has no collected gentxs" >&2
    exit 1
fi

# Configure for local development
echo "[7/8] Configuring for local development..."

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

# Enable REST API (first enable = false is the API server toggle)
sed -i '0,/enable = false/s//enable = true/' "$APP_TOML"
sed -i 's/swagger = false/swagger = true/' "$APP_TOML"
sed -i 's/address = "tcp:\/\/localhost:1317"/address = "tcp:\/\/0.0.0.0:1317"/g' "$APP_TOML"
sed -i 's/enabled-unsafe-cors = false/enabled-unsafe-cors = true/g' "$APP_TOML"

# Enable gRPC
sed -i 's/address = "localhost:9090"/address = "0.0.0.0:9090"/g' "$APP_TOML"

# Set minimum gas price to 0 for easier testing
sed -i 's/minimum-gas-prices = ""/minimum-gas-prices = "0uluna"/g' "$APP_TOML"

# Disable oracle slashing — LocalTerra runs a single validator with no oracle
# feeder, so without this patch the validator gets jailed at block 100800
# (the default slash_window) and the chain permanently stops producing blocks.
echo "[8/8] Patching genesis: disabling oracle slashing..."
GENESIS="$TERRA_HOME/config/genesis.json"
if jq -e '.app_state.oracle.params' "$GENESIS" > /dev/null 2>&1; then
  jq '.app_state.oracle.params.slash_fraction = "0.000000000000000000"
    | .app_state.oracle.params.min_valid_per_window = "0.000000000000000000"
    | .app_state.oracle.params.whitelist = []' "$GENESIS" > "${GENESIS}.tmp" && mv "${GENESIS}.tmp" "$GENESIS"
else
  echo "WARNING: genesis has no app_state.oracle.params — skipping oracle slash patch"
fi

echo ""
echo "=== LocalTerra Initialized ==="
echo ""
echo "Test Account:"
echo "  Address:  $TEST_ADDRESS"
echo "  Mnemonic: $TEST_MNEMONIC"
echo ""
echo "Balances:"
echo "  10,000,000 LUNC (10000000000000 uluna)"
echo "  100,000,000 USTC (100000000000000 uusd)"
echo "  + other stablecoins"
echo ""
echo "Endpoints (after start):"
echo "  RPC:  http://localhost:26657"
echo "  LCD:  http://localhost:1317"
echo "  gRPC: localhost:9090"
echo ""
