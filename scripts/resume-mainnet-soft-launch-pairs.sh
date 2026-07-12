#!/usr/bin/env bash
# Resume soft-launch pair creation + LP after an interrupt.
# Skips pairs that already have pool liquidity (total_share > 0).
# Completes discount-registry-all + governance handoff at the end.
#
# Usage:
#   source-ish env from deployments/mainnet-soft-launch/addresses.env (auto-loaded)
#   read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
#   ./scripts/resume-mainnet-soft-launch-pairs.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=lib/mainnet-soft-launch-defaults.sh
source "$SCRIPT_DIR/lib/mainnet-soft-launch-defaults.sh"
# shellcheck source=lib/terrad-host.sh
source "$SCRIPT_DIR/lib/terrad-host.sh"
# shellcheck source=lib/terrad-tx-events.sh
source "$SCRIPT_DIR/lib/terrad-tx-events.sh"

ENV_FILE="${MAINNET_SOFT_LAUNCH_ENV:-$REPO_ROOT/deployments/mainnet-soft-launch/addresses.env}"
[[ -f "$ENV_FILE" ]] || {
  echo "ERROR: missing $ENV_FILE" >&2
  exit 1
}
# shellcheck disable=SC1090
set -a
# shellcheck source=/dev/null
source "$ENV_FILE"
set +a

TERRAD_HOST_KEY="$MAINNET_SOFT_LAUNCH_DEPLOY_KEY"
TERRAD_HOST_EXPECTED_ADDR="$MAINNET_SOFT_LAUNCH_DEPLOY_ADDR"
TERRAD_HOST_CHAIN_ID="${TERRAD_HOST_CHAIN_ID:-columbus-5}"
TERRAD_HOST_NODE="${TERRAD_HOST_NODE:-https://terra-classic-rpc.publicnode.com:443}"

FACTORY_ADDRESS="${FACTORY_ADDRESS:?}"
FEE_DISCOUNT_ADDRESS="${FEE_DISCOUNT_ADDRESS:?}"
FINAL_GOVERNANCE="${MAINNET_SOFT_LAUNCH_GOVERNANCE}"
PAIR_CREATION_FEE="${MAINNET_SOFT_LAUNCH_PAIR_CREATION_FEE_ULUNA}uluna"

declare -A TOKEN_ADDR_BY_SYM=(
  [EMBER]="${TOKEN_EMBER_ADDRESS:?}"
  [CORAL]="${TOKEN_CORAL_ADDRESS:?}"
  [JADE]="${TOKEN_JADE_ADDRESS:?}"
  [ONYX]="${TOKEN_ONYX_ADDRESS:?}"
  [RUBY]="${TOKEN_RUBY_ADDRESS:?}"
  [TOPAZ]="${TOKEN_TOPAZ_ADDRESS:?}"
  [QUARTZ]="${TOKEN_QUARTZ_ADDRESS:?}"
  [PEARL]="${TOKEN_PEARL_ADDRESS:?}"
)

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

execute_msg() {
  local contract="$1"
  local msg="$2"
  local label="$3"
  broadcast_and_wait "$label" wasm execute "$contract" "$msg" >/dev/null
}

pair_env_key() {
  echo "PAIR_$(echo "$1" | tr '/' '_' | tr '[:lower:]' '[:upper:]')_ADDRESS"
}

pair_pool_share() {
  local pair="$1"
  local q
  q="$(jq -nc '{pool:{}}')"
  DRY_RUN_ALLOW_QUERY=1 terrad_host_query wasm contract-state smart "$pair" "$q" \
    | jq -r '.data.total_share // .total_share // "0"'
}

resolve_pair_addr() {
  local sym_a="$1" sym_b="$2" a="$3" b="$4"
  local env_key env_val q out
  env_key="$(pair_env_key "$sym_a/$sym_b")"
  env_val="${!env_key:-}"
  if [[ -n "$env_val" && "$env_val" != terra1dryrun* ]]; then
    printf '%s' "$env_val"
    return 0
  fi
  q="$(jq -nc --arg a "$a" --arg b "$b" \
    '{pair: {asset_infos: [{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')"
  # Missing pairs return "pair not found" — treat as empty, don't spam the console.
  set +e
  out="$(DRY_RUN_ALLOW_QUERY=1 terrad_host_query wasm contract-state smart "$FACTORY_ADDRESS" "$q" 2>/dev/null)"
  set -e
  printf '%s' "$out" | jq -r '.data.pair.contract_addr // .data.contract_addr // .contract_addr // empty'
}

echo "=============================================="
echo "Resume soft-launch pairs"
echo "=============================================="
echo "Factory:      $FACTORY_ADDRESS"
echo "Fee discount: $FEE_DISCOUNT_ADDRESS"
echo "Env:          $ENV_FILE"
echo ""

DEPLOY_ADDR="$(terrad_host_key_address)" || exit 1
[[ "$DEPLOY_ADDR" == "$MAINNET_SOFT_LAUNCH_DEPLOY_ADDR" ]] || {
  echo "ERROR: unexpected deployer $DEPLOY_ADDR" >&2
  exit 1
}

PAIR_ADDRESSES=()

for pair_entry in "${MAINNET_SOFT_LAUNCH_PAIRS[@]}"; do
  IFS='|' read -r SYM_A SYM_B AMT_A AMT_B <<<"$pair_entry"
  ADDR_A="${TOKEN_ADDR_BY_SYM[$SYM_A]}"
  ADDR_B="${TOKEN_ADDR_BY_SYM[$SYM_B]}"

  echo "  Pair $SYM_A/$SYM_B"
  PAIR_ADDR="$(resolve_pair_addr "$SYM_A" "$SYM_B" "$ADDR_A" "$ADDR_B" || true)"
  if [[ -n "$PAIR_ADDR" ]]; then
    SHARE="$(pair_pool_share "$PAIR_ADDR")"
    if [[ "$SHARE" != "0" && -n "$SHARE" ]]; then
      echo "    skip (already has liquidity total_share=$SHARE) pair=$PAIR_ADDR"
      PAIR_ADDRESSES+=("$SYM_A/$SYM_B=$PAIR_ADDR")
      continue
    fi
    echo "    existing pair (empty pool): $PAIR_ADDR"
  else
    CREATE_MSG="$(jq -nc \
      --arg a "$ADDR_A" --arg b "$ADDR_B" \
      '{create_pair: {asset_infos: [
        {token: {contract_addr: $a}},
        {token: {contract_addr: $b}}
      ]}}')"
    set +e
    out="$(terrad_host_tx wasm execute "$FACTORY_ADDRESS" "$CREATE_MSG" --amount "$PAIR_CREATION_FEE")"
    st=$?
    set -e
    tx_hash="$(printf '%s' "$out" | jq -r '.txhash // empty')"
    if [[ -n "$tx_hash" ]]; then
      echo "  → create_pair $SYM_A/$SYM_B"
      echo "    tx: $tx_hash"
      terrad_host_wait_tx_inclusion "$tx_hash"
      TX_JSON="$(terrad_host_wait_tx_query "$tx_hash")"
      PAIR_ADDR="$(printf '%s' "$TX_JSON" | terrad_jq_contract_address_from_tx_json | head -1)"
    fi
    if [[ -z "$PAIR_ADDR" ]]; then
      # Already exists or broadcast failed — resolve from factory.
      PAIR_ADDR="$(resolve_pair_addr "$SYM_A" "$SYM_B" "$ADDR_A" "$ADDR_B" || true)"
    fi
    if [[ -z "$PAIR_ADDR" ]]; then
      echo "ERROR: no pair addr for $SYM_A/$SYM_B (create st=$st)" >&2
      printf '%s\n' "$out" >&2
      exit 1
    fi
    echo "    pair: $PAIR_ADDR"
    REG_MSG="$(jq -nc --arg pair "$PAIR_ADDR" --arg reg "$FEE_DISCOUNT_ADDRESS" \
      '{set_discount_registry: {pair: $pair, registry: $reg}}')"
    execute_msg "$FACTORY_ADDRESS" "$REG_MSG" "set_discount_registry $SYM_A/$SYM_B"
  fi

  PAIR_ADDRESSES+=("$SYM_A/$SYM_B=$PAIR_ADDR")

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

echo ""
echo "[7] set_discount_registry_all"
ALL_MSG="$(jq -nc --arg r "$FEE_DISCOUNT_ADDRESS" '{set_discount_registry_all: {registry: $r}}')"
execute_msg "$FACTORY_ADDRESS" "$ALL_MSG" "set_discount_registry_all"

echo ""
echo "[7b] Handoff config governance → multisig"
FEE_HANDOFF="$(jq -nc --arg gov "$FINAL_GOVERNANCE" '{update_config: {governance: $gov}}')"
execute_msg "$FEE_DISCOUNT_ADDRESS" "$FEE_HANDOFF" "fee-discount update_config governance"
FACTORY_HANDOFF="$(jq -nc --arg gov "$FINAL_GOVERNANCE" '{update_config: {governance: $gov}}')"
execute_msg "$FACTORY_ADDRESS" "$FACTORY_HANDOFF" "factory update_config governance"

OUT_DIR="$(dirname "$ENV_FILE")"
{
  echo "# Updated by resume-mainnet-soft-launch-pairs.sh — $(date -u +%Y-%m-%dT%H:%MZ)"
  grep -E '^(NETWORK|CHAIN_ID|DEPLOYER_|GOVERNANCE_|TREASURY_|CW20_|FACTORY_CODE|PAIR_CODE|ROUTER_CODE|FEE_DISCOUNT_CODE|FACTORY_ADDRESS|ROUTER_ADDRESS|FEE_DISCOUNT_ADDRESS|CL8Y_|TOKEN_)' "$ENV_FILE" || true
  for row in "${PAIR_ADDRESSES[@]}"; do
    sym="${row%%=*}"
    addr="${row#*=}"
    key="PAIR_$(echo "$sym" | tr '/' '_' | tr '[:lower:]' '[:upper:]')_ADDRESS"
    echo "$key=$addr"
  done
} >"$OUT_DIR/addresses.env.tmp"
mv "$OUT_DIR/addresses.env.tmp" "$OUT_DIR/addresses.env"

echo "Done. Updated $OUT_DIR/addresses.env"
echo "unset TERRAD_HOST_KEYRING_PASS when finished."
