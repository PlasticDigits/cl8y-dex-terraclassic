#!/usr/bin/env bash
set -e

# CW20 labels: avoid ${SYM,,} — needs bash 4+, fails under /bin/sh, and ./script with #!/bin/bash uses
# system /bin/bash (often 3.2 on macOS), not necessarily the newer `bash` from PATH (e.g. Homebrew 5.x).
to_lower() {
    printf '%s' "$1" | tr '[:upper:]' '[:lower:]'
}

CHAIN_ID="localterra"
# Host-facing URLs (curl from host, Vite, indexer): match docker published ports (remap when QA_SHARED_HOST).
_HOST_RPC_PORT="${DEX_TERRA_RPC_PORT:-26657}"
_HOST_LCD_PORT="${DEX_TERRA_LCD_PORT:-1317}"
NODE="${TERRA_RPC_URL:-http://localhost:${_HOST_RPC_PORT}}"
LCD="${TERRA_LCD_URL:-http://localhost:${_HOST_LCD_PORT}}"
# terrad runs inside the LocalTerra container — always use the in-container Tendermint RPC port.
TERRAD_NODE="${TERRAD_NODE:-http://127.0.0.1:26657}"
TEST_ADDRESS="terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v"
CONTAINER_NAME=$(docker compose ps -q localterra 2>/dev/null | head -1)
if [ -z "$CONTAINER_NAME" ]; then
    echo "ERROR: localterra container not found. Run 'make start' first."
    exit 1
fi
ARTIFACTS_DIR="$(cd "$(dirname "$0")/../smartcontracts/artifacts" && pwd)"
CONTRACTS_DIR="$(cd "$(dirname "$0")/../smartcontracts/contracts" && pwd)"
# shellcheck source=scripts/lib/terrad-tx-events.sh
source "$(cd "$(dirname "$0")" && pwd)/lib/terrad-tx-events.sh"
# shellcheck source=scripts/lib/terrad-wait-tx.sh
source "$(cd "$(dirname "$0")" && pwd)/lib/terrad-wait-tx.sh"
# shellcheck source=scripts/lib/qa-phase-timing.sh
source "$(cd "$(dirname "$0")" && pwd)/lib/qa-phase-timing.sh"

QA_DEPLOY_SEED="${QA_DEPLOY_SEED:-full}"
case "$QA_DEPLOY_SEED" in
  full | minimal | charts | wallet) ;;
  *)
    echo "ERROR: unknown QA_DEPLOY_SEED=$QA_DEPLOY_SEED (full|minimal|charts|wallet)" >&2
    exit 1
    ;;
esac

# ── Staleness check ────────────────────────────────────────────────────
# Fail fast if any WASM artifact is older than its source, so QA doesn't
# chase phantom contract errors from a stale build.
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=scripts/lib/wasm-artifacts-stale.sh
source "$(cd "$(dirname "$0")" && pwd)/lib/wasm-artifacts-stale.sh"
if ! dex_wasm_stale_vs_sources "$REPO_ROOT"; then
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

# Seed profiles (GitLab #325): trim tokens/pairs/phases for faster QA when full history is unnecessary.
case "$QA_DEPLOY_SEED" in
  minimal | charts)
    TOKEN_NAMES=("Ember" "Coral")
    TOKEN_SYMBOLS=("EMBER" "CORAL")
    NOWHITELIST_NAMES=()
    NOWHITELIST_SYMBOLS=()
    UNPAIRED_NAMES=()
    UNPAIRED_SYMBOLS=()
    PAIR_CONFIGS=("0:1:100000000000:100000000000")
  ;;
  wallet)
    TOKEN_NAMES=("Ember" "Coral" "Jade")
    TOKEN_SYMBOLS=("EMBER" "CORAL" "JADE")
    NOWHITELIST_NAMES=()
    NOWHITELIST_SYMBOLS=()
    UNPAIRED_NAMES=()
    UNPAIRED_SYMBOLS=()
    PAIR_CONFIGS=("0:1:100000000000:100000000000")
  ;;
esac

terrad_tx() {
    docker exec "$CONTAINER_NAME" terrad tx "$@" \
        --from test1 \
        --keyring-backend test \
        --chain-id "$CHAIN_ID" \
        --gas auto \
        --gas-adjustment 1.3 \
        --gas-prices "${DEPLOY_GAS_PRICES:-28.325uluna}" \
        --node "$TERRAD_NODE" \
        --broadcast-mode sync \
        -y --output json
}

terrad_query() {
    docker exec "$CONTAINER_NAME" terrad query "$@" \
        --node "$TERRAD_NODE" \
        --output json
}

# Host-published :26657 can hang when many curl clients leave CLOSE-WAIT on docker-proxy;
# readiness checks always hit RPC inside the container (GitLab #206 verification).
localterra_rpc_ready() {
    docker exec "$CONTAINER_NAME" curl -m 3 -sf "http://127.0.0.1:26657/status" >/dev/null 2>&1
}

get_code_id() {
    local TX_HASH="$1"
    local RESULT
    RESULT=$(terrad_wait_tx_query "$CONTAINER_NAME" "$TX_HASH" "$TERRAD_NODE")
    echo "$RESULT" | terrad_jq_code_id_from_tx_json
}

get_contract_address() {
    local TX_HASH="$1"
    local RESULT
    RESULT=$(terrad_wait_tx_query "$CONTAINER_NAME" "$TX_HASH" "$TERRAD_NODE")
    echo "$RESULT" | terrad_jq_contract_address_from_tx_json
}

wait_tx() {
    terrad_wait_tx_inclusion "$CONTAINER_NAME" "$1" "$TERRAD_NODE"
}

echo "=============================================="
echo "CL8Y DEX - Local Deployment (seed=${QA_DEPLOY_SEED})"
echo "  ${#TOKEN_SYMBOLS[@]} Tokens, ${#UNPAIRED_SYMBOLS[@]} Unpaired, ${#NOWHITELIST_SYMBOLS[@]} Non-Whitelisted, ${#PAIR_CONFIGS[@]} Pairs"
echo "=============================================="

qa_timing_begin_session

# ── Phase 1: Infrastructure ─────────────────────────────────────────────

qa_timing_phase_start "infrastructure"
echo ""
echo "[Phase 1] Infrastructure Setup"
echo "----------------------------------------------"

echo ""
echo "[1] Waiting for LocalTerra to be ready..."
for i in $(seq 1 60); do
    if localterra_rpc_ready; then
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
    # workspace-optimizer writes root-owned files under CW20_TMP_DIR
    if [ -d "$CW20_TMP_DIR" ]; then
      docker run --rm -v "$CW20_TMP_DIR":/w alpine:3.20 rm -rf /w 2>/dev/null \
        || rm -rf "$CW20_TMP_DIR" 2>/dev/null \
        || true
    fi
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
# GitLab #276/#318: the factory charges a governance-settable pair-creation fee (uluna) forwarded to
# treasury. Deploy with the real fee so local matches mainnet economics and exercises the #276 fee
# path end to end. Treasury == test1 here, so the fee returns to the deployer (net cost is gas plus
# the chain's ~0.5% transfer tax on each fee send — a few LUNC across the whole deploy).
# Set LOCAL_PAIR_CREATION_FEE_ULUNA=0 for a fee-free local chain.
FACTORY_PAIR_CREATION_FEE="${LOCAL_PAIR_CREATION_FEE_ULUNA:-100000000}"
FACTORY_INIT_MSG="{\"governance\":\"$TEST_ADDRESS\",\"treasury\":\"$TEST_ADDRESS\",\"default_fee_bps\":180,\"pair_code_id\":$PAIR_CODE_ID,\"lp_token_code_id\":$CW20_CODE_ID,\"whitelisted_code_ids\":[$CW20_CODE_ID],\"pair_creation_fee_uluna\":\"$FACTORY_PAIR_CREATION_FEE\"}"
TX_HASH=$(terrad_tx wasm instantiate "$FACTORY_CODE_ID" "$FACTORY_INIT_MSG" \
    --label "cl8y-dex-factory" \
    --admin "$TEST_ADDRESS" | jq -r '.txhash')
echo "  TX: $TX_HASH"
FACTORY_ADDRESS=$(get_contract_address "$TX_HASH")
echo "  Factory Address: $FACTORY_ADDRESS"

# GitLab #318: read the authoritative pair-creation fee from the factory config so every create_pair
# attaches exactly what the contract requires — even if governance changes it on a long-lived chain.
PAIR_CREATION_FEE_ULUNA=$(terrad_query wasm contract-state smart "$FACTORY_ADDRESS" '{"config":{}}' \
  | jq -r '.data.pair_creation_fee_uluna // "0"')
echo "  Pair creation fee: ${PAIR_CREATION_FEE_ULUNA} uluna"

# Attach the on-chain pair-creation fee on every factory create_pair (GitLab #276/#318). FEE_ARGS stays
# empty when the fee is disabled (0), so a fee-free local chain still works.
factory_create_pair() {
    local CREATE_MSG="$1"
    local FEE_ARGS=()
    if [ -n "$PAIR_CREATION_FEE_ULUNA" ] && [ "$PAIR_CREATION_FEE_ULUNA" != "0" ]; then
        FEE_ARGS=(--amount "${PAIR_CREATION_FEE_ULUNA}uluna")
    fi
    terrad_tx wasm execute "$FACTORY_ADDRESS" "$CREATE_MSG" "${FEE_ARGS[@]}"
}

# Pre-flight: make sure test1 can cover the fee for every pair we create (Phase 4/4b/4c =
# PAIR_CONFIGS + 3 unpaired + 2 wrapped-native). The fee returns to treasury (== test1), so this is a
# generous headroom check; it fails fast with an actionable message instead of dying mid-Phase-4.
if [ "$PAIR_CREATION_FEE_ULUNA" != "0" ]; then
    _qa_extra_pairs=0
    case "$QA_DEPLOY_SEED" in
      full) _qa_extra_pairs=5 ;;
      wallet) _qa_extra_pairs=2 ;;
    esac
    PAIRS_TO_CREATE=$(( ${#PAIR_CONFIGS[@]} + _qa_extra_pairs ))
    FEE_NEEDED=$(( PAIR_CREATION_FEE_ULUNA * PAIRS_TO_CREATE + 5000000000 ))
    TEST1_ULUNA=$(terrad_query bank balances "$TEST_ADDRESS" \
      | jq -r '(.balances[]? | select(.denom=="uluna") | .amount) // empty' | head -1)
    if [ -n "$TEST1_ULUNA" ] && [ "$TEST1_ULUNA" -lt "$FEE_NEEDED" ] 2>/dev/null; then
        echo "  ERROR: test1 uluna ($TEST1_ULUNA) < pair-creation fee headroom ($FEE_NEEDED for" \
             "$PAIRS_TO_CREATE pairs at ${PAIR_CREATION_FEE_ULUNA}uluna). Re-fund test1 or set" \
             "LOCAL_PAIR_CREATION_FEE_ULUNA=0." >&2
        exit 1
    fi
    echo "  test1 uluna OK for $PAIRS_TO_CREATE pairs (${TEST1_ULUNA:-unknown} available)."
fi

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

TREASURY_ADDRESS=""
WRAP_MAPPER_ADDRESS=""
LUNC_C_ADDRESS=""
USTC_C_ADDRESS=""

if [ "$QA_DEPLOY_SEED" = "full" ] || [ "$QA_DEPLOY_SEED" = "wallet" ]; then
qa_timing_phase_start "treasury-wrap"
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
    if [ -d "$USTR_TMP_DIR" ]; then
      docker run --rm -v "$USTR_TMP_DIR":/w alpine:3.20 rm -rf /w 2>/dev/null \
        || rm -rf "$USTR_TMP_DIR" 2>/dev/null \
        || true
    fi
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
LUNC_C_INIT_MSG="{\"name\":\"Wrapped Luna Classic\",\"symbol\":\"LUNC-C\",\"decimals\":6,\"initial_balances\":[{\"address\":\"$TEST_ADDRESS\",\"amount\":\"1000000000000000\"}],\"mint\":{\"minter\":\"$WRAP_MAPPER_ADDRESS\"}}"
TX_HASH=$(terrad_tx wasm instantiate "$CW20_CODE_ID" "$LUNC_C_INIT_MSG" \
    --label "lunc-c-token" \
    --admin "$TEST_ADDRESS" | jq -r '.txhash')
echo "  TX: $TX_HASH"
LUNC_C_ADDRESS=$(get_contract_address "$TX_HASH")
echo "  LUNC-C Address: $LUNC_C_ADDRESS"

echo ""
echo "[9b.6] Creating USTC-C (Wrapped TerraClassicUSD) CW20 token..."
USTC_C_INIT_MSG="{\"name\":\"Wrapped TerraClassicUSD\",\"symbol\":\"USTC-C\",\"decimals\":6,\"initial_balances\":[{\"address\":\"$TEST_ADDRESS\",\"amount\":\"1000000000000000\"}],\"mint\":{\"minter\":\"$WRAP_MAPPER_ADDRESS\"}}"
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
wait_tx "$TX_HASH"
TX_HASH=$(terrad_tx wasm execute "$WRAP_MAPPER_ADDRESS" \
  "{\"set_denom_mapping\":{\"denom\":\"uusd\",\"cw20_addr\":\"$USTC_C_ADDRESS\"}}" | jq -r '.txhash')
echo "  uusd -> USTC-C: $TX_HASH"
wait_tx "$TX_HASH"

echo ""
echo "[9b.8] Registering wrappers on Treasury..."
TX_HASH=$(terrad_tx wasm execute "$TREASURY_ADDRESS" \
  "{\"set_denom_wrapper\":{\"denom\":\"uluna\",\"wrapper\":\"$WRAP_MAPPER_ADDRESS\"}}" | jq -r '.txhash')
echo "  uluna wrapper: $TX_HASH"
wait_tx "$TX_HASH"
TX_HASH=$(terrad_tx wasm execute "$TREASURY_ADDRESS" \
  "{\"set_denom_wrapper\":{\"denom\":\"uusd\",\"wrapper\":\"$WRAP_MAPPER_ADDRESS\"}}" | jq -r '.txhash')
echo "  uusd wrapper: $TX_HASH"
wait_tx "$TX_HASH"

echo ""
echo "[9b.9] Setting Wrap-Mapper on Router..."
TX_HASH=$(terrad_tx wasm execute "$ROUTER_ADDRESS" \
  "{\"set_wrap_mapper\":{\"wrap_mapper\":\"$WRAP_MAPPER_ADDRESS\"}}" | jq -r '.txhash')
echo "  Set wrap-mapper: $TX_HASH"
wait_tx "$TX_HASH"

echo ""
# SDK 0.53 LocalTerra genesis: 10M LUNC + 100M USTC on test1 (GitLab #292, #372). Keep headroom for deploy gas.
TREASURY_FUND_COINS="${DEPLOY_TREASURY_FUND_COINS:-20000000000000uusd,2000000000000uluna}"
echo "[9b.10] Funding Treasury ($TREASURY_FUND_COINS)..."
TX_HASH=$(terrad_tx bank send test1 "$TREASURY_ADDRESS" \
  "$TREASURY_FUND_COINS" | jq -r '.txhash')
echo "  Fund treasury: $TX_HASH"
wait_tx "$TX_HASH"
if terrad_query tx "$TX_HASH" | jq -e '.code == 0' >/dev/null 2>&1; then
  echo "  Treasury funded: $TREASURY_FUND_COINS"
else
  echo "  ERROR: Treasury fund tx failed (code != 0). Wrap E2E cannot proceed; aborting." >&2
  terrad_query tx "$TX_HASH" | jq -r '.raw_log // "no log"' >&2
  exit 1
fi
qa_timing_phase_end
else
  echo ""
  echo "[Phase 1b] Skipped (seed=${QA_DEPLOY_SEED} — no wrap/treasury E2E)"
fi

qa_timing_phase_start "tokens-pairs"

# ── Phase 2: Tokens ─────────────────────────────────────────────────────

echo ""
echo "[Phase 2] Creating ${#TOKEN_NAMES[@]} Test Tokens"
echo "----------------------------------------------"

for i in "${!TOKEN_NAMES[@]}"; do
    NAME="${TOKEN_NAMES[$i]}"
    SYM="${TOKEN_SYMBOLS[$i]}"
    echo ""
    echo "[10.$((i+1))] Instantiating $NAME ($SYM)..."
    INIT_MSG="{\"name\":\"$NAME\",\"symbol\":\"$SYM\",\"decimals\":6,\"initial_balances\":[{\"address\":\"$TEST_ADDRESS\",\"amount\":\"10000000000000\"}],\"mint\":{\"minter\":\"$TEST_ADDRESS\"}}"
    TX_HASH=$(terrad_tx wasm instantiate "$CW20_CODE_ID" "$INIT_MSG" \
        --label "test-token-$(to_lower "$SYM")" \
        --admin "$TEST_ADDRESS" | jq -r '.txhash')
    echo "  TX: $TX_HASH"
    ADDR=$(get_contract_address "$TX_HASH")
    TOKEN_ADDRESSES+=("$ADDR")
    echo "  $SYM Address: $ADDR"
done

echo ""
echo "  All ${#TOKEN_NAMES[@]} tokens created."

# ── Phase 2b: Non-Whitelisted Tokens ────────────────────────────────────

if [ "${#NOWHITELIST_NAMES[@]}" -gt 0 ]; then
echo ""
echo "[Phase 2b] Creating ${#NOWHITELIST_NAMES[@]} Non-Whitelisted Tokens (code_id=$CW20_CODE_ID_NOWHITELIST)"
echo "----------------------------------------------"

for i in "${!NOWHITELIST_NAMES[@]}"; do
    NAME="${NOWHITELIST_NAMES[$i]}"
    SYM="${NOWHITELIST_SYMBOLS[$i]}"
    echo ""
    echo "[10b.$((i+1))] Instantiating $NAME ($SYM) — NOT whitelisted..."
    INIT_MSG="{\"name\":\"$NAME\",\"symbol\":\"$SYM\",\"decimals\":6,\"initial_balances\":[{\"address\":\"$TEST_ADDRESS\",\"amount\":\"10000000000000\"}],\"mint\":{\"minter\":\"$TEST_ADDRESS\"}}"
    TX_HASH=$(terrad_tx wasm instantiate "$CW20_CODE_ID_NOWHITELIST" "$INIT_MSG" \
        --label "test-token-$(to_lower "$SYM")" \
        --admin "$TEST_ADDRESS" | jq -r '.txhash')
    echo "  TX: $TX_HASH"
    ADDR=$(get_contract_address "$TX_HASH")
    NOWHITELIST_ADDRESSES+=("$ADDR")
    echo "  $SYM Address: $ADDR (code_id=$CW20_CODE_ID_NOWHITELIST, NOT whitelisted)"
done

echo ""
echo "  All ${#NOWHITELIST_NAMES[@]} non-whitelisted tokens created."
fi

# ── Phase 2c: Unpaired Tokens ───────────────────────────────────────────

if [ "${#UNPAIRED_NAMES[@]}" -gt 0 ]; then
echo ""
echo "[Phase 2c] Creating ${#UNPAIRED_NAMES[@]} Unpaired/Minimally-Paired Tokens"
echo "----------------------------------------------"

for i in "${!UNPAIRED_NAMES[@]}"; do
    NAME="${UNPAIRED_NAMES[$i]}"
    SYM="${UNPAIRED_SYMBOLS[$i]}"
    echo ""
    echo "[10c.$((i+1))] Instantiating $NAME ($SYM)..."
    INIT_MSG="{\"name\":\"$NAME\",\"symbol\":\"$SYM\",\"decimals\":6,\"initial_balances\":[{\"address\":\"$TEST_ADDRESS\",\"amount\":\"10000000000000\"}],\"mint\":{\"minter\":\"$TEST_ADDRESS\"}}"
    TX_HASH=$(terrad_tx wasm instantiate "$CW20_CODE_ID" "$INIT_MSG" \
        --label "test-token-$(to_lower "$SYM")" \
        --admin "$TEST_ADDRESS" | jq -r '.txhash')
    echo "  TX: $TX_HASH"
    ADDR=$(get_contract_address "$TX_HASH")
    UNPAIRED_ADDRESSES+=("$ADDR")
    echo "  $SYM Address: $ADDR"
done

echo ""
echo "  All ${#UNPAIRED_NAMES[@]} unpaired tokens created."
echo "  ZINC: 0 pairs | IRON: will get 1 pair | NEON: will get 2 pairs"
fi

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
wait_tx "$TX_HASH"

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
    TX_HASH=$(factory_create_pair "$CREATE_MSG" | jq -r '.txhash')
    echo "  TX: $TX_HASH"
    wait_tx "$TX_HASH"
    PAIR_RESULT=$(terrad_query tx "$TX_HASH")
    PAIR_ADDR=$(echo "$PAIR_RESULT" | terrad_jq_contract_address_from_tx_json | head -1)
    PAIR_ADDRESSES+=("$PAIR_ADDR")
    echo "  Pair Address: $PAIR_ADDR"

    # Set discount registry
    TX_HASH=$(terrad_tx wasm execute "$FACTORY_ADDRESS" \
      "{\"set_discount_registry\":{\"pair\":\"$PAIR_ADDR\",\"registry\":\"$FEE_DISCOUNT_ADDRESS\"}}" | jq -r '.txhash')
    echo "  Set discount registry: $TX_HASH"
    wait_tx "$TX_HASH"

    # Approve tokens for pair
    TX_HASH=$(terrad_tx wasm execute "$ADDR_A" \
      "{\"increase_allowance\":{\"spender\":\"$PAIR_ADDR\",\"amount\":\"$LIQ_A\",\"expires\":{\"never\":{}}}}" | jq -r '.txhash')
    echo "  Approved $SYM_A: $TX_HASH"
    wait_tx "$TX_HASH"
    TX_HASH=$(terrad_tx wasm execute "$ADDR_B" \
      "{\"increase_allowance\":{\"spender\":\"$PAIR_ADDR\",\"amount\":\"$LIQ_B\",\"expires\":{\"never\":{}}}}" | jq -r '.txhash')
    echo "  Approved $SYM_B: $TX_HASH"
    wait_tx "$TX_HASH"

    # Provide liquidity
    PROVIDE_MSG="{\"provide_liquidity\":{\"assets\":[{\"info\":{\"token\":{\"contract_addr\":\"$ADDR_A\"}},\"amount\":\"$LIQ_A\"},{\"info\":{\"token\":{\"contract_addr\":\"$ADDR_B\"}},\"amount\":\"$LIQ_B\"}],\"slippage_tolerance\":null,\"receiver\":null,\"deadline\":null}}"
    TX_HASH=$(terrad_tx wasm execute "$PAIR_ADDR" "$PROVIDE_MSG" | jq -r '.txhash')
    echo "  Liquidity provided ($LIQ_A / $LIQ_B): $TX_HASH"
    wait_tx "$TX_HASH"
done

echo ""
echo "  All ${#PAIR_CONFIGS[@]} pairs created with liquidity."

# ── Phase 4a: Liquidity Withdraw + Re-Provide Cycle ─────────────────────
# Generate at least one remove + re-add event for the first pair so the
# indexer's liquidity_events table is populated with both event types.

if [ "$QA_DEPLOY_SEED" = "full" ]; then
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
wait_tx "$TX_HASH"

READD_A=5000000000
READD_B=5000000000
READD_ADDR_A="${TOKEN_ADDRESSES[0]}"
READD_ADDR_B="${TOKEN_ADDRESSES[1]}"

echo "  Re-approving tokens for re-provide..."
TX_HASH=$(terrad_tx wasm execute "$READD_ADDR_A" \
  "{\"increase_allowance\":{\"spender\":\"$LP_PAIR_ADDR\",\"amount\":\"$READD_A\",\"expires\":{\"never\":{}}}}" | jq -r '.txhash')
echo "  Approved ${TOKEN_SYMBOLS[0]}: $TX_HASH"
wait_tx "$TX_HASH"
TX_HASH=$(terrad_tx wasm execute "$READD_ADDR_B" \
  "{\"increase_allowance\":{\"spender\":\"$LP_PAIR_ADDR\",\"amount\":\"$READD_B\",\"expires\":{\"never\":{}}}}" | jq -r '.txhash')
echo "  Approved ${TOKEN_SYMBOLS[1]}: $TX_HASH"
wait_tx "$TX_HASH"

echo "  Re-providing liquidity ($READD_A / $READD_B)..."
READD_MSG="{\"provide_liquidity\":{\"assets\":[{\"info\":{\"token\":{\"contract_addr\":\"$READD_ADDR_A\"}},\"amount\":\"$READD_A\"},{\"info\":{\"token\":{\"contract_addr\":\"$READD_ADDR_B\"}},\"amount\":\"$READD_B\"}],\"slippage_tolerance\":null,\"receiver\":null,\"deadline\":null}}"
TX_HASH=$(terrad_tx wasm execute "$LP_PAIR_ADDR" "$READD_MSG" | jq -r '.txhash')
echo "  Re-provide TX: $TX_HASH"
wait_tx "$TX_HASH"

echo "  Liquidity cycle complete (1 withdraw + 1 re-provide)."
fi

# ── Phase 4b: Unpaired Token Pairs ──────────────────────────────────────
# IRON gets 1 pair, NEON gets 2 pairs, ZINC stays at 0 pairs

if [ "$QA_DEPLOY_SEED" = "full" ]; then
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
    TX_HASH=$(factory_create_pair "$CREATE_MSG" | jq -r '.txhash')
    echo "  TX: $TX_HASH"
    wait_tx "$TX_HASH"
    PAIR_RESULT=$(terrad_query tx "$TX_HASH")
    PAIR_ADDR=$(echo "$PAIR_RESULT" | terrad_jq_contract_address_from_tx_json | head -1)
    echo "  Pair Address: $PAIR_ADDR"

    TX_HASH=$(terrad_tx wasm execute "$FACTORY_ADDRESS" \
      "{\"set_discount_registry\":{\"pair\":\"$PAIR_ADDR\",\"registry\":\"$FEE_DISCOUNT_ADDRESS\"}}" | jq -r '.txhash')
    echo "  Set discount registry: $TX_HASH"
    wait_tx "$TX_HASH"

    TX_HASH=$(terrad_tx wasm execute "$ADDR_A" \
      "{\"increase_allowance\":{\"spender\":\"$PAIR_ADDR\",\"amount\":\"$LIQ_A\",\"expires\":{\"never\":{}}}}" | jq -r '.txhash')
    echo "  Approved $SYM_A: $TX_HASH"
    wait_tx "$TX_HASH"
    TX_HASH=$(terrad_tx wasm execute "$ADDR_B" \
      "{\"increase_allowance\":{\"spender\":\"$PAIR_ADDR\",\"amount\":\"$LIQ_B\",\"expires\":{\"never\":{}}}}" | jq -r '.txhash')
    echo "  Approved $SYM_B: $TX_HASH"
    wait_tx "$TX_HASH"

    PROVIDE_MSG="{\"provide_liquidity\":{\"assets\":[{\"info\":{\"token\":{\"contract_addr\":\"$ADDR_A\"}},\"amount\":\"$LIQ_A\"},{\"info\":{\"token\":{\"contract_addr\":\"$ADDR_B\"}},\"amount\":\"$LIQ_B\"}],\"slippage_tolerance\":null,\"receiver\":null,\"deadline\":null}}"
    TX_HASH=$(terrad_tx wasm execute "$PAIR_ADDR" "$PROVIDE_MSG" | jq -r '.txhash')
    echo "  Liquidity provided ($LIQ_A / $LIQ_B): $TX_HASH"
    wait_tx "$TX_HASH"
done

echo ""
echo "  $UNPAIRED_PAIR_NUM unpaired-token pairs created (ZINC: 0, IRON: 1, NEON: 2)."
fi

# ── Phase 4c: Wrapped-native pairs (wrap / native swap E2E, GitLab #201) ──

if { [ "$QA_DEPLOY_SEED" = "full" ] || [ "$QA_DEPLOY_SEED" = "wallet" ]; } && [ -n "$LUNC_C_ADDRESS" ]; then
echo ""
echo "[Phase 4c] Creating wrapped-native pairs for wrap E2E"
echo "----------------------------------------------"

WRAP_PAIR_NUM=0
for wp in \
  "$LUNC_C_ADDRESS:LUNC-C:${TOKEN_ADDRESSES[0]}:EMBER:100000000000:100000000000" \
  "$USTC_C_ADDRESS:USTC-C:${TOKEN_ADDRESSES[1]}:CORAL:100000000000:100000000000"
do
  IFS=':' read -r ADDR_A SYM_A ADDR_B SYM_B LIQ_A LIQ_B <<< "$wp"
  WRAP_PAIR_NUM=$((WRAP_PAIR_NUM + 1))
  echo ""
  echo "[14c.$WRAP_PAIR_NUM] Creating pair $SYM_A/$SYM_B..."

  CREATE_MSG="{\"create_pair\":{\"asset_infos\":[{\"token\":{\"contract_addr\":\"$ADDR_A\"}},{\"token\":{\"contract_addr\":\"$ADDR_B\"}}]}}"
  TX_HASH=$(factory_create_pair "$CREATE_MSG" | jq -r '.txhash')
  echo "  TX: $TX_HASH"
  wait_tx "$TX_HASH"
  PAIR_RESULT=$(terrad_query tx "$TX_HASH")
  PAIR_ADDR=$(echo "$PAIR_RESULT" | terrad_jq_contract_address_from_tx_json | head -1)
  echo "  Pair Address: $PAIR_ADDR"

  TX_HASH=$(terrad_tx wasm execute "$FACTORY_ADDRESS" \
    "{\"set_discount_registry\":{\"pair\":\"$PAIR_ADDR\",\"registry\":\"$FEE_DISCOUNT_ADDRESS\"}}" | jq -r '.txhash')
  echo "  Set discount registry: $TX_HASH"
  wait_tx "$TX_HASH"

  TX_HASH=$(terrad_tx wasm execute "$ADDR_A" \
    "{\"increase_allowance\":{\"spender\":\"$PAIR_ADDR\",\"amount\":\"$LIQ_A\",\"expires\":{\"never\":{}}}}" | jq -r '.txhash')
  echo "  Approved $SYM_A: $TX_HASH"
  wait_tx "$TX_HASH"
  TX_HASH=$(terrad_tx wasm execute "$ADDR_B" \
    "{\"increase_allowance\":{\"spender\":\"$PAIR_ADDR\",\"amount\":\"$LIQ_B\",\"expires\":{\"never\":{}}}}" | jq -r '.txhash')
  echo "  Approved $SYM_B: $TX_HASH"
  wait_tx "$TX_HASH"

  PROVIDE_MSG="{\"provide_liquidity\":{\"assets\":[{\"info\":{\"token\":{\"contract_addr\":\"$ADDR_A\"}},\"amount\":\"$LIQ_A\"},{\"info\":{\"token\":{\"contract_addr\":\"$ADDR_B\"}},\"amount\":\"$LIQ_B\"}],\"slippage_tolerance\":null,\"receiver\":null,\"deadline\":null}}"
  TX_HASH=$(terrad_tx wasm execute "$PAIR_ADDR" "$PROVIDE_MSG" | jq -r '.txhash')
  echo "  Liquidity provided ($LIQ_A / $LIQ_B): $TX_HASH"
  wait_tx "$TX_HASH"
done

echo ""
echo "  $WRAP_PAIR_NUM wrapped-native pairs created."
fi

# ── Phase 5: Test Swaps ─────────────────────────────────────────────────

if [ "$QA_DEPLOY_SEED" = "full" ] || [ "$QA_DEPLOY_SEED" = "charts" ]; then
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
    wait_tx "$TX_HASH"

    echo "  Swap $((SWAP_COUNT+1)): $SWAP_B1 $SYM_B -> $SYM_A"
    TX_HASH=$(terrad_tx wasm execute "$ADDR_B" \
      "{\"send\":{\"contract\":\"$PAIR_ADDR\",\"amount\":\"$SWAP_B1\",\"msg\":\"$SWAP_HOOK\"}}" | jq -r '.txhash')
    echo "    TX: $TX_HASH"
    SWAP_COUNT=$((SWAP_COUNT+1))
    wait_tx "$TX_HASH"

    echo "  Swap $((SWAP_COUNT+1)): $SWAP_A2 $SYM_A -> $SYM_B"
    TX_HASH=$(terrad_tx wasm execute "$ADDR_A" \
      "{\"send\":{\"contract\":\"$PAIR_ADDR\",\"amount\":\"$SWAP_A2\",\"msg\":\"$SWAP_HOOK\"}}" | jq -r '.txhash')
    echo "    TX: $TX_HASH"
    SWAP_COUNT=$((SWAP_COUNT+1))
    wait_tx "$TX_HASH"
done

echo ""
echo "  $SWAP_COUNT total swaps executed across ${#PAIR_CONFIGS[@]} pairs."
else
  SWAP_COUNT=0
  echo ""
  echo "[Phase 5] Skipped (seed=${QA_DEPLOY_SEED} — no swap history seed)"
fi

qa_timing_phase_end

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
# Must match TEST_MNEMONIC in docker/init-chain.sh (Simulated Wallet / local QA; GitLab #118).
TEST_MNEMONIC_DEV_WALLET="$(sed -n 's/^TEST_MNEMONIC="\(.*\)"/\1/p' "$REPO_ROOT/docker/init-chain.sh" | head -1)"
if [ -z "$TEST_MNEMONIC_DEV_WALLET" ]; then
  echo "ERROR: could not read TEST_MNEMONIC from $REPO_ROOT/docker/init-chain.sh (required for VITE_DEV_MNEMONIC)."
  exit 1
fi

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
VITE_GAS_PRICE_ULUNA=28.325
VITE_DEV_MODE=true
VITE_TREASURY_ADDRESS=$TREASURY_ADDRESS
VITE_WRAP_MAPPER_ADDRESS=$WRAP_MAPPER_ADDRESS
VITE_LUNC_C_TOKEN_ADDRESS=$LUNC_C_ADDRESS
VITE_USTC_C_TOKEN_ADDRESS=$USTC_C_ADDRESS
VITE_NOWHITELIST_TOKEN_1=${NOWHITELIST_ADDRESSES[0]:-}
VITE_NOWHITELIST_TOKEN_2=${NOWHITELIST_ADDRESSES[1]:-}
VITE_UNPAIRED_TOKEN_ZINC=${UNPAIRED_ADDRESSES[0]:-}
VITE_UNPAIRED_TOKEN_IRON=${UNPAIRED_ADDRESSES[1]:-}
VITE_UNPAIRED_TOKEN_NEON=${UNPAIRED_ADDRESSES[2]:-}
# Use 127.0.0.1 so the browser does not resolve "localhost" to ::1 while API_BIND is IPv4-only.
VITE_INDEXER_URL=http://127.0.0.1:${API_PORT:-3001}
ENVEOF
echo "  Written to frontend-dapp/.env.local"

# Dev-only: Vite does not load .env.development for `vite build` (production), so the mnemonic is not a prod-bundle risk.
cat > "$REPO_ROOT/frontend-dapp/.env.development" <<DEVMNEF
# LocalTerra test account (same as TEST_MNEMONIC in docker/init-chain.sh). GitLab #118.
VITE_DEV_MNEMONIC="${TEST_MNEMONIC_DEV_WALLET}"
DEVMNEF
echo "  Written to frontend-dapp/.env.development (VITE_DEV_MNEMONIC for Simulated Wallet)"

echo ""
echo "[Phase 6.2] Writing indexer/.env..."
set -a
if [ -f "$REPO_ROOT/.env" ]; then
  # shellcheck source=/dev/null
  source "$REPO_ROOT/.env"
fi
# shellcheck source=scripts/lib/postgres-dev.env
source "$REPO_ROOT/scripts/lib/postgres-dev.env"
set +a
chmod +x "$REPO_ROOT/scripts/setup-postgres-dev-databases.sh"
"$REPO_ROOT/scripts/setup-postgres-dev-databases.sh"
cat > "$REPO_ROOT/indexer/.env" <<ENVEOF
DATABASE_URL=$DATABASE_URL
TEST_DATABASE_URL=$TEST_DATABASE_URL
FACTORY_ADDRESS=$FACTORY_ADDRESS
ROUTER_ADDRESS=$ROUTER_ADDRESS
FEE_DISCOUNT_ADDRESS=$FEE_DISCOUNT_ADDRESS
LCD_URLS=$LCD
CORS_ORIGINS=http://localhost:3000,http://localhost:5173,http://localhost:4173,http://127.0.0.1:3000,http://127.0.0.1:5173,http://127.0.0.1:4173
API_PORT=${API_PORT:-3001}
API_BIND=127.0.0.1
POLL_INTERVAL_MS=2000
# 0 disables the global tower-governor locally so Playwright + React Query bursts do not 429 the UI.
RATE_LIMIT_RPS=0
# LCD-heavy routes keep explicit 10 RPS cap for QA template (#363); applies even when global RATE_LIMIT_RPS=0.
RATE_LIMIT_LCD_HEAVY_RPS=10
ENVEOF
echo "  Written to indexer/.env"

echo ""
echo "[Phase 6.3] Writing QA deploy stamp (.qa-deploy-stamp)..."
DEPLOY_GIT_SHA="$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
DEPLOY_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
VERIFY_PAIR="${PAIR_ADDRESSES[0]:-}"
cat > "$REPO_ROOT/.qa-deploy-stamp" <<STAMPEOF
# Written by scripts/deploy-dex-local.sh — sourced by scripts/qa/verify-deploy.sh (GitLab #203)
git_sha=${DEPLOY_GIT_SHA}
deployed_at=${DEPLOY_AT}
factory_address=${FACTORY_ADDRESS}
pair_address=${VERIFY_PAIR}
STAMPEOF
echo "  git_sha=${DEPLOY_GIT_SHA} pair=${VERIFY_PAIR}"

echo ""
echo "Test address: $TEST_ADDRESS"
echo "  seed=${QA_DEPLOY_SEED}: ${#TOKEN_SYMBOLS[@]} tokens, ${#UNPAIRED_SYMBOLS[@]} unpaired, ${#NOWHITELIST_SYMBOLS[@]} non-whitelisted, ${#PAIR_CONFIGS[@]} pairs, $SWAP_COUNT swaps executed"
qa_timing_session_end
