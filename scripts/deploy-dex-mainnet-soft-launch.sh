#!/usr/bin/env bash
# Single-script columbus-5 soft launch: DEX contracts + non-economic CW20s + pairs + liquidity.
#
# Prerequisites:
#   - make build-optimized (DEX wasm in smartcontracts/artifacts/)
#   - terrad key MAINNET_SOFT_LAUNCH_DEPLOY_KEY (default cl8ydeploy) funded for gas + pair fees
#   - Governance multisig already exists (admin/treasury); deployer is NOT governance
#   - MAINNET_CW20_BASE_CODE_ID defaults to 6036 (Terraswap cw20-base); empty → store artifact
#
# Usage:
#   ./scripts/deploy-dex-mainnet-soft-launch.sh
#   DRY_RUN=1 ./scripts/deploy-dex-mainnet-soft-launch.sh   # no broadcast
#   SKIP_LIQUIDITY=1 ./scripts/deploy-dex-mainnet-soft-launch.sh
#
# Outputs:
#   deployments/mainnet-soft-launch/addresses.env
#   deployments/mainnet-soft-launch/frontend.env.example
#   deployments/mainnet-soft-launch/indexer.env.example
#
# Docs: docs/runbooks/mainnet-soft-launch.md
# Skill: skills/AGENTS_MAINNET_SOFT_LAUNCH.md
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=lib/governance-multisig.sh
source "$SCRIPT_DIR/lib/governance-multisig.sh"
# shellcheck source=lib/mainnet-soft-launch-defaults.sh
source "$SCRIPT_DIR/lib/mainnet-soft-launch-defaults.sh"
# shellcheck source=lib/terrad-host.sh
source "$SCRIPT_DIR/lib/terrad-host.sh"
# shellcheck source=lib/terrad-tx-events.sh
source "$SCRIPT_DIR/lib/terrad-tx-events.sh"

ARTIFACTS_DIR="${ARTIFACTS_DIR:-$REPO_ROOT/smartcontracts/artifacts}"
OUT_DIR="${MAINNET_SOFT_LAUNCH_OUT_DIR:-$REPO_ROOT/deployments/mainnet-soft-launch}"
CW20_BASE_WASM="${CW20_BASE_WASM:-$ARTIFACTS_DIR/cw20_base.wasm}"
CW20_MINTABLE_WASM="${CW20_MINTABLE_WASM:-$ARTIFACTS_DIR/cw20_mintable.wasm}"
CW20_MINTABLE_FALLBACK="${CW20_MINTABLE_FALLBACK:-$HOME/repos/cw20-mintable/artifacts/cw20_mintable.wasm}"

TERRAD_HOST_KEY="$MAINNET_SOFT_LAUNCH_DEPLOY_KEY"
TERRAD_HOST_EXPECTED_ADDR="$MAINNET_SOFT_LAUNCH_DEPLOY_ADDR"
TERRAD_HOST_CHAIN_ID="${TERRAD_HOST_CHAIN_ID:-columbus-5}"
TERRAD_HOST_NODE="${TERRAD_HOST_NODE:-https://terra-classic-rpc.publicnode.com:443}"

mkdir -p "$OUT_DIR"

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "ERROR: required command not found: $1" >&2
    exit 1
  }
}

require_artifact() {
  local path="$1"
  [[ -f "$path" ]] || {
    echo "ERROR: missing wasm artifact: $path" >&2
    echo "  Run: make build-optimized" >&2
    exit 1
  }
}

broadcast_and_wait() {
  local label="$1"
  shift
  local out tx_hash
  echo "  → $label" >&2
  out="$(terrad_host_tx "$@")"
  tx_hash="$(printf '%s' "$out" | jq -r '.txhash // empty')"
  [[ -n "$tx_hash" ]] || {
    echo "ERROR: no txhash from: $label" >&2
    printf '%s\n' "$out" >&2
    exit 1
  }
  echo "    tx: $tx_hash" >&2
  terrad_host_wait_tx_inclusion "$tx_hash"
  printf '%s' "$tx_hash"
}

store_code() {
  local wasm="$1"
  local label="$2"
  local tx_hash code_id
  tx_hash="$(broadcast_and_wait "store $label" wasm store "$wasm")"
  code_id="$(terrad_host_code_id_from_store_tx "$tx_hash")"
  [[ -n "$code_id" ]] || {
    echo "ERROR: could not parse code_id for $label" >&2
    exit 1
  }
  echo "    code_id: $code_id" >&2
  printf '%s' "$code_id"
}

instantiate_no_funds() {
  local code_id="$1"
  local init_msg="$2"
  local label="$3"
  local admin="$4"
  local out tx_hash addr
  echo "  → instantiate $label" >&2
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    tx_hash="DRY_RUN_TX"
    echo "    tx: $tx_hash" >&2
  else
    out="$(terrad_host_exec tx wasm instantiate "$code_id" "$init_msg" \
      --label "$label" \
      --admin "$admin" \
      --from "$TERRAD_HOST_KEY" \
      $(terrad_host_common_flags) \
      --gas auto \
      --gas-adjustment "$TERRAD_HOST_GAS_ADJUSTMENT" \
      $(terrad_host_fee_flags) \
      --broadcast-mode "$TERRAD_HOST_BROADCAST_MODE" \
      -y --output json)"
    tx_hash="$(printf '%s' "$out" | jq -r '.txhash // empty')"
    [[ -n "$tx_hash" ]] || {
      echo "ERROR: instantiate failed: $label" >&2
      printf '%s\n' "$out" >&2
      exit 1
    }
    echo "    tx: $tx_hash" >&2
    terrad_host_wait_tx_inclusion "$tx_hash"
  fi
  addr="$(DRY_RUN_LABEL="$label" terrad_host_contract_address_from_instantiate_tx "$tx_hash")"
  echo "    address: $addr" >&2
  printf '%s' "$addr"
}

execute_msg() {
  local contract="$1"
  local msg="$2"
  local label="$3"
  local amount_flag=()
  if [[ -n "${4:-}" ]]; then
    amount_flag=(--amount "$4")
  fi
  broadcast_and_wait "$label" wasm execute "$contract" "$msg" "${amount_flag[@]}" >/dev/null
}

echo "=============================================="
echo "CL8Y DEX mainnet soft launch (non-economic)"
echo "=============================================="
echo "Chain:      $TERRAD_HOST_CHAIN_ID"
echo "Node:       $TERRAD_HOST_NODE"
echo "Deploy key: $TERRAD_HOST_KEY"
terrad_host_resolve_keyring_backend
echo "Keyring:    $TERRAD_HOST_KEYRING_BACKEND ($TERRAD_HOST_HOME)"
echo "Governance: $MAINNET_SOFT_LAUNCH_GOVERNANCE"
echo "Treasury:   $MAINNET_SOFT_LAUNCH_TREASURY"
echo "Tokens:     $(mainnet_soft_launch_token_count)"
echo "Pairs:      $(mainnet_soft_launch_pair_count)"
echo "DRY_RUN:    ${DRY_RUN:-0}"
echo ""

require_cmd terrad
require_cmd jq
require_cmd sha256sum

DEPLOY_ADDR="$(terrad_host_key_address)" || exit 1
if [[ "$DEPLOY_ADDR" != "$MAINNET_SOFT_LAUNCH_DEPLOY_ADDR" ]]; then
  echo "ERROR: key $TERRAD_HOST_KEY address is $DEPLOY_ADDR" >&2
  echo "  expected $MAINNET_SOFT_LAUNCH_DEPLOY_ADDR" >&2
  exit 1
fi
echo "Deployer address OK: $DEPLOY_ADDR"

require_artifact "$ARTIFACTS_DIR/cl8y_dex_factory.wasm"
require_artifact "$ARTIFACTS_DIR/cl8y_dex_pair.wasm"
require_artifact "$ARTIFACTS_DIR/cl8y_dex_router.wasm"
require_artifact "$ARTIFACTS_DIR/cl8y_dex_fee_discount.wasm"

if [[ ! -f "$CW20_MINTABLE_WASM" && -f "$CW20_MINTABLE_FALLBACK" ]]; then
  echo "Copying cw20_mintable.wasm from $CW20_MINTABLE_FALLBACK"
  cp "$CW20_MINTABLE_FALLBACK" "$CW20_MINTABLE_WASM"
fi

# ── CW20 code IDs (SL1 / SL2) ───────────────────────────────────────────
echo ""
echo "[1] CW20 code IDs (standard + mintable only)"
CW20_MINTABLE_CODE_ID="$MAINNET_CW20_MINTABLE_CODE_ID"
if [[ "${FORCE_STORE_CW20_MINTABLE:-0}" == "1" ]]; then
  require_artifact "$CW20_MINTABLE_WASM"
  CW20_MINTABLE_CODE_ID="$(store_code "$CW20_MINTABLE_WASM" "cw20_mintable")"
else
  echo "  Reusing mainnet cw20-mintable code_id=$CW20_MINTABLE_CODE_ID"
fi

CW20_BASE_CODE_ID="${MAINNET_CW20_BASE_CODE_ID:-}"
if [[ -z "$CW20_BASE_CODE_ID" ]]; then
  if [[ ! -f "$CW20_BASE_WASM" ]]; then
    echo "  cw20_base.wasm missing — building via scripts/build-cw20-base-artifact.sh"
    if [[ "${DRY_RUN:-0}" == "1" ]]; then
      CW20_BASE_CODE_ID="${DRY_RUN_CW20_BASE_CODE_ID:-999002}"
      echo "  DRY_RUN: pretending cw20_base code_id=$CW20_BASE_CODE_ID"
    else
      bash "$SCRIPT_DIR/build-cw20-base-artifact.sh"
      require_artifact "$CW20_BASE_WASM"
      CW20_BASE_CODE_ID="$(store_code "$CW20_BASE_WASM" "cw20_base")"
    fi
  else
    if [[ "${DRY_RUN:-0}" == "1" ]]; then
      CW20_BASE_CODE_ID="${DRY_RUN_CW20_BASE_CODE_ID:-999002}"
    else
      CW20_BASE_CODE_ID="$(store_code "$CW20_BASE_WASM" "cw20_base")"
    fi
  fi
else
  echo "  Reusing MAINNET_CW20_BASE_CODE_ID=$CW20_BASE_CODE_ID"
fi

echo "  Whitelist code IDs: base=$CW20_BASE_CODE_ID mintable=$CW20_MINTABLE_CODE_ID"

# ── Store DEX contracts ─────────────────────────────────────────────────
echo ""
echo "[2] Store DEX contracts"
if [[ -n "${FACTORY_CODE_ID:-}" && -n "${PAIR_CODE_ID:-}" && -n "${ROUTER_CODE_ID:-}" && -n "${FEE_DISCOUNT_CODE_ID:-}" ]]; then
  echo "  Reusing FACTORY_CODE_ID=$FACTORY_CODE_ID PAIR_CODE_ID=$PAIR_CODE_ID ROUTER_CODE_ID=$ROUTER_CODE_ID FEE_DISCOUNT_CODE_ID=$FEE_DISCOUNT_CODE_ID"
elif [[ "${SKIP_STORE:-0}" == "1" ]]; then
  echo "ERROR: SKIP_STORE=1 requires FACTORY_CODE_ID PAIR_CODE_ID ROUTER_CODE_ID FEE_DISCOUNT_CODE_ID" >&2
  exit 1
else
  FACTORY_CODE_ID="$(store_code "$ARTIFACTS_DIR/cl8y_dex_factory.wasm" "factory")"
  PAIR_CODE_ID="$(store_code "$ARTIFACTS_DIR/cl8y_dex_pair.wasm" "pair")"
  ROUTER_CODE_ID="$(store_code "$ARTIFACTS_DIR/cl8y_dex_router.wasm" "router")"
  FEE_DISCOUNT_CODE_ID="$(store_code "$ARTIFACTS_DIR/cl8y_dex_fee_discount.wasm" "fee_discount")"
fi

# LP shares use standard cw20-base (18 decimals set by pair instantiate).
LP_TOKEN_CODE_ID="$CW20_BASE_CODE_ID"

# Bootstrap governance = deployer so add_tier / set_discount_registry work.
# Wasm --admin and treasury stay multisig; config.governance is handed off after setup (SL4).
BOOTSTRAP_GOVERNANCE="$DEPLOY_ADDR"
FINAL_GOVERNANCE="$MAINNET_SOFT_LAUNCH_GOVERNANCE"

# ── Instantiate factory / router / fee-discount ─────────────────────────
echo ""
echo "[3] Instantiate factory, router, fee-discount"
echo "  Bootstrap governance (config): $BOOTSTRAP_GOVERNANCE"
echo "  Wasm admin / final governance: $FINAL_GOVERNANCE"
echo "  Treasury:                      $MAINNET_SOFT_LAUNCH_TREASURY"
FACTORY_INIT="$(jq -nc \
  --arg gov "$BOOTSTRAP_GOVERNANCE" \
  --arg treasury "$MAINNET_SOFT_LAUNCH_TREASURY" \
  --argjson fee "$MAINNET_SOFT_LAUNCH_DEFAULT_FEE_BPS" \
  --argjson pair_code "$PAIR_CODE_ID" \
  --argjson lp_code "$LP_TOKEN_CODE_ID" \
  --argjson base_code "$CW20_BASE_CODE_ID" \
  --argjson mint_code "$CW20_MINTABLE_CODE_ID" \
  --arg pair_fee "$MAINNET_SOFT_LAUNCH_PAIR_CREATION_FEE_ULUNA" \
  '{
    governance: $gov,
    treasury: $treasury,
    default_fee_bps: $fee,
    pair_code_id: $pair_code,
    lp_token_code_id: $lp_code,
    whitelisted_code_ids: [$base_code, $mint_code],
    pair_creation_fee_uluna: $pair_fee
  }')"

# Wasm admin = multisig (migrate control). Config governance = deployer until handoff.
FACTORY_ADDRESS="$(instantiate_no_funds "$FACTORY_CODE_ID" "$FACTORY_INIT" "cl8y-dex-factory" "$FINAL_GOVERNANCE")"

ROUTER_INIT="$(jq -nc --arg factory "$FACTORY_ADDRESS" '{factory: $factory}')"
ROUTER_ADDRESS="$(instantiate_no_funds "$ROUTER_CODE_ID" "$ROUTER_INIT" "cl8y-dex-router" "$FINAL_GOVERNANCE")"

FEE_INIT="$(jq -nc \
  --arg gov "$BOOTSTRAP_GOVERNANCE" \
  --arg cl8y "$MAINNET_CL8Y_TOKEN_ADDRESS" \
  '{governance: $gov, cl8y_token: $cl8y}')"
FEE_DISCOUNT_ADDRESS="$(instantiate_no_funds "$FEE_DISCOUNT_CODE_ID" "$FEE_INIT" "cl8y-dex-fee-discount" "$FINAL_GOVERNANCE")"

echo ""
echo "[4] Fee-discount tiers + trusted router"
while IFS= read -r TIER_MSG; do
  [[ -n "$TIER_MSG" ]] || continue
  execute_msg "$FEE_DISCOUNT_ADDRESS" "$TIER_MSG" "add_tier"
done < <(mainnet_soft_launch_fee_discount_tier_msgs)

TRUST_MSG="$(jq -nc --arg r "$ROUTER_ADDRESS" '{add_trusted_router: {router: $r}}')"
execute_msg "$FEE_DISCOUNT_ADDRESS" "$TRUST_MSG" "add_trusted_router"

# ── Instantiate tokens ──────────────────────────────────────────────────
echo ""
echo "[5] Instantiate non-economic CW20 tokens"
declare -A TOKEN_ADDR_BY_SYM
declare -a TOKEN_ADDR_ORDER=()

for entry in "${MAINNET_SOFT_LAUNCH_TOKENS[@]}"; do
  IFS='|' read -r NAME SYM DECIMALS KIND AMOUNT <<<"$entry"
  if [[ "$KIND" == "mintable" ]]; then
    CODE_ID="$CW20_MINTABLE_CODE_ID"
    INIT="$(jq -nc \
      --arg name "$NAME" \
      --arg symbol "$SYM" \
      --argjson decimals "$DECIMALS" \
      --arg addr "$DEPLOY_ADDR" \
      --arg amount "$AMOUNT" \
      '{
        name: $name,
        symbol: $symbol,
        decimals: $decimals,
        initial_balances: [{address: $addr, amount: $amount}],
        mint: {minter: $addr}
      }')"
  elif [[ "$KIND" == "base" ]]; then
    CODE_ID="$CW20_BASE_CODE_ID"
    INIT="$(jq -nc \
      --arg name "$NAME" \
      --arg symbol "$SYM" \
      --argjson decimals "$DECIMALS" \
      --arg addr "$DEPLOY_ADDR" \
      --arg amount "$AMOUNT" \
      '{
        name: $name,
        symbol: $symbol,
        decimals: $decimals,
        initial_balances: [{address: $addr, amount: $amount}]
      }')"
  else
    echo "ERROR: unknown token kind '$KIND' for $SYM" >&2
    exit 1
  fi
  ADDR="$(instantiate_no_funds "$CODE_ID" "$INIT" "soft-$SYM" "$MAINNET_SOFT_LAUNCH_GOVERNANCE")"
  TOKEN_ADDR_BY_SYM["$SYM"]="$ADDR"
  TOKEN_ADDR_ORDER+=("$SYM=$ADDR")
  echo "  $SYM ($KIND) → $ADDR"
done

# ── Create pairs + liquidity ────────────────────────────────────────────
echo ""
echo "[6] Create pairs, discount registry, liquidity"
PAIR_ADDRESSES=()
PAIR_CREATION_FEE="${MAINNET_SOFT_LAUNCH_PAIR_CREATION_FEE_ULUNA}uluna"

for pair_entry in "${MAINNET_SOFT_LAUNCH_PAIRS[@]}"; do
  IFS='|' read -r SYM_A SYM_B AMT_A AMT_B <<<"$pair_entry"
  ADDR_A="${TOKEN_ADDR_BY_SYM[$SYM_A]:-}"
  ADDR_B="${TOKEN_ADDR_BY_SYM[$SYM_B]:-}"
  [[ -n "$ADDR_A" && -n "$ADDR_B" ]] || {
    echo "ERROR: missing token addr for pair $SYM_A/$SYM_B" >&2
    exit 1
  }

  CREATE_MSG="$(jq -nc \
    --arg a "$ADDR_A" \
    --arg b "$ADDR_B" \
    '{create_pair: {asset_infos: [
      {token: {contract_addr: $a}},
      {token: {contract_addr: $b}}
    ]}}')"

  echo "  Creating $SYM_A/$SYM_B"
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    PAIR_ADDR="terra1dryrunpair$(echo "$SYM_A$SYM_B" | tr '[:upper:]' '[:lower:]')"
    echo "    DRY_RUN pair=$PAIR_ADDR"
  else
    TX_HASH="$(broadcast_and_wait "create_pair $SYM_A/$SYM_B" \
      wasm execute "$FACTORY_ADDRESS" "$CREATE_MSG" \
      --amount "$PAIR_CREATION_FEE")"
    TX_JSON="$(terrad_host_wait_tx_query "$TX_HASH")"
    PAIR_ADDR="$(printf '%s' "$TX_JSON" | terrad_jq_contract_address_from_tx_json | head -1)"
    if [[ -z "$PAIR_ADDR" ]]; then
      PAIR_Q="$(jq -nc --arg a "$ADDR_A" --arg b "$ADDR_B" \
        '{pair: {asset_infos: [{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')"
      PAIR_ADDR="$(terrad_host_query wasm contract-state smart "$FACTORY_ADDRESS" "$PAIR_Q" \
        | jq -r '.data.pair.contract_addr // .data.contract_addr // .contract_addr // empty')"
    fi
    [[ -n "$PAIR_ADDR" ]] || {
      echo "ERROR: could not resolve pair address for $SYM_A/$SYM_B" >&2
      exit 1
    }
  fi
  echo "    pair: $PAIR_ADDR"
  PAIR_ADDRESSES+=("$SYM_A/$SYM_B=$PAIR_ADDR")

  REG_MSG="$(jq -nc --arg pair "$PAIR_ADDR" --arg reg "$FEE_DISCOUNT_ADDRESS" \
    '{set_discount_registry: {pair: $pair, registry: $reg}}')"
  execute_msg "$FACTORY_ADDRESS" "$REG_MSG" "set_discount_registry $SYM_A/$SYM_B"

  if [[ "${SKIP_LIQUIDITY:-0}" == "1" ]]; then
    continue
  fi

  ALLOW_A="$(jq -nc --arg spender "$PAIR_ADDR" --arg amount "$AMT_A" \
    '{increase_allowance: {spender: $spender, amount: $amount, expires: {never: {}}}}')"
  ALLOW_B="$(jq -nc --arg spender "$PAIR_ADDR" --arg amount "$AMT_B" \
    '{increase_allowance: {spender: $spender, amount: $amount, expires: {never: {}}}}')"
  execute_msg "$ADDR_A" "$ALLOW_A" "allowance $SYM_A"
  execute_msg "$ADDR_B" "$ALLOW_B" "allowance $SYM_B"

  PROVIDE="$(jq -nc \
    --arg a "$ADDR_A" --arg b "$ADDR_B" \
    --arg aa "$AMT_A" --arg ba "$AMT_B" \
    '{provide_liquidity: {assets: [
      {info: {token: {contract_addr: $a}}, amount: $aa},
      {info: {token: {contract_addr: $b}}, amount: $ba}
    ]}}')"
  execute_msg "$PAIR_ADDR" "$PROVIDE" "provide_liquidity $SYM_A/$SYM_B"
done

if [[ "${SKIP_LIQUIDITY:-0}" == "1" ]]; then
  echo "  SKIP_LIQUIDITY=1 — pairs created without LP seed"
fi

# Belt-and-suspenders for ≤10 pairs (soft-launch default is 10).
echo ""
echo "[7] set_discount_registry_all"
ALL_MSG="$(jq -nc --arg r "$FEE_DISCOUNT_ADDRESS" '{set_discount_registry_all: {registry: $r}}')"
execute_msg "$FACTORY_ADDRESS" "$ALL_MSG" "set_discount_registry_all"

# ── Handoff config.governance to multisig (wasm admin already multisig) ─
echo ""
echo "[7b] Handoff config governance → multisig"
FEE_HANDOFF="$(jq -nc --arg gov "$FINAL_GOVERNANCE" '{update_config: {governance: $gov}}')"
execute_msg "$FEE_DISCOUNT_ADDRESS" "$FEE_HANDOFF" "fee-discount update_config governance"
FACTORY_HANDOFF="$(jq -nc --arg gov "$FINAL_GOVERNANCE" '{update_config: {governance: $gov}}')"
execute_msg "$FACTORY_ADDRESS" "$FACTORY_HANDOFF" "factory update_config governance"

# ── Write env outputs ───────────────────────────────────────────────────
echo ""
echo "[8] Writing $OUT_DIR"

{
  echo "# Generated by deploy-dex-mainnet-soft-launch.sh — $(date -u +%Y-%m-%dT%H:%MZ)"
  echo "NETWORK=mainnet"
  echo "CHAIN_ID=$TERRAD_HOST_CHAIN_ID"
  echo "DEPLOYER_ADDRESS=$DEPLOY_ADDR"
  echo "GOVERNANCE_ADDRESS=$MAINNET_SOFT_LAUNCH_GOVERNANCE"
  echo "TREASURY_ADDRESS=$MAINNET_SOFT_LAUNCH_TREASURY"
  echo "CW20_BASE_CODE_ID=$CW20_BASE_CODE_ID"
  echo "CW20_MINTABLE_CODE_ID=$CW20_MINTABLE_CODE_ID"
  echo "FACTORY_CODE_ID=$FACTORY_CODE_ID"
  echo "PAIR_CODE_ID=$PAIR_CODE_ID"
  echo "ROUTER_CODE_ID=$ROUTER_CODE_ID"
  echo "FEE_DISCOUNT_CODE_ID=$FEE_DISCOUNT_CODE_ID"
  echo "FACTORY_ADDRESS=$FACTORY_ADDRESS"
  echo "ROUTER_ADDRESS=$ROUTER_ADDRESS"
  echo "FEE_DISCOUNT_ADDRESS=$FEE_DISCOUNT_ADDRESS"
  echo "CL8Y_TOKEN_ADDRESS=$MAINNET_CL8Y_TOKEN_ADDRESS"
  for row in "${TOKEN_ADDR_ORDER[@]}"; do
    sym="${row%%=*}"
    addr="${row#*=}"
    echo "TOKEN_${sym}_ADDRESS=$addr"
  done
  for pa in "${PAIR_ADDRESSES[@]}"; do
    label="${pa%%=*}"
    addr="${pa#*=}"
    safe="$(echo "$label" | tr '/-' '_')"
    echo "PAIR_${safe}_ADDRESS=$addr"
  done
} >"$OUT_DIR/addresses.env"

{
  echo "# Coolify / static frontend build args for $MAINNET_SOFT_LAUNCH_FRONTEND_ORIGIN"
  echo "VITE_NETWORK=mainnet"
  echo "VITE_FACTORY_ADDRESS=$FACTORY_ADDRESS"
  echo "VITE_ROUTER_ADDRESS=$ROUTER_ADDRESS"
  echo "VITE_FEE_DISCOUNT_ADDRESS=$FEE_DISCOUNT_ADDRESS"
  echo "VITE_CL8Y_TOKEN_ADDRESS=$MAINNET_CL8Y_TOKEN_ADDRESS"
  echo "VITE_INDEXER_URL=$MAINNET_SOFT_LAUNCH_INDEXER_ORIGIN"
  echo "VITE_TERRA_LCD_URL=https://terra-classic-lcd.publicnode.com"
  echo "VITE_TERRA_RPC_URL=https://terra-classic-rpc.publicnode.com:443"
  echo "VITE_GAS_PRICE_ULUNA=28.325"
  echo "VITE_WC_PROJECT_ID="
  echo "# Post-SL5 wrap enablement (GitLab #507) — set in Coolify after Phase 3; do NOT uncomment in soft-launch defaults."
  echo "# VITE_TREASURY_ADDRESS must be ustr-cmm CMM treasury (terra16j5u6…), NOT governance multisig."
  echo "# VITE_WRAP_MAPPER_ADDRESS=terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2"
  echo "# VITE_TREASURY_ADDRESS=terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2"
  echo "# VITE_LUNC_C_TOKEN_ADDRESS=terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg"
  echo "# VITE_USTC_C_TOKEN_ADDRESS=terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch"
} >"$OUT_DIR/frontend.env.example"

{
  echo "# Coolify indexer env for $MAINNET_SOFT_LAUNCH_INDEXER_ORIGIN"
  echo "RUN_MODE=prod"
  echo "API_BIND=0.0.0.0"
  echo "API_PORT=3001"
  echo "FACTORY_ADDRESS=$FACTORY_ADDRESS"
  echo "ROUTER_ADDRESS=$ROUTER_ADDRESS"
  echo "FEE_DISCOUNT_ADDRESS=$FEE_DISCOUNT_ADDRESS"
  echo "CORS_ORIGINS=$MAINNET_SOFT_LAUNCH_FRONTEND_ORIGIN"
  echo "LCD_URLS="
  echo "DATABASE_URL="
  echo "RATE_LIMIT_RPS=60"
  echo "RATE_LIMIT_LCD_HEAVY_RPS=10"
} >"$OUT_DIR/indexer.env.example"

echo ""
echo "=============================================="
echo "Soft launch deploy complete"
echo "  Factory:      $FACTORY_ADDRESS"
echo "  Router:       $ROUTER_ADDRESS"
echo "  Fee discount: $FEE_DISCOUNT_ADDRESS"
echo "  Addresses:    $OUT_DIR/addresses.env"
echo "=============================================="
echo "Next: configure Coolify with docker/indexer + docker/frontend"
echo "  Frontend origin: $MAINNET_SOFT_LAUNCH_FRONTEND_ORIGIN"
echo "  Indexer origin:  $MAINNET_SOFT_LAUNCH_INDEXER_ORIGIN"
