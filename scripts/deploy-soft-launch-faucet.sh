#!/usr/bin/env bash
# Deploy soft-launch faucet (GitLab #473) on columbus-5 (or LocalTerra via env overrides).
#
# Prerequisites:
#   - Soft-launch DEX + mintable tokens already deployed (addresses.env)
#   - Optimized wasm: smartcontracts/artifacts/cl8y_dex_faucet.wasm
#   - Deploy key is primary CW20 minter (cl8ydeploy) so AddMinter works (F5/F6)
#
# Usage:
#   DRY_RUN=1 ./scripts/deploy-soft-launch-faucet.sh
#   ./scripts/deploy-soft-launch-faucet.sh
#   SKIP_ADD_MINTER=1 ./scripts/deploy-soft-launch-faucet.sh   # instantiate only
#
# Outputs (appends / updates):
#   deployments/mainnet-soft-launch/addresses.env  (FAUCET_*)
#   deployments/mainnet-soft-launch/frontend.env.example
#   deployments/mainnet-soft-launch/faucet-trace.md
#
# Docs: docs/runbooks/soft-launch-faucet.md
# Skill: skills/AGENTS_SOFT_LAUNCH_FAUCET.md
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=lib/mainnet-soft-launch-defaults.sh
source "$SCRIPT_DIR/lib/mainnet-soft-launch-defaults.sh"
# shellcheck source=lib/terrad-host.sh
source "$SCRIPT_DIR/lib/terrad-host.sh"
# shellcheck source=lib/terrad-tx-events.sh
source "$SCRIPT_DIR/lib/terrad-tx-events.sh"

ARTIFACTS_DIR="${ARTIFACTS_DIR:-$REPO_ROOT/smartcontracts/artifacts}"
OUT_DIR="${MAINNET_SOFT_LAUNCH_OUT_DIR:-$REPO_ROOT/deployments/mainnet-soft-launch}"
ADDRESSES_ENV="${ADDRESSES_ENV:-$OUT_DIR/addresses.env}"
FAUCET_WASM="${FAUCET_WASM:-$ARTIFACTS_DIR/cl8y_dex_faucet.wasm}"

# Soft-launch faucet defaults (F2/F3).
FAUCET_DRIP_AMOUNT="${FAUCET_DRIP_AMOUNT:-100000000}"
FAUCET_COOLDOWN_SECONDS="${FAUCET_COOLDOWN_SECONDS:-300}"

TERRAD_HOST_KEY="${TERRAD_HOST_KEY:-$MAINNET_SOFT_LAUNCH_DEPLOY_KEY}"
TERRAD_HOST_EXPECTED_ADDR="${TERRAD_HOST_EXPECTED_ADDR:-$MAINNET_SOFT_LAUNCH_DEPLOY_ADDR}"
TERRAD_HOST_CHAIN_ID="${TERRAD_HOST_CHAIN_ID:-columbus-5}"
TERRAD_HOST_NODE="${TERRAD_HOST_NODE:-https://terra-classic-rpc.publicnode.com:443}"

mkdir -p "$OUT_DIR"

require_cmd() { command -v "$1" >/dev/null 2>&1 || { echo "ERROR: missing command: $1" >&2; exit 1; }; }

require_artifact() {
  local path="$1"
  [[ -f "$path" ]] || {
    echo "ERROR: missing artifact: $path" >&2
    echo "  Run: make build-optimized" >&2
    exit 1
  }
}

broadcast_and_wait() {
  local label="$1"
  shift
  local out tx_hash
  echo "  → $label" >&2
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "    DRY_RUN (no broadcast)" >&2
    printf '%s' "DRY_RUN_TX"
    return
  fi
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
  local out tx_hash code_id
  echo "  → store $label ($(basename "$wasm"))" >&2
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    code_id="${DRY_RUN_FAUCET_CODE_ID:-999473}"
    echo "    DRY_RUN code_id=$code_id" >&2
    printf '%s' "$code_id"
    return
  fi
  out="$(terrad_host_exec tx wasm store "$wasm" \
    --from "$TERRAD_HOST_KEY" \
    $(terrad_host_common_flags) \
    --gas auto \
    --gas-adjustment "$TERRAD_HOST_GAS_ADJUSTMENT" \
    $(terrad_host_fee_flags) \
    --broadcast-mode "$TERRAD_HOST_BROADCAST_MODE" \
    -y --output json)"
  tx_hash="$(printf '%s' "$out" | jq -r '.txhash // empty')"
  [[ -n "$tx_hash" ]] || {
    echo "ERROR: store failed: $label" >&2
    printf '%s\n' "$out" >&2
    exit 1
  }
  echo "    tx: $tx_hash" >&2
  terrad_host_wait_tx_inclusion "$tx_hash"
  code_id="$(terrad_host_code_id_from_store_tx "$tx_hash")"
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
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "  → DRY_RUN execute $label on $contract" >&2
    echo "    msg: $msg" >&2
    return
  fi
  broadcast_and_wait "$label" wasm execute "$contract" "$msg" >/dev/null
}

upsert_env_var() {
  local file="$1"
  local key="$2"
  local value="$3"
  touch "$file"
  if grep -q "^${key}=" "$file" 2>/dev/null; then
    # portable in-place replace
    local tmp
    tmp="$(mktemp)"
    awk -v k="$key" -v v="$value" 'BEGIN{FS=OFS="="} $1==k {$0=k"="v} {print}' "$file" >"$tmp"
    mv "$tmp" "$file"
  else
    echo "${key}=${value}" >>"$file"
  fi
}

echo "=============================================="
echo "CL8Y DEX soft-launch faucet (GitLab #473)"
echo "=============================================="
echo "Chain:      $TERRAD_HOST_CHAIN_ID"
echo "Node:       $TERRAD_HOST_NODE"
echo "Deploy key: $TERRAD_HOST_KEY"
terrad_host_resolve_keyring_backend
echo "Keyring:    $TERRAD_HOST_KEYRING_BACKEND ($TERRAD_HOST_HOME)"
echo "Addresses:  $ADDRESSES_ENV"
echo "DRY_RUN:    ${DRY_RUN:-0}"
echo "SKIP_ADD_MINTER: ${SKIP_ADD_MINTER:-0}"
echo ""

require_cmd terrad
require_cmd jq
require_artifact "$FAUCET_WASM"

if [[ ! -f "$ADDRESSES_ENV" ]]; then
  echo "ERROR: missing $ADDRESSES_ENV — deploy soft-launch DEX first" >&2
  exit 1
fi

# shellcheck disable=SC1090
set -a
source "$ADDRESSES_ENV"
set +a

DEPLOY_ADDR="$(terrad_host_key_address)" || exit 1
if [[ "$DEPLOY_ADDR" != "$TERRAD_HOST_EXPECTED_ADDR" ]]; then
  echo "ERROR: key $TERRAD_HOST_KEY address is $DEPLOY_ADDR" >&2
  echo "  expected $TERRAD_HOST_EXPECTED_ADDR" >&2
  exit 1
fi
echo "Deployer address OK: $DEPLOY_ADDR"

# Mintable allowlist only (F1/F4) — never QUARTZ/PEARL.
MINTABLE_SYMBOLS=(EMBER CORAL JADE ONYX RUBY TOPAZ)
ALLOWED_JSON="["
ALLOWED_HUMAN=()
first=1
for sym in "${MINTABLE_SYMBOLS[@]}"; do
  var="TOKEN_${sym}_ADDRESS"
  addr="${!var:-}"
  if [[ -z "$addr" ]]; then
    echo "ERROR: $var missing in $ADDRESSES_ENV" >&2
    exit 1
  fi
  ALLOWED_HUMAN+=("$sym=$addr")
  if [[ $first -eq 1 ]]; then
    first=0
  else
    ALLOWED_JSON+=","
  fi
  ALLOWED_JSON+="\"$addr\""
done
ALLOWED_JSON+="]"

echo "Allowlist (mintable only):"
for line in "${ALLOWED_HUMAN[@]}"; do
  echo "  $line"
done

# ── Store / instantiate ─────────────────────────────────────────────────
echo ""
echo "[1] Store faucet wasm"
if [[ -n "${FAUCET_CODE_ID:-}" && "${FORCE_STORE_FAUCET:-0}" != "1" ]]; then
  echo "  Reusing FAUCET_CODE_ID=$FAUCET_CODE_ID (set FORCE_STORE_FAUCET=1 to re-store)"
else
  FAUCET_CODE_ID="$(store_code "$FAUCET_WASM" "faucet")"
fi

INIT_MSG="$(jq -nc \
  --arg admin "$DEPLOY_ADDR" \
  --argjson tokens "$ALLOWED_JSON" \
  --arg drip "$FAUCET_DRIP_AMOUNT" \
  --arg cooldown "$FAUCET_COOLDOWN_SECONDS" \
  '{admin:$admin,allowed_tokens:$tokens,drip_amount:$drip,cooldown_seconds:($cooldown|tonumber)}')"

echo ""
echo "[2] Instantiate faucet"
echo "  init: $INIT_MSG"
FAUCET_ADDRESS="$(instantiate_no_funds "$FAUCET_CODE_ID" "$INIT_MSG" "cl8y-dex-faucet" "$DEPLOY_ADDR")"

# ── AddMinter grants (F5) — signed by primary minter cl8ydeploy (F6) ───
echo ""
echo "[3] AddMinter grants (primary minter = deploy key)"
ADD_MINTER_TXS=()
if [[ "${SKIP_ADD_MINTER:-0}" == "1" ]]; then
  echo "  SKIP_ADD_MINTER=1 — skipping grants"
else
  for sym in "${MINTABLE_SYMBOLS[@]}"; do
    var="TOKEN_${sym}_ADDRESS"
    token="${!var}"
    msg="$(jq -nc --arg m "$FAUCET_ADDRESS" '{add_minter:{minter:$m}}')"
    echo "  → AddMinter $sym ($token)"
    if [[ "${DRY_RUN:-0}" == "1" ]]; then
      echo "    DRY_RUN msg: $msg"
      ADD_MINTER_TXS+=("DRY_RUN:$sym")
    else
      tx="$(broadcast_and_wait "add_minter $sym" wasm execute "$token" "$msg")"
      ADD_MINTER_TXS+=("$sym:$tx")
      echo "    tx: $tx"
    fi
  done
fi

# ── Persist env + trace ─────────────────────────────────────────────────
echo ""
echo "[4] Writing addresses + frontend env"
TRACE="$OUT_DIR/faucet-trace.md"
if [[ "${DRY_RUN:-0}" == "1" ]]; then
  echo "  DRY_RUN=1 — skipping writes to addresses.env / frontend.env.example / faucet-trace.md"
else
  upsert_env_var "$ADDRESSES_ENV" "FAUCET_CODE_ID" "$FAUCET_CODE_ID"
  upsert_env_var "$ADDRESSES_ENV" "FAUCET_ADDRESS" "$FAUCET_ADDRESS"
  upsert_env_var "$ADDRESSES_ENV" "FAUCET_DRIP_AMOUNT" "$FAUCET_DRIP_AMOUNT"
  upsert_env_var "$ADDRESSES_ENV" "FAUCET_COOLDOWN_SECONDS" "$FAUCET_COOLDOWN_SECONDS"

  FRONTEND_ENV="$OUT_DIR/frontend.env.example"
  if [[ -f "$FRONTEND_ENV" ]]; then
    upsert_env_var "$FRONTEND_ENV" "VITE_FAUCET_ADDRESS" "$FAUCET_ADDRESS"
    for sym in "${MINTABLE_SYMBOLS[@]}"; do
      var="TOKEN_${sym}_ADDRESS"
      upsert_env_var "$FRONTEND_ENV" "VITE_TOKEN_${sym}_ADDRESS" "${!var}"
    done
  else
    {
      echo "# Coolify / static frontend build args (faucet fragment — GitLab #473)"
      echo "VITE_FAUCET_ADDRESS=$FAUCET_ADDRESS"
      for sym in "${MINTABLE_SYMBOLS[@]}"; do
        var="TOKEN_${sym}_ADDRESS"
        echo "VITE_TOKEN_${sym}_ADDRESS=${!var}"
      done
    } >"$FRONTEND_ENV"
  fi

  {
    echo "# Soft-launch faucet deploy trace (GitLab #473)"
    echo ""
    echo "- **When:** $(date -u +%Y-%m-%dT%H:%MZ)"
    echo "- **Chain:** $TERRAD_HOST_CHAIN_ID"
    echo "- **Deployer / faucet admin / primary CW20 minter:** \`$DEPLOY_ADDR\`"
    echo "- **FAUCET_CODE_ID:** \`$FAUCET_CODE_ID\`"
    echo "- **FAUCET_ADDRESS:** \`$FAUCET_ADDRESS\`"
    echo "- **drip_amount:** \`$FAUCET_DRIP_AMOUNT\` (100 human units @ 6 decimals)"
    echo "- **cooldown_seconds:** \`$FAUCET_COOLDOWN_SECONDS\` (global per wallet)"
    echo "- **Allowlist:** EMBER, CORAL, JADE, ONYX, RUBY, TOPAZ (not QUARTZ/PEARL)"
    echo "- **F6:** \`cl8ydeploy\` remains primary CW20 minter; no governance minter handoff"
    echo "- **F7:** Faucet code id is **not** on factory CW20 whitelist"
    echo ""
    echo "## AddMinter txs"
    echo ""
    if [[ ${#ADD_MINTER_TXS[@]} -eq 0 ]]; then
      echo "_Skipped (SKIP_ADD_MINTER=1)_"
    else
      for entry in "${ADD_MINTER_TXS[@]}"; do
        echo "- \`$entry\`"
      done
    fi
    echo ""
    echo "## Verification"
    echo ""
    echo '```bash'
    echo "# Faucet config"
    echo "terrad query wasm contract-state smart $FAUCET_ADDRESS '{\"config\":{}}' --node $TERRAD_HOST_NODE --output json"
    echo "# Sample Minters (EMBER)"
    echo "terrad query wasm contract-state smart \$TOKEN_EMBER_ADDRESS '{\"minters\":{}}' --node $TERRAD_HOST_NODE --output json"
    echo '```'
  } >"$TRACE"
fi

echo ""
echo "=============================================="
echo "Faucet deploy complete"
echo "  Code ID:  $FAUCET_CODE_ID"
echo "  Address:  $FAUCET_ADDRESS"
if [[ "${DRY_RUN:-0}" == "1" ]]; then
  echo "  Trace:    (skipped DRY_RUN)"
else
  echo "  Trace:    $TRACE"
fi
echo "=============================================="
echo "Next: set Coolify VITE_FAUCET_ADDRESS + VITE_TOKEN_*_ADDRESS; smoke Drip from a gas-funded wallet"
