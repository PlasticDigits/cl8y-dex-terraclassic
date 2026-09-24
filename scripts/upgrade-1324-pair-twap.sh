#!/usr/bin/env bash
# Forgejo #1324 — migrate every columbus-5 factory pair to cw2 1.18.0.
#
# Pair TWAP price×dt and both running cumulatives are Uint256. A stored u128
# decimal string zero-extends on load. MigrateMsg is {} and must not rewrite
# OBSERVATIONS or add a cumulative setter (#1224 / #1322).
#
# Factory stays on cw2 1.10.0 / code 11629. This script does not store or
# migrate factory wasm. It does UpdateConfig { pair_code_id } so the next
# CreatePair instantiates 1.18.0, then wasm-migrates each existing pair.
#
# Keys on this workstation (terrad keys list):
#   store:    cl8ydeploy     terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv
#   gov:      multisig_2of3 terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7
#   signers:  multisig1     terra13d6jycp9hv8u64t92j2htdr53sn9f88r4uqtxm
#             multisig2     terra1lsewv7zjf2pe535lpdgh9dx2n928yn3ker76mq
#   spare:    multisig3 is in the 2-of-3 set and is not a default signer
#
# The on-disk smartcontracts/artifacts/cl8y_dex_pair.wasm from 2026-09-12
# embeds cw2 1.17.0. Rebuild before a live store:
#   make build-optimized
#
# Usage:
#   UPGRADE1324_PROBE_ONLY=1 ./scripts/upgrade-1324-pair-twap.sh  # read-only post-migrate check
#   DRY_RUN=1 ./scripts/upgrade-1324-pair-twap.sh
#   ./scripts/upgrade-1324-pair-twap.sh
#   UPGRADE1324_SKIP_STORE=1 UPGRADE1324_PAIR_CODE_ID=<id> \
#     ./scripts/upgrade-1324-pair-twap.sh
#
# Does not broadcast a swap, provide, or withdraw. After OK, confirm a small
# reserve move on UST1/USTR by hand. Indexer/dApp deploy of c17e71d3 is separate.
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

ARTIFACTS="${UPGRADE1324_ARTIFACTS:-$REPO_ROOT/smartcontracts/artifacts}"
PAIR_WASM="${UPGRADE1324_PAIR_WASM:-$ARTIFACTS/cl8y_dex_pair.wasm}"
FACTORY="${UPGRADE1324_FACTORY_ADDRESS:-${FACTORY_ADDRESS:-$UST1_SEC_FACTORY_ADDRESS}}"
DEX_GOV="${UPGRADE1324_DEX_GOVERNANCE:-terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7}"
UST1_USTR="${UPGRADE1324_UST1_USTR:-terra16vxrhpvpcucu05y0nr862vf9hnqeh274uaff4s7hz4n0ea74006qf5hgqy}"
PAIR_VERSION="${UPGRADE1324_PAIR_VERSION:-1.18.0}"
LCD_URL="${LCD_URL:-${TERRA_LCD_URL:-https://terra-classic-lcd.publicnode.com}}"
LCD_URL="${LCD_URL%/}"
LOCALTERRA_CURL_MAX_TIME="${LOCALTERRA_CURL_MAX_TIME:-25}"
LOCALTERRA_CURL_CONNECT_TIMEOUT="${LOCALTERRA_CURL_CONNECT_TIMEOUT:-5}"
TERRAD_HOST_KEY="${TERRAD_HOST_KEY:-cl8ydeploy}"

upgrade1324_die() { echo "ERROR: $*" >&2; exit 1; }

upgrade1324_assert_pair_wasm() {
  local wasm="$1"
  [[ -f "$wasm" ]] || upgrade1324_die "missing $wasm — run: make build-optimized (checked-in artifact is cw2 1.17.0, not ${PAIR_VERSION})"
  local err
  if ! err="$(python3 - "$wasm" "$PAIR_VERSION" 2>&1 <<'PY'
import sys
from pathlib import Path
data = Path(sys.argv[1]).read_bytes()
want = sys.argv[2].encode()
name = b"cl8y-dex-pair"
idx = data.find(name)
if idx < 0:
    sys.exit("wasm missing cw2 name cl8y-dex-pair")
window = data[idx:idx + 48]
if want not in window:
    sys.exit(
        "pair wasm cw2 next to cl8y-dex-pair is not %s (rebuild with make build-optimized)"
        % want.decode()
    )
print("cw2 %s" % want.decode())
PY
  )"; then
    upgrade1324_die "refusing store: ${err:-pair wasm is not cw2 ${PAIR_VERSION}}"
  fi
  echo "  $err ($wasm)"
}

upgrade1324_obs_raw() {
  local addr="$1"
  local index="$2"
  local key_b64 json data
  key_b64="$(python3 -c 'import base64,sys,urllib.parse; ns=b"observations"; i=int(sys.argv[1]); key=len(ns).to_bytes(2,"big")+ns+i.to_bytes(2,"big"); print(urllib.parse.quote(base64.b64encode(key).decode(), safe=""))' "$index")"
  json="$(upgrade582_lcd_get "/cosmwasm/wasm/v1/contract/${addr}/raw/${key_b64}" 2>/dev/null || true)"
  data="$(printf '%s' "$json" | jq -r '.data // empty')"
  [[ -n "$data" ]] || return 1
  printf '%s' "$data" | base64 -d
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
  [[ -n "$tx_hash" ]] || upgrade1324_die "no txhash from: $label"
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
  echo "    DEX 2-of-3 (multisig1 + multisig2). Passphrase already captured unless this is the first sign." >&2
  local out tx_hash
  out="$("$SCRIPT_DIR/multisig-2of3-host-tx.sh" "$@" | tee /dev/stderr)"
  tx_hash="$(printf '%s' "$out" | awk '/^OK / { print $2 }' | tail -1)"
  [[ -n "$tx_hash" ]] || upgrade1324_die "no txhash from 2-of-3: $label"
  printf '%s' "$tx_hash"
}

store_pair() {
  local tx_hash code_id
  tx_hash="$(broadcast_and_wait "store pair" wasm store "$PAIR_WASM")"
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "0"
    return 0
  fi
  code_id="$(terrad_host_code_id_from_store_tx "$tx_hash")"
  [[ -n "$code_id" ]] || upgrade1324_die "could not parse pair code_id from store tx"
  echo "    code_id: $code_id" >&2
  printf '%s' "$code_id"
}

echo "=============================================="
echo "Forgejo #1324 pair cw2 ${PAIR_VERSION} migrate"
echo "=============================================="
echo "Factory:    $FACTORY"
echo "DEX gov:    $DEX_GOV"
echo "UST1/USTR:  $UST1_USTR"
echo "LCD:        $LCD_URL"
echo "Chain:      ${TERRAD_HOST_CHAIN_ID:-}"
echo "Store key:  ${TERRAD_HOST_KEY}"
echo "DRY_RUN:    ${DRY_RUN:-0}"
echo "PROBE_ONLY: ${UPGRADE1324_PROBE_ONLY:-0}"
echo "Migrate:    '{}'  (does not rewrite OBSERVATIONS)"
echo ""

[[ "${TERRAD_HOST_CHAIN_ID:-columbus-5}" == "columbus-5" ]] \
  || upgrade1324_die "refusing chain ${TERRAD_HOST_CHAIN_ID} (this script is columbus-5 only)"
[[ "$TERRAD_HOST_KEY" == "cl8ydeploy" || "${UPGRADE1324_ALLOW_STORE_KEY:-0}" == "1" ]] \
  || upgrade1324_die "store key is ${TERRAD_HOST_KEY}; columbus-5 store is cl8ydeploy"
[[ -n "$FACTORY" ]] || upgrade1324_die "set UPGRADE1324_FACTORY_ADDRESS"

if [[ "${UPGRADE1324_PROBE_ONLY:-0}" != "1" && "${UPGRADE1324_SKIP_STORE:-0}" != "1" ]]; then
  echo "[wasm] require cw2 ${PAIR_VERSION} embedded next to cl8y-dex-pair"
  upgrade1324_assert_pair_wasm "$PAIR_WASM"
fi

if [[ "${UPGRADE1324_SKIP_STORE:-0}" == "1" && "${UPGRADE1324_PROBE_ONLY:-0}" != "1" ]]; then
  [[ "${UPGRADE1324_PAIR_CODE_ID:-}" =~ ^[0-9]+$ ]] \
    || upgrade1324_die "UPGRADE1324_SKIP_STORE=1 needs numeric UPGRADE1324_PAIR_CODE_ID"
fi

query_live() {
  [[ "${DRY_RUN:-0}" == "1" && -z "${UPGRADE1324_DRY_QUERY:-}" && "${UPGRADE1324_PROBE_ONLY:-0}" != "1" ]] && return 1
  return 0
}

if ! query_live; then
  echo "[dry] skipping LCD enumeration (set UPGRADE1324_DRY_QUERY=1 to query)"
  echo "[dry] would store pair wasm as cl8ydeploy, UpdateConfig pair_code_id, then 2-of-3 wasm migrate PAIR CODE '{}'"
  echo "DRY_RUN skip: 2-of-3 wasm migrate <pair> <code> '{}'"
  echo "OK"
  exit 0
fi

echo "[1] enumerate pairs + factory cw2 (no factory migrate)"
FACTORY_CW2="$(upgrade582_cw2_version "$FACTORY" || true)"
FACTORY_CODE="$(upgrade582_contract_info_code_id "$FACTORY" || true)"
echo "  factory code_id=${FACTORY_CODE:-<unreadable>} cw2=${FACTORY_CW2:-<unreadable>}"
upgrade582_version_ge "${FACTORY_CW2:-0}" "$UPGRADE582_MIN_FACTORY_VERSION" \
  || upgrade1324_die "factory cw2 ${FACTORY_CW2:-unreadable} < ${UPGRADE582_MIN_FACTORY_VERSION}; refusing pair migrate"
[[ "$FACTORY_CODE" == "11629" || "${UPGRADE1324_ALLOW_FACTORY_CODE:-0}" == "1" ]] \
  || upgrade1324_die "factory code_id is ${FACTORY_CODE:-empty}, want 11629 (set UPGRADE1324_ALLOW_FACTORY_CODE=1 only if that id is still cw2 ≥ ${UPGRADE582_MIN_FACTORY_VERSION})"

mapfile -t PAIR_JSON < <(upgrade582_enumerate_pairs)
PAIR_ADDRS=()
declare -A PAIR_CODE_BY_ADDR=()
if [[ "${#PAIR_JSON[@]}" -gt 0 ]]; then
  mapfile -t PAIR_ADDRS < <(printf '%s\n' "${PAIR_JSON[@]}" | jq -r '.contract_addr // empty')
fi
PAIR_COUNT="$(upgrade582_get_pair_count)"
echo "  enumerated=${#PAIR_ADDRS[@]} GetPairCount=${PAIR_COUNT}"
[[ "${#PAIR_ADDRS[@]}" == "$PAIR_COUNT" ]] \
  || upgrade1324_die "GetPairCount mismatch: enumerated ${#PAIR_ADDRS[@]} vs count=${PAIR_COUNT}"
[[ "${#PAIR_ADDRS[@]}" -gt 0 ]] || upgrade1324_die "no factory pairs"

found_ustr=0
for pair in "${PAIR_ADDRS[@]}"; do
  info="$(upgrade582_contract_info_json "$pair" 2>/dev/null || true)"
  admin="$(printf '%s' "$info" | jq -r '.contract_info.admin // empty')"
  code="$(printf '%s' "$info" | jq -r '.contract_info.code_id // empty')"
  label="$(printf '%s' "$info" | jq -r '.contract_info.label // empty')"
  [[ "$admin" == "$DEX_GOV" ]] || upgrade1324_die "pair $pair admin is ${admin:-empty}, want $DEX_GOV"
  echo "  $code  $label  $pair"
  PAIR_CODE_BY_ADDR["$pair"]="$code"
  [[ "$pair" == "$UST1_USTR" ]] && found_ustr=1
done
[[ "$found_ustr" == "1" ]] || upgrade1324_die "UST1/USTR $UST1_USTR is not in the factory pair list"

if [[ "${UPGRADE1324_PROBE_ONLY:-0}" == "1" ]]; then
  echo ""
  echo "[probe] verify all listed pairs + UST1/USTR after migration (read-only)"
  current_pair_code="$(upgrade582_factory_pair_code_id || true)"
  [[ "$current_pair_code" =~ ^[0-9]+$ ]] \
    || upgrade1324_die "factory config.pair_code_id is unreadable"
  echo "  factory pair_code_id=$current_pair_code"
  for pair in "${PAIR_ADDRS[@]}"; do
    code="${PAIR_CODE_BY_ADDR[$pair]}"
    cw2="$(upgrade582_cw2_version "$pair" || true)"
    [[ "$code" == "$current_pair_code" ]] \
      || upgrade1324_die "$pair code_id=$code differs from factory pair_code_id=$current_pair_code"
    [[ "$cw2" == "$PAIR_VERSION" ]] \
      || upgrade1324_die "$pair cw2 is ${cw2:-empty}, want $PAIR_VERSION"
    echo "  cw2=$cw2 code_id=$code $pair"
  done
  info="$(upgrade582_query_smart "$UST1_USTR" '{"oracle_info":{}}')"
  obs_stored="$(printf '%s' "$info" | jq -r '.observations_stored // 0')"
  [[ "$obs_stored" =~ ^[0-9]+$ && "$obs_stored" -gt 0 ]] \
    || upgrade1324_die "UST1/USTR has no readable stored observations"
  obs="$(upgrade582_query_smart "$UST1_USTR" '{"observe":{"seconds_ago":[0,60]}}')" \
    || upgrade1324_die "UST1/USTR Observe [0,60] failed after migrate"
  na="$(printf '%s' "$obs" | jq -r '.price_a_cumulatives | length')"
  nb="$(printf '%s' "$obs" | jq -r '.price_b_cumulatives | length')"
  [[ "$na" == "2" && "$nb" == "2" ]] \
    || upgrade1324_die "Observe returned a=$na b=$nb, want 2 values each: $obs"
  echo "  observations_stored=$obs_stored Observe [0,60] returned 2 offsets per side"
  echo "OK"
  exit 0
fi

if [[ "${DRY_RUN:-0}" != "1" ]]; then
  terrad_host_ensure_keyring_pass
fi

echo ""
echo "[2] store pair wasm (cl8ydeploy). Factory wasm is not stored."
if [[ "${UPGRADE1324_SKIP_STORE:-0}" == "1" ]]; then
  PAIR_CODE="${UPGRADE1324_PAIR_CODE_ID}"
  echo "  reuse pair code_id=$PAIR_CODE"
else
  PAIR_CODE="$(store_pair)"
fi
[[ "$PAIR_CODE" =~ ^[0-9]+$ ]] || upgrade1324_die "pair code id is not numeric: ${PAIR_CODE:-empty}"

echo ""
echo "[3] UpdateConfig pair_code_id=$PAIR_CODE (factory migrate skipped)"
current_pair_code="$(upgrade582_factory_pair_code_id || true)"
echo "  current config.pair_code_id=${current_pair_code:-<unreadable>}"
if [[ "$current_pair_code" == "$PAIR_CODE" ]]; then
  echo "  already pair_code_id=$PAIR_CODE"
else
  update_msg="$(jq -nc --argjson id "$PAIR_CODE" '{update_config:{pair_code_id:$id}}')"
  gov_tx "UpdateConfig pair_code_id" wasm execute "$FACTORY" "$update_msg" >/dev/null
  if [[ "${DRY_RUN:-0}" != "1" ]]; then
    after_pair_code="$(upgrade582_factory_pair_code_id || true)"
    [[ "$after_pair_code" == "$PAIR_CODE" ]] \
      || upgrade1324_die "config.pair_code_id after UpdateConfig is ${after_pair_code:-<unreadable>} want $PAIR_CODE"
    echo "  pair_code_id: ${current_pair_code:-?} → $after_pair_code"
  fi
fi

echo ""
echo "[4] migrate ${#PAIR_ADDRS[@]} pairs → $PAIR_CODE with '{}'"
# Unbrick the saturated pair before the rest of the fleet.
ORDERED_PAIRS=("$UST1_USTR")
for pair in "${PAIR_ADDRS[@]}"; do
  [[ "$pair" == "$UST1_USTR" ]] && continue
  ORDERED_PAIRS+=("$pair")
done
pair_i=0
for pair in "${ORDERED_PAIRS[@]}"; do
  pair_i=$((pair_i + 1))
  echo "  pair ${pair_i}/${#PAIR_ADDRS[@]} $pair"
  live_code="$(upgrade582_contract_info_code_id "$pair" || true)"
  if [[ "$live_code" == "$PAIR_CODE" ]]; then
    echo "    skip: already code_id=$PAIR_CODE"
    continue
  fi
  info_json="$(upgrade582_query_smart "$pair" '{"oracle_info":{}}')"
  obs_index="$(printf '%s' "$info_json" | jq -r '.observation_index // empty')"
  [[ "$obs_index" =~ ^[0-9]+$ ]] || upgrade1324_die "oracle_info.observation_index missing on $pair"
  before_obs="$(upgrade1324_obs_raw "$pair" "$obs_index" || true)"
  [[ -n "$before_obs" ]] || upgrade1324_die "could not read observations[$obs_index] on $pair before migrate"
  gov_tx "migrate $pair" wasm migrate "$pair" "$PAIR_CODE" '{}' >/dev/null
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    continue
  fi
  after_code="$(upgrade582_contract_info_code_id "$pair" || true)"
  [[ "$after_code" == "$PAIR_CODE" ]] || upgrade1324_die "$pair code_id is ${after_code:-empty} after migrate, want $PAIR_CODE"
  after_cw2="$(upgrade582_cw2_version "$pair" || true)"
  [[ "$after_cw2" == "$PAIR_VERSION" ]] || upgrade1324_die "$pair cw2 is ${after_cw2:-empty}, want $PAIR_VERSION"
  after_obs="$(upgrade1324_obs_raw "$pair" "$obs_index" || true)"
  [[ "$after_obs" == "$before_obs" ]] \
    || upgrade1324_die "observations[$obs_index] changed on $pair — migrate must leave OBSERVATIONS in place"
  echo "    cw2=$after_cw2 observations[$obs_index] unchanged"
done

echo ""
echo "[5] UST1/USTR Observe [0, 60]"
if [[ "${DRY_RUN:-0}" == "1" ]]; then
  echo "  DRY_RUN skip Observe"
else
  obs="$(upgrade582_query_smart "$UST1_USTR" '{"observe":{"seconds_ago":[0,60]}}')" \
    || upgrade1324_die "Observe failed on $UST1_USTR after migrate"
  na="$(printf '%s' "$obs" | jq -r '.price_a_cumulatives | length')"
  nb="$(printf '%s' "$obs" | jq -r '.price_b_cumulatives | length')"
  [[ "$na" == "2" && "$nb" == "2" ]] || upgrade1324_die "Observe returned a=$na b=$nb, want 2 and 2: $obs"
  echo "  Observe ok"
  printf '%s\n' "$obs" | jq .
fi

echo ""
echo "Done. Pairs are cw2 ${PAIR_VERSION}. Factory was not migrated."
echo "UST1/USTR $UST1_USTR should accept swap / provide / withdraw again."
echo "This script does not send that reserve-moving tx."
echo "Indexer https://indexer.dex.cl8y.com and dApp https://dex.cl8y.com still need a deploy at or after c17e71d3 (candle usd_leg, /api/v1/evidence/daily, ALPHA charts)."
echo "OK"
