#!/usr/bin/env bash
# GitLab #514 — migrate fee-discount (limit_discount_bps backfill) + pairs
# (placement uses the limit discount; swap/take stays on discount_bps).
#
# Usage:
#   DRY_RUN=1 ./scripts/upgrade-514-limit-discount.sh
#   UPGRADE514_LOCAL=1 ./scripts/upgrade-514-limit-discount.sh
#   ./scripts/upgrade-514-limit-discount.sh
#
# Optional:
#   UPGRADE514_SKIP_STORE=1  + FEE_DISCOUNT_CODE_ID / PAIR_CODE_ID
#   UPGRADE514_SKIP_MIGRATE=1
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=lib/terrad-host.sh
source "$SCRIPT_DIR/lib/terrad-host.sh"
# shellcheck source=lib/terrad-tx-events.sh
source "$SCRIPT_DIR/lib/terrad-tx-events.sh"
# shellcheck source=lib/lcd-smart-query.sh
source "$SCRIPT_DIR/lib/lcd-smart-query.sh"

ARTIFACTS="${UPGRADE514_ARTIFACTS:-$REPO_ROOT/smartcontracts/artifacts}"
PAIR_WASM="${UPGRADE514_PAIR_WASM:-$ARTIFACTS/cl8y_dex_pair.wasm}"
FD_WASM="${UPGRADE514_FEE_DISCOUNT_WASM:-$ARTIFACTS/cl8y_dex_fee_discount.wasm}"

if [[ "${UPGRADE514_LOCAL:-0}" == "1" ]]; then
  ENV_LOCAL="$REPO_ROOT/frontend-dapp/.env.local"
  if [[ -f "$ENV_LOCAL" ]]; then
    # shellcheck disable=SC1090
    set -a
    eval "$(grep -E '^(VITE_FACTORY_ADDRESS|VITE_FEE_DISCOUNT_ADDRESS|VITE_LCD_URL)=' "$ENV_LOCAL" | sed 's/^VITE_FACTORY_ADDRESS=/FACTORY_ADDRESS=/; s/^VITE_FEE_DISCOUNT_ADDRESS=/FEE_DISCOUNT_ADDRESS=/; s/^VITE_LCD_URL=/LCD_URL=/')"
    set +a
  fi
  TERRAD_HOST_CHAIN_ID="${TERRAD_HOST_CHAIN_ID:-localterra}"
  TERRAD_HOST_NODE="${TERRAD_HOST_NODE:-http://127.0.0.1:26657}"
  LCD_URL="${LCD_URL:-http://127.0.0.1:1317}"
  TERRAD_HOST_KEY="${TERRAD_HOST_KEY:-test1}"
fi

FACTORY="${UPGRADE514_FACTORY_ADDRESS:-${FACTORY_ADDRESS:-}}"
FEE_DISCOUNT="${UPGRADE514_FEE_DISCOUNT_ADDRESS:-${FEE_DISCOUNT_ADDRESS:-}}"
LCD_URL="${LCD_URL:-${TERRA_LCD_URL:-https://terra-classic-lcd.publicnode.com}}"
LCD_URL="${LCD_URL%/}"

echo "=============================================="
echo "GitLab #514 limit-discount upgrade"
echo "=============================================="
echo "Factory:      $FACTORY"
echo "Fee-discount: $FEE_DISCOUNT"
echo "LCD:          $LCD_URL"
echo "DRY_RUN:      ${DRY_RUN:-0}"
echo "LOCAL:        ${UPGRADE514_LOCAL:-0}"
echo ""

die() { echo "ERROR: $*" >&2; exit 1; }

need_wasm() {
  local f="$1"
  [[ -f "$f" ]] || die "missing wasm: $f (make build-optimized, or set UPGRADE514_SKIP_STORE=1)"
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
[[ -n "$FACTORY" ]] || die "set UPGRADE514_FACTORY_ADDRESS or FACTORY_ADDRESS"
[[ -n "$FEE_DISCOUNT" ]] || die "set UPGRADE514_FEE_DISCOUNT_ADDRESS or FEE_DISCOUNT_ADDRESS"
if [[ "${UPGRADE514_SKIP_STORE:-0}" != "1" ]]; then
  need_wasm "$PAIR_WASM"
  need_wasm "$FD_WASM"
fi

echo ""
echo "[2] store wasm"
if [[ "${UPGRADE514_SKIP_STORE:-0}" == "1" ]]; then
  PAIR_CODE="${UPGRADE514_PAIR_CODE_ID:-}"
  FD_CODE="${UPGRADE514_FEE_DISCOUNT_CODE_ID:-}"
  [[ -n "$PAIR_CODE" && -n "$FD_CODE" ]] \
    || die "UPGRADE514_SKIP_STORE=1 needs UPGRADE514_PAIR_CODE_ID and UPGRADE514_FEE_DISCOUNT_CODE_ID"
  echo "  reuse pair=$PAIR_CODE fee-discount=$FD_CODE"
else
  PAIR_CODE="$(store_code "$PAIR_WASM" pair)"
  FD_CODE="$(store_code "$FD_WASM" fee-discount)"
fi

echo ""
echo "[3] migrate fee-discount → $FD_CODE (backfill standard limit_discount_bps)"
if [[ "${UPGRADE514_SKIP_MIGRATE:-0}" == "1" ]]; then
  echo "  skipped"
else
  broadcast_and_wait "migrate fee-discount" wasm migrate "$FEE_DISCOUNT" "$FD_CODE" '{}' >/dev/null
fi

echo ""
echo "[4] migrate pairs → $PAIR_CODE (placement reads limit_discount_bps)"
PAIRS_DOC="$(query_json "$FACTORY" '{"pairs":{"start_after":null,"limit":60}}')"
PAIR_ADDRS=()
if [[ "${DRY_RUN:-0}" != "1" ]]; then
  mapfile -t PAIR_ADDRS < <(printf '%s' "$PAIRS_DOC" | jq -r '.pairs[]?.contract_addr // empty')
fi
echo "  pairs: ${#PAIR_ADDRS[@]}"
if [[ "${UPGRADE514_SKIP_MIGRATE:-0}" != "1" ]]; then
  for pair in "${PAIR_ADDRS[@]+"${PAIR_ADDRS[@]}"}"; do
    broadcast_and_wait "migrate $pair" wasm migrate "$pair" "$PAIR_CODE" '{}' >/dev/null
  done
fi

echo ""
echo "Done. Confirm GetTiers limit_discount_bps (tier 9 → 10000) and a T9 place is 0 maker fee."
echo "Invariant I13: docs/reference/fee-discount-tiers.md"
