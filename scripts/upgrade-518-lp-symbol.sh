#!/usr/bin/env bash
# GitLab #518 — upgrade factory so create_pair keeps digits in LP tickers.
#
# Stores new factory + pair wasm, migrates factory, then UpdateConfig
# pair_code_id + lp_token_code_id (digit-allowing cw20-mintable).
#
# Usage:
#   DRY_RUN=1 ./scripts/upgrade-518-lp-symbol.sh
#   # LocalTerra (reads frontend-dapp/.env.local):
#   UPGRADE518_LOCAL=1 ./scripts/upgrade-518-lp-symbol.sh
#   # columbus-5 (governance / wasm-admin key):
#   ./scripts/upgrade-518-lp-symbol.sh
#
# Optional:
#   UPGRADE518_LP_CODE_ID=10184   reuse on-chain mintable (skip LP store)
#   UPGRADE518_SKIP_STORE=1       use UPGRADE518_PAIR_CODE_ID / FACTORY_CODE_ID / LP_CODE_ID
#   UPGRADE518_SKIP_FACTORY_MIGRATE=1   factory already on 1.6.0+ (UpdateConfig has code ids)
#
# Existing pairs keep their LP tokens. New create_pair uses the new codes.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=lib/terrad-host.sh
source "$SCRIPT_DIR/lib/terrad-host.sh"
# shellcheck source=lib/terrad-tx-events.sh
source "$SCRIPT_DIR/lib/terrad-tx-events.sh"
# shellcheck source=lib/lcd-smart-query.sh
source "$SCRIPT_DIR/lib/lcd-smart-query.sh"
# shellcheck source=lib/ust1-secondary-pair-defaults.sh
source "$SCRIPT_DIR/lib/ust1-secondary-pair-defaults.sh"

ARTIFACTS="${UPGRADE518_ARTIFACTS:-$REPO_ROOT/smartcontracts/artifacts}"
PAIR_WASM="${UPGRADE518_PAIR_WASM:-$ARTIFACTS/cl8y_dex_pair.wasm}"
FACTORY_WASM="${UPGRADE518_FACTORY_WASM:-$ARTIFACTS/cl8y_dex_factory.wasm}"
LP_WASM="${UPGRADE518_LP_WASM:-$ARTIFACTS/cw20_mintable.wasm}"

if [[ "${UPGRADE518_LOCAL:-0}" == "1" ]]; then
  ENV_LOCAL="$REPO_ROOT/frontend-dapp/.env.local"
  if [[ -f "$ENV_LOCAL" ]]; then
    # shellcheck disable=SC1090
    set -a
    # grep keeps this from sourcing unrelated Vite keys with spaces
    eval "$(grep -E '^(VITE_FACTORY_ADDRESS|VITE_LCD_URL)=' "$ENV_LOCAL" | sed 's/^VITE_FACTORY_ADDRESS=/FACTORY_ADDRESS=/; s/^VITE_LCD_URL=/LCD_URL=/')"
    set +a
  fi
  TERRAD_HOST_CHAIN_ID="${TERRAD_HOST_CHAIN_ID:-localterra}"
  TERRAD_HOST_NODE="${TERRAD_HOST_NODE:-http://127.0.0.1:26657}"
  LCD_URL="${LCD_URL:-http://127.0.0.1:1317}"
  TERRAD_HOST_KEY="${TERRAD_HOST_KEY:-test1}"
fi

FACTORY="${UPGRADE518_FACTORY_ADDRESS:-${FACTORY_ADDRESS:-$UST1_SEC_FACTORY_ADDRESS}}"
LCD_URL="${LCD_URL:-${TERRA_LCD_URL:-https://terra-classic-lcd.publicnode.com}}"
LCD_URL="${LCD_URL%/}"
REUSE_LP_CODE="${UPGRADE518_LP_CODE_ID:-}"

echo "=============================================="
echo "GitLab #518 LP ticker upgrade"
echo "=============================================="
echo "Factory:  $FACTORY"
echo "LCD:      $LCD_URL"
echo "DRY_RUN:  ${DRY_RUN:-0}"
echo "LOCAL:    ${UPGRADE518_LOCAL:-0}"
echo ""

die() { echo "ERROR: $*" >&2; exit 1; }

need_wasm() {
  local f="$1"
  [[ -f "$f" ]] || die "missing wasm: $f (make build-optimized, or set UPGRADE518_SKIP_STORE=1)"
}

broadcast_and_wait() {
  local label="$1"
  shift
  echo "  → $label" >&2
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
  if [[ "${UPGRADE518_SKIP_STORE:-0}" == "1" ]]; then
    die "UPGRADE518_SKIP_STORE=1 but store_code called for $label"
  fi
  local tx_hash code_id
  tx_hash="$(broadcast_and_wait "store $label" wasm store "$wasm")"
  code_id="$(terrad_host_code_id_from_store_tx "$tx_hash")"
  [[ -n "$code_id" ]] || die "could not parse code_id for $label"
  echo "    code_id: $code_id" >&2
  printf '%s' "$code_id"
}

query_factory_config() {
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo '{"data":{"pair_code_id":1,"lp_token_code_id":2}}'
    return 0
  fi
  lcd_smart_query_raw "$LCD_URL" "$FACTORY" '{"config":{}}'
}

cfg_field() {
  local json="$1"
  local key="$2"
  printf '%s' "$json" | jq -r --arg k "$key" '
    (.data // .) | if type=="object" then .[$k] else empty end
  '
}

echo "[1] preflight"
[[ -n "$FACTORY" ]] || die "set UPGRADE518_FACTORY_ADDRESS or FACTORY_ADDRESS"
if [[ "${UPGRADE518_SKIP_STORE:-0}" != "1" ]]; then
  need_wasm "$PAIR_WASM"
  need_wasm "$FACTORY_WASM"
  if [[ -z "$REUSE_LP_CODE" ]]; then
    need_wasm "$LP_WASM"
  fi
fi

CFG_JSON="$(query_factory_config)"
OLD_PAIR="$(cfg_field "$CFG_JSON" pair_code_id)"
OLD_LP="$(cfg_field "$CFG_JSON" lp_token_code_id)"
echo "  current pair_code_id=$OLD_PAIR lp_token_code_id=$OLD_LP"

echo ""
echo "[2] store wasm"
if [[ "${UPGRADE518_SKIP_STORE:-0}" == "1" ]]; then
  PAIR_CODE="${UPGRADE518_PAIR_CODE_ID:-}"
  FACTORY_CODE="${UPGRADE518_FACTORY_CODE_ID:-}"
  LP_CODE="${REUSE_LP_CODE:-${UPGRADE518_LP_CODE_ID:-}}"
  [[ -n "$PAIR_CODE" && -n "$FACTORY_CODE" && -n "$LP_CODE" ]] \
    || die "UPGRADE518_SKIP_STORE=1 needs UPGRADE518_PAIR_CODE_ID, UPGRADE518_FACTORY_CODE_ID, UPGRADE518_LP_CODE_ID"
  echo "  reuse pair=$PAIR_CODE factory=$FACTORY_CODE lp=$LP_CODE"
else
  PAIR_CODE="$(store_code "$PAIR_WASM" pair)"
  FACTORY_CODE="$(store_code "$FACTORY_WASM" factory)"
  if [[ -n "$REUSE_LP_CODE" ]]; then
    LP_CODE="$REUSE_LP_CODE"
    echo "  reuse lp_token_code_id=$LP_CODE"
  else
    LP_CODE="$(store_code "$LP_WASM" cw20_mintable)"
  fi
fi

echo ""
echo "[3] migrate factory → $FACTORY_CODE"
if [[ "${UPGRADE518_SKIP_FACTORY_MIGRATE:-0}" == "1" ]]; then
  echo "  skipped (UPGRADE518_SKIP_FACTORY_MIGRATE=1)"
else
  MIGRATE_TX="$(broadcast_and_wait "migrate factory" wasm migrate "$FACTORY" "$FACTORY_CODE" '{}')"
  echo "  migrate tx: $MIGRATE_TX"
fi

echo ""
echo "[4] UpdateConfig pair_code_id=$PAIR_CODE lp_token_code_id=$LP_CODE"
UPDATE_MSG="$(jq -nc --argjson pair "$PAIR_CODE" --argjson lp "$LP_CODE" \
  '{update_config:{pair_code_id:$pair,lp_token_code_id:$lp}}')"
UPDATE_TX="$(broadcast_and_wait "UpdateConfig code ids" wasm execute "$FACTORY" "$UPDATE_MSG")"
echo "  update tx: $UPDATE_TX"

echo ""
echo "[5] verify"
NEW_JSON="$(query_factory_config)"
NEW_PAIR="$(cfg_field "$NEW_JSON" pair_code_id)"
NEW_LP="$(cfg_field "$NEW_JSON" lp_token_code_id)"
echo "  pair_code_id:     $OLD_PAIR → $NEW_PAIR"
echo "  lp_token_code_id: $OLD_LP → $NEW_LP"
if [[ "${DRY_RUN:-0}" != "1" ]]; then
  [[ "$NEW_PAIR" == "$PAIR_CODE" ]] || die "pair_code_id mismatch: got $NEW_PAIR want $PAIR_CODE"
  [[ "$NEW_LP" == "$LP_CODE" ]] || die "lp_token_code_id mismatch: got $NEW_LP want $LP_CODE"
fi

echo ""
echo "OK — new create_pair uses digit-allowing LP tickers (UST1-CUST-LP, CL8Y-CLUN-LP)."
echo "Existing pairs were not migrated (instantiate-only). Re-simulate UST1/CL8Y create_pair."
