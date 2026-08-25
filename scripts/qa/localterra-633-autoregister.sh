#!/usr/bin/env bash
# GitLab #633 — LocalTerra live rungs (R633). Never columbus-5.
#
# Proves on a fresh (or current-wasm) LocalTerra factory:
#   1. Seed tax/EMBER pair is still registered (L620 / R633 seed)
#   2. test1 is IsProtocolExempt.manager without ExemptionDirectory (R633-1)
#   3. Factory CreatePair tax/UST1 autoregisters — no hand-rolled register (R633-2)
#   4. Honest/honest CreatePair succeeds (no blind tax execute)
#   5. Manager provide + Send+Swap are Honest; a third wallet extra-debits
#   6. AutoLP UpdateConfig { pair } is idempotent on an already-registered pair
#
# Requires: make has-localterra, frontend-dapp/.env.local tax pins,
#           mintable artifact (or deploy-local artifacts).
# Do not AddWhitelistedCodeId columbus-5 11611/11619/8654.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PROVIDE_RAW="${VERIFY633_PROVIDE_RAW:-100000000}"
SWAP_RAW="${VERIFY633_SWAP_RAW:-1000000}"
FUND_RAW="${VERIFY633_FUND_RAW:-50000000}"

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

[[ -n "${VITE_FACTORY_ADDRESS:-}" && -n "${VITE_TOKEN_COMMUNITY_TAX_ADDRESS:-}" ]] || {
  echo "FAIL: VITE_FACTORY_ADDRESS / VITE_TOKEN_COMMUNITY_TAX_ADDRESS unset." >&2
  exit 1
}
[[ -n "${VITE_TOKEN_EMBER_ADDRESS:-}" && -n "${VITE_UST1_TOKEN_ADDRESS:-}" ]] || {
  echo "FAIL: VITE_TOKEN_EMBER_ADDRESS / VITE_UST1_TOKEN_ADDRESS unset." >&2
  exit 1
}
[[ -n "${VITE_PAIR_COMMUNITY_TAX_EMBER:-}" ]] || {
  echo "FAIL: VITE_PAIR_COMMUNITY_TAX_EMBER unset." >&2
  exit 1
}

TAX="$VITE_TOKEN_COMMUNITY_TAX_ADDRESS"
FACTORY="$VITE_FACTORY_ADDRESS"
EMBER="$VITE_TOKEN_EMBER_ADDRESS"
UST1="$VITE_UST1_TOKEN_ADDRESS"
SEED_PAIR="$VITE_PAIR_COMMUNITY_TAX_EMBER"

terrad_tx() { tax_on_terrad_tx "$@"; }
exec_ok() { tax_on_exec_ok "$@"; }

lcd_code_id() {
  local addr="$1"
  local raw
  raw="$(localterra_lcd_curl "$LCD" "/cosmwasm/wasm/v1/contract/${addr}")"
  echo "$raw" | jq -r '.contract_info.code_id // empty'
}

ensure_key() {
  local name="$1"
  local addr
  addr="$(layer_addr_of "$name")"
  if [[ "$addr" != terra1* ]]; then
    echo "633-lt: adding LocalTerra key $name (ephemeral; not persisted)" >&2
    localterra_docker_exec "$CONTAINER" terrad keys add "$name" --keyring-backend test --output json >/dev/null
    addr="$(layer_addr_of "$name")"
  fi
  [[ "$addr" == terra1* ]] || {
    echo "FAIL: could not create $name key" >&2
    return 1
  }
  local uluna
  uluna="$(localterra_docker_exec "$CONTAINER" terrad query bank balances "$addr" \
    --node "$TERRAD_NODE" --output json 2>/dev/null \
    | jq -r '.balances[]? | select(.denom=="uluna") | .amount' || true)"
  uluna="${uluna:-0}"
  if ! python3 -c "import sys; sys.exit(0 if int(sys.argv[1]) >= 10000000000 else 1)" "$uluna"; then
    local out tx
    out="$(layer_terrad_tx_from test1 bank send test1 "$addr" "50000000000uluna")"
    tx="$(layer_txhash "$out")"
    [[ -n "$tx" ]] || {
      echo "FAIL: bank send to $name produced no txhash" >&2
      printf '%s\n' "$out" >&2
      return 1
    }
    layer_wait_tx "$tx"
  fi
  printf '%s' "$addr"
}

echo "633-lt: seed + factory CreatePair autoregister + manager Honest / retail extra-debit"

# --- seed still registered ---
SEED_EXEMPT="$(layer_smart "$TAX" \
  "$(jq -nc --arg p "$SEED_PAIR" '{is_protocol_exempt:{address:$p}}')")"
echo "$SEED_EXEMPT" | jq -e '.protocol == true' >/dev/null || {
  echo "FAIL: seed pair $SEED_PAIR is not registered: $SEED_EXEMPT" >&2
  exit 1
}
echo "633-lt: seed pair registered protocol=true"

# --- manager role skip without ExemptionDirectory ---
MGR_EXEMPT="$(layer_smart "$TAX" \
  "$(jq -nc --arg a "$TEST_ADDRESS" '{is_protocol_exempt:{address:$a}}')")"
echo "$MGR_EXEMPT" | jq -e '.manager == true and .protocol == false' >/dev/null || {
  echo "FAIL: test1 must be manager-skip and not protocol-exempt: $MGR_EXEMPT" >&2
  exit 1
}
EXEMPTIONS="$(layer_smart "$TAX" '{"get_exemptions":{"limit":30}}')"
echo "$EXEMPTIONS" | jq -e --arg m "$TEST_ADDRESS" \
  '.manager | map(.) | index($m) != null' >/dev/null || {
  echo "FAIL: GetExemptions.manager missing test1 (role skip): $EXEMPTIONS" >&2
  exit 1
}
# Directory must not be the only reason — MANAGER_EXEMPT list can include the
# role wallet, but protocol must stay empty of test1.
echo "$EXEMPTIONS" | jq -e --arg m "$TEST_ADDRESS" \
  '.protocol | map(.) | index($m) == null' >/dev/null || {
  echo "FAIL: test1 must not be protocol-exempt: $EXEMPTIONS" >&2
  exit 1
}
echo "633-lt: test1 manager=true protocol=false (no ExemptionDirectory required)"

CFG="$(layer_smart "$TAX" '{"get_config":{}}')"
SELL_BPS="$(echo "$CFG" | jq -r '.sell_bps')"
BUY_BPS="$(echo "$CFG" | jq -r '.buy_bps')"
TREASURY="$(echo "$CFG" | jq -r '.treasury')"
AUTOLP="$(echo "$CFG" | jq -r '.autolp // empty')"
[[ "$SELL_BPS" =~ ^[0-9]+$ && "$SELL_BPS" -gt 0 ]] || {
  echo "FAIL: seed token sell_bps=$SELL_BPS (need >0 for retail extra-debit)" >&2
  exit 1
}

# --- factory B2: tax/UST1 CreatePair autoregisters ---
EXISTING_UST1="$(tax_on_resolve_pair "$FACTORY" "$TAX" "$UST1")"
if [[ "$EXISTING_UST1" == terra1* ]]; then
  echo "633-lt: tax/UST1 already exists $EXISTING_UST1 — checking register (no new CreatePair)"
  NEW_PAIR="$EXISTING_UST1"
else
  echo "633-lt: CreatePair tax/UST1 (no hand-rolled register_listed_pair)"
  NEW_PAIR="$(tax_on_create_pair "$FACTORY" "$TAX" "$UST1")"
fi
[[ "$NEW_PAIR" == terra1* ]] || {
  echo "FAIL: tax/UST1 pair missing" >&2
  exit 1
}
NEW_EXEMPT="$(layer_smart "$TAX" \
  "$(jq -nc --arg p "$NEW_PAIR" '{is_protocol_exempt:{address:$p}}')")"
echo "$NEW_EXEMPT" | jq -e '.protocol == true' >/dev/null || {
  echo "FAIL: factory CreatePair did not autoregister $NEW_PAIR: $NEW_EXEMPT" >&2
  echo "Hint: factory must be the #633 wasm (fresh deploy or factory migrate)." >&2
  exit 1
}
echo "633-lt: tax/UST1 $NEW_PAIR protocol=true without terrad register_listed_pair"

# --- honest/honest CreatePair must not revert (no blind tax execute) ---
EMBER_CODE="$(lcd_code_id "$EMBER")"
[[ "$EMBER_CODE" =~ ^[0-9]+$ ]] || {
  echo "FAIL: EMBER code_id missing" >&2
  exit 1
}
STAMP="$(date +%s)"
HONEST_INIT="$(jq -nc --arg a "$TEST_ADDRESS" --arg n "H633A" --arg s "HAA" \
  '{name:$n,symbol:$s,decimals:6,initial_balances:[{address:$a,amount:"1000000000000"}],mint:{minter:$a}}')"
H1_TX="$(exec_ok wasm instantiate "$EMBER_CODE" "$HONEST_INIT" \
  --label "633-h1-${STAMP}" --admin "$TEST_ADDRESS")"
H1="$(tax_on_contract_from_tx "$H1_TX")"
HONEST_INIT2="$(jq -nc --arg a "$TEST_ADDRESS" --arg n "H633B" --arg s "HBB" \
  '{name:$n,symbol:$s,decimals:6,initial_balances:[{address:$a,amount:"1000000000000"}],mint:{minter:$a}}')"
H2_TX="$(exec_ok wasm instantiate "$EMBER_CODE" "$HONEST_INIT2" \
  --label "633-h2-${STAMP}" --admin "$TEST_ADDRESS")"
H2="$(tax_on_contract_from_tx "$H2_TX")"
[[ "$H1" == terra1* && "$H2" == terra1* ]] || {
  echo "FAIL: honest mintable instantiate failed" >&2
  exit 1
}
# Same code id as EMBER is already factory-whitelisted (10184 analogue).
HONEST_PAIR="$(tax_on_create_pair "$FACTORY" "$H1" "$H2")"
[[ "$HONEST_PAIR" == terra1* ]] || {
  echo "FAIL: honest/honest CreatePair failed (B2 must not execute tax register on honest CW20)" >&2
  exit 1
}
echo "633-lt: honest/honest CreatePair $HONEST_PAIR (no tax execute)"

# --- manager provide + Honest swap; retail extra-debit ---
tax_on_provide_1to1 "$TAX" "$UST1" "$NEW_PAIR" "$PROVIDE_RAW"

# Manager TaxPreview + execute must be Honest (debit == amount).
HOOK='{"swap":{"max_spread":"1"}}'
HOOK_B64="$(printf '%s' "$HOOK" | base64 -w0 2>/dev/null || printf '%s' "$HOOK" | base64 | tr -d '\n')"
MGR_PREVIEW="$(layer_smart "$TAX" \
  "$(jq -nc --arg f "$TEST_ADDRESS" --arg t "$NEW_PAIR" --arg amt "$SWAP_RAW" --arg m "$HOOK_B64" \
    '{tax_preview:{from:$f,to:$t,amount:$amt,send_msg:$m}}')")"
echo "$MGR_PREVIEW" | jq -e --arg a "$SWAP_RAW" \
  '.kind == "honest" and .debit == $a and .credit == $a' >/dev/null || {
  echo "FAIL: manager TaxPreview not Honest: $MGR_PREVIEW" >&2
  exit 1
}
MGR_BEFORE="$(layer_cw20_balance "$TAX" "$TEST_ADDRESS")"
PAIR_BEFORE="$(layer_cw20_balance "$TAX" "$NEW_PAIR")"
SINK_BEFORE="$(layer_cw20_balance "$TAX" "$TREASURY")"
tax_on_send_cw20_hook "$TAX" "$NEW_PAIR" "$SWAP_RAW" "$HOOK" >/dev/null
MGR_AFTER="$(layer_cw20_balance_changed "$TAX" "$TEST_ADDRESS" "$MGR_BEFORE")"
PAIR_AFTER="$(layer_cw20_balance_changed "$TAX" "$NEW_PAIR" "$PAIR_BEFORE")"
SINK_AFTER="$(layer_cw20_balance "$TAX" "$TREASURY")"
python3 -c '
import sys
ub, ua, pb, pa, sb, sa, amt = (int(x) for x in sys.argv[1:])
if ub - ua != amt:
    sys.stderr.write(f"FAIL: manager sell debit {ub}->{ua} != {amt} (R633-1 Honest)\n")
    sys.exit(1)
if pa - pb != amt:
    sys.stderr.write(f"FAIL: manager pair credit {pb}->{pa} != {amt} (inbound 1:1)\n")
    sys.exit(1)
if sa != sb:
    sys.stderr.write(f"FAIL: manager sell moved treasury {sb}->{sa}\n")
    sys.exit(1)
print("633-lt: manager Send+Swap Honest + inbound 1:1")
' "$MGR_BEFORE" "$MGR_AFTER" "$PAIR_BEFORE" "$PAIR_AFTER" "$SINK_BEFORE" "$SINK_AFTER" "$SWAP_RAW"

RETAIL="$(ensure_key test3)"
[[ "$RETAIL" != "$TEST_ADDRESS" && "$RETAIL" != "$TREASURY" ]] || {
  echo "FAIL: retail wallet must not be manager or treasury ($RETAIL)" >&2
  exit 1
}
XFER="$(jq -nc --arg r "$RETAIL" --arg amt "$FUND_RAW" '{transfer:{recipient:$r,amount:$amt}}')"
exec_ok wasm execute "$TAX" "$XFER" >/dev/null
RETAIL_EXEMPT="$(layer_smart "$TAX" \
  "$(jq -nc --arg a "$RETAIL" '{is_protocol_exempt:{address:$a}}')")"
echo "$RETAIL_EXEMPT" | jq -e '.manager == false and .protocol == false' >/dev/null || {
  echo "FAIL: retail $RETAIL unexpectedly exempt: $RETAIL_EXEMPT" >&2
  exit 1
}
EXPECT_TAX="$(tax_on_preview_sell "$TAX" "$RETAIL" "$NEW_PAIR" "$SWAP_RAW" "$SELL_BPS")"
R_BEFORE="$(layer_cw20_balance "$TAX" "$RETAIL")"
P2_BEFORE="$(layer_cw20_balance "$TAX" "$NEW_PAIR")"
S2_BEFORE="$(layer_cw20_balance "$TAX" "$TREASURY")"
tax_on_send_cw20_hook_from test3 "$TAX" "$NEW_PAIR" "$SWAP_RAW" "$HOOK" >/dev/null
R_AFTER="$(layer_cw20_balance_changed "$TAX" "$RETAIL" "$R_BEFORE")"
P2_AFTER="$(layer_cw20_balance_changed "$TAX" "$NEW_PAIR" "$P2_BEFORE")"
S2_AFTER="$(layer_cw20_balance_changed "$TAX" "$TREASURY" "$S2_BEFORE")"
tax_on_assert_sell "$R_BEFORE" "$R_AFTER" "$P2_BEFORE" "$P2_AFTER" \
  "$S2_BEFORE" "$S2_AFTER" "$SWAP_RAW" "$EXPECT_TAX"
echo "633-lt: retail extra-debit tax=$EXPECT_TAX (sell_bps=$SELL_BPS)"

# --- AutoLP re-bind is idempotent (R633-3) ---
if [[ "$AUTOLP" == terra1* ]]; then
  REBIND="$(jq -nc --arg p "$NEW_PAIR" '{update_config:{pair:$p}}')"
  exec_ok wasm execute "$AUTOLP" "$REBIND" >/dev/null
  ALP_CFG="$(layer_smart "$AUTOLP" '{"get_config":{}}')"
  echo "$ALP_CFG" | jq -e --arg p "$NEW_PAIR" '.pair == $p' >/dev/null || {
    echo "FAIL: AutoLP pair not $NEW_PAIR after UpdateConfig: $ALP_CFG" >&2
    exit 1
  }
  REBIND_EXEMPT="$(layer_smart "$TAX" \
    "$(jq -nc --arg p "$NEW_PAIR" '{is_protocol_exempt:{address:$p}}')")"
  echo "$REBIND_EXEMPT" | jq -e '.protocol == true' >/dev/null || {
    echo "FAIL: AutoLP re-bind dropped register: $REBIND_EXEMPT" >&2
    exit 1
  }
  echo "633-lt: AutoLP re-bind pair=$NEW_PAIR idempotent protocol=true"
else
  echo "633-lt: WARN no AutoLP on seed token — skip re-bind (R633-3 crate still covers)"
fi

echo "==> GitLab #633 LocalTerra live rungs passed"
echo "633-lt: pair=$NEW_PAIR manager=$TEST_ADDRESS retail=$RETAIL"
