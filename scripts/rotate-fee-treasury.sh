#!/usr/bin/env bash
# Point factory + every registered pair treasury at the ustr-cmm CMM.
#
# Stores factory 1.7.0 + pair 1.11.0, migrates both, UpdateConfig { treasury },
# then SetPairTreasuryAll (or Batch when PAIR_COUNT > 10).
#
# Usage:
#   DRY_RUN=1 ./scripts/rotate-fee-treasury.sh
#   ROTATE_TREASURY_LOCAL=1 ./scripts/rotate-fee-treasury.sh
#   ./scripts/rotate-fee-treasury.sh
#
# Optional:
#   ROTATE_TREASURY_SKIP_STORE=1  + FACTORY_CODE_ID / PAIR_CODE_ID
#   ROTATE_TREASURY_SKIP_MIGRATE=1
#   ROTATE_TREASURY_ADDRESS=terra16j5u6…   override target
#
# columbus-5 store (2026-08-15, cl8ydeploy — permissionless):
#   pair 11577  factory 11578
# Migrate / UpdateConfig / SetPairTreasury* must be signed by wasm admin
# terra1zlmv2… (2-of-3). cl8ydeploy will get "can not migrate: unauthorized".
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=lib/terrad-host.sh
source "$SCRIPT_DIR/lib/terrad-host.sh"
# shellcheck source=lib/terrad-tx-events.sh
source "$SCRIPT_DIR/lib/terrad-tx-events.sh"
# shellcheck source=lib/lcd-smart-query.sh
source "$SCRIPT_DIR/lib/lcd-smart-query.sh"
# shellcheck source=lib/ust1-wrap-ops-defaults.sh
source "$SCRIPT_DIR/lib/ust1-wrap-ops-defaults.sh"
# shellcheck source=lib/mainnet-soft-launch-defaults.sh
source "$SCRIPT_DIR/lib/mainnet-soft-launch-defaults.sh"

ARTIFACTS="${ROTATE_TREASURY_ARTIFACTS:-$REPO_ROOT/smartcontracts/artifacts}"
PAIR_WASM="${ROTATE_TREASURY_PAIR_WASM:-$ARTIFACTS/cl8y_dex_pair.wasm}"
FACTORY_WASM="${ROTATE_TREASURY_FACTORY_WASM:-$ARTIFACTS/cl8y_dex_factory.wasm}"
CMM="${ROTATE_TREASURY_ADDRESS:-${MAINNET_CMM_TREASURY:-$UST1_OPS_TREASURY}}"

if [[ "${ROTATE_TREASURY_LOCAL:-0}" == "1" ]]; then
  ENV_LOCAL="$REPO_ROOT/frontend-dapp/.env.local"
  if [[ -f "$ENV_LOCAL" ]]; then
    # shellcheck disable=SC1090
    set -a
    eval "$(grep -E '^(VITE_FACTORY_ADDRESS|VITE_LCD_URL)=' "$ENV_LOCAL" | sed 's/^VITE_FACTORY_ADDRESS=/FACTORY_ADDRESS=/; s/^VITE_LCD_URL=/LCD_URL=/')"
    set +a
  fi
  TERRAD_HOST_CHAIN_ID="${TERRAD_HOST_CHAIN_ID:-localterra}"
  TERRAD_HOST_NODE="${TERRAD_HOST_NODE:-http://127.0.0.1:26657}"
  LCD_URL="${LCD_URL:-http://127.0.0.1:1317}"
  TERRAD_HOST_KEY="${TERRAD_HOST_KEY:-test1}"
fi

FACTORY="${ROTATE_TREASURY_FACTORY_ADDRESS:-${FACTORY_ADDRESS:-$UST1_OPS_FACTORY}}"
LCD_URL="${LCD_URL:-${TERRA_LCD_URL:-https://terra-classic-lcd.publicnode.com}}"
LCD_URL="${LCD_URL%/}"

echo "=============================================="
echo "Rotate DEX fee treasury → CMM"
echo "=============================================="
echo "Factory:  $FACTORY"
echo "Treasury: $CMM"
echo "LCD:      $LCD_URL"
echo "DRY_RUN:  ${DRY_RUN:-0}"
echo "LOCAL:    ${ROTATE_TREASURY_LOCAL:-0}"
echo ""

die() { echo "ERROR: $*" >&2; exit 1; }

need_wasm() {
  local f="$1"
  [[ -f "$f" ]] || die "missing wasm: $f (make build-optimized, or set ROTATE_TREASURY_SKIP_STORE=1)"
}

broadcast_and_wait() {
  local label="$1"
  shift
  echo "  → $label" >&2
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "    DRY_RUN skip" >&2
    echo "dry-run"
    return 0
  fi
  local out tx_hash
  out="$(terrad_host_tx "$@")"
  tx_hash="$(printf '%s' "$out" | jq -r '.txhash // empty')"
  [[ -n "$tx_hash" ]] || die "no txhash from: $label"
  echo "    tx: $tx_hash" >&2
  terrad_host_wait_tx_inclusion "$tx_hash"
  printf '%s' "$tx_hash"
}

store_code() {
  local wasm="$1"
  local label="$2"
  local tx_hash code_id
  tx_hash="$(broadcast_and_wait "store $label" wasm store "$wasm")"
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "0"
    return 0
  fi
  code_id="$(terrad_host_code_id_from_store_tx "$tx_hash")"
  [[ -n "$code_id" ]] || die "could not parse code_id for $label"
  echo "    code_id: $code_id" >&2
  printf '%s' "$code_id"
}

query_json() {
  local contract="$1"
  local msg="$2"
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo '{}'
    return 0
  fi
  lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD_URL" "$contract" "$msg")"
}

echo "[1] preflight"
[[ -n "$FACTORY" ]] || die "set ROTATE_TREASURY_FACTORY_ADDRESS or FACTORY_ADDRESS"
[[ -n "$CMM" ]] || die "set ROTATE_TREASURY_ADDRESS"
if [[ "${ROTATE_TREASURY_SKIP_STORE:-0}" != "1" ]]; then
  need_wasm "$PAIR_WASM"
  need_wasm "$FACTORY_WASM"
fi

if [[ "${DRY_RUN:-0}" != "1" && "${ROTATE_TREASURY_SKIP_MIGRATE:-0}" != "1" ]]; then
  FACTORY_INFO="$(curl -sS -A 'cl8y-dex-ops/1.0' --max-time 15 \
    "$LCD_URL/cosmwasm/wasm/v1/contract/${FACTORY}")"
  WASM_ADMIN="$(printf '%s' "$FACTORY_INFO" | jq -r '.contract_info.admin // empty')"
  LIVE_CODE="$(printf '%s' "$FACTORY_INFO" | jq -r '.contract_info.code_id // empty')"
  echo "  factory wasm admin=$WASM_ADMIN code_id=$LIVE_CODE"
  terrad_host_resolve_keyring_backend
  SIGNER_ADDR=""
  if SIGNER_ADDR="$(terrad_host_exec keys show "$TERRAD_HOST_KEY" \
    --keyring-backend "$TERRAD_HOST_KEYRING_BACKEND" \
    --home "$TERRAD_HOST_HOME" \
    --address 2>/dev/null)"; then
    echo "  signer $TERRAD_HOST_KEY=$SIGNER_ADDR"
  fi
  if [[ -n "$WASM_ADMIN" && -n "$SIGNER_ADDR" && "$SIGNER_ADDR" != "$WASM_ADMIN" ]]; then
    die "signer is not factory wasm admin.
  migrate / SetPairTreasury must be signed by $WASM_ADMIN (2-of-3), not $SIGNER_ADDR.
  Store already landed (pair 11577, factory 11578). Resume with the multisig:

  TERRAD_HOST_KEY=<multisig-key-name> \\
    ROTATE_TREASURY_SKIP_STORE=1 \\
    ROTATE_TREASURY_PAIR_CODE_ID=11577 \\
    ROTATE_TREASURY_FACTORY_CODE_ID=11578 \\
    ./scripts/rotate-fee-treasury.sh

  If the key is a Cosmos multisig, use generate-only + 2-of-3 sign (see
  docs/runbooks/rotate-fee-treasury.md). Do not re-store wasm."
  fi
fi

echo ""
echo "[2] store wasm"
if [[ "${ROTATE_TREASURY_SKIP_STORE:-0}" == "1" ]]; then
  PAIR_CODE="${ROTATE_TREASURY_PAIR_CODE_ID:-}"
  FACTORY_CODE="${ROTATE_TREASURY_FACTORY_CODE_ID:-}"
  [[ -n "$PAIR_CODE" && -n "$FACTORY_CODE" ]] \
    || die "ROTATE_TREASURY_SKIP_STORE=1 needs ROTATE_TREASURY_PAIR_CODE_ID and ROTATE_TREASURY_FACTORY_CODE_ID"
  echo "  reuse pair=$PAIR_CODE factory=$FACTORY_CODE"
else
  PAIR_CODE="$(store_code "$PAIR_WASM" pair)"
  FACTORY_CODE="$(store_code "$FACTORY_WASM" factory)"
fi

echo ""
echo "[3] migrate factory → $FACTORY_CODE"
if [[ "${ROTATE_TREASURY_SKIP_MIGRATE:-0}" == "1" ]]; then
  echo "  skipped"
else
  broadcast_and_wait "migrate factory" wasm migrate "$FACTORY" "$FACTORY_CODE" '{}' >/dev/null
fi

echo ""
echo "[4] migrate pairs → $PAIR_CODE"
PAIRS_DOC="$(query_json "$FACTORY" '{"pairs":{"start_after":null,"limit":60}}')"
PAIR_ADDRS=()
if [[ "${DRY_RUN:-0}" != "1" ]]; then
  mapfile -t PAIR_ADDRS < <(printf '%s' "$PAIRS_DOC" | jq -r '.pairs[]?.contract_addr // empty')
fi
echo "  pairs: ${#PAIR_ADDRS[@]}"
if [[ "${ROTATE_TREASURY_SKIP_MIGRATE:-0}" != "1" ]]; then
  for pair in "${PAIR_ADDRS[@]+"${PAIR_ADDRS[@]}"}"; do
    broadcast_and_wait "migrate $pair" wasm migrate "$pair" "$PAIR_CODE" '{}' >/dev/null
  done
fi

echo ""
echo "[5] UpdateConfig treasury=$CMM"
UPDATE_MSG="$(jq -nc --arg t "$CMM" '{update_config:{treasury:$t}}')"
broadcast_and_wait "UpdateConfig treasury" wasm execute "$FACTORY" "$UPDATE_MSG" >/dev/null

echo ""
echo "[6] SetPairTreasury on all pairs"
COUNT="$(query_json "$FACTORY" '{"get_pair_count":{}}' | jq -r '.count // 0')"
if [[ "${DRY_RUN:-0}" == "1" ]]; then
  COUNT=0
fi
echo "  pair_count=$COUNT"
if [[ "$COUNT" -le 10 ]]; then
  ALL_MSG="$(jq -nc --arg t "$CMM" '{set_pair_treasury_all:{treasury:$t}}')"
  broadcast_and_wait "SetPairTreasuryAll" wasm execute "$FACTORY" "$ALL_MSG" >/dev/null
else
  START=""
  while true; do
    if [[ -z "$START" ]]; then
      BATCH_MSG="$(jq -nc --arg t "$CMM" '{set_pair_treasury_batch:{treasury:$t,start_after:null,limit:10}}')"
    else
      BATCH_MSG="$(jq -nc --arg t "$CMM" --argjson s "$START" \
        '{set_pair_treasury_batch:{treasury:$t,start_after:$s,limit:10}}')"
    fi
    broadcast_and_wait "SetPairTreasuryBatch start_after=${START:-null}" \
      wasm execute "$FACTORY" "$BATCH_MSG" >/dev/null
    if [[ "${DRY_RUN:-0}" == "1" || -z "$START" ]]; then
      # Exclusive cursor: first page scanned 0..9 → next start_after is 9.
      START=9
    else
      START=$((START + 10))
    fi
    if [[ "${DRY_RUN:-0}" == "1" ]]; then
      break
    fi
    [[ "$START" -ge $((COUNT - 1)) ]] && break
  done
fi

echo ""
echo "[7] verify"
if [[ "${DRY_RUN:-0}" == "1" ]]; then
  echo "  DRY_RUN — skipped live queries"
  echo "OK (dry-run)"
  exit 0
fi

FACTORY_TREASURY="$(query_json "$FACTORY" '{"config":{}}' | jq -r '.treasury // empty')"
echo "  factory.config.treasury=$FACTORY_TREASURY"
[[ "$FACTORY_TREASURY" == "$CMM" ]] || die "factory treasury mismatch: $FACTORY_TREASURY"

PAIRS_DOC="$(query_json "$FACTORY" '{"pairs":{"start_after":null,"limit":60}}')"
mapfile -t PAIR_ADDRS < <(printf '%s' "$PAIRS_DOC" | jq -r '.pairs[]?.contract_addr // empty')
for pair in "${PAIR_ADDRS[@]}"; do
  got="$(query_json "$pair" '{"get_fee_config":{}}' | jq -r '.fee_config.treasury // empty')"
  echo "  $pair treasury=$got"
  [[ "$got" == "$CMM" ]] || die "pair treasury mismatch: $pair → $got"
done

echo ""
echo "OK — factory and ${#PAIR_ADDRS[@]} pair(s) send commissions to $CMM"
