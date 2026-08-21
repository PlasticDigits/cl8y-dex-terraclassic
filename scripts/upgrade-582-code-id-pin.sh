#!/usr/bin/env bash
# GitLab #584 — order-enforcing factory 1.9.0 then pair 1.15.0 migrate (invariant F6).
#
# Opposite of #514: factory MUST be ≥1.9.0 (IsCodeIdWhitelisted live) before any
# pair 1.15.0 migrate. Pair-first freezes every gated write until factory catches up.
#
# Usage:
#   DRY_RUN=1 ./scripts/upgrade-582-code-id-pin.sh
#   UPGRADE582_LOCAL=1 ./scripts/upgrade-582-code-id-pin.sh
#   UPGRADE582_PROBE_ONLY=1 ./scripts/upgrade-582-code-id-pin.sh   # columbus-5 read-only
#   ./scripts/upgrade-582-code-id-pin.sh
#
# Optional:
#   UPGRADE582_SKIP_STORE=1 + UPGRADE582_FACTORY_CODE_ID / UPGRADE582_PAIR_CODE_ID
#   UPGRADE582_SKIP_PAIR_MIGRATE=1   factory-only retry (smoke still runs; unmigrated fails)
#   UPGRADE582_SKIP_FACTORY_MIGRATE=1  factory already on 1.9.0 (still asserts)
#   UPGRADE582_REFRESH=1             paginated RefreshPairAssetCodeIdsBatch (parses wasm has_more)
#   UPGRADE582_SKIP_UPDATE_CONFIG=1  factory pair_code_id already on new pair wasm
#   UPGRADE582_FORCE_FACTORY_VERSION  test hook (verify-issue-584 negative path)
#   UPGRADE582_FORCE_WHITELIST_JSON   test hook (must be parseable {whitelisted:bool})
#   UPGRADE582_SKIP_CONTRACT_INFO_PROBE=1  DRY_RUN dummy factory only
#
# Does NOT AddWhitelistedCodeId / RemoveWhitelistedCodeId.
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
# shellcheck source=lib/upgrade-582-code-id-pin.sh
source "$SCRIPT_DIR/lib/upgrade-582-code-id-pin.sh"

ARTIFACTS="${UPGRADE582_ARTIFACTS:-$REPO_ROOT/smartcontracts/artifacts}"
PAIR_WASM="${UPGRADE582_PAIR_WASM:-$ARTIFACTS/cl8y_dex_pair.wasm}"
FACTORY_WASM="${UPGRADE582_FACTORY_WASM:-$ARTIFACTS/cl8y_dex_factory.wasm}"

if [[ "${UPGRADE582_LOCAL:-0}" == "1" ]]; then
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

FACTORY="${UPGRADE582_FACTORY_ADDRESS:-${FACTORY_ADDRESS:-$UST1_SEC_FACTORY_ADDRESS}}"
LCD_URL="${LCD_URL:-${TERRA_LCD_URL:-https://terra-classic-lcd.publicnode.com}}"
LCD_URL="${LCD_URL%/}"
# Public LCD ContractInfo pages can exceed the LocalTerra 8s default.
LOCALTERRA_CURL_MAX_TIME="${LOCALTERRA_CURL_MAX_TIME:-25}"
LOCALTERRA_CURL_CONNECT_TIMEOUT="${LOCALTERRA_CURL_CONNECT_TIMEOUT:-5}"

echo "=============================================="
echo "GitLab #584 / #582 F6 code-id pin upgrade"
echo "=============================================="
echo "Factory:     $FACTORY"
echo "LCD:         $LCD_URL"
echo "Chain:       ${TERRAD_HOST_CHAIN_ID:-}"
echo "DRY_RUN:     ${DRY_RUN:-0}"
echo "LOCAL:       ${UPGRADE582_LOCAL:-0}"
echo "PROBE_ONLY:  ${UPGRADE582_PROBE_ONLY:-0}"
echo "Order:       factory ${UPGRADE582_MIN_FACTORY_VERSION} then pairs ${UPGRADE582_PAIR_VERSION}"
echo ""

need_wasm() {
  local f="$1"
  [[ -f "$f" ]] || upgrade582_die "missing wasm: $f (make build-optimized, or set UPGRADE582_SKIP_STORE=1)"
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
  [[ -n "$tx_hash" ]] || upgrade582_die "no txhash from: $label"
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
  [[ -n "$code_id" ]] || upgrade582_die "could not parse code_id for $label"
  echo "    code_id: $code_id" >&2
  printf '%s' "$code_id"
}

query_live() {
  local contract="$1"
  local msg="$2"
  if [[ "${DRY_RUN:-0}" == "1" && -z "${UPGRADE582_DRY_QUERY:-}" ]]; then
    echo '{}'
    return 0
  fi
  upgrade582_query_smart "$contract" "$msg"
}

assert_factory_ready() {
  local ver
  ver="$(upgrade582_cw2_version "$FACTORY" || true)"
  echo "  factory cw2 version=${ver:-<unreadable>}"
  if [[ -z "$ver" ]]; then
    upgrade582_die "could not read factory cw2 version (set UPGRADE582_FORCE_FACTORY_VERSION for DRY_RUN tests, or fix LCD raw contract_info)"
  fi
  if ! upgrade582_version_ge "$ver" "$UPGRADE582_MIN_FACTORY_VERSION"; then
    upgrade582_die "factory cw2 $ver < ${UPGRADE582_MIN_FACTORY_VERSION}; refusing pair ${UPGRADE582_PAIR_VERSION} migrate (pair-first would freeze all gated writes)"
  fi

  local wl
  wl="$(upgrade582_whitelist_bool "$UPGRADE582_WHITELIST_PROBE_CODE_ID" 2>/dev/null || true)"
  [[ "$wl" == "true" || "$wl" == "false" ]] \
    || upgrade582_die "IsCodeIdWhitelisted smart-query failed (factory < 1.9.0, LCD flake, or unparseable stub). Refusing pair migrate."
  echo "  IsCodeIdWhitelisted code_id=${UPGRADE582_WHITELIST_PROBE_CODE_ID} → ${wl}"
}

collect_pairs_and_assets() {
  local lines
  if [[ "${DRY_RUN:-0}" == "1" && -z "${UPGRADE582_DRY_QUERY:-}" ]]; then
    PAIR_JSON_LINES=""
    PAIR_ADDRS=()
    ASSET_ADDRS=()
    PAIR_COUNT_ONCHAIN="0"
    echo "  DRY_RUN: skipping live pairs pagination (set UPGRADE582_DRY_QUERY=1 to query)"
    return 0
  fi
  lines="$(upgrade582_enumerate_pairs)"
  PAIR_JSON_LINES="$lines"
  PAIR_ADDRS=()
  if [[ -n "$lines" ]]; then
    mapfile -t PAIR_ADDRS < <(printf '%s\n' "$lines" | jq -r '.contract_addr // empty')
  fi
  ASSET_ADDRS=()
  if [[ -n "$lines" ]]; then
    mapfile -t ASSET_ADDRS < <(upgrade582_unique_listed_assets "$lines")
  fi
  PAIR_COUNT_ONCHAIN="$(upgrade582_get_pair_count)"
  if [[ "${#PAIR_ADDRS[@]}" -eq 0 ]]; then
    pair_pages=0
  else
    pair_pages=$(( (${#PAIR_ADDRS[@]} + UPGRADE582_PAIR_PAGE_LIMIT - 1) / UPGRADE582_PAIR_PAGE_LIMIT ))
  fi
  echo "  enumerated pairs=${#PAIR_ADDRS[@]} pages=${pair_pages} GetPairCount=${PAIR_COUNT_ONCHAIN}"
  if [[ "${#PAIR_ADDRS[@]}" != "$PAIR_COUNT_ONCHAIN" ]]; then
    upgrade582_die "GetPairCount mismatch: enumerated ${#PAIR_ADDRS[@]} vs count=${PAIR_COUNT_ONCHAIN}. A pair may have been created mid-page — fail closed; re-run (do not continue)."
  fi
}

probe_contract_info() {
  if [[ "${UPGRADE582_SKIP_CONTRACT_INFO_PROBE:-0}" == "1" ]]; then
    echo "  SKIP_CONTRACT_INFO_PROBE=1"
    return 0
  fi
  echo "  LCD ContractInfo: GET $(upgrade582_lcd_base)/cosmwasm/wasm/v1/contract/{addr}"
  local code
  code="$(upgrade582_require_contract_info_code_id "$FACTORY" factory)"
  echo "  factory code_id=$code"
  local addr
  for addr in "${ASSET_ADDRS[@]+"${ASSET_ADDRS[@]}"}"; do
    code="$(upgrade582_require_contract_info_code_id "$addr" asset)"
    echo "  asset $addr code_id=$code"
  done
}

echo "[1] preflight"
[[ -n "$FACTORY" ]] || upgrade582_die "set UPGRADE582_FACTORY_ADDRESS or FACTORY_ADDRESS"
if [[ "${UPGRADE582_SKIP_STORE:-0}" != "1" && "${UPGRADE582_PROBE_ONLY:-0}" != "1" ]]; then
  need_wasm "$PAIR_WASM"
  need_wasm "$FACTORY_WASM"
fi
if [[ "${UPGRADE582_SKIP_STORE:-0}" == "1" && "${UPGRADE582_PROBE_ONLY:-0}" != "1" ]]; then
  [[ -n "${UPGRADE582_FACTORY_CODE_ID:-}" && -n "${UPGRADE582_PAIR_CODE_ID:-}" ]] \
    || upgrade582_die "UPGRADE582_SKIP_STORE=1 needs UPGRADE582_FACTORY_CODE_ID and UPGRADE582_PAIR_CODE_ID"
fi

echo ""
echo "[1b] enumerate pairs (limit: 30, start_after=last asset_infos) + GetPairCount"
collect_pairs_and_assets

echo ""
echo "[1c] ContractInfo probe (factory + every listed asset)"
probe_contract_info

if [[ "${UPGRADE582_PROBE_ONLY:-0}" == "1" ]]; then
  echo ""
  echo "PROBE_ONLY done. Paste this output onto #584 / #391 deploy-trace."
  echo "OK"
  exit 0
fi

echo ""
echo "[2] store wasm"
if [[ "${UPGRADE582_SKIP_STORE:-0}" == "1" ]]; then
  PAIR_CODE="${UPGRADE582_PAIR_CODE_ID}"
  FACTORY_CODE="${UPGRADE582_FACTORY_CODE_ID}"
  echo "  reuse pair=$PAIR_CODE factory=$FACTORY_CODE"
else
  PAIR_CODE="$(store_code "$PAIR_WASM" pair)"
  FACTORY_CODE="$(store_code "$FACTORY_WASM" factory)"
fi

echo ""
echo "[3] migrate factory → $FACTORY_CODE (must reach cw2 ≥ ${UPGRADE582_MIN_FACTORY_VERSION} before pairs)"
if [[ "${UPGRADE582_SKIP_FACTORY_MIGRATE:-0}" == "1" ]]; then
  echo "  skipped (UPGRADE582_SKIP_FACTORY_MIGRATE=1)"
else
  broadcast_and_wait "migrate factory" wasm migrate "$FACTORY" "$FACTORY_CODE" '{}' >/dev/null
fi

echo ""
echo "[4] assert factory cw2 ≥ ${UPGRADE582_MIN_FACTORY_VERSION} and IsCodeIdWhitelisted"
assert_factory_ready

echo ""
echo "[4b] UpdateConfig pair_code_id=$PAIR_CODE (new CreatePair must instantiate ${UPGRADE582_PAIR_VERSION})"
if [[ "${UPGRADE582_SKIP_UPDATE_CONFIG:-0}" == "1" ]]; then
  echo "  skipped (UPGRADE582_SKIP_UPDATE_CONFIG=1)"
else
  echo "  UPDATE_CONFIG_BEGIN pair_code_id=$PAIR_CODE"
  current_pair_code="$(upgrade582_factory_pair_code_id || true)"
  echo "  current config.pair_code_id=${current_pair_code:-<unreadable>}"
  if [[ "$current_pair_code" == "$PAIR_CODE" ]]; then
    echo "  already pair_code_id=$PAIR_CODE"
  else
    update_msg="$(jq -nc --argjson id "$PAIR_CODE" '{update_config:{pair_code_id:$id}}')"
    broadcast_and_wait "UpdateConfig pair_code_id" wasm execute "$FACTORY" "$update_msg" >/dev/null
    if [[ "${DRY_RUN:-0}" != "1" || -n "${UPGRADE582_DRY_QUERY:-}" ]]; then
      after_pair_code="$(upgrade582_factory_pair_code_id || true)"
      [[ "$after_pair_code" == "$PAIR_CODE" ]] \
        || upgrade582_die "config.pair_code_id after UpdateConfig is ${after_pair_code:-<unreadable>} want $PAIR_CODE"
      echo "  pair_code_id: ${current_pair_code:-?} → $after_pair_code"
    fi
  fi
fi

echo ""
echo "[5] paginated pair migrate → $PAIR_CODE (cw2 ${UPGRADE582_PAIR_VERSION})"
if [[ "${UPGRADE582_SKIP_PAIR_MIGRATE:-0}" == "1" ]]; then
  echo "  SKIP_PAIR_MIGRATE=1 — not broadcasting pair migrates; smoke will still hard-fail unmigrated pairs"
else
  echo "  PAIR_MIGRATE_BEGIN count=${#PAIR_ADDRS[@]}"
  pair_i=0
  for pair in "${PAIR_ADDRS[@]+"${PAIR_ADDRS[@]}"}"; do
    pair_i=$((pair_i + 1))
    echo "  pair ${pair_i}/${#PAIR_ADDRS[@]} $pair"
    if [[ "${DRY_RUN:-0}" != "1" || -n "${UPGRADE582_DRY_QUERY:-}" ]]; then
      live_code="$(upgrade582_contract_info_code_id "$pair" || true)"
      if [[ -n "$PAIR_CODE" && "$live_code" == "$PAIR_CODE" ]]; then
        echo "    skip: already code_id=$PAIR_CODE (retry-safe after RPC RST)"
        continue
      fi
    fi
    if ! broadcast_and_wait "migrate $pair" wasm migrate "$pair" "$PAIR_CODE" '{}' >/dev/null; then
      upgrade582_die "pair migrate failed at $pair — stopping (do not claim success; retry is safe)"
    fi
  done
  if [[ "${DRY_RUN:-0}" != "1" || -n "${UPGRADE582_DRY_QUERY:-}" ]]; then
    after_count="$(upgrade582_get_pair_count)"
    if [[ "${#PAIR_ADDRS[@]}" != "$after_count" ]]; then
      upgrade582_die "post-migrate GetPairCount mismatch: migrated ${#PAIR_ADDRS[@]} vs count=${after_count}"
    fi
  fi
fi

echo ""
echo "[6] optional RefreshPairAssetCodeIdsBatch (default off — migrate backfills pins)"
if [[ "${UPGRADE582_REFRESH:-0}" == "1" ]]; then
  refresh_start=""
  refresh_batch=0
  while true; do
    refresh_batch=$((refresh_batch + 1))
    [[ "$refresh_batch" -le "$UPGRADE582_REFRESH_MAX_BATCHES" ]] \
      || upgrade582_die "RefreshPairAssetCodeIdsBatch exceeded ${UPGRADE582_REFRESH_MAX_BATCHES} pages (has_more loop)"
    if [[ -z "$refresh_start" ]]; then
      refresh_msg='{"refresh_pair_asset_code_ids_batch":{"start_after":null,"limit":30}}'
    else
      refresh_msg="$(jq -nc --argjson s "$refresh_start" '{"refresh_pair_asset_code_ids_batch":{"start_after":$s,"limit":30}}')"
    fi
    echo "  refresh batch ${refresh_batch} start_after=${refresh_start:-null}"
    if [[ "${DRY_RUN:-0}" == "1" && -z "${UPGRADE582_FORCE_REFRESH_TX_JSON:-}" ]]; then
      refresh_tx_json='{"events":[{"type":"wasm","attributes":[{"key":"has_more","value":"false"}]}]}'
      echo "    DRY_RUN: parsing fixture has_more=false (set UPGRADE582_FORCE_REFRESH_TX_JSON to inject events)"
    else
      refresh_tx=""
      if ! refresh_tx="$(broadcast_and_wait "RefreshPairAssetCodeIdsBatch" wasm execute "$FACTORY" "$refresh_msg")"; then
        echo "ERROR: batch refresh reverted (likely one unlisted live id)." >&2
        upgrade582_print_batch_refresh_skip
        exit 1
      fi
      if [[ -n "${UPGRADE582_FORCE_REFRESH_TX_JSON:-}" ]]; then
        refresh_tx_json="$UPGRADE582_FORCE_REFRESH_TX_JSON"
      else
        refresh_tx_json="$(terrad_host_wait_tx_query "$refresh_tx")"
      fi
    fi
    refresh_cursor="$(upgrade582_refresh_batch_cursor "$refresh_tx_json" || true)"
    if [[ -z "$refresh_cursor" ]]; then
      upgrade582_print_batch_refresh_skip
      upgrade582_die "RefreshPairAssetCodeIdsBatch tx missing parseable wasm has_more (do not assume a single page)"
    fi
    refresh_has_more="${refresh_cursor%%$'\t'*}"
    refresh_next="${refresh_cursor#*$'\t'}"
    echo "    has_more=${refresh_has_more} next_start_after=${refresh_next:-<none>}"
    if [[ "$refresh_has_more" != "true" ]]; then
      break
    fi
    [[ -n "$refresh_next" ]] \
      || upgrade582_die "has_more=true but next_start_after missing — refusing to loop from start_after=null"
    refresh_start="$refresh_next"
  done
else
  echo "  skipped (UPGRADE582_REFRESH=0). Pins come from pair migrate ContractInfo backfill."
fi

echo ""
echo "[7] post-migrate smoke: GetAssetCodeIds + IsCodeIdWhitelisted + HybridSimulation"
echo "  NOTE: Simulation/HybridSimulation are ungated — a quote does NOT mean the pair is tradable."
if [[ "${DRY_RUN:-0}" == "1" && -z "${UPGRADE582_DRY_QUERY:-}" ]]; then
  echo "  DRY_RUN: smoke skipped (no live pairs)"
else
  if [[ "${#PAIR_ADDRS[@]}" -eq 0 ]]; then
    upgrade582_die "smoke: no pairs enumerated"
  fi
  printf '%-44s %10s %10s %s\n' "pair" "pin0" "pin1" "sim"
  for pair in "${PAIR_ADDRS[@]}"; do
    pins="$(upgrade582_query_smart "$pair" '{"get_asset_code_ids":{}}' 2>/dev/null || true)"
    pin0="$(printf '%s' "$pins" | jq -r '.code_ids[0] // empty')"
    pin1="$(printf '%s' "$pins" | jq -r '.code_ids[1] // empty')"
    if [[ ! "$pin0" =~ ^[0-9]+$ || ! "$pin1" =~ ^[0-9]+$ ]]; then
      upgrade582_die "GetAssetCodeIds failed or empty pins for $pair (pre-1.15.0 hard-errors; not 'empty ok'). body=$(printf '%s' "$pins" | tr '\n' ' ')"
    fi
    wl0="$(upgrade582_whitelist_bool "$pin0" || true)"
    wl1="$(upgrade582_whitelist_bool "$pin1" || true)"
    [[ "$wl0" == "true" && "$wl1" == "true" ]] \
      || upgrade582_die "pin not factory-whitelisted for $pair pin0=$pin0 wl0=${wl0:-empty} pin1=$pin1 wl1=${wl1:-empty} (empty wl is an LCD flake, not a missing pin)"
    asset0="$(printf '%s\n' "$PAIR_JSON_LINES" | jq -r --arg p "$pair" 'select(.contract_addr==$p) | .asset_infos[0].token.contract_addr // empty')"
    sim_ok="fail"
    if [[ -n "$asset0" ]]; then
      sim_msg="$(jq -nc --arg addr "$asset0" \
        '{hybrid_simulation:{offer_asset:{info:{token:{contract_addr:$addr}},amount:"1000"},hybrid:{pool_input:"1000",book_input:"0",max_maker_fills:1,book_start_hint:null}}}')"
      if upgrade582_query_smart "$pair" "$sim_msg" >/dev/null 2>&1; then
        sim_ok="ok"
      fi
    fi
    printf '%-44s %10s %10s %s\n' "$pair" "$pin0" "$pin1" "$sim_ok"
    [[ "$sim_ok" == "ok" ]] || upgrade582_die "HybridSimulation failed for $pair (pair still must answer queries)"
  done
fi

echo ""
echo "Done. Factory ${UPGRADE582_MIN_FACTORY_VERSION} then pairs ${UPGRADE582_PAIR_VERSION}."
echo "Record LCD probe, cw2 before/after, GetPairCount, migrate tx hashes, and the smoke table on #584 / #391."
echo "Future F6 wasm upgrades still use this script (factory first, UpdateConfig pair_code_id, LCD retries)."
echo "OK"
