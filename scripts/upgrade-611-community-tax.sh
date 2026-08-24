#!/usr/bin/env bash
# GitLab #611 / #612 / #616 — columbus-5 community-tax rotate.
#
# One store from current main (must include !409 option-2). Then:
#   cl8ydeploy  → store token + launcher + AutoLP
#   DEX 2-of-3  → migrate canonical launcher 11614, AddWhitelistedCodeId (new token only)
# Does NOT CMM-migrate token / AutoLP instances (admin is CMM, not the DEX multisig).
# Does NOT whitelist launcher / AutoLP / 11612 / ALPHA 8654.
# After this crate's UpdateConfig is stored + the instance is migrated, set
# UPGRADE611_UPDATE_CONFIG=1 to rotate token_code_id / autolp_code_id.
#
# Usage:
#   DRY_RUN=1 ./scripts/upgrade-611-community-tax.sh
#   ./scripts/upgrade-611-community-tax.sh                  # store + launcher migrate
#   UPGRADE611_589_GO=1 UPGRADE611_SKIP_STORE=1 \
#     UPGRADE611_TOKEN_CODE_ID=<n> UPGRADE611_LAUNCHER_CODE_ID=<n> \
#     UPGRADE611_AUTOLP_CODE_ID=<n> \
#     ./scripts/upgrade-611-community-tax.sh               # whitelist after #589 REPORT GO
#   UPGRADE611_STORE_LAUNCHER_ONLY=1 UPGRADE611_SKIP_WHITELIST=1 \
#     UPGRADE611_UPDATE_CONFIG=1 \
#     UPGRADE611_TOKEN_CODE_ID=11619 UPGRADE611_AUTOLP_CODE_ID=11621 \
#     ./scripts/upgrade-611-community-tax.sh               # store+migrate launcher, then UpdateConfig
#   UPGRADE611_LOCAL=1 ./scripts/upgrade-611-community-tax.sh
#
# Optional:
#   UPGRADE611_SKIP_STORE=1 + TOKEN / LAUNCHER / AUTOLP code ids
#   UPGRADE611_STORE_LAUNCHER_ONLY=1  store new launcher; reuse TOKEN / AUTOLP env ids
#   UPGRADE611_SKIP_LAUNCHER_MIGRATE=1
#   UPGRADE611_SKIP_WHITELIST=1
#   UPGRADE611_589_GO=1          required to factory-list the new token
#   UPGRADE611_UPDATE_CONFIG=1   wasm-admin UpdateConfig after launcher migrate
#   UPGRADE611_REFRESH=1         RefreshPairAssetCodeIdsBatch after CMM migrate
#   UPGRADE611_STORE_ONLY=1      stop after store (print resume)
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

ARTIFACTS="${UPGRADE611_ARTIFACTS:-$REPO_ROOT/smartcontracts/artifacts}"
TOKEN_WASM="${UPGRADE611_TOKEN_WASM:-$ARTIFACTS/cl8y_community_tax_token.wasm}"
LAUNCHER_WASM="${UPGRADE611_LAUNCHER_WASM:-$ARTIFACTS/cl8y_community_token_launcher.wasm}"
AUTOLP_WASM="${UPGRADE611_AUTOLP_WASM:-$ARTIFACTS/cl8y_community_tax_autolp.wasm}"

OLD_TOKEN_CODE="${UPGRADE611_OLD_TOKEN_CODE_ID:-11611}"
OLD_LAUNCHER_CODE="${UPGRADE611_OLD_LAUNCHER_CODE_ID:-11614}"
OLD_AUTOLP_CODE="${UPGRADE611_OLD_AUTOLP_CODE_ID:-11613}"
UNUSED_LAUNCHER="${UPGRADE611_UNUSED_LAUNCHER:-terra1af9xm63mev4hnf4z0nmmcsnd9f4lpac2vs205rmaeg3kdqlqudhq894lyz}"
CMM="${UPGRADE611_CMM:-$UST1_OPS_TREASURY}"
DEX_GOV="${UPGRADE611_DEX_GOVERNANCE:-$UST1_OPS_DEX_GOVERNANCE}"
CMM_GOV="${UPGRADE611_CMM_GOVERNANCE:-$UST1_OPS_WRAP_GOVERNANCE}"

if [[ "${UPGRADE611_LOCAL:-0}" == "1" ]]; then
  ENV_LOCAL="$REPO_ROOT/frontend-dapp/.env.local"
  if [[ -f "$ENV_LOCAL" ]]; then
    # shellcheck disable=SC1090
    set -a
    eval "$(grep -E '^(VITE_FACTORY_ADDRESS|VITE_COMMUNITY_TOKEN_LAUNCHER|VITE_LCD_URL)=' "$ENV_LOCAL" | sed 's/^VITE_FACTORY_ADDRESS=/FACTORY_ADDRESS=/; s/^VITE_COMMUNITY_TOKEN_LAUNCHER=/LAUNCHER_ADDRESS=/; s/^VITE_LCD_URL=/LCD_URL=/')"
    set +a
  fi
  TERRAD_HOST_CHAIN_ID="${TERRAD_HOST_CHAIN_ID:-localterra}"
  TERRAD_HOST_NODE="${TERRAD_HOST_NODE:-http://127.0.0.1:26657}"
  LCD_URL="${LCD_URL:-http://127.0.0.1:1317}"
  TERRAD_HOST_KEY="${TERRAD_HOST_KEY:-test1}"
fi

FACTORY="${UPGRADE611_FACTORY_ADDRESS:-${FACTORY_ADDRESS:-$UST1_OPS_FACTORY}}"
LAUNCHER="${UPGRADE611_LAUNCHER_ADDRESS:-${LAUNCHER_ADDRESS:-terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze}}"
LCD_URL="${LCD_URL:-${TERRA_LCD_URL:-$UST1_OPS_LCD_URL}}"
LCD_URL="${LCD_URL%/}"
LOCALTERRA_CURL_MAX_TIME="${LOCALTERRA_CURL_MAX_TIME:-25}"
LOCALTERRA_CURL_CONNECT_TIMEOUT="${LOCALTERRA_CURL_CONNECT_TIMEOUT:-5}"

# Store stays on cl8ydeploy (columbus-5) unless the caller already set a key.
if [[ "${UPGRADE611_LOCAL:-0}" != "1" ]]; then
  TERRAD_HOST_KEY="${TERRAD_HOST_KEY:-cl8ydeploy}"
fi

echo "=============================================="
echo "GitLab #611 / #612 / #616 community-tax rotate"
echo "=============================================="
echo "Factory:   $FACTORY"
echo "Launcher:  $LAUNCHER"
echo "CMM admin: $CMM"
echo "LCD:       $LCD_URL"
echo "Chain:     ${TERRAD_HOST_CHAIN_ID:-columbus-5}"
echo "Store key: $TERRAD_HOST_KEY"
echo "DRY_RUN:   ${DRY_RUN:-0}"
echo "LOCAL:     ${UPGRADE611_LOCAL:-0}"
echo "589 GO:    ${UPGRADE611_589_GO:-0}"
echo ""

need_wasm() {
  local f="$1"
  [[ -f "$f" ]] || upgrade611_die "missing wasm: $f (make build-optimized, or set UPGRADE611_SKIP_STORE=1)"
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
  [[ -n "$tx_hash" ]] || upgrade611_die "no txhash from: $label"
  echo "    tx: $tx_hash" >&2
  terrad_host_wait_tx_inclusion "$tx_hash"
  printf '%s' "$tx_hash"
}

store_code() {
  local wasm="$1"
  local label="$2"
  local tx_hash code_id
  # Columbus-5 terrad treats unknown store flags as a second filename
  # ("accepts 1 arg(s), received 2"). Same as #601: wasm store <file> only.
  # Optional extras: UPGRADE611_STORE_EXTRA='--instantiate-everybody' (if your CLI has it).
  if [[ -n "${UPGRADE611_STORE_EXTRA:-}" ]]; then
    # shellcheck disable=SC2086
    tx_hash="$(broadcast_and_wait "store $label" wasm store "$wasm" $UPGRADE611_STORE_EXTRA)"
  else
    tx_hash="$(broadcast_and_wait "store $label" wasm store "$wasm")"
  fi
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    case "$label" in
      token) echo "${DRY_RUN_TOKEN_CODE_ID:-999011}" ;;
      launcher) echo "${DRY_RUN_LAUNCHER_CODE_ID:-999014}" ;;
      autolp) echo "${DRY_RUN_AUTOLP_CODE_ID:-999013}" ;;
      *) echo "${DRY_RUN_CODE_ID:-999001}" ;;
    esac
    return 0
  fi
  code_id="$(terrad_host_code_id_from_store_tx "$tx_hash")"
  [[ -n "$code_id" ]] || upgrade611_die "could not parse code_id for $label"
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

# Launcher migrate + factory execute: LocalTerra uses the store key; columbus-5 uses 2-of-3.
gov_tx() {
  local label="$1"
  shift
  echo "  → $label"
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "    DRY_RUN skip: $*"
    return 0
  fi
  if [[ "${UPGRADE611_LOCAL:-0}" == "1" ]]; then
    broadcast_and_wait "$label" "$@" >/dev/null
    return 0
  fi
  echo "    2-of-3 $(printf '%s ' "$@")" >&2
  "$SCRIPT_DIR/multisig-2of3-host-tx.sh" "$@"
}

print_resume() {
  cat <<EOF

Resume after #589 REPORT GO (do not re-store):
  UPGRADE611_589_GO=1 \\
    UPGRADE611_SKIP_STORE=1 \\
    UPGRADE611_SKIP_LAUNCHER_MIGRATE=1 \\
    UPGRADE611_TOKEN_CODE_ID=${TOKEN_CODE:-<new-token>} \\
    UPGRADE611_LAUNCHER_CODE_ID=${LAUNCHER_CODE:-<new-launcher>} \\
    UPGRADE611_AUTOLP_CODE_ID=${AUTOLP_CODE:-<new-autolp>} \\
    ./scripts/upgrade-611-community-tax.sh

Rotate launcher pins (needs UpdateConfig wasm + new store):
  UPGRADE611_STORE_LAUNCHER_ONLY=1 \\
    UPGRADE611_SKIP_WHITELIST=1 \\
    UPGRADE611_UPDATE_CONFIG=1 \\
    UPGRADE611_TOKEN_CODE_ID=${TOKEN_CODE:-11619} \\
    UPGRADE611_AUTOLP_CODE_ID=${AUTOLP_CODE:-11621} \\
    ./scripts/upgrade-611-community-tax.sh

#589 intake:
  ./cw20-codeid-audits/scripts/fetch-lcd-wasm.sh ${TOKEN_CODE:-<new-token>}
  ./cw20-codeid-audits/scripts/fingerprint-wasm.sh ${TOKEN_CODE:-<new-token>}
  ./cw20-codeid-audits/scripts/decompile-wasm.sh ${TOKEN_CODE:-<new-token>}
  CODE_ID=${TOKEN_CODE:-<new-token>} LAYER_B_LT=1 make verify-issue-589
  # copy report-template.md → cw20-codeid-audits/codeids/${TOKEN_CODE:-<id>}/REPORT.md  GO
EOF
}

echo "[1] preflight"
[[ -n "$FACTORY" ]] || upgrade611_die "set UPGRADE611_FACTORY_ADDRESS"
[[ -n "$LAUNCHER" ]] || upgrade611_die "set UPGRADE611_LAUNCHER_ADDRESS"
if [[ "$LAUNCHER" == "$UNUSED_LAUNCHER" ]]; then
  upgrade611_die "refusing unused 11612 launcher $UNUSED_LAUNCHER — use terra126pr5… (11614)"
fi
if [[ "${UPGRADE611_SKIP_STORE:-0}" != "1" && "${DRY_RUN:-0}" != "1" ]]; then
  if [[ "${UPGRADE611_STORE_LAUNCHER_ONLY:-0}" == "1" ]]; then
    need_wasm "$LAUNCHER_WASM"
  else
    need_wasm "$TOKEN_WASM"
    need_wasm "$LAUNCHER_WASM"
    need_wasm "$AUTOLP_WASM"
  fi
fi
if [[ "${DRY_RUN:-0}" != "1" && "${UPGRADE611_LOCAL:-0}" != "1" ]]; then
  INFO="$(upgrade611_lcd_contract_info "$LCD_URL" "$LAUNCHER" || true)"
  ADMIN="$(printf '%s' "$INFO" | jq -r '.contract_info.admin // empty')"
  LIVE="$(printf '%s' "$INFO" | jq -r '.contract_info.code_id // empty')"
  echo "  launcher wasm admin=$ADMIN code_id=$LIVE"
  [[ -z "$ADMIN" || "$ADMIN" == "$DEX_GOV" ]] \
    || echo "  WARN: launcher admin is $ADMIN (expected DEX 2-of-3 $DEX_GOV)" >&2
  [[ -z "$LIVE" || "$LIVE" == "$OLD_LAUNCHER_CODE" ]] \
    || echo "  WARN: launcher already on code_id $LIVE (old pin $OLD_LAUNCHER_CODE)" >&2
fi

echo ""
echo "[2] store wasm (permissionless — $TERRAD_HOST_KEY)"
if [[ "${UPGRADE611_SKIP_STORE:-0}" == "1" ]]; then
  TOKEN_CODE="${UPGRADE611_TOKEN_CODE_ID:-}"
  LAUNCHER_CODE="${UPGRADE611_LAUNCHER_CODE_ID:-}"
  AUTOLP_CODE="${UPGRADE611_AUTOLP_CODE_ID:-}"
  [[ -n "$TOKEN_CODE" && -n "$LAUNCHER_CODE" && -n "$AUTOLP_CODE" ]] \
    || upgrade611_die "UPGRADE611_SKIP_STORE=1 needs UPGRADE611_TOKEN_CODE_ID, UPGRADE611_LAUNCHER_CODE_ID, UPGRADE611_AUTOLP_CODE_ID"
  echo "  reuse token=$TOKEN_CODE launcher=$LAUNCHER_CODE autolp=$AUTOLP_CODE"
elif [[ "${UPGRADE611_STORE_LAUNCHER_ONLY:-0}" == "1" ]]; then
  TOKEN_CODE="${UPGRADE611_TOKEN_CODE_ID:-}"
  AUTOLP_CODE="${UPGRADE611_AUTOLP_CODE_ID:-}"
  [[ -n "$TOKEN_CODE" && -n "$AUTOLP_CODE" ]] \
    || upgrade611_die "UPGRADE611_STORE_LAUNCHER_ONLY=1 needs UPGRADE611_TOKEN_CODE_ID and UPGRADE611_AUTOLP_CODE_ID"
  echo "  reuse token=$TOKEN_CODE autolp=$AUTOLP_CODE"
  LAUNCHER_CODE="$(store_code "$LAUNCHER_WASM" launcher)"
else
  TOKEN_CODE="$(store_code "$TOKEN_WASM" token)"
  LAUNCHER_CODE="$(store_code "$LAUNCHER_WASM" launcher)"
  AUTOLP_CODE="$(store_code "$AUTOLP_WASM" autolp)"
fi

if [[ "${DRY_RUN:-0}" != "1" ]]; then
  echo "  token    $TOKEN_CODE  hash=$(upgrade611_lcd_code_hash "$LCD_URL" "$TOKEN_CODE")"
  echo "  launcher $LAUNCHER_CODE  hash=$(upgrade611_lcd_code_hash "$LCD_URL" "$LAUNCHER_CODE")"
  echo "  autolp   $AUTOLP_CODE  hash=$(upgrade611_lcd_code_hash "$LCD_URL" "$AUTOLP_CODE")"
fi

if [[ "${UPGRADE611_STORE_ONLY:-0}" == "1" ]]; then
  echo ""
  echo "STORE_ONLY — stopping before migrate / whitelist."
  print_resume
  echo "OK (store-only)"
  exit 0
fi

echo ""
echo "[3] migrate launcher $LAUNCHER → $LAUNCHER_CODE"
if [[ "${UPGRADE611_SKIP_LAUNCHER_MIGRATE:-0}" == "1" ]]; then
  echo "  skipped"
else
  if [[ "${DRY_RUN:-0}" != "1" && "$LAUNCHER_CODE" == "$OLD_LAUNCHER_CODE" ]]; then
    echo "  skip — already on $OLD_LAUNCHER_CODE"
  else
    gov_tx "migrate launcher" wasm migrate "$LAUNCHER" "$LAUNCHER_CODE" '{}'
  fi
fi

echo ""
echo "[4] probe live $OLD_TOKEN_CODE / $OLD_AUTOLP_CODE instances (CMM migrate — not this script)"
TOKEN_INSTANCES=()
AUTOLP_INSTANCES=()
if [[ "${DRY_RUN:-0}" != "1" ]]; then
  mapfile -t TOKEN_INSTANCES < <(upgrade611_lcd_code_contracts "$LCD_URL" "$OLD_TOKEN_CODE")
  mapfile -t AUTOLP_INSTANCES < <(upgrade611_lcd_code_contracts "$LCD_URL" "$OLD_AUTOLP_CODE")
fi
echo "  $OLD_TOKEN_CODE tokens:  ${#TOKEN_INSTANCES[@]}"
echo "  $OLD_AUTOLP_CODE autolps: ${#AUTOLP_INSTANCES[@]}"
for addr in "${TOKEN_INSTANCES[@]+"${TOKEN_INSTANCES[@]}"}"; do
  info="$(upgrade611_lcd_contract_info "$LCD_URL" "$addr" || true)"
  echo "    token  $addr  admin=$(printf '%s' "$info" | jq -r '.contract_info.admin // empty')"
done
for addr in "${AUTOLP_INSTANCES[@]+"${AUTOLP_INSTANCES[@]}"}"; do
  info="$(upgrade611_lcd_contract_info "$LCD_URL" "$addr" || true)"
  echo "    autolp $addr  admin=$(printf '%s' "$info" | jq -r '.contract_info.admin // empty')"
done
echo "  unused 11612 $UNUSED_LAUNCHER — do not migrate or retarget Coolify"

echo ""
echo "[5] factory AddWhitelistedCodeId $TOKEN_CODE"
if [[ "${UPGRADE611_SKIP_WHITELIST:-0}" == "1" ]]; then
  echo "  skipped"
elif [[ "${UPGRADE611_589_GO:-0}" != "1" ]]; then
  echo "  blocked — set UPGRADE611_589_GO=1 after cw20-codeid-audits/codeids/${TOKEN_CODE}/REPORT.md is GO"
  print_resume
else
  upgrade611_assert_whitelist_ok "$TOKEN_CODE" "$LAUNCHER_CODE" "$AUTOLP_CODE"
  WL_MSG="$(jq -nc --argjson id "$TOKEN_CODE" '{add_whitelisted_code_id:{code_id:$id}}')"
  gov_tx "AddWhitelistedCodeId $TOKEN_CODE" wasm execute "$FACTORY" "$WL_MSG"
fi

echo ""
echo "[6] launcher UpdateConfig token_code_id=$TOKEN_CODE autolp_code_id=$AUTOLP_CODE"
if [[ "${UPGRADE611_UPDATE_CONFIG:-0}" != "1" ]]; then
  echo "  skipped (set UPGRADE611_UPDATE_CONFIG=1 after migrating a launcher wasm that has UpdateConfig)"
else
  CFG_MSG="$(jq -nc --argjson token "$TOKEN_CODE" --argjson autolp "$AUTOLP_CODE" \
    '{update_config:{token_code_id:$token,autolp_code_id:$autolp}}')"
  gov_tx "UpdateConfig token/autolp pins" wasm execute "$LAUNCHER" "$CFG_MSG"
  if [[ "${DRY_RUN:-0}" != "1" ]]; then
    LIVE_CFG="$(lcd_smart_query_raw "$LCD_URL" "$LAUNCHER" '{"get_config":{}}' || true)"
    LIVE_CFG="$(lcd_decode_smart_data "$LIVE_CFG" || true)"
    echo "  GetConfig token_code_id=$(printf '%s' "$LIVE_CFG" | jq -r '.token_code_id // empty') autolp_code_id=$(printf '%s' "$LIVE_CFG" | jq -r '.autolp_code_id // empty')"
  fi
fi

echo ""
echo "[7] RefreshPairAssetCodeIdsBatch"
if [[ "${UPGRADE611_REFRESH:-0}" != "1" ]]; then
  echo "  skipped (set UPGRADE611_REFRESH=1 after CMM migrate of listed tokens)"
else
  REFRESH_MSG='{"refresh_pair_asset_code_ids_batch":{"start_after":null,"limit":20}}'
  gov_tx "RefreshPairAssetCodeIdsBatch" wasm execute "$FACTORY" "$REFRESH_MSG"
fi

echo ""
echo "[8] leftover (not signed here)"
cat <<EOF
CMM migrate (admin $CMM; signer $CMM_GOV / ustr-cmm — NOT DEX 2-of-3):
  # if ContractInfo.admin is the CMM contract, use ustr-cmm WasmMsg::Migrate
  # if admin is $CMM_GOV:
  terrad tx wasm migrate <token> $TOKEN_CODE '{}' --from cl8y2_admin \\
    --chain-id columbus-5 --node https://terra-classic-rpc.publicnode.com:443 \\
    --gas auto --gas-adjustment 1.4 --gas-prices 28.325uluna
  # AutoLP instances: same, code $AUTOLP_CODE  msg '{"factory":"$FACTORY"}' if pre-#610
  # 0 token / AutoLP instances as of 2026-08-24 — no CMM migrate required.

Launcher token_code_id / autolp_code_id (11620 has no UpdateConfig):
  make build-optimized
  UPGRADE611_STORE_LAUNCHER_ONLY=1 \\
    UPGRADE611_SKIP_WHITELIST=1 \\
    UPGRADE611_UPDATE_CONFIG=1 \\
    UPGRADE611_TOKEN_CODE_ID=$TOKEN_CODE \\
    UPGRADE611_AUTOLP_CODE_ID=$AUTOLP_CODE \\
    ./scripts/upgrade-611-community-tax.sh
  # Do not whitelist the new launcher store id.

Coolify after instances actually run option-2 bytes:
  COMMUNITY_TAX_OPTION2_CODE_IDS=$TOKEN_CODE
  UST1_WINDOW_ADDRESS=$UST1_OPS_WINDOW
  # keep VITE_COMMUNITY_TAX_CODE_ID=$OLD_TOKEN_CODE until UpdateConfig + catalog accept 11619
  # VITE_COMMUNITY_TOKEN_LAUNCHER=$LAUNCHER

Do not whitelist $LAUNCHER_CODE / $AUTOLP_CODE / 11612 / 8654.
Do not RemoveWhitelistedCodeId $OLD_TOKEN_CODE until every pair pin is refreshed.
EOF

if [[ "${DRY_RUN:-0}" == "1" ]]; then
  echo ""
  echo "OK (dry-run) token=$TOKEN_CODE launcher=$LAUNCHER_CODE autolp=$AUTOLP_CODE"
  exit 0
fi

echo ""
echo "OK — stored token=$TOKEN_CODE launcher=$LAUNCHER_CODE autolp=$AUTOLP_CODE"
