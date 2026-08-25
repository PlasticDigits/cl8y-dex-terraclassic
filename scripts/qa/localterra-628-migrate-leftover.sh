#!/usr/bin/env bash
# GitLab #628 — LocalTerra migrate leftovers P3 / P7 / P11. Never columbus-5.
# Script: localterra-628-migrate-leftover.sh
#
# Proves (M628-6):
#   P3  factory-listed mintable adopt — balances + total_supply unchanged,
#       inbound Transfer to a CL8Y pair 1:1, wasm admin CMM
#   P7  pair pinned to the old source id fail-closes until Refresh;
#       after Refresh + register, extra-debit sell works
#   P11 adopted token answers GetFeatures / GetConfig (Manage tax SKUs)
#
# Do not Refresh a pair whose other asset is unlisted.
# Do not RegisterListedPair a Terraport/GDEX addr (sibling #634).
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
CMM="${VITE_CMM_GOVERNANCE_ADDR:-$TEST_ADDRESS}"
SELL_BPS="${VERIFY628_SELL_BPS:-100}"
PROVIDE_RAW="${VERIFY628_PROVIDE_RAW:-${TAX_ON_PROVIDE_RAW}}"
SWAP_RAW="${VERIFY628_SWAP_RAW:-${TAX_ON_SWAP_RAW}}"
XFER_RAW="${VERIFY628_XFER_RAW:-${TRANSFER_RAW}}"
GENESIS="1000000000000"

ensure_lt_key() {
  local name="$1" purpose="$2"
  local addr
  addr="$(docker exec "$CONTAINER" terrad keys show "$name" -a --keyring-backend test 2>/dev/null \
    || true)"
  if [[ -z "$addr" ]]; then
    echo "628-lt: adding LocalTerra key $name ($purpose)" >&2
    docker exec "$CONTAINER" terrad keys add "$name" --keyring-backend test --output json >/dev/null
    addr="$(docker exec "$CONTAINER" terrad keys show "$name" -a --keyring-backend test \
      --output json | jq -r '.address // empty')"
  fi
  [[ "$addr" == terra1* ]] || {
    echo "FAIL: could not resolve $name address" >&2
    exit 1
  }
  printf '%s' "$addr"
}

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
if int(live_t) in ids and int(live_o) in ids:
    sys.exit(1)
sys.exit(0)
' "$pins" "$live_t" "$live_o"
}

terrad_tx() { tax_on_terrad_tx "$@"; }
exec_ok() { tax_on_exec_ok "$@"; }

echo "628-lt: mintable adopt leftovers P3 / P7 / P11 (never columbus-5)"

TREASURY="$(ensure_lt_key test2 "treasury sink; not the trader")"
TRADER="$(ensure_lt_key test3 "non-manager extra-debit seller")"
[[ "$TREASURY" != "$TEST_ADDRESS" && "$TRADER" != "$TEST_ADDRESS" && "$TRADER" != "$TREASURY" ]] || {
  echo "FAIL: manager/treasury/trader must be three wallets (M628-6 extra-debit)" >&2
  exit 1
}

EMBER_CODE="$(lcd_code_id "$EMBER")"
TAX_CODE="$(lcd_code_id "$TAX_TEMPLATE")"
[[ "$EMBER_CODE" =~ ^[0-9]+$ && "$TAX_CODE" =~ ^[0-9]+$ ]] || {
  echo "FAIL: EMBER/tax code_id missing (ember=$EMBER_CODE tax=$TAX_CODE)" >&2
  exit 1
}
if tax_on_is_forbidden_code "$TAX_CODE"; then
  echo "FAIL: local tax pin $TAX_CODE collides with a columbus-5 id" >&2
  exit 1
fi

STAMP="$(date +%s)"
GEM_INIT="$(jq -nc --arg a "$TEST_ADDRESS" --arg g "$GENESIS" \
  '{name:"LefGem",symbol:"LGEM",decimals:6,initial_balances:[{address:$a,amount:$g}],mint:{minter:$a}}')"
GEM_TX="$(exec_ok wasm instantiate "$EMBER_CODE" "$GEM_INIT" \
  --label "628-gem-${STAMP}" --admin "$TEST_ADDRESS")"
GEM="$(tax_on_contract_from_tx "$GEM_TX")"
[[ "$GEM" == terra1* ]] || {
  echo "FAIL: mintable instantiate address missing" >&2
  exit 1
}
echo "628-lt: mintable $GEM code=$EMBER_CODE admin=$(lcd_admin "$GEM")"

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
echo "628-lt: CL8Y pair $PAIR (pre-adopt)"

# Liquidity before adopt so P7 can attempt a swap against a live pool.
tax_on_provide_1to1 "$GEM" "$EMBER" "$PAIR" "$PROVIDE_RAW"

SUPPLY_BEFORE="$(layer_cw20_token_info "$GEM" | jq -r '.total_supply')"
USER_BEFORE="$(layer_cw20_balance "$GEM" "$TEST_ADDRESS")"
PAIR_BEFORE="$(layer_cw20_balance "$GEM" "$PAIR")"
[[ "$SUPPLY_BEFORE" == "$GENESIS" ]] || {
  echo "FAIL: pre-adopt total_supply $SUPPLY_BEFORE != $GENESIS" >&2
  exit 1
}

ADOPT="$(jq -nc \
  --arg m "$TEST_ADDRESS" --arg t "$TREASURY" --arg f "$FACTORY" --arg r "$ROUTER" \
  --arg u "$UST1" --arg l "$LAUNCHER" --arg c "$CMM" --argjson src "$EMBER_CODE" \
  --argjson sell "$SELL_BPS" \
  '{
    adopt:{
      manager:$m, treasury:$t, factory:$f,
      router: (if $r == "" then null else $r end),
      ust1:$u, cmm_treasury:$c, official_launcher:$l,
      buy_bps:0, sell_bps:$sell, transfer_bps:null,
      max_buy_bps:0, max_sell_bps:$sell, max_transfer_bps:0,
      source_code_id:$src
    }
  }')"
MIG_TX="$(exec_ok wasm migrate "$GEM" "$TAX_CODE" "$ADOPT")"
echo "628-lt: adopt tx=$MIG_TX → code=$(lcd_code_id "$GEM")"
[[ "$(lcd_code_id "$GEM")" == "$TAX_CODE" ]] || {
  echo "FAIL: adopt did not move $GEM onto tax code $TAX_CODE" >&2
  exit 1
}

# Retail page bundles MsgUpdateAdmin → CMM. LocalTerra CMM stand-in is test1.
if [[ "$(lcd_admin "$GEM")" != "$CMM" ]]; then
  exec_ok wasm set-contract-admin "$GEM" "$CMM" >/dev/null
fi
[[ "$(lcd_admin "$GEM")" == "$CMM" ]] || {
  echo "FAIL: wasm admin $(lcd_admin "$GEM") != CMM $CMM (P3)" >&2
  exit 1
}

SUPPLY_AFTER="$(layer_cw20_token_info "$GEM" | jq -r '.total_supply')"
USER_AFTER="$(layer_cw20_balance "$GEM" "$TEST_ADDRESS")"
PAIR_AFTER="$(layer_cw20_balance "$GEM" "$PAIR")"
[[ "$SUPPLY_AFTER" == "$SUPPLY_BEFORE" && "$USER_AFTER" == "$USER_BEFORE" && "$PAIR_AFTER" == "$PAIR_BEFORE" ]] || {
  echo "FAIL: adopt mutated balances/supply (P3)" >&2
  echo "  supply $SUPPLY_BEFORE -> $SUPPLY_AFTER user $USER_BEFORE -> $USER_AFTER pair $PAIR_BEFORE -> $PAIR_AFTER" >&2
  exit 1
}

ORIGIN="$(layer_smart "$GEM" '{"get_migrate_origin":{}}')"
echo "$ORIGIN" | jq -e '.source_cw2 | type == "string" and test("cw20-mintable|cw20-base")' >/dev/null || {
  echo "FAIL: GetMigrateOrigin.source_cw2 not honest mintable/base: $ORIGIN" >&2
  exit 1
}

# P3: inbound Transfer to the CL8Y pair stays 1:1 (transfer tax off).
XFER="$(jq -nc --arg p "$PAIR" --arg amt "$XFER_RAW" '{transfer:{recipient:$p,amount:$amt}}')"
PAIR_X0="$(layer_cw20_balance "$GEM" "$PAIR")"
exec_ok wasm execute "$GEM" "$XFER" >/dev/null
PAIR_X1="$(layer_cw20_balance_changed "$GEM" "$PAIR" "$PAIR_X0")"
python3 -c '
import sys
b, a, amt = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
if a - b != amt:
    sys.stderr.write(f"FAIL: inbound Transfer pair credit {b}->{a} != {amt} (P3)\n")
    sys.exit(1)
print(f"628-lt: P3 inbound Transfer 1:1 ({amt})")
' "$PAIR_X0" "$PAIR_X1" "$XFER_RAW"

if pair_frozen "$PAIR" "$GEM" "$EMBER"; then
  echo "628-lt: P7 pair F6-frozen after adopt (Refresh required)"
else
  echo "FAIL: expected F6 freeze after mintable→tax adopt" >&2
  echo "  pins=$(pair_pins "$PAIR") live_gem=$(lcd_code_id "$GEM") live_ember=$(lcd_code_id "$EMBER")" >&2
  exit 1
fi

# P7: swap fail-closes while pins still point at the old source id.
SWAP_HOOK='{"swap":{"max_spread":"1"}}'
SWAP_OUT="$(tax_on_try_tx wasm execute "$GEM" "$(jq -nc \
  --arg c "$PAIR" --arg amt "$SWAP_RAW" --arg m "$(printf '%s' "$SWAP_HOOK" | base64 -w0 2>/dev/null || printf '%s' "$SWAP_HOOK" | base64 | tr -d '\n')" \
  '{send:{contract:$c,amount:$amt,msg:$m}}')")"
if ! layer_execute_rejected "$SWAP_OUT"; then
  echo "FAIL: sell succeeded before Refresh (P7 fail-close)" >&2
  printf '%s\n' "$SWAP_OUT" >&2
  exit 1
fi
echo "628-lt: P7 swap rejected while frozen"

REFRESH="$(jq -nc --arg p "$PAIR" '{refresh_pair_asset_code_ids:{pair:$p}}')"
exec_ok wasm execute "$FACTORY" "$REFRESH" >/dev/null
if pair_frozen "$PAIR" "$GEM" "$EMBER"; then
  echo "FAIL: pair still frozen after RefreshPairAssetCodeIds $PAIR" >&2
  exit 1
fi
echo "628-lt: Refresh unfroze $PAIR (ops only — migrate page must not send this)"

tax_on_register_listed "$GEM" "$PAIR"

# Manager (test1) skips sell tax (R633). Extra-debit uses test3; treasury stays test2.
FUND3="$(jq -nc --arg t "$TRADER" --arg amt "$((SWAP_RAW * 3))" \
  '{transfer:{recipient:$t,amount:$amt}}')"
exec_ok wasm execute "$GEM" "$FUND3" >/dev/null
gas_out="$(layer_terrad_tx_from test1 bank send test1 "$TRADER" "20000000uluna")"
gas_tx="$(layer_txhash "$gas_out")"
[[ -n "$gas_tx" ]] || {
  echo "FAIL: bank send to test3 produced no txhash" >&2
  printf '%s\n' "$gas_out" >&2
  exit 1
}
layer_wait_tx "$gas_tx"
echo "628-lt: funded non-manager trader $TRADER (treasury stays $TREASURY)"

EXPECT_TAX="$(tax_on_preview_sell "$GEM" "$TRADER" "$PAIR" "$SWAP_RAW" "$SELL_BPS")"
USER_S0="$(layer_cw20_balance "$GEM" "$TRADER")"
PAIR_S0="$(layer_cw20_balance "$GEM" "$PAIR")"
SINK_S0="$(layer_cw20_balance "$GEM" "$TREASURY")"
tax_on_send_cw20_hook_from test3 "$GEM" "$PAIR" "$SWAP_RAW" "$SWAP_HOOK" >/dev/null
USER_S1="$(layer_cw20_balance_changed "$GEM" "$TRADER" "$USER_S0")"
PAIR_S1="$(layer_cw20_balance_changed "$GEM" "$PAIR" "$PAIR_S0")"
SINK_S1="$(layer_cw20_balance "$GEM" "$TREASURY")"
tax_on_assert_sell "$USER_S0" "$USER_S1" "$PAIR_S0" "$PAIR_S1" \
  "$SINK_S0" "$SINK_S1" "$SWAP_RAW" "$EXPECT_TAX"
echo "628-lt: P7 extra-debit sell after Refresh"

# P11: adopted token is the tax template — Manage SKUs query.
FEATS="$(layer_smart "$GEM" '{"get_features":{}}')"
echo "$FEATS" | jq -e 'has("variable_rates") and has("transfer_tax") and has("auto_v2_lp")' >/dev/null || {
  echo "FAIL: GetFeatures missing tax SKU flags (P11): $FEATS" >&2
  exit 1
}
CFG="$(layer_smart "$GEM" '{"get_config":{}}')"
echo "$CFG" | jq -e --arg m "$TEST_ADDRESS" --arg t "$TREASURY" \
  '.manager == $m and .treasury == $t and (.sell_bps|tonumber) == '"$SELL_BPS" >/dev/null || {
  echo "FAIL: GetConfig after adopt not Manage-ready (P11): $CFG" >&2
  exit 1
}
echo "628-lt: P11 GetFeatures + GetConfig (Manage tax SKUs)"

echo "==> GitLab #628 LocalTerra leftovers passed"
echo "628-lt: gem=$GEM pair=$PAIR treasury=$TREASURY"
