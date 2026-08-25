#!/usr/bin/env bash
# GitLab #634 — LocalTerra migrate pair inventory (M634). Never columbus-5.
#
# Proves:
#   1. Factory-listed mintable with an existing CL8Y pair is discoverable
#   2. Honest adopt does **not** RegisterListedPair (M626-10 / M634-4)
#   3. After adopt the CL8Y pair is F6-frozen until governance Refresh
#   4. Refresh (ops, not the retail page) then register lists **only** that factory pair
#   5. Terraport factory probe degrades (no invented rows); GDEX stays instruction-only
#   6. Mintable with no CL8Y pair is a valid empty inventory (M634-6)
#   7. Register of a non-factory addr is rejected (T592-9)
#
# Retail `/token/migrate` never sends Refresh / pause / whitelist (grep in
# verify-issue-634). This script is the LocalTerra analogue of "confirm names
# that pair as governance-refresh; after Refresh, register only that factory pair."
# LocalTerra cw20-mintable writes cw2 `crates.io:cw20-base` (10184 analogue).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/lib/localterra-host-curl.sh
source "$REPO_ROOT/scripts/lib/localterra-host-curl.sh"
# shellcheck source=scripts/lib/e2e-terrad-tx.sh
source "$REPO_ROOT/scripts/lib/e2e-terrad-tx.sh"
# shellcheck source=scripts/lib/terrad-tx-events.sh
source "$REPO_ROOT/scripts/lib/terrad-tx-events.sh"
# shellcheck source=scripts/lib/terrad-wait-tx.sh
source "$REPO_ROOT/scripts/lib/terrad-wait-tx.sh"
# shellcheck source=scripts/lib/lcd-smart-query.sh
source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
# shellcheck source=cw20-codeid-audits/scripts/lib-layer-lt.sh
source "$REPO_ROOT/cw20-codeid-audits/scripts/lib-layer-lt.sh"
# shellcheck source=cw20-codeid-audits/scripts/lib-tax-on.sh
source "$REPO_ROOT/cw20-codeid-audits/scripts/lib-tax-on.sh"

# Columbus-5 Terraport factory — LocalTerra must not invent pairs from this probe.
TERRAPORT_FACTORY="terra1n75fgfc8clsssrm2k0fswgtzsvstdaah7la6sfu96szdu22xta0q57rqqr"

layer_require_localterra
ENV_LOCAL="$(layer_find_env_local || true)"
[[ -n "$ENV_LOCAL" ]] || {
  echo "FAIL: frontend-dapp/.env.local missing (make deploy-local)." >&2
  exit 1
}
layer_load_env_local "$ENV_LOCAL"
LCD="${VITE_TERRA_LCD_URL:-http://localhost:1317}"
LCD="${LCD%/}"

[[ -n "${VITE_FACTORY_ADDRESS:-}" && -n "${VITE_TOKEN_EMBER_ADDRESS:-}" ]] || {
  echo "FAIL: VITE_FACTORY_ADDRESS / VITE_TOKEN_EMBER_ADDRESS unset." >&2
  exit 1
}
[[ -n "${VITE_TOKEN_COMMUNITY_TAX_ADDRESS:-}" && -n "${VITE_UST1_TOKEN_ADDRESS:-}" ]] || {
  echo "FAIL: community-tax / UST1 pins unset." >&2
  exit 1
}
[[ -n "${VITE_COMMUNITY_TOKEN_LAUNCHER:-}" ]] || {
  echo "FAIL: VITE_COMMUNITY_TOKEN_LAUNCHER unset." >&2
  exit 1
}

FACTORY="$VITE_FACTORY_ADDRESS"
EMBER="$VITE_TOKEN_EMBER_ADDRESS"
TAX_TEMPLATE="$VITE_TOKEN_COMMUNITY_TAX_ADDRESS"
UST1="$VITE_UST1_TOKEN_ADDRESS"
LAUNCHER="$VITE_COMMUNITY_TOKEN_LAUNCHER"
ROUTER="${VITE_ROUTER_ADDRESS:-}"

terrad_tx() { tax_on_terrad_tx "$@"; }
exec_ok() { tax_on_exec_ok "$@"; }

lcd_contract() {
  local addr="$1"
  localterra_lcd_curl "$LCD" "/cosmwasm/wasm/v1/contract/${addr}"
}

lcd_code_id() {
  lcd_contract "$1" | jq -r '.contract_info.code_id // empty'
}

lcd_admin() {
  lcd_contract "$1" | jq -r '.contract_info.admin // empty'
}

pair_pins() {
  layer_smart "$1" '{"get_asset_code_ids":{}}' | jq -r '.code_ids // .asset_code_ids // empty'
}

pair_frozen() {
  local pair="$1" token="$2" other="$3"
  local pins live_t live_o
  pins="$(pair_pins "$pair")"
  live_t="$(lcd_code_id "$token")"
  live_o="$(lcd_code_id "$other")"
  python3 -c '
import json, sys
pins, live_t, live_o = sys.argv[1], sys.argv[2], sys.argv[3]
try:
    ids = json.loads(pins) if pins.startswith("[") else [int(x) for x in pins.replace(",", " ").split() if x]
except Exception:
    sys.stderr.write(f"FAIL: GetAssetCodeIds unreadable: {pins}\n")
    sys.exit(2)
ids = [int(x) for x in ids]
lt, lo = int(live_t), int(live_o)
# Frozen when either live id is missing from the pin pair.
if lt in ids and lo in ids:
    sys.exit(1)
sys.exit(0)
' "$pins" "$live_t" "$live_o"
}

echo "634-lt: mintable + CL8Y pair → adopt → F6 freeze → Refresh → register factory only"

EMBER_CODE="$(lcd_code_id "$EMBER")"
TAX_CODE="$(lcd_code_id "$TAX_TEMPLATE")"
[[ "$EMBER_CODE" =~ ^[0-9]+$ && "$TAX_CODE" =~ ^[0-9]+$ ]] || {
  echo "FAIL: EMBER/tax code_id missing (ember=$EMBER_CODE tax=$TAX_CODE)" >&2
  exit 1
}

# --- mintable with a CL8Y pair (honest CreatePair does not register) ---
STAMP="$(date +%s)"
GEM_INIT="$(jq -nc --arg a "$TEST_ADDRESS" \
  '{name:"MigGem",symbol:"MGEM",decimals:6,initial_balances:[{address:$a,amount:"1000000000000"}],mint:{minter:$a}}')"
GEM_TX="$(exec_ok wasm instantiate "$EMBER_CODE" "$GEM_INIT" \
  --label "634-gem-${STAMP}" --admin "$TEST_ADDRESS")"
GEM="$(tax_on_contract_from_tx "$GEM_TX")"
[[ "$GEM" == terra1* ]] || {
  echo "FAIL: mintable instantiate address missing" >&2
  exit 1
}
echo "634-lt: mintable $GEM code=$EMBER_CODE admin=$(lcd_admin "$GEM")"

PAIR="$(tax_on_create_pair "$FACTORY" "$GEM" "$EMBER")"
[[ "$PAIR" == terra1* ]] || {
  echo "FAIL: CreatePair mintable/EMBER failed" >&2
  exit 1
}
FACTORY_PAIR="$(tax_on_resolve_pair "$FACTORY" "$GEM" "$EMBER")"
[[ "$FACTORY_PAIR" == "$PAIR" ]] || {
  echo "FAIL: factory Pair lookup $FACTORY_PAIR != $PAIR" >&2
  exit 1
}
echo "634-lt: CL8Y pair $PAIR (pre-adopt inventory row)"

# Pre-adopt mintable has no IsProtocolExempt — adopt must not register.
# --- honest adopt (tax-off zeros); do not batch register ---
ADOPT="$(jq -nc \
  --arg m "$TEST_ADDRESS" --arg f "$FACTORY" --arg r "$ROUTER" \
  --arg u "$UST1" --arg l "$LAUNCHER" --argjson src "$EMBER_CODE" \
  '{
    adopt:{
      manager:$m, treasury:$m, factory:$f,
      router: (if $r == "" then null else $r end),
      ust1:$u, cmm_treasury:$m, official_launcher:$l,
      buy_bps:0, sell_bps:0, transfer_bps:null,
      max_buy_bps:0, max_sell_bps:0, max_transfer_bps:0,
      source_code_id:$src
    }
  }')"
MIG_TX="$(exec_ok wasm migrate "$GEM" "$TAX_CODE" "$ADOPT")"
echo "634-lt: adopt tx=$MIG_TX → code=$(lcd_code_id "$GEM")"
[[ "$(lcd_code_id "$GEM")" == "$TAX_CODE" ]] || {
  echo "FAIL: adopt did not move $GEM onto tax code $TAX_CODE" >&2
  exit 1
}

ORIGIN="$(layer_smart "$GEM" '{"get_migrate_origin":{}}')"
echo "$ORIGIN" | jq -e '.source_cw2 | type == "string" and test("cw20-mintable|cw20-base")' >/dev/null || {
  echo "FAIL: GetMigrateOrigin.source_cw2 not honest mintable/base: $ORIGIN" >&2
  exit 1
}
LAUNCHER_ORIGIN="$(layer_smart "$GEM" '{"get_launcher_origin":{}}')"
echo "$LAUNCHER_ORIGIN" | jq -e --arg l "$LAUNCHER" '.launcher == $l' >/dev/null || {
  echo "FAIL: GetLauncherOrigin.launcher != official launcher: $LAUNCHER_ORIGIN" >&2
  exit 1
}
CFG="$(layer_smart "$GEM" '{"get_config":{}}')"
echo "$CFG" | jq -e --arg m "$TEST_ADDRESS" '.manager == $m' >/dev/null || {
  echo "FAIL: adopt manager != test1: $CFG" >&2
  exit 1
}

POST_EXEMPT="$(layer_smart "$GEM" \
  "$(jq -nc --arg p "$PAIR" '{is_protocol_exempt:{address:$p}}')")"
echo "$POST_EXEMPT" | jq -e '.protocol == false' >/dev/null || {
  echo "FAIL: adopt registered the CL8Y pair (M626-10 / M634-4): $POST_EXEMPT" >&2
  exit 1
}
echo "634-lt: adopt left pair unregistered (register is a later tool)"

if pair_frozen "$PAIR" "$GEM" "$EMBER"; then
  echo "634-lt: CL8Y pair F6-frozen after adopt (governance Refresh required)"
else
  echo "FAIL: expected F6 freeze after mintable→tax adopt (live ids vs pair pins)" >&2
  echo "  pins=$(pair_pins "$PAIR") live_gem=$(lcd_code_id "$GEM") live_ember=$(lcd_code_id "$EMBER")" >&2
  exit 1
fi

# Register before Refresh must stay a later tool — on-chain register is still
# permissionless, but F6 writes fail-closed. We do **not** register here.
# Ops Refresh (not the retail page).
REFRESH="$(jq -nc --arg p "$PAIR" '{refresh_pair_asset_code_ids:{pair:$p}}')"
exec_ok wasm execute "$FACTORY" "$REFRESH" >/dev/null
if pair_frozen "$PAIR" "$GEM" "$EMBER"; then
  echo "FAIL: pair still frozen after RefreshPairAssetCodeIds $PAIR" >&2
  echo "  pins=$(pair_pins "$PAIR") live_gem=$(lcd_code_id "$GEM")" >&2
  exit 1
fi
echo "634-lt: Refresh unfroze $PAIR (ops only — migrate page must not send this)"

REG="$(jq -nc --arg p "$PAIR" '{register_listed_pair:{pair:$p}}')"
exec_ok wasm execute "$GEM" "$REG" >/dev/null
REG_EXEMPT="$(layer_smart "$GEM" \
  "$(jq -nc --arg p "$PAIR" '{is_protocol_exempt:{address:$p}}')")"
echo "$REG_EXEMPT" | jq -e '.protocol == true' >/dev/null || {
  echo "FAIL: register did not list factory pair: $REG_EXEMPT" >&2
  exit 1
}
echo "634-lt: register listed only factory pair $PAIR"

# Non-factory / Terraport-shaped addr must not register.
FAKE="terra1n75fgfc8clsssrm2k0fswgtzsvstdaah7la6sfu96szdu22xta0q57rqqr"
FAKE_MSG="$(jq -nc --arg p "$FAKE" '{register_listed_pair:{pair:$p}}')"
FAKE_OUT="$(tax_on_try_tx wasm execute "$GEM" "$FAKE_MSG")"
if ! layer_execute_rejected "$FAKE_OUT"; then
  echo "FAIL: RegisterListedPair accepted a non-factory addr $FAKE" >&2
  printf '%s\n' "$FAKE_OUT" >&2
  exit 1
fi
echo "634-lt: non-factory register rejected (T592-9 / M634-4)"

# Terraport factory on LocalTerra: probe must not invent a pair for this gem.
set +e
TP_RAW="$(lcd_smart_query_raw "$LCD" "$TERRAPORT_FACTORY" \
  "$(jq -nc --arg a "$GEM" '{pair:{asset_infos:[{token:{contract_addr:$a}},{native_token:{denom:"uluna"}}]}}')" 2>/dev/null)"
TP_EC=$?
set -e
if [[ "$TP_EC" -eq 0 ]]; then
  TP_PAIR="$(lcd_decode_smart_data "$TP_RAW" 2>/dev/null | jq -r '.contract_addr // .pair.contract_addr // empty' || true)"
  if [[ "$TP_PAIR" == terra1* ]]; then
    echo "FAIL: LocalTerra invented a Terraport pair $TP_PAIR for $GEM" >&2
    exit 1
  fi
fi
echo "634-lt: Terraport factory probe empty/error (incomplete inventory OK — M634-8)"

# Empty CL8Y: second mintable, no CreatePair.
EMPTY_INIT="$(jq -nc --arg a "$TEST_ADDRESS" \
  '{name:"NoPool",symbol:"NPOL",decimals:6,initial_balances:[{address:$a,amount:"1"}],mint:{minter:$a}}')"
EMPTY_TX="$(exec_ok wasm instantiate "$EMBER_CODE" "$EMPTY_INIT" \
  --label "634-empty-${STAMP}" --admin "$TEST_ADDRESS")"
EMPTY="$(tax_on_contract_from_tx "$EMPTY_TX")"
EMPTY_PAIR="$(tax_on_resolve_pair "$FACTORY" "$EMPTY" "$EMBER")"
[[ -z "$EMPTY_PAIR" ]] || {
  echo "FAIL: empty mintable unexpectedly has factory pair $EMPTY_PAIR" >&2
  exit 1
}
echo "634-lt: no-CL8Y mintable $EMPTY is a valid empty inventory (Create Pair next)"

echo "==> GitLab #634 LocalTerra live rungs passed"
echo "634-lt: gem=$GEM pair=$PAIR empty=$EMPTY"
