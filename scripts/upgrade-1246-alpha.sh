#!/usr/bin/env bash
# Forgejo #1246 — columbus-5 ALPHA (11630) store + whitelist + CMM migrate + Refresh.
#
# ALPHA terra1x6e64…zysuxz wasm admin is CMM (not an EOA). Do not
# `wasm migrate` ALPHA. After CMM is treasury 0.2.2, DEX 2-of-3 executes
# `migrate_owned_contract` on CMM. Refresh only when ALPHA LCD code_id is 11666.
# Keep 11630 listed until Refresh. Do NOT whitelist 8654 / launcher / AutoLP.
#
# Usage:
#   DRY_RUN=1 ./scripts/upgrade-1246-alpha.sh
#   UPGRADE1246_WHITELIST=1 ./scripts/upgrade-1246-alpha.sh
#   UPGRADE1246_SKIP_STORE=1 UPGRADE1246_TOKEN_CODE_ID=11666 \
#     UPGRADE1246_SKIP_WHITELIST=1 UPGRADE1246_SKIP_LAUNCHER_CONFIG=1 \
#     UPGRADE1246_STORE_CMM=1 UPGRADE1246_REFRESH=1 \
#     ./scripts/upgrade-1246-alpha.sh
#
# Keys: cl8ydeploy stores token + treasury. DEX 2-of-3: whitelist, launcher,
# CMM wasm migrate, MigrateOwnedContract, Refresh.
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

ARTIFACTS="${UPGRADE1246_ARTIFACTS:-$REPO_ROOT/smartcontracts/artifacts}"
TOKEN_WASM="${UPGRADE1246_TOKEN_WASM:-$ARTIFACTS/cl8y_community_tax_token.wasm}"
ALPHA="${UPGRADE1246_ALPHA:-terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz}"
OLD_TOKEN_CODE="${UPGRADE1246_OLD_TOKEN_CODE_ID:-11630}"
NEVER_LIST="8654 11612 11613 11614 11620 11621 11622 11633 11628 11629"

FACTORY="${UPGRADE1246_FACTORY_ADDRESS:-${FACTORY_ADDRESS:-$UST1_OPS_FACTORY}}"
LAUNCHER="${UPGRADE1246_LAUNCHER_ADDRESS:-terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze}"
CMM="${UPGRADE1246_CMM:-$UST1_OPS_TREASURY}"
CMM_MIN_CW2="${UPGRADE1246_CMM_MIN_CW2:-0.2.2}"
TREASURY_WASM="${UPGRADE1246_TREASURY_WASM:-$REPO_ROOT/../ustr-cmm/contracts/artifacts/treasury.wasm}"
DEX_GOV="${UPGRADE1246_DEX_GOVERNANCE:-$UST1_OPS_DEX_GOVERNANCE}"
LCD_URL="${LCD_URL:-${TERRA_LCD_URL:-$UST1_OPS_LCD_URL}}"
LCD_URL="${LCD_URL%/}"
LOCALTERRA_CURL_MAX_TIME="${LOCALTERRA_CURL_MAX_TIME:-25}"
LOCALTERRA_CURL_CONNECT_TIMEOUT="${LOCALTERRA_CURL_CONNECT_TIMEOUT:-5}"

if [[ "${UPGRADE1246_LOCAL:-0}" != "1" ]]; then
  TERRAD_HOST_KEY="${TERRAD_HOST_KEY:-cl8ydeploy}"
fi

upgrade1246_die() { echo "ERROR: $*" >&2; exit 1; }

query_json() {
  local contract="$1"
  local msg="$2"
  if [[ "${DRY_RUN:-0}" == "1" && -z "${UPGRADE1246_DRY_QUERY:-}" ]]; then
    echo '{}'
    return 0
  fi
  lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD_URL" "$contract" "$msg")"
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
  [[ -n "$tx_hash" ]] || upgrade1246_die "no txhash from: $label"
  echo "    tx: $tx_hash" >&2
  terrad_host_wait_tx_inclusion "$tx_hash"
  printf '%s' "$tx_hash"
}

gov_tx() {
  local label="$1"
  shift
  echo "  → $label" >&2
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "    DRY_RUN skip: 2-of-3 $*" >&2
    echo "dry-run"
    return 0
  fi
  if [[ "${UPGRADE1246_LOCAL:-0}" == "1" ]]; then
    broadcast_and_wait "$label" "$@"
    return 0
  fi
  echo "    DEX 2-of-3 (not cl8ydeploy)" >&2
  local out tx_hash
  out="$("$SCRIPT_DIR/multisig-2of3-host-tx.sh" "$@" | tee /dev/stderr)"
  tx_hash="$(printf '%s' "$out" | awk '/^OK / { print $2 }' | tail -1)"
  [[ -n "$tx_hash" ]] || upgrade1246_die "no txhash from 2-of-3: $label"
  printf '%s' "$tx_hash"
}

upgrade1246_cw2_version() {
  local addr="$1"
  local key_b64 json data ver
  if [[ "$(uname)" == Darwin ]]; then
    key_b64="$(printf 'contract_info' | base64 | tr -d '\n')"
  else
    key_b64="$(printf 'contract_info' | base64 -w0)"
  fi
  json="$(localterra_lcd_curl "$LCD_URL" "/cosmwasm/wasm/v1/contract/${addr}/raw/${key_b64}" || true)"
  data="$(printf '%s' "$json" | jq -r '.data // empty')"
  [[ -n "$data" ]] || return 1
  ver="$(printf '%s' "$data" | base64 -d 2>/dev/null | jq -r '.version // empty')"
  [[ -n "$ver" ]] || return 1
  printf '%s' "$ver"
}

upgrade1246_assert_treasury_wasm_022() {
  local wasm="$1"
  [[ -f "$wasm" ]] || upgrade1246_die "missing $wasm — optimizer in ustr-cmm/contracts (not cargo wasm; refuse the Aug 2026 0.2.1 artifact)"
  python3 - "$wasm" <<'PY' || upgrade1246_die "refusing $wasm — wasm must embed cw2 0.2.2 (stale 0.2.1 artifact is not storeable)"
import sys
from pathlib import Path
data = Path(sys.argv[1]).read_bytes()
if b"0.2.2" not in data:
    sys.exit(1)
PY
}

upgrade1246_refresh_alpha_code() {
  INFO="$(upgrade611_lcd_contract_info "$LCD_URL" "$ALPHA" || true)"
  ALPHA_CODE="$(printf '%s' "$INFO" | jq -r '.contract_info.code_id // empty')"
}

alpha_factory_pairs() {
  local start="null"
  local page pairs_json
  PAIR_ADDRS=()
  while true; do
    if [[ "$start" == "null" ]]; then
      pairs_json="$(query_json "$FACTORY" '{"pairs":{"start_after":null,"limit":30}}')"
    else
      pairs_json="$(query_json "$FACTORY" "$(jq -nc --argjson sa "$start" '{"pairs":{"start_after":$sa,"limit":30}}')")"
    fi
    page="$(printf '%s' "$pairs_json" | jq -c '.pairs // []')"
    local n
    n="$(printf '%s' "$page" | jq 'length')"
    [[ "$n" =~ ^[0-9]+$ ]] || break
    local i addr infos
    for i in $(seq 0 $((n - 1))); do
      addr="$(printf '%s' "$page" | jq -r --argjson i "$i" '.[$i].contract_addr // empty')"
      infos="$(printf '%s' "$page" | jq -c --argjson i "$i" '.[$i].asset_infos')"
      if printf '%s' "$infos" | jq -e --arg a "$ALPHA" 'map(.token.contract_addr // empty) | index($a) != null' >/dev/null; then
        PAIR_ADDRS+=("$addr")
      fi
      start="$infos"
    done
    [[ "$n" -eq 30 ]] || break
    [[ -n "$start" && "$start" != "null" ]] || break
  done
}

echo "=============================================="
echo "Forgejo #1246 ALPHA tax-token rotate"
echo "=============================================="
echo "Factory:   $FACTORY"
echo "ALPHA:     $ALPHA"
echo "Launcher:  $LAUNCHER"
echo "CMM admin: $CMM"
echo "LCD:       $LCD_URL"
echo "Store key: $TERRAD_HOST_KEY"
echo "DRY_RUN:   ${DRY_RUN:-0}"
echo "WHITELIST: ${UPGRADE1246_WHITELIST:-0}"
echo ""

echo "[1] preflight"
[[ -n "$FACTORY" ]] || upgrade1246_die "FACTORY unset"
if [[ "${UPGRADE1246_SKIP_STORE:-0}" != "1" && "${DRY_RUN:-0}" != "1" ]]; then
  [[ -f "$TOKEN_WASM" ]] || upgrade1246_die "missing $TOKEN_WASM (make build-optimized)"
fi
if [[ "${DRY_RUN:-0}" != "1" ]]; then
  terrad_host_ensure_keyring_pass
fi

echo ""
echo "[1b] ALPHA + CMM + launcher probe"
if [[ "${DRY_RUN:-0}" == "1" && -z "${UPGRADE1246_DRY_QUERY:-}" ]]; then
  echo "  DRY_RUN: skip live LCD (set UPGRADE1246_DRY_QUERY=1)"
  ALPHA_ADMIN=""
  ALPHA_CODE="$OLD_TOKEN_CODE"
  LAUNCHER_TOKEN="$OLD_TOKEN_CODE"
  LAUNCHER_AUTOLP="11633"
else
  INFO="$(upgrade611_lcd_contract_info "$LCD_URL" "$ALPHA" || true)"
  ALPHA_ADMIN="$(printf '%s' "$INFO" | jq -r '.contract_info.admin // empty')"
  ALPHA_CODE="$(printf '%s' "$INFO" | jq -r '.contract_info.code_id // empty')"
  echo "  ALPHA code_id=$ALPHA_CODE admin=$ALPHA_ADMIN"
  [[ "$ALPHA_CODE" == "$OLD_TOKEN_CODE" || "${UPGRADE1246_REFRESH:-0}" == "1" ]] \
    || echo "  WARN: ALPHA already on code_id $ALPHA_CODE (old pin $OLD_TOKEN_CODE)" >&2
  CFG="$(query_json "$ALPHA" '{"get_config":{}}')"
  FEAT="$(query_json "$ALPHA" '{"get_features":{}}')"
  echo "  manager=$(printf '%s' "$CFG" | jq -r '.manager // empty')"
  echo "  launch_guards=$(printf '%s' "$CFG" | jq -c '.launch_guards')"
  echo "  features.launch_guards=$(printf '%s' "$FEAT" | jq -r '.launch_guards')"
  LCFG="$(query_json "$LAUNCHER" '{"get_config":{}}')"
  LAUNCHER_TOKEN="$(printf '%s' "$LCFG" | jq -r '.token_code_id // empty')"
  LAUNCHER_AUTOLP="$(printf '%s' "$LCFG" | jq -r '.autolp_code_id // empty')"
  echo "  launcher token_code_id=$LAUNCHER_TOKEN autolp_code_id=$LAUNCHER_AUTOLP"
  CMM_INFO="$(upgrade611_lcd_contract_info "$LCD_URL" "$CMM" || true)"
  CMM_CODE_LIVE="$(printf '%s' "$CMM_INFO" | jq -r '.contract_info.code_id // empty')"
  CMM_ADMIN="$(printf '%s' "$CMM_INFO" | jq -r '.contract_info.admin // empty')"
  CMM_CW2="$(upgrade1246_cw2_version "$CMM" || true)"
  echo "  CMM code_id=$CMM_CODE_LIVE admin=$CMM_ADMIN cw2=$CMM_CW2"
fi

echo ""
echo "[1c] factory pairs that hold ALPHA"
if [[ "${DRY_RUN:-0}" == "1" && -z "${UPGRADE1246_DRY_QUERY:-}" ]]; then
  PAIR_ADDRS=()
  echo "  DRY_RUN: skip pairs pagination"
else
  alpha_factory_pairs
  echo "  count=${#PAIR_ADDRS[@]}"
  for pair in "${PAIR_ADDRS[@]+"${PAIR_ADDRS[@]}"}"; do
    pins="$(query_json "$pair" '{"get_asset_code_ids":{}}' || true)"
    paused="$(query_json "$pair" '{"is_paused":{}}' || true)"
    exempt="$(query_json "$ALPHA" "$(jq -nc --arg p "$pair" '{is_protocol_exempt:{address:$p}}')" || true)"
    echo "  $pair paused=$(printf '%s' "$paused" | jq -c '.') pins=$(printf '%s' "$pins" | jq -c '.code_ids // .') listed=$(printf '%s' "$exempt" | jq -r '.protocol // empty')"
  done
fi

echo ""
echo "[2] store token wasm (permissionless — $TERRAD_HOST_KEY)"
if [[ "${UPGRADE1246_SKIP_STORE:-0}" == "1" ]]; then
  TOKEN_CODE="${UPGRADE1246_TOKEN_CODE_ID:-}"
  [[ -n "$TOKEN_CODE" ]] || upgrade1246_die "UPGRADE1246_SKIP_STORE=1 needs UPGRADE1246_TOKEN_CODE_ID"
  echo "  reuse token=$TOKEN_CODE"
else
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "  → store token"
    echo "    DRY_RUN skip"
    TOKEN_CODE="${DRY_RUN_TOKEN_CODE_ID:-999130}"
  else
    local_tx="$(broadcast_and_wait "store token" wasm store "$TOKEN_WASM")"
    TOKEN_CODE="$(terrad_host_code_id_from_store_tx "$local_tx")"
    [[ -n "$TOKEN_CODE" ]] || upgrade1246_die "could not parse token code_id"
    echo "    code_id: $TOKEN_CODE" >&2
  fi
fi
for bad in $NEVER_LIST $OLD_TOKEN_CODE; do
  [[ "$TOKEN_CODE" != "$bad" ]] || upgrade1246_die "refusing token code_id $TOKEN_CODE (keep $OLD_TOKEN_CODE listed; never list $bad)"
done
upgrade611_assert_whitelist_ok "$TOKEN_CODE"

echo ""
echo "[3] factory AddWhitelistedCodeId $TOKEN_CODE (keep $OLD_TOKEN_CODE listed)"
if [[ "${UPGRADE1246_SKIP_WHITELIST:-0}" == "1" ]]; then
  echo "  skipped"
elif [[ "${UPGRADE1246_WHITELIST:-0}" != "1" ]]; then
  echo "  blocked — set UPGRADE1246_WHITELIST=1 (11630 REPORT is GO; this is the 1.1.0 same-crate bump)."
  echo "  Do not whitelist launcher / AutoLP / 8654."
else
  WL_MSG="$(jq -nc --argjson id "$TOKEN_CODE" '{add_whitelisted_code_id:{code_id:$id}}')"
  gov_tx "AddWhitelistedCodeId $TOKEN_CODE" wasm execute "$FACTORY" "$WL_MSG" >/dev/null
fi

echo ""
echo "[4] launcher UpdateConfig token_code_id=$TOKEN_CODE (keep autolp $LAUNCHER_AUTOLP)"
if [[ "${UPGRADE1246_SKIP_LAUNCHER_CONFIG:-0}" == "1" ]]; then
  echo "  skipped"
elif [[ "${UPGRADE1246_WHITELIST:-0}" != "1" && "${DRY_RUN:-0}" != "1" ]]; then
  echo "  skipped until whitelist flag (new Create Token would instantiate an unlisted id)"
else
  autolp="${LAUNCHER_AUTOLP:-11633}"
  CFG_MSG="$(jq -nc --argjson token "$TOKEN_CODE" --argjson autolp "$autolp" \
    '{update_config:{token_code_id:$token,autolp_code_id:$autolp}}')"
  gov_tx "UpdateConfig token_code_id" wasm execute "$LAUNCHER" "$CFG_MSG" >/dev/null
fi

echo ""
echo "[5] CMM migrate ALPHA → $TOKEN_CODE"
echo "  ALPHA admin is CMM $CMM. Do not wasm migrate ALPHA; 2-of-3 ($DEX_GOV) executes migrate_owned_contract."
if [[ "${DRY_RUN:-0}" == "1" && -z "${UPGRADE1246_DRY_QUERY:-}" ]]; then
  echo "  DRY_RUN: skip CMM store/migrate/execute"
  echo "    leftover until live CMM cw2 is $CMM_MIN_CW2: store optimizer treasury.wasm, 2-of-3 wasm migrate CMM, then migrate_owned_contract"
else
  if [[ -n "${ALPHA_ADMIN:-}" && "$ALPHA_ADMIN" != "$CMM" ]]; then
    upgrade1246_die "ALPHA admin is $ALPHA_ADMIN not CMM $CMM"
  fi
  if [[ "${ALPHA_CODE:-}" == "$TOKEN_CODE" ]]; then
    echo "  ALPHA already code_id=$TOKEN_CODE"
  else
    CMM_CW2="${CMM_CW2:-$(upgrade1246_cw2_version "$CMM" || true)}"
    if [[ "$CMM_CW2" != "$CMM_MIN_CW2" ]]; then
      echo "  CMM cw2=$CMM_CW2 (want $CMM_MIN_CW2)"
      CMM_NEW_CODE="${UPGRADE1246_CMM_CODE_ID:-}"
      if [[ -z "$CMM_NEW_CODE" && "${UPGRADE1246_STORE_CMM:-0}" == "1" ]]; then
        upgrade1246_assert_treasury_wasm_022 "$TREASURY_WASM"
        local_tx="$(broadcast_and_wait "store treasury 0.2.2" wasm store "$TREASURY_WASM")"
        CMM_NEW_CODE="$(terrad_host_code_id_from_store_tx "$local_tx")"
        [[ -n "$CMM_NEW_CODE" ]] || upgrade1246_die "could not parse treasury code_id"
        echo "    treasury code_id: $CMM_NEW_CODE" >&2
      fi
      if [[ -z "$CMM_NEW_CODE" ]]; then
        upgrade1246_die "CMM still $CMM_CW2. Optimizer ustr-cmm/contracts → artifacts/treasury.wasm (must embed 0.2.2; refuse Aug 2026 0.2.1 file), then:
  UPGRADE1246_SKIP_STORE=1 UPGRADE1246_TOKEN_CODE_ID=$TOKEN_CODE \\
    UPGRADE1246_SKIP_WHITELIST=1 UPGRADE1246_SKIP_LAUNCHER_CONFIG=1 \\
    UPGRADE1246_STORE_CMM=1 UPGRADE1246_REFRESH=1 $0
or set UPGRADE1246_CMM_CODE_ID=<T> after a manual store"
      fi
      gov_tx "migrate CMM → $CMM_NEW_CODE" wasm migrate "$CMM" "$CMM_NEW_CODE" '{}' >/dev/null
      CMM_CW2="$(upgrade1246_cw2_version "$CMM" || true)"
      [[ "$CMM_CW2" == "$CMM_MIN_CW2" ]] || upgrade1246_die "CMM cw2=$CMM_CW2 after migrate, want $CMM_MIN_CW2"
    fi
    OWNED="$(jq -nc --arg c "$ALPHA" --argjson id "$TOKEN_CODE" \
      '{migrate_owned_contract:{contract:$c,new_code_id:$id}}')"
    gov_tx "MigrateOwnedContract ALPHA → $TOKEN_CODE" wasm execute "$CMM" "$OWNED" >/dev/null
    for i in 1 2 3 4 5 6; do
      upgrade1246_refresh_alpha_code
      [[ "$ALPHA_CODE" == "$TOKEN_CODE" ]] && break
      sleep 2
    done
    [[ "$ALPHA_CODE" == "$TOKEN_CODE" ]] \
      || upgrade1246_die "ALPHA still code_id=$ALPHA_CODE after MigrateOwnedContract (want $TOKEN_CODE)"
  fi
fi
if [[ "${UPGRADE1246_REFRESH:-0}" == "1" ]]; then
  if [[ "${DRY_RUN:-0}" != "1" && -n "${ALPHA_CODE:-}" && "$ALPHA_CODE" != "$TOKEN_CODE" ]]; then
    upgrade1246_die "UPGRADE1246_REFRESH=1 but ALPHA live code_id=$ALPHA_CODE want $TOKEN_CODE (CMM migrate first)"
  fi
fi

echo ""
echo "[6] RegisterListedPair (permissionless) + optional Refresh"
for pair in "${PAIR_ADDRS[@]+"${PAIR_ADDRS[@]}"}"; do
  REG="$(jq -nc --arg p "$pair" '{register_listed_pair:{pair:$p}}')"
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "  → register $pair"
    echo "    DRY_RUN skip"
  else
    echo "  → register $pair" >&2
    set +e
    out="$(terrad_host_tx wasm execute "$ALPHA" "$REG")"
    st=$?
    set -e
    if [[ "$st" -eq 0 ]]; then
      tx_hash="$(printf '%s' "$out" | jq -r '.txhash // empty')"
      echo "    tx: $tx_hash" >&2
      terrad_host_wait_tx_inclusion "$tx_hash" || true
    else
      echo "    register skipped (already listed or LCD/RPC)" >&2
    fi
  fi
  if [[ "${UPGRADE1246_REFRESH:-0}" == "1" ]]; then
    REF="$(jq -nc --arg p "$pair" '{refresh_pair_asset_code_ids:{pair:$p}}')"
    gov_tx "RefreshPairAssetCodeIds $pair" wasm execute "$FACTORY" "$REF" >/dev/null
  fi
done
if [[ "${#PAIR_ADDRS[@]}" -eq 0 ]]; then
  echo "  no factory ALPHA pairs enumerated"
fi

echo ""
echo "[7] trading smoke"
if [[ "${DRY_RUN:-0}" == "1" && -z "${UPGRADE1246_DRY_QUERY:-}" ]]; then
  echo "  DRY_RUN: smoke skipped"
else
  WL="$(query_json "$FACTORY" '{"get_whitelisted_code_ids":{}}')"
  echo "  whitelist=$(printf '%s' "$WL" | jq -c '.code_ids')"
  printf '%s' "$WL" | jq -e --argjson old "$OLD_TOKEN_CODE" '.code_ids | index($old) != null' >/dev/null \
    || upgrade1246_die "$OLD_TOKEN_CODE dropped from whitelist (keep until Refresh)"
  if [[ "${UPGRADE1246_WHITELIST:-0}" == "1" ]]; then
    printf '%s' "$WL" | jq -e --argjson id "$TOKEN_CODE" '.code_ids | index($id) != null' >/dev/null \
      || upgrade1246_die "new token $TOKEN_CODE not factory-listed"
  fi
  printf '%s' "$WL" | jq -e '.code_ids | index(8654) == null' >/dev/null \
    || upgrade1246_die "8654 must stay off the whitelist"
  for pair in "${PAIR_ADDRS[@]+"${PAIR_ADDRS[@]}"}"; do
    paused="$(query_json "$pair" '{"is_paused":{}}')"
    [[ "$(printf '%s' "$paused" | jq -r '.paused')" == "false" ]] \
      || upgrade1246_die "$pair is paused"
    exempt="$(query_json "$ALPHA" "$(jq -nc --arg p "$pair" '{is_protocol_exempt:{address:$p}}')")"
    [[ "$(printf '%s' "$exempt" | jq -r '.protocol')" == "true" ]] \
      || upgrade1246_die "$pair is not RegisterListedPair (sell extra-debit off)"
    pins="$(query_json "$pair" '{"get_asset_code_ids":{}}')"
    echo "  $pair pins=$(printf '%s' "$pins" | jq -c '.code_ids') paused=false listed=true"
    if [[ "${UPGRADE1246_REFRESH:-0}" == "1" ]]; then
      printf '%s' "$pins" | jq -e --argjson id "$TOKEN_CODE" '.code_ids | index($id) != null' >/dev/null \
        || upgrade1246_die "$pair pin missing $TOKEN_CODE after Refresh"
    fi
  done
  FEAT="$(query_json "$ALPHA" '{"get_features":{}}')"
  if [[ "$(printf '%s' "$FEAT" | jq -r '.launch_guards')" == "true" ]]; then
    CFG="$(query_json "$ALPHA" '{"get_config":{}}')"
    [[ "$(printf '%s' "$CFG" | jq -r '.launch_guards.trading_enabled')" == "true" ]] \
      || upgrade1246_die "launch_guards.trading_enabled=false (H-5 locks exits)"
  else
    echo "  launch_guards SKU off — no trading_enabled lock"
  fi
fi

echo ""
echo "OK token=$TOKEN_CODE ALPHA=$ALPHA keep_listed=$OLD_TOKEN_CODE ALPHA_code=${ALPHA_CODE:-}"
echo "CMM cw2 want $CMM_MIN_CW2 then MigrateOwnedContract; Refresh only when ALPHA LCD is $TOKEN_CODE."
echo "Resume: UPGRADE1246_SKIP_STORE=1 UPGRADE1246_TOKEN_CODE_ID=$TOKEN_CODE UPGRADE1246_SKIP_WHITELIST=1 UPGRADE1246_SKIP_LAUNCHER_CONFIG=1 UPGRADE1246_STORE_CMM=1 UPGRADE1246_REFRESH=1 $0"
