#!/bin/bash
set -e

CHAIN_ID="localterra"
NODE="http://localhost:26657"
LCD="http://localhost:1317"
TEST_ADDRESS="terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v"
CONTAINER_NAME=$(docker compose ps -q localterra 2>/dev/null | head -1)
if [ -z "$CONTAINER_NAME" ]; then
    echo "ERROR: localterra container not found. Run 'make start' first."
    exit 1
fi
ARTIFACTS_DIR="$(cd "$(dirname "$0")/../smartcontracts/artifacts" && pwd)"
CONTRACTS_DIR="$(cd "$(dirname "$0")/../smartcontracts/contracts" && pwd)"

# ── Staleness check ────────────────────────────────────────────────────
# Fail fast if any WASM artifact is older than its source, so QA doesn't
# chase phantom contract errors from a stale build.
STALE_CONTRACTS=()
for wasm in "$ARTIFACTS_DIR"/cl8y_dex_*.wasm; do
    [ -f "$wasm" ] || continue
    basename=$(basename "$wasm" .wasm)
    # cl8y_dex_factory.wasm  -> contracts/factory
    # cl8y_dex_burn_hook.wasm -> contracts/hooks/burn-hook
    short=${basename#cl8y_dex_}            # e.g. "factory", "burn_hook"
    short_dash=${short//_/-}               # e.g. "factory", "burn-hook"
    if [ -d "$CONTRACTS_DIR/$short_dash" ]; then
        src_dir="$CONTRACTS_DIR/$short_dash"
    elif [ -d "$CONTRACTS_DIR/hooks/$short_dash" ]; then
        src_dir="$CONTRACTS_DIR/hooks/$short_dash"
    else
        continue
    fi
    newest_src=$(find "$src_dir/src" -name '*.rs' -newer "$wasm" 2>/dev/null | head -1)
    if [ -n "$newest_src" ]; then
        STALE_CONTRACTS+=("$basename")
    fi
done
if [ ${#STALE_CONTRACTS[@]} -gt 0 ]; then
    echo ""
    echo "ERROR: Stale WASM artifacts detected — source is newer than the build:"
    for sc in "${STALE_CONTRACTS[@]}"; do
        echo "  - $sc.wasm"
    done
    echo ""
    echo "Run 'make build-optimized' first, then re-run this script."
    exit 1
fi

TOKEN_NAMES=("Ember" "Coral" "Jade" "Onyx" "Ruby" "Topaz" "Opal" "Cobalt" "Slate" "Amber")
TOKEN_SYMBOLS=("EMBER" "CORAL" "JADE" "ONYX" "RUBY" "TOPAZ" "OPAL" "COBALT" "SLATE" "AMBER")
TOKEN_ADDRESSES=()

NOWHITELIST_NAMES=("Rogue" "Bogus")
NOWHITELIST_SYMBOLS=("ROGUE" "BOGUS")
NOWHITELIST_ADDRESSES=()

UNPAIRED_NAMES=("Zinc" "Iron" "Neon")
UNPAIRED_SYMBOLS=("ZINC" "IRON" "NEON")
UNPAIRED_ADDRESSES=()

# Pair configs: tokenA_index:tokenB_index:liquidityA(micro):liquidityB(micro)
PAIR_CONFIGS=(
  "0:1:100000000000:100000000000"     # EMBER/CORAL     1:1
  "0:2:1000000000:100000000000"       # EMBER/JADE      1:100
  "0:3:2000000000:100000000000"       # EMBER/ONYX      1:50
  "1:2:100000000000:1000000000"       # CORAL/JADE      100:1
  "1:3:100000000000:1000000000"       # CORAL/ONYX      100:1
  "1:4:50000000000:1000000000"        # CORAL/RUBY      50:1
  "3:4:10000000000:100000000000"      # ONYX/RUBY       1:10
  "3:5:20000000000:100000000000"      # ONYX/TOPAZ      1:5
  "3:6:50000000000:100000000000"      # ONYX/OPAL       1:2
  "2:5:2000000000:100000000000"       # JADE/TOPAZ      1:50
  "2:6:5000000000:100000000000"       # JADE/OPAL       1:20
  "2:4:3000000000:90000000000"        # JADE/RUBY       1:30
  "4:5:50000000000:100000000000"      # RUBY/TOPAZ      1:2
  "4:7:1000000000:100000000000"       # RUBY/COBALT     1:100
  "5:6:33000000000:100000000000"      # TOPAZ/OPAL      1:3
  "7:8:10000000000:100000000000"      # COBALT/SLATE    1:10
  "7:9:20000000000:100000000000"      # COBALT/AMBER    1:5
  "8:9:50000000000:100000000000"      # SLATE/AMBER     1:2
  "0:7:200000000:100000000000"        # EMBER/COBALT    1:500
  "6:9:25000000000:100000000000"      # OPAL/AMBER      1:4
)
PAIR_ADDRESSES=()

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
echo "  10 Tokens, 3 Unpaired Tokens, 2 Non-Whitelisted Tokens, 23 Pairs"
echo "=============================================="

# ── Phase 1: Infrastructure ─────────────────────────────────────────────

echo ""
echo "[Phase 1] Infrastructure Setup"
echo "----------------------------------------------"

echo ""
echo "[1] Waiting for LocalTerra to be ready..."
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
echo "[2] Copying wasm artifacts into container..."
if [ ! -d "$ARTIFACTS_DIR" ]; then
    echo "ERROR: artifacts/ directory not found at $ARTIFACTS_DIR"
    echo "Run 'make build-optimized' first."
    exit 1
fi
docker cp "$ARTIFACTS_DIR/." "$CONTAINER_NAME:/tmp/artifacts/"
echo "Artifacts copied."

echo ""
echo "[3] Uploading CW20 Mintable wasm..."
if [ ! -f "$ARTIFACTS_DIR/cw20_mintable.wasm" ] && [ ! -f "$ARTIFACTS_DIR/cw20_base.wasm" ]; then
    echo "  cw20_mintable.wasm not found in artifacts — building from source..."
    CW20_TMP_DIR=$(mktemp -d)
    git clone --depth 1 https://github.com/PlasticDigits/cw20-mintable.git "$CW20_TMP_DIR" 2>&1 | tail -1
    docker run --rm -v "$CW20_TMP_DIR":/code \
        --mount type=volume,source=cw20_mintable_cache,target=/code/target \
        --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
        cosmwasm/workspace-optimizer:0.16.1
    cp "$CW20_TMP_DIR/artifacts/cw20_mintable.wasm" "$ARTIFACTS_DIR/"
    rm -rf "$CW20_TMP_DIR"
    echo "  cw20_mintable.wasm built and copied to artifacts."
    docker cp "$ARTIFACTS_DIR/cw20_mintable.wasm" "$CONTAINER_NAME:/tmp/artifacts/"
fi
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
echo "[3b] Uploading CW20 wasm again (for non-whitelisted code ID)..."
TX_HASH=$(terrad_tx wasm store "$CW20_WASM" | jq -r '.txhash')
echo "  TX: $TX_HASH"
CW20_CODE_ID_NOWHITELIST=$(get_code_id "$TX_HASH")
echo "  Non-whitelisted CW20 Code ID: $CW20_CODE_ID_NOWHITELIST"

echo ""
echo "[4] Uploading cl8y_dex_factory.wasm..."
TX_HASH=$(terrad_tx wasm store /tmp/artifacts/cl8y_dex_factory.wasm | jq -r '.txhash')
echo "  TX: $TX_HASH"
FACTORY_CODE_ID=$(get_code_id "$TX_HASH")
echo "  Factory Code ID: $FACTORY_CODE_ID"

echo ""
echo "[5] Uploading cl8y_dex_pair.wasm..."
TX_HASH=$(terrad_tx wasm store /tmp/artifacts/cl8y_dex_pair.wasm | jq -r '.txhash')
echo "  TX: $TX_HASH"
PAIR_CODE_ID=$(get_code_id "$TX_HASH")
echo "  Pair Code ID: $PAIR_CODE_ID"

echo ""
echo "[6] Uploading cl8y_dex_router.wasm..."
TX_HASH=$(terrad_tx wasm store /tmp/artifacts/cl8y_dex_router.wasm | jq -r '.txhash')
echo "  TX: $TX_HASH"
ROUTER_CODE_ID=$(get_code_id "$TX_HASH")
echo "  Router Code ID: $ROUTER_CODE_ID"

echo ""
echo "[7] Uploading cl8y_dex_fee_discount.wasm..."
TX_HASH=$(terrad_tx wasm store /tmp/artifacts/cl8y_dex_fee_discount.wasm | jq -r '.txhash')
echo "  TX: $TX_HASH"
FEE_DISCOUNT_CODE_ID=$(get_code_id "$TX_HASH")
echo "  Fee Discount Code ID: $FEE_DISCOUNT_CODE_ID"

echo ""
echo "[8] Instantiating Factory..."
FACTORY_INIT_MSG="{\"governance\":\"$TEST_ADDRESS\",\"treasury\":\"$TEST_ADDRESS\",\"default_fee_bps\":180,\"pair_code_id\":$PAIR_CODE_ID,\"lp_token_code_id\":$CW20_CODE_ID,\"whitelisted_code_ids\":[$CW20_CODE_ID]}"
TX_HASH=$(terrad_tx wasm instantiate "$FACTORY_CODE_ID" "$FACTORY_INIT_MSG" \
    --label "cl8y-dex-factory" \
    --admin "$TEST_ADDRESS" | jq -r '.txhash')
echo "  TX: $TX_HASH"
FACTORY_ADDRESS=$(get_contract_address "$TX_HASH")
echo "  Factory Address: $FACTORY_ADDRESS"

echo ""
echo "[9] Instantiating Router..."
ROUTER_INIT_MSG="{\"factory\": \"$FACTORY_ADDRESS\"}"
TX_HASH=$(terrad_tx wasm instantiate "$ROUTER_CODE_ID" "$ROUTER_INIT_MSG" \
    --label "cl8y-dex-router" \
    --admin "$TEST_ADDRESS" | jq -r '.txhash')
echo "  TX: $TX_HASH"
ROUTER_ADDRESS=$(get_contract_address "$TX_HASH")
echo "  Router Address: $ROUTER_ADDRESS"

# ── Phase 1b: Treasury & Wrap-Mapper ────────────────────────────────────

echo ""
echo "[Phase 1b] Treasury & Wrap-Mapper Setup"
echo "----------------------------------------------"

echo ""
echo "[9b.1] Uploading treasury.wasm..."
if [ ! -f "$ARTIFACTS_DIR/treasury.wasm" ]; then
    echo "  treasury.wasm not found in artifacts — building from source..."
    USTR_TMP_DIR=$(mktemp -d)
    git clone --depth 1 https://gitlab.com/PlasticDigits/ustr-cmm.git "$USTR_TMP_DIR" 2>&1 | tail -1
    git -C "$USTR_TMP_DIR" submodule update --init --recursive 2>&1 | tail -1
    docker run --rm -v "$USTR_TMP_DIR/contracts":/code \
        --mount type=volume,source=ustr_cmm_cache,target=/code/target \
        --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
        cosmwasm/workspace-optimizer:0.16.1
    cp "$USTR_TMP_DIR/contracts/artifacts/treasury.wasm" "$ARTIFACTS_DIR/"
    cp "$USTR_TMP_DIR/contracts/artifacts/wrap_mapper.wasm" "$ARTIFACTS_DIR/"
    rm -rf "$USTR_TMP_DIR"
    echo "  treasury.wasm and wrap_mapper.wasm built and copied to artifacts."
    docker cp "$ARTIFACTS_DIR/treasury.wasm" "$CONTAINER_NAME:/tmp/artifacts/"
    docker cp "$ARTIFACTS_DIR/wrap_mapper.wasm" "$CONTAINER_NAME:/tmp/artifacts/"
fi
TX_HASH=$(terrad_tx wasm store /tmp/artifacts/treasury.wasm | jq -r '.txhash')
echo "  TX: $TX_HASH"
TREASURY_CODE_ID=$(get_code_id "$TX_HASH")
echo "  Treasury Code ID: $TREASURY_CODE_ID"

echo ""
echo "[9b.2] Uploading wrap_mapper.wasm..."
TX_HASH=$(terrad_tx wasm store /tmp/artifacts/wrap_mapper.wasm | jq -r '.txhash')
echo "  TX: $TX_HASH"
WRAP_MAPPER_CODE_ID=$(get_code_id "$TX_HASH")
echo "  Wrap-Mapper Code ID: $WRAP_MAPPER_CODE_ID"

echo ""
echo "[9b.3] Instantiating Treasury..."
TREASURY_INIT_MSG="{\"governance\":\"$TEST_ADDRESS\"}"
TX_HASH=$(terrad_tx wasm instantiate "$TREASURY_CODE_ID" "$TREASURY_INIT_MSG" \
    --label "ustr-treasury" \
    --admin "$TEST_ADDRESS" | jq -r '.txhash')
echo "  TX: $TX_HASH"
TREASURY_ADDRESS=$(get_contract_address "$TX_HASH")
echo "  Treasury Address: $TREASURY_ADDRESS"

echo ""
echo "[9b.4] Instantiating Wrap-Mapper..."
WRAP_MAPPER_INIT_MSG="{\"governance\":\"$TEST_ADDRESS\",\"treasury\":\"$TREASURY_ADDRESS\",\"fee_bps\":50}"
TX_HASH=$(terrad_tx wasm instantiate "$WRAP_MAPPER_CODE_ID" "$WRAP_MAPPER_INIT_MSG" \
    --label "ustr-wrap-mapper" \
    --admin "$TEST_ADDRESS" | jq -r '.txhash')
echo "  TX: $TX_HASH"
WRAP_MAPPER_ADDRESS=$(get_contract_address "$TX_HASH")
echo "  Wrap-Mapper Address: $WRAP_MAPPER_ADDRESS"

echo ""
echo "[9b.5] Creating LUNC-C (Wrapped Luna Classic) CW20 token..."
LUNC_C_INIT_MSG="{\"name\":\"Wrapped Luna Classic\",\"symbol\":\"LUNC-C\",\"decimals\":6,\"initial_balances\":[],\"mint\":{\"minter\":\"$WRAP_MAPPER_ADDRESS\"}}"
TX_HASH=$(terrad_tx wasm instantiate "$CW20_CODE_ID" "$LUNC_C_INIT_MSG" \
    --label "lunc-c-token" \
    --admin "$TEST_ADDRESS" | jq -r '.txhash')
echo "  TX: $TX_HASH"
LUNC_C_ADDRESS=$(get_contract_address "$TX_HASH")
echo "  LUNC-C Address: $LUNC_C_ADDRESS"

echo ""
echo "[9b.6] Creating USTC-C (Wrapped TerraClassicUSD) CW20 token..."
USTC_C_INIT_MSG="{\"name\":\"Wrapped TerraClassicUSD\",\"symbol\":\"USTC-C\",\"decimals\":6,\"initial_balances\":[],\"mint\":{\"minter\":\"$WRAP_MAPPER_ADDRESS\"}}"
TX_HASH=$(terrad_tx wasm instantiate "$CW20_CODE_ID" "$USTC_C_INIT_MSG" \
    --label "ustc-c-token" \
    --admin "$TEST_ADDRESS" | jq -r '.txhash')
echo "  TX: $TX_HASH"
USTC_C_ADDRESS=$(get_contract_address "$TX_HASH")
echo "  USTC-C Address: $USTC_C_ADDRESS"

echo ""
echo "[9b.7] Registering denom mappings on Wrap-Mapper..."
TX_HASH=$(terrad_tx wasm execute "$WRAP_MAPPER_ADDRESS" \
  "{\"set_denom_mapping\":{\"denom\":\"uluna\",\"cw20_addr\":\"$LUNC_C_ADDRESS\"}}" | jq -r '.txhash')
echo "  uluna -> LUNC-C: $TX_HASH"
sleep 3
TX_HASH=$(terrad_tx wasm execute "$WRAP_MAPPER_ADDRESS" \
  "{\"set_denom_mapping\":{\"denom\":\"uusd\",\"cw20_addr\":\"$USTC_C_ADDRESS\"}}" | jq -r '.txhash')
echo "  uusd -> USTC-C: $TX_HASH"
sleep 3

echo ""
echo "[9b.8] Registering wrappers on Treasury..."
TX_HASH=$(terrad_tx wasm execute "$TREASURY_ADDRESS" \
  "{\"set_denom_wrapper\":{\"denom\":\"uluna\",\"wrapper\":\"$WRAP_MAPPER_ADDRESS\"}}" | jq -r '.txhash')
echo "  uluna wrapper: $TX_HASH"
sleep 3
TX_HASH=$(terrad_tx wasm execute "$TREASURY_ADDRESS" \
  "{\"set_denom_wrapper\":{\"denom\":\"uusd\",\"wrapper\":\"$WRAP_MAPPER_ADDRESS\"}}" | jq -r '.txhash')
echo "  uusd wrapper: $TX_HASH"
sleep 3

echo ""
echo "[9b.9] Setting Wrap-Mapper on Router..."
TX_HASH=$(terrad_tx wasm execute "$ROUTER_ADDRESS" \
  "{\"set_wrap_mapper\":{\"wrap_mapper\":\"$WRAP_MAPPER_ADDRESS\"}}" | jq -r '.txhash')
echo "  Set wrap-mapper: $TX_HASH"
sleep 3

echo ""
echo "[9b.10] Funding Treasury with 40M USTC and 10M LUNC..."
TX_HASH=$(terrad_tx bank send test1 "$TREASURY_ADDRESS" \
  "40000000000000uusd,10000000000000uluna" | jq -r '.txhash')
echo "  Fund treasury: $TX_HASH"
sleep 3
echo "  Treasury funded: 40,000,000 USTC + 10,000,000 LUNC"

# ── Phase 2: Tokens ─────────────────────────────────────────────────────

echo ""
echo "[Phase 2] Creating ${#TOKEN_NAMES[@]} Test Tokens"
echo "----------------------------------------------"

for i in "${!TOKEN_NAMES[@]}"; do
    NAME="${TOKEN_NAMES[$i]}"
    SYM="${TOKEN_SYMBOLS[$i]}"
    echo ""
    echo "[10.$((i+1))] Instantiating $NAME ($SYM)..."
    INIT_MSG="{\"name\":\"$NAME\",\"symbol\":\"$SYM\",\"decimals\":6,\"initial_balances\":[{\"address\":\"$TEST_ADDRESS\",\"amount\":\"1000000000000\"}],\"mint\":{\"minter\":\"$TEST_ADDRESS\"}}"
    TX_HASH=$(terrad_tx wasm instantiate "$CW20_CODE_ID" "$INIT_MSG" \
        --label "test-token-${SYM,,}" \
        --admin "$TEST_ADDRESS" | jq -r '.txhash')
    echo "  TX: $TX_HASH"
    ADDR=$(get_contract_address "$TX_HASH")
    TOKEN_ADDRESSES+=("$ADDR")
    echo "  $SYM Address: $ADDR"
done

echo ""
echo "  All ${#TOKEN_NAMES[@]} tokens created."

# ── Phase 2b: Non-Whitelisted Tokens ────────────────────────────────────

echo ""
echo "[Phase 2b] Creating ${#NOWHITELIST_NAMES[@]} Non-Whitelisted Tokens (code_id=$CW20_CODE_ID_NOWHITELIST)"
echo "----------------------------------------------"

for i in "${!NOWHITELIST_NAMES[@]}"; do
    NAME="${NOWHITELIST_NAMES[$i]}"
    SYM="${NOWHITELIST_SYMBOLS[$i]}"
    echo ""
    echo "[10b.$((i+1))] Instantiating $NAME ($SYM) — NOT whitelisted..."
    INIT_MSG="{\"name\":\"$NAME\",\"symbol\":\"$SYM\",\"decimals\":6,\"initial_balances\":[{\"address\":\"$TEST_ADDRESS\",\"amount\":\"1000000000000\"}],\"mint\":{\"minter\":\"$TEST_ADDRESS\"}}"
    TX_HASH=$(terrad_tx wasm instantiate "$CW20_CODE_ID_NOWHITELIST" "$INIT_MSG" \
        --label "test-token-${SYM,,}" \
        --admin "$TEST_ADDRESS" | jq -r '.txhash')
    echo "  TX: $TX_HASH"
    ADDR=$(get_contract_address "$TX_HASH")
    NOWHITELIST_ADDRESSES+=("$ADDR")
    echo "  $SYM Address: $ADDR (code_id=$CW20_CODE_ID_NOWHITELIST, NOT whitelisted)"
done

echo ""
echo "  All ${#NOWHITELIST_NAMES[@]} non-whitelisted tokens created."

# ── Phase 2c: Unpaired Tokens ───────────────────────────────────────────

echo ""
echo "[Phase 2c] Creating ${#UNPAIRED_NAMES[@]} Unpaired/Minimally-Paired Tokens"
echo "----------------------------------------------"

for i in "${!UNPAIRED_NAMES[@]}"; do
    NAME="${UNPAIRED_NAMES[$i]}"
    SYM="${UNPAIRED_SYMBOLS[$i]}"
    echo ""
    echo "[10c.$((i+1))] Instantiating $NAME ($SYM)..."
    INIT_MSG="{\"name\":\"$NAME\",\"symbol\":\"$SYM\",\"decimals\":6,\"initial_balances\":[{\"address\":\"$TEST_ADDRESS\",\"amount\":\"1000000000000\"}],\"mint\":{\"minter\":\"$TEST_ADDRESS\"}}"
    TX_HASH=$(terrad_tx wasm instantiate "$CW20_CODE_ID" "$INIT_MSG" \
        --label "test-token-${SYM,,}" \
        --admin "$TEST_ADDRESS" | jq -r '.txhash')
    echo "  TX: $TX_HASH"
    ADDR=$(get_contract_address "$TX_HASH")
    UNPAIRED_ADDRESSES+=("$ADDR")
    echo "  $SYM Address: $ADDR"
done

echo ""
echo "  All ${#UNPAIRED_NAMES[@]} unpaired tokens created."
echo "  ZINC: 0 pairs | IRON: will get 1 pair | NEON: will get 2 pairs"

# ── Phase 3: Fee Discount ───────────────────────────────────────────────

echo ""
echo "[Phase 3] Fee Discount Setup"
echo "----------------------------------------------"

echo ""
echo "[11] Instantiating Fee Discount contract..."
FEE_DISCOUNT_INIT_MSG="{\"governance\":\"$TEST_ADDRESS\",\"cl8y_token\":\"${TOKEN_ADDRESSES[0]}\"}"
TX_HASH=$(terrad_tx wasm instantiate "$FEE_DISCOUNT_CODE_ID" "$FEE_DISCOUNT_INIT_MSG" \
    --label "cl8y-dex-fee-discount" \
    --admin "$TEST_ADDRESS" | jq -r '.txhash')
echo "  TX: $TX_HASH"
FEE_DISCOUNT_ADDRESS=$(get_contract_address "$TX_HASH")
echo "  Fee Discount Address: $FEE_DISCOUNT_ADDRESS"

echo ""
echo "[12] Adding fee discount tiers..."
for TIER_DATA in \
  '{"add_tier":{"tier_id":0,"min_cl8y_balance":"0","discount_bps":10000,"governance_only":true}}' \
  '{"add_tier":{"tier_id":1,"min_cl8y_balance":"1000000000000000000","discount_bps":250,"governance_only":false}}' \
  '{"add_tier":{"tier_id":2,"min_cl8y_balance":"5000000000000000000","discount_bps":1000,"governance_only":false}}' \
  '{"add_tier":{"tier_id":3,"min_cl8y_balance":"20000000000000000000","discount_bps":2000,"governance_only":false}}' \
  '{"add_tier":{"tier_id":4,"min_cl8y_balance":"75000000000000000000","discount_bps":3500,"governance_only":false}}' \
  '{"add_tier":{"tier_id":5,"min_cl8y_balance":"200000000000000000000","discount_bps":5000,"governance_only":false}}' \
  '{"add_tier":{"tier_id":6,"min_cl8y_balance":"500000000000000000000","discount_bps":6000,"governance_only":false}}' \
  '{"add_tier":{"tier_id":7,"min_cl8y_balance":"1500000000000000000000","discount_bps":7500,"governance_only":false}}' \
  '{"add_tier":{"tier_id":8,"min_cl8y_balance":"3500000000000000000000","discount_bps":8500,"governance_only":false}}' \
  '{"add_tier":{"tier_id":9,"min_cl8y_balance":"7500000000000000000000","discount_bps":9500,"governance_only":false}}' \
  '{"add_tier":{"tier_id":255,"min_cl8y_balance":"0","discount_bps":0,"governance_only":true}}'
do
  TX_HASH=$(terrad_tx wasm execute "$FEE_DISCOUNT_ADDRESS" "$TIER_DATA" | jq -r '.txhash')
  echo "  Added tier: $TX_HASH"
  sleep 2
done
echo "  All tiers added."

echo ""
echo "[13] Adding trusted router..."
TX_HASH=$(terrad_tx wasm execute "$FEE_DISCOUNT_ADDRESS" \
  "{\"add_trusted_router\":{\"router\":\"$ROUTER_ADDRESS\"}}" | jq -r '.txhash')
echo "  Added trusted router: $TX_HASH"
sleep 3

# ── Phase 4: Pairs, Liquidity & Discount Registries ─────────────────────

echo ""
echo "[Phase 4] Creating ${#PAIR_CONFIGS[@]} Pairs with Liquidity"
echo "----------------------------------------------"

for p in "${!PAIR_CONFIGS[@]}"; do
    IFS=':' read -r A_IDX B_IDX LIQ_A LIQ_B <<< "${PAIR_CONFIGS[$p]}"
    SYM_A="${TOKEN_SYMBOLS[$A_IDX]}"
    SYM_B="${TOKEN_SYMBOLS[$B_IDX]}"
    ADDR_A="${TOKEN_ADDRESSES[$A_IDX]}"
    ADDR_B="${TOKEN_ADDRESSES[$B_IDX]}"
    PAIR_NUM=$((p+1))

    echo ""
    echo "[14.$PAIR_NUM] Creating pair $SYM_A/$SYM_B..."

    # Create pair via factory
    CREATE_MSG="{\"create_pair\":{\"asset_infos\":[{\"token\":{\"contract_addr\":\"$ADDR_A\"}},{\"token\":{\"contract_addr\":\"$ADDR_B\"}}]}}"
    TX_HASH=$(terrad_tx wasm execute "$FACTORY_ADDRESS" "$CREATE_MSG" | jq -r '.txhash')
    echo "  TX: $TX_HASH"
    sleep 3
    PAIR_RESULT=$(terrad_query tx "$TX_HASH")
    PAIR_ADDR=$(echo "$PAIR_RESULT" | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value' | head -1)
    PAIR_ADDRESSES+=("$PAIR_ADDR")
    echo "  Pair Address: $PAIR_ADDR"

    # Set discount registry
    TX_HASH=$(terrad_tx wasm execute "$FACTORY_ADDRESS" \
      "{\"set_discount_registry\":{\"pair\":\"$PAIR_ADDR\",\"registry\":\"$FEE_DISCOUNT_ADDRESS\"}}" | jq -r '.txhash')
    echo "  Set discount registry: $TX_HASH"
    sleep 3

    # Approve tokens for pair
    TX_HASH=$(terrad_tx wasm execute "$ADDR_A" \
      "{\"increase_allowance\":{\"spender\":\"$PAIR_ADDR\",\"amount\":\"$LIQ_A\",\"expires\":{\"never\":{}}}}" | jq -r '.txhash')
    echo "  Approved $SYM_A: $TX_HASH"
    sleep 3
    TX_HASH=$(terrad_tx wasm execute "$ADDR_B" \
      "{\"increase_allowance\":{\"spender\":\"$PAIR_ADDR\",\"amount\":\"$LIQ_B\",\"expires\":{\"never\":{}}}}" | jq -r '.txhash')
    echo "  Approved $SYM_B: $TX_HASH"
    sleep 3

    # Provide liquidity
    PROVIDE_MSG="{\"provide_liquidity\":{\"assets\":[{\"info\":{\"token\":{\"contract_addr\":\"$ADDR_A\"}},\"amount\":\"$LIQ_A\"},{\"info\":{\"token\":{\"contract_addr\":\"$ADDR_B\"}},\"amount\":\"$LIQ_B\"}],\"slippage_tolerance\":null,\"receiver\":null,\"deadline\":null}}"
    TX_HASH=$(terrad_tx wasm execute "$PAIR_ADDR" "$PROVIDE_MSG" | jq -r '.txhash')
    echo "  Liquidity provided ($LIQ_A / $LIQ_B): $TX_HASH"
    sleep 3
done

echo ""
echo "  All ${#PAIR_CONFIGS[@]} pairs created with liquidity."

# ── Phase 4a: Liquidity Withdraw + Re-Provide Cycle ─────────────────────
# Generate at least one remove + re-add event for the first pair so the
# indexer's liquidity_events table is populated with both event types.

echo ""
echo "[Phase 4a] Liquidity withdraw/re-provide cycle (pair 1: ${TOKEN_SYMBOLS[0]}/${TOKEN_SYMBOLS[1]})"
echo "----------------------------------------------"

LP_PAIR_ADDR="${PAIR_ADDRESSES[0]}"

echo "  Querying pair info for LP token address..."
LP_TOKEN=$(terrad_query wasm contract-state smart "$LP_PAIR_ADDR" '{"pair":{}}' | jq -r '.data.liquidity_token')
echo "  LP Token: $LP_TOKEN"

echo "  Querying LP balance..."
LP_BALANCE=$(terrad_query wasm contract-state smart "$LP_TOKEN" \
  "{\"balance\":{\"address\":\"$TEST_ADDRESS\"}}" | jq -r '.data.balance')
echo "  LP Balance: $LP_BALANCE"

WITHDRAW_AMOUNT=$((LP_BALANCE / 10))
echo "  Withdrawing 10% of LP ($WITHDRAW_AMOUNT)..."
WITHDRAW_HOOK=$(echo -n '{"withdraw_liquidity":{}}' | base64 -w0)
TX_HASH=$(terrad_tx wasm execute "$LP_TOKEN" \
  "{\"send\":{\"contract\":\"$LP_PAIR_ADDR\",\"amount\":\"$WITHDRAW_AMOUNT\",\"msg\":\"$WITHDRAW_HOOK\"}}" | jq -r '.txhash')
echo "  Withdraw TX: $TX_HASH"
sleep 3

READD_A=5000000000
READD_B=5000000000
READD_ADDR_A="${TOKEN_ADDRESSES[0]}"
READD_ADDR_B="${TOKEN_ADDRESSES[1]}"

echo "  Re-approving tokens for re-provide..."
TX_HASH=$(terrad_tx wasm execute "$READD_ADDR_A" \
  "{\"increase_allowance\":{\"spender\":\"$LP_PAIR_ADDR\",\"amount\":\"$READD_A\",\"expires\":{\"never\":{}}}}" | jq -r '.txhash')
echo "  Approved ${TOKEN_SYMBOLS[0]}: $TX_HASH"
sleep 3
TX_HASH=$(terrad_tx wasm execute "$READD_ADDR_B" \
  "{\"increase_allowance\":{\"spender\":\"$LP_PAIR_ADDR\",\"amount\":\"$READD_B\",\"expires\":{\"never\":{}}}}" | jq -r '.txhash')
echo "  Approved ${TOKEN_SYMBOLS[1]}: $TX_HASH"
sleep 3

echo "  Re-providing liquidity ($READD_A / $READD_B)..."
READD_MSG="{\"provide_liquidity\":{\"assets\":[{\"info\":{\"token\":{\"contract_addr\":\"$READD_ADDR_A\"}},\"amount\":\"$READD_A\"},{\"info\":{\"token\":{\"contract_addr\":\"$READD_ADDR_B\"}},\"amount\":\"$READD_B\"}],\"slippage_tolerance\":null,\"receiver\":null,\"deadline\":null}}"
TX_HASH=$(terrad_tx wasm execute "$LP_PAIR_ADDR" "$READD_MSG" | jq -r '.txhash')
echo "  Re-provide TX: $TX_HASH"
sleep 3

echo "  Liquidity cycle complete (1 withdraw + 1 re-provide)."

# ── Phase 4b: Unpaired Token Pairs ──────────────────────────────────────
# IRON gets 1 pair, NEON gets 2 pairs, ZINC stays at 0 pairs

echo ""
echo "[Phase 4b] Creating Pairs for Unpaired Tokens"
echo "----------------------------------------------"

UNPAIRED_PAIR_CONFIGS=(
  "1:0:50000000000:100000000000"     # IRON/EMBER  1:2
  "2:1:100000000000:100000000000"    # NEON/CORAL   1:1
  "2:3:20000000000:100000000000"     # NEON/ONYX    1:5
)

UNPAIRED_PAIR_NUM=0
for upc in "${UNPAIRED_PAIR_CONFIGS[@]}"; do
    IFS=':' read -r UNPAIRED_IDX MAIN_IDX LIQ_A LIQ_B <<< "$upc"
    SYM_A="${UNPAIRED_SYMBOLS[$UNPAIRED_IDX]}"
    SYM_B="${TOKEN_SYMBOLS[$MAIN_IDX]}"
    ADDR_A="${UNPAIRED_ADDRESSES[$UNPAIRED_IDX]}"
    ADDR_B="${TOKEN_ADDRESSES[$MAIN_IDX]}"
    UNPAIRED_PAIR_NUM=$((UNPAIRED_PAIR_NUM+1))

    echo ""
    echo "[14b.$UNPAIRED_PAIR_NUM] Creating pair $SYM_A/$SYM_B..."

    CREATE_MSG="{\"create_pair\":{\"asset_infos\":[{\"token\":{\"contract_addr\":\"$ADDR_A\"}},{\"token\":{\"contract_addr\":\"$ADDR_B\"}}]}}"
    TX_HASH=$(terrad_tx wasm execute "$FACTORY_ADDRESS" "$CREATE_MSG" | jq -r '.txhash')
    echo "  TX: $TX_HASH"
    sleep 3
    PAIR_RESULT=$(terrad_query tx "$TX_HASH")
    PAIR_ADDR=$(echo "$PAIR_RESULT" | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value' | head -1)
    echo "  Pair Address: $PAIR_ADDR"

    TX_HASH=$(terrad_tx wasm execute "$FACTORY_ADDRESS" \
      "{\"set_discount_registry\":{\"pair\":\"$PAIR_ADDR\",\"registry\":\"$FEE_DISCOUNT_ADDRESS\"}}" | jq -r '.txhash')
    echo "  Set discount registry: $TX_HASH"
    sleep 3

    TX_HASH=$(terrad_tx wasm execute "$ADDR_A" \
      "{\"increase_allowance\":{\"spender\":\"$PAIR_ADDR\",\"amount\":\"$LIQ_A\",\"expires\":{\"never\":{}}}}" | jq -r '.txhash')
    echo "  Approved $SYM_A: $TX_HASH"
    sleep 3
    TX_HASH=$(terrad_tx wasm execute "$ADDR_B" \
      "{\"increase_allowance\":{\"spender\":\"$PAIR_ADDR\",\"amount\":\"$LIQ_B\",\"expires\":{\"never\":{}}}}" | jq -r '.txhash')
    echo "  Approved $SYM_B: $TX_HASH"
    sleep 3

    PROVIDE_MSG="{\"provide_liquidity\":{\"assets\":[{\"info\":{\"token\":{\"contract_addr\":\"$ADDR_A\"}},\"amount\":\"$LIQ_A\"},{\"info\":{\"token\":{\"contract_addr\":\"$ADDR_B\"}},\"amount\":\"$LIQ_B\"}],\"slippage_tolerance\":null,\"receiver\":null,\"deadline\":null}}"
    TX_HASH=$(terrad_tx wasm execute "$PAIR_ADDR" "$PROVIDE_MSG" | jq -r '.txhash')
    echo "  Liquidity provided ($LIQ_A / $LIQ_B): $TX_HASH"
    sleep 3
done

echo ""
echo "  $UNPAIRED_PAIR_NUM unpaired-token pairs created (ZINC: 0, IRON: 1, NEON: 2)."

# ── Phase 5: Test Swaps ─────────────────────────────────────────────────

echo ""
echo "[Phase 5] Executing Test Swaps"
echo "----------------------------------------------"

SWAP_HOOK=$(echo -n '{"swap":{"belief_price":null,"max_spread":"0.50","to":null,"deadline":null,"trader":null}}' | base64 -w0)

SWAP_COUNT=0
for p in "${!PAIR_CONFIGS[@]}"; do
    IFS=':' read -r A_IDX B_IDX LIQ_A LIQ_B <<< "${PAIR_CONFIGS[$p]}"
    SYM_A="${TOKEN_SYMBOLS[$A_IDX]}"
    SYM_B="${TOKEN_SYMBOLS[$B_IDX]}"
    ADDR_A="${TOKEN_ADDRESSES[$A_IDX]}"
    ADDR_B="${TOKEN_ADDRESSES[$B_IDX]}"
    PAIR_ADDR="${PAIR_ADDRESSES[$p]}"
    PAIR_NUM=$((p+1))

    # Swap amounts: fractions of the liquidity to create price movement
    SWAP_A1=$((LIQ_A / 200))   # 0.5% of A
    SWAP_B1=$((LIQ_B / 333))   # 0.3% of B
    SWAP_A2=$((LIQ_A / 125))   # 0.8% of A

    # Skip if amounts are too small
    if [ "$SWAP_A1" -lt 1000 ]; then SWAP_A1=1000000; fi
    if [ "$SWAP_B1" -lt 1000 ]; then SWAP_B1=1000000; fi
    if [ "$SWAP_A2" -lt 1000 ]; then SWAP_A2=1000000; fi

    echo ""
    echo "[15.$PAIR_NUM] Swaps on $SYM_A/$SYM_B..."

    echo "  Swap $((SWAP_COUNT+1)): $SWAP_A1 $SYM_A -> $SYM_B"
    TX_HASH=$(terrad_tx wasm execute "$ADDR_A" \
      "{\"send\":{\"contract\":\"$PAIR_ADDR\",\"amount\":\"$SWAP_A1\",\"msg\":\"$SWAP_HOOK\"}}" | jq -r '.txhash')
    echo "    TX: $TX_HASH"
    SWAP_COUNT=$((SWAP_COUNT+1))
    sleep 3

    echo "  Swap $((SWAP_COUNT+1)): $SWAP_B1 $SYM_B -> $SYM_A"
    TX_HASH=$(terrad_tx wasm execute "$ADDR_B" \
      "{\"send\":{\"contract\":\"$PAIR_ADDR\",\"amount\":\"$SWAP_B1\",\"msg\":\"$SWAP_HOOK\"}}" | jq -r '.txhash')
    echo "    TX: $TX_HASH"
    SWAP_COUNT=$((SWAP_COUNT+1))
    sleep 3

    echo "  Swap $((SWAP_COUNT+1)): $SWAP_A2 $SYM_A -> $SYM_B"
    TX_HASH=$(terrad_tx wasm execute "$ADDR_A" \
      "{\"send\":{\"contract\":\"$PAIR_ADDR\",\"amount\":\"$SWAP_A2\",\"msg\":\"$SWAP_HOOK\"}}" | jq -r '.txhash')
    echo "    TX: $TX_HASH"
    SWAP_COUNT=$((SWAP_COUNT+1))
    sleep 3
done

echo ""
echo "  $SWAP_COUNT total swaps executed across ${#PAIR_CONFIGS[@]} pairs."

# ── Phase 6: Summary ────────────────────────────────────────────────────

echo ""
echo "=============================================="
echo "  Deployment Complete!"
echo "=============================================="
echo ""
echo "  Factory:       $FACTORY_ADDRESS"
echo "  Router:        $ROUTER_ADDRESS"
echo "  Fee Discount:  $FEE_DISCOUNT_ADDRESS"
echo "  Treasury:      $TREASURY_ADDRESS"
echo "  Wrap-Mapper:   $WRAP_MAPPER_ADDRESS"
echo "  LUNC-C:        $LUNC_C_ADDRESS"
echo "  USTC-C:        $USTC_C_ADDRESS"
echo ""
echo "  Tokens (whitelisted, code_id=$CW20_CODE_ID):"
for i in "${!TOKEN_SYMBOLS[@]}"; do
    printf "    %-8s %s\n" "${TOKEN_SYMBOLS[$i]}" "${TOKEN_ADDRESSES[$i]}"
done
echo ""
echo "  Non-Whitelisted Tokens (code_id=$CW20_CODE_ID_NOWHITELIST):"
for i in "${!NOWHITELIST_SYMBOLS[@]}"; do
    printf "    %-8s %s\n" "${NOWHITELIST_SYMBOLS[$i]}" "${NOWHITELIST_ADDRESSES[$i]}"
done
echo ""
echo "  Unpaired/Minimally-Paired Tokens (whitelisted, code_id=$CW20_CODE_ID):"
printf "    %-8s %s  (0 pairs)\n" "${UNPAIRED_SYMBOLS[0]}" "${UNPAIRED_ADDRESSES[0]}"
printf "    %-8s %s  (1 pair)\n" "${UNPAIRED_SYMBOLS[1]}" "${UNPAIRED_ADDRESSES[1]}"
printf "    %-8s %s  (2 pairs)\n" "${UNPAIRED_SYMBOLS[2]}" "${UNPAIRED_ADDRESSES[2]}"
echo ""
echo "  Pairs:"
for p in "${!PAIR_CONFIGS[@]}"; do
    IFS=':' read -r A_IDX B_IDX _ _ <<< "${PAIR_CONFIGS[$p]}"
    printf "    %-14s %s\n" "${TOKEN_SYMBOLS[$A_IDX]}/${TOKEN_SYMBOLS[$B_IDX]}" "${PAIR_ADDRESSES[$p]}"
done
echo ""
echo "=============================================="
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo ""
echo "[Phase 6.1] Writing frontend-dapp/.env.local..."
cat > "$REPO_ROOT/frontend-dapp/.env.local" <<ENVEOF
VITE_NETWORK=local
VITE_FACTORY_ADDRESS=$FACTORY_ADDRESS
VITE_ROUTER_ADDRESS=$ROUTER_ADDRESS
VITE_FEE_DISCOUNT_ADDRESS=$FEE_DISCOUNT_ADDRESS
VITE_CL8Y_TOKEN_ADDRESS=${TOKEN_ADDRESSES[0]}
VITE_TERRA_LCD_URL=$LCD
VITE_TERRA_RPC_URL=$NODE
VITE_DEV_MODE=true
VITE_TREASURY_ADDRESS=$TREASURY_ADDRESS
VITE_WRAP_MAPPER_ADDRESS=$WRAP_MAPPER_ADDRESS
VITE_LUNC_C_TOKEN_ADDRESS=$LUNC_C_ADDRESS
VITE_USTC_C_TOKEN_ADDRESS=$USTC_C_ADDRESS
VITE_NOWHITELIST_TOKEN_1=${NOWHITELIST_ADDRESSES[0]}
VITE_NOWHITELIST_TOKEN_2=${NOWHITELIST_ADDRESSES[1]}
VITE_UNPAIRED_TOKEN_ZINC=${UNPAIRED_ADDRESSES[0]}
VITE_UNPAIRED_TOKEN_IRON=${UNPAIRED_ADDRESSES[1]}
VITE_UNPAIRED_TOKEN_NEON=${UNPAIRED_ADDRESSES[2]}
ENVEOF
echo "  Written to frontend-dapp/.env.local"

echo ""
echo "[Phase 6.2] Writing indexer/.env..."
cat > "$REPO_ROOT/indexer/.env" <<ENVEOF
DATABASE_URL=postgres://postgres:postgres@localhost:5432/dex_indexer
FACTORY_ADDRESS=$FACTORY_ADDRESS
ROUTER_ADDRESS=$ROUTER_ADDRESS
FEE_DISCOUNT_ADDRESS=$FEE_DISCOUNT_ADDRESS
LCD_URLS=http://localhost:1317
CORS_ORIGINS=http://localhost:3000,http://localhost:5173,http://localhost:4173,http://127.0.0.1:3000,http://127.0.0.1:5173,http://127.0.0.1:4173
API_PORT=3001
API_BIND=127.0.0.1
POLL_INTERVAL_MS=2000
RATE_LIMIT_RPS=100
ENVEOF
echo "  Written to indexer/.env"

echo ""
echo "Test address: $TEST_ADDRESS"
echo "  10 tokens, 3 unpaired tokens, 2 non-whitelisted tokens, 23 pairs, $SWAP_COUNT swaps executed"
