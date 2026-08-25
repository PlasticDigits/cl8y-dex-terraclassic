#!/usr/bin/env bash
# GitLab #635 — columbus-5 leftover of #633 (factory CreatePair autoregister).
#
# Does:
#   cl8ydeploy     → store factory wasm (optional token wasm)
#   DEX 2-of-3     → wasm migrate factory (cw2 1.9.0 → 1.10.0)
#   probe          → 11611 / 11619 / 11626 instances (CMM migrate)
#
# Does NOT:
#   migrate pairs (leave 11601 / 1.15.0)
#   factory-whitelist 8654 / launcher / AutoLP / unused 11612
#   UpdateConfig pair_code_id
#   CMM-execute via ustr-cmm when token admin is the CMM contract
#
# Keys (this machine's file keyring):
#   cl8ydeploy      terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv   store
#   multisig_2of3   terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7   factory admin
#   multisig1       terra13d6jycp9hv8u64t92j2htdr53sn9f88r4uqtxm   2-of-3 signer
#   multisig2       terra1lsewv7zjf2pe535lpdgh9dx2n928yn3ker76mq   2-of-3 signer
#   cl8y2_admin     terra1xsecn4snv94ezcez0z3vq8an9j4h4kxxcydp8l   CMM/UST1 gov
#   mywallet / multisig3 — unused
#
# Usage:
#   DRY_RUN=1 ./scripts/upgrade-635-autoregister.sh
#   ./scripts/upgrade-635-autoregister.sh
#   UPGRADE635_PROBE_ONLY=1 ./scripts/upgrade-635-autoregister.sh
#   UPGRADE635_SKIP_STORE=1 UPGRADE635_FACTORY_CODE_ID=<n> \
#     ./scripts/upgrade-635-autoregister.sh
#   UPGRADE635_STORE_TOKEN=1 ./scripts/upgrade-635-autoregister.sh
#   UPGRADE635_CMM_MIGRATE=1 UPGRADE635_TOKEN_CODE_ID=<n> \
#     ./scripts/upgrade-635-autoregister.sh
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
# shellcheck source=lib/upgrade-611-community-tax.sh
source "$SCRIPT_DIR/lib/upgrade-611-community-tax.sh"
# shellcheck source=lib/upgrade-635-autoregister.sh
source "$SCRIPT_DIR/lib/upgrade-635-autoregister.sh"

ARTIFACTS="${UPGRADE635_ARTIFACTS:-$REPO_ROOT/smartcontracts/artifacts}"
FACTORY_WASM="${UPGRADE635_FACTORY_WASM:-$ARTIFACTS/cl8y_dex_factory.wasm}"
TOKEN_WASM="${UPGRADE635_TOKEN_WASM:-$ARTIFACTS/cl8y_community_tax_token.wasm}"

OLD_FACTORY_CODE="${UPGRADE635_OLD_FACTORY_CODE_ID:-11602}"
OLD_TOKEN_CODES="${UPGRADE635_OLD_TOKEN_CODE_IDS:-11611 11619 11626}"
WANT_FACTORY_CW2="${UPGRADE635_WANT_FACTORY_CW2:-1.10.0}"

CMM="${UPGRADE635_CMM:-$UST1_OPS_TREASURY}"
DEX_GOV="${UPGRADE635_DEX_GOVERNANCE:-$UST1_OPS_DEX_GOVERNANCE}"
CMM_GOV="${UPGRADE635_CMM_GOVERNANCE:-${UST1_OPS_WRAP_WASM_ADMIN:-$UST1_OPS_WINDOW_GOVERNANCE}}"
FACTORY="${UPGRADE635_FACTORY_ADDRESS:-${FACTORY_ADDRESS:-$UST1_OPS_FACTORY}}"
LCD_URL="${LCD_URL:-${TERRA_LCD_URL:-$UST1_OPS_LCD_URL}}"
LCD_URL="${LCD_URL%/}"
LOCALTERRA_CURL_MAX_TIME="${LOCALTERRA_CURL_MAX_TIME:-25}"
LOCALTERRA_CURL_CONNECT_TIMEOUT="${LOCALTERRA_CURL_CONNECT_TIMEOUT:-5}"

if [[ "${UPGRADE635_LOCAL:-0}" != "1" ]]; then
  TERRAD_HOST_KEY="${TERRAD_HOST_KEY:-cl8ydeploy}"
fi

echo "=============================================="
echo "GitLab #635 factory + CMM token leftover"
echo "=============================================="
echo "Factory:     $FACTORY"
echo "CMM admin:   $CMM"
echo "CMM gov:     $CMM_GOV  (cl8y2_admin)"
echo "DEX gov:     $DEX_GOV  (multisig_2of3)"
echo "Store key:   $TERRAD_HOST_KEY"
echo "LCD:         $LCD_URL"
echo "Chain:       ${TERRAD_HOST_CHAIN_ID:-columbus-5}"
echo "DRY_RUN:     ${DRY_RUN:-0}"
echo "PROBE_ONLY:  ${UPGRADE635_PROBE_ONLY:-0}"
echo "STORE_TOKEN: ${UPGRADE635_STORE_TOKEN:-0}"
echo "CMM_MIGRATE: ${UPGRADE635_CMM_MIGRATE:-0}"
echo ""

need_wasm() {
  local f="$1"
  [[ -f "$f" ]] || upgrade635_die "missing wasm: $f (make build-optimized, or set UPGRADE635_SKIP_STORE=1)"
}

broadcast_and_wait() {
  local label="$1"
  shift
  echo "  → $label" >&2
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "    DRY_RUN skip: terrad tx $*" >&2
    echo "dry-run"
    return 0
  fi
  local out tx_hash
  out="$(terrad_host_tx "$@")"
  tx_hash="$(printf '%s' "$out" | jq -r '.txhash // empty')"
  [[ -n "$tx_hash" ]] || upgrade635_die "no txhash from: $label"
  echo "    tx: $tx_hash" >&2
  terrad_host_wait_tx_inclusion "$tx_hash"
  printf '%s' "$tx_hash"
}

store_code() {
  local wasm="$1"
  local label="$2"
  local tx_hash code_id
  if [[ -n "${UPGRADE635_STORE_EXTRA:-}" ]]; then
    # shellcheck disable=SC2086
    tx_hash="$(broadcast_and_wait "store $label" wasm store "$wasm" $UPGRADE635_STORE_EXTRA)"
  else
    tx_hash="$(broadcast_and_wait "store $label" wasm store "$wasm")"
  fi
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    case "$label" in
      factory) echo "${DRY_RUN_FACTORY_CODE_ID:-999102}" ;;
      token) echo "${DRY_RUN_TOKEN_CODE_ID:-999111}" ;;
      *) echo "${DRY_RUN_CODE_ID:-999001}" ;;
    esac
    return 0
  fi
  code_id="$(terrad_host_code_id_from_store_tx "$tx_hash")"
  [[ -n "$code_id" ]] || upgrade635_die "could not parse code_id for $label"
  echo "    code_id: $code_id" >&2
  printf '%s' "$code_id"
}

# Factory migrate: LocalTerra uses the store key; columbus-5 uses 2-of-3.
gov_tx() {
  local label="$1"
  shift
  echo "  → $label"
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "    DRY_RUN skip: $*"
    return 0
  fi
  if [[ "${UPGRADE635_LOCAL:-0}" == "1" ]]; then
    broadcast_and_wait "$label" "$@" >/dev/null
    return 0
  fi
  echo "    2-of-3 $(printf '%s ' "$@")" >&2
  "$SCRIPT_DIR/multisig-2of3-host-tx.sh" "$@"
}

print_leftover() {
  cat <<EOF

Leftover (not signed here unless UPGRADE635_CMM_MIGRATE=1 and admin is $CMM_GOV):
  CMM migrate — token ContractInfo.admin is usually $CMM (not cl8y2_admin).
  If admin is $CMM, use ustr-cmm WasmMsg::Migrate (CMM execute, signer cl8y2_admin).
  If admin is $CMM_GOV:
    TERRAD_HOST_KEY=cl8y2_admin terrad tx wasm migrate <token> ${TOKEN_CODE:-<new-token>} '{}' \\
      --chain-id columbus-5 --node https://terra-classic-rpc.publicnode.com:443 \\
      --gas auto --gas-adjustment 1.4 --gas-prices 28.325uluna

Do not factory-whitelist 8654 / 11612 / 11621 / 11622.
Do not migrate pairs. Do not UpdateConfig pair_code_id.
#589 REPORT GO is required before AddWhitelistedCodeId of a new token store.
Manage UI leftover stays on #635 (alert + highest-LP button).
EOF
}

echo "[1] preflight"
[[ -n "$FACTORY" ]] || upgrade635_die "set UPGRADE635_FACTORY_ADDRESS"
if [[ "${UPGRADE635_SKIP_STORE:-0}" != "1" && "${UPGRADE635_PROBE_ONLY:-0}" != "1" && "${DRY_RUN:-0}" != "1" ]]; then
  need_wasm "$FACTORY_WASM"
  if [[ "${UPGRADE635_STORE_TOKEN:-0}" == "1" ]]; then
    need_wasm "$TOKEN_WASM"
  fi
fi

if [[ "${DRY_RUN:-0}" != "1" ]]; then
  INFO="$(upgrade611_lcd_contract_info "$LCD_URL" "$FACTORY" || true)"
  ADMIN="$(printf '%s' "$INFO" | jq -r '.contract_info.admin // empty')"
  LIVE="$(printf '%s' "$INFO" | jq -r '.contract_info.code_id // empty')"
  CW2="$(upgrade635_cw2_version "$LCD_URL" "$FACTORY" || true)"
  echo "  factory admin=$ADMIN code_id=$LIVE cw2=${CW2:-<unreadable>}"
  [[ -z "$ADMIN" || "$ADMIN" == "$DEX_GOV" ]] \
    || upgrade635_die "factory admin is $ADMIN (expected DEX 2-of-3 $DEX_GOV)"
  [[ -z "$LIVE" || "$LIVE" == "$OLD_FACTORY_CODE" ]] \
    || echo "  WARN: factory already on code_id $LIVE (old pin $OLD_FACTORY_CODE)" >&2
fi

echo ""
echo "[2] probe live token instances (CMM migrate targets)"
TOKEN_ROWS=()
if [[ "${DRY_RUN:-0}" != "1" ]]; then
  for old in $OLD_TOKEN_CODES; do
    mapfile -t found < <(upgrade611_lcd_code_contracts "$LCD_URL" "$old")
    echo "  $old tokens: ${#found[@]}"
    for addr in "${found[@]+"${found[@]}"}"; do
      info="$(upgrade611_lcd_contract_info "$LCD_URL" "$addr" || true)"
      admin="$(printf '%s' "$info" | jq -r '.contract_info.admin // empty')"
      echo "    $addr  admin=$admin"
      TOKEN_ROWS+=("$old $addr $admin")
    done
  done
else
  echo "  DRY_RUN: skip LCD instance list"
fi

if [[ "${UPGRADE635_PROBE_ONLY:-0}" == "1" ]]; then
  print_leftover
  echo ""
  echo "OK (probe-only)"
  exit 0
fi

echo ""
echo "[3] store wasm (permissionless — $TERRAD_HOST_KEY)"
FACTORY_CODE=""
TOKEN_CODE="${UPGRADE635_TOKEN_CODE_ID:-}"
if [[ "${UPGRADE635_SKIP_STORE:-0}" == "1" ]]; then
  FACTORY_CODE="${UPGRADE635_FACTORY_CODE_ID:-}"
  [[ -n "$FACTORY_CODE" ]] || upgrade635_die "UPGRADE635_SKIP_STORE=1 needs UPGRADE635_FACTORY_CODE_ID"
  echo "  reuse factory=$FACTORY_CODE token=${TOKEN_CODE:-<none>}"
else
  FACTORY_CODE="$(store_code "$FACTORY_WASM" factory)"
  if [[ "${UPGRADE635_STORE_TOKEN:-0}" == "1" ]]; then
    TOKEN_CODE="$(store_code "$TOKEN_WASM" token)"
  fi
fi
if [[ "${DRY_RUN:-0}" != "1" ]]; then
  echo "  factory $FACTORY_CODE  hash=$(upgrade611_lcd_code_hash "$LCD_URL" "$FACTORY_CODE")"
  if [[ -n "$TOKEN_CODE" ]]; then
    echo "  token   $TOKEN_CODE  hash=$(upgrade611_lcd_code_hash "$LCD_URL" "$TOKEN_CODE")"
  fi
fi

if [[ "${UPGRADE635_STORE_ONLY:-0}" == "1" ]]; then
  echo ""
  echo "STORE_ONLY — stopping before factory migrate."
  print_leftover
  echo "OK (store-only) factory=$FACTORY_CODE token=${TOKEN_CODE:-<none>}"
  exit 0
fi

echo ""
echo "[4] migrate factory $FACTORY → $FACTORY_CODE (want cw2 $WANT_FACTORY_CW2)"
if [[ "${UPGRADE635_SKIP_FACTORY_MIGRATE:-0}" == "1" ]]; then
  echo "  skipped"
else
  if [[ "${DRY_RUN:-0}" != "1" && "$FACTORY_CODE" == "$OLD_FACTORY_CODE" ]]; then
    echo "  skip — already on $OLD_FACTORY_CODE"
  else
    gov_tx "migrate factory" wasm migrate "$FACTORY" "$FACTORY_CODE" '{}'
  fi
  if [[ "${DRY_RUN:-0}" != "1" ]]; then
    AFTER="$(upgrade611_lcd_contract_info "$LCD_URL" "$FACTORY" || true)"
    AFTER_CODE="$(printf '%s' "$AFTER" | jq -r '.contract_info.code_id // empty')"
    AFTER_CW2="$(upgrade635_cw2_version "$LCD_URL" "$FACTORY" || true)"
    echo "  factory code_id=$AFTER_CODE cw2=${AFTER_CW2:-<unreadable>}"
    [[ "$AFTER_CODE" == "$FACTORY_CODE" ]] \
      || upgrade635_die "factory code_id after migrate is $AFTER_CODE want $FACTORY_CODE"
    if [[ -n "$AFTER_CW2" && "$AFTER_CW2" != "$WANT_FACTORY_CW2" ]]; then
      echo "  WARN: factory cw2 $AFTER_CW2 (wanted $WANT_FACTORY_CW2)" >&2
    fi
  fi
fi

echo ""
echo "[5] CMM token migrate"
if [[ "${UPGRADE635_CMM_MIGRATE:-0}" != "1" ]]; then
  echo "  skipped (set UPGRADE635_CMM_MIGRATE=1 after a new #633 token store)"
elif [[ -z "$TOKEN_CODE" ]]; then
  upgrade635_die "UPGRADE635_CMM_MIGRATE=1 needs UPGRADE635_TOKEN_CODE_ID or UPGRADE635_STORE_TOKEN=1"
elif [[ "${#TOKEN_ROWS[@]}" -eq 0 ]]; then
  echo "  no 11611 / 11619 / 11626 instances — CMM migrate is a no-op"
else
  upgrade635_assert_whitelist_ok "$TOKEN_CODE"
  saved_key="$TERRAD_HOST_KEY"
  TERRAD_HOST_KEY="${UPGRADE635_CMM_KEY:-cl8y2_admin}"
  for row in "${TOKEN_ROWS[@]}"; do
    # row: "<old_code> <addr> <admin>"
    old="${row%% *}"
    rest="${row#* }"
    addr="${rest%% *}"
    admin="${rest#* }"
    echo "  $old $addr admin=$admin"
    if [[ "$admin" == "$CMM_GOV" ]]; then
      broadcast_and_wait "migrate $addr" wasm migrate "$addr" "$TOKEN_CODE" '{}' >/dev/null
    elif [[ "$admin" == "$CMM" ]]; then
      echo "    leftover — admin is CMM contract; use ustr-cmm WasmMsg::Migrate (signer $TERRAD_HOST_KEY)"
    else
      echo "    leftover — unexpected admin $admin (not CMM / cl8y2_admin)"
    fi
  done
  TERRAD_HOST_KEY="$saved_key"
fi

print_leftover

if [[ "${DRY_RUN:-0}" == "1" ]]; then
  echo ""
  echo "OK (dry-run) factory=$FACTORY_CODE token=${TOKEN_CODE:-<none>}"
  exit 0
fi

echo ""
echo "OK — factory=$FACTORY_CODE token=${TOKEN_CODE:-<none>}"
