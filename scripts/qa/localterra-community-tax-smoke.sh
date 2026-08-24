#!/usr/bin/env bash
# GitLab #601 — LocalTerra smoke for community tax CW20 (T592 + O601).
#
# Proves on a live LocalTerra factory:
#   1. Free-profile launcher CreateToken (0 SKU, no UST1 Send)
#   2. ContractInfo.admin == CMM stand-in; GetLauncherOrigin set
#   3. Rogue --admin instantiate has no launcher origin
#   4. CreatePair + RegisterListedPair + provide 1:1
#   5. Sell extra-debit + TaxPreview (max-button) + buy outbound split
#   6. SKU unlock 50 UST1 via the official launcher Enable Feature path (#606);
#      settings batch 50 UST1 still targets the token
#   7. MintControl instantiate + RevokeMint one-way
#   8. Paid create with one SKU, then Enable Feature a second SKU (#612)
#
# C605-2: do not send launch_guards / transfer_bps / sinks unless that SKU is
# selected — current launcher rejects SKU payloads without the feature.
# C605-4: without variable_rates, max_* must equal current rates.
#
# Never AddWhitelistedCodeId columbus-5 11611 from this evidence — only the
# LocalTerra store id. Do not whitelist launcher/AutoLP/8654.
#
# Requires: make has-localterra, frontend-dapp/.env.local (or VERIFY_ENV_LOCAL),
#           cw20-codeid-audits/codeids/11611/token.wasm (pin check only),
#           smartcontracts/artifacts/cl8y_community_tax_token.wasm (instantiate;
#           11611 cannot accept post-#605 launcher `initial_exempt`),
#           smartcontracts/artifacts/cw20_mintable.wasm (deploy-local artifacts).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

AUDIT="$REPO_ROOT/cw20-codeid-audits"
OUT_JSON="${VERIFY601_SMOKE_JSON:-/tmp/cl8y-601-smoke.json}"
CMM_PIN="terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2"
INVOICE="50000000"
SELL_BPS=500
BUY_BPS=500
PROVIDE_RAW="${VERIFY601_PROVIDE_RAW:-100000000}"
SWAP_RAW="${VERIFY601_SWAP_RAW:-1000000}"

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
source "$AUDIT/scripts/lib-layer-lt.sh"

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

# 11611 pin stays the #589 / O601-1 LCD artifact. Post-#605 launcher
# InstantiateMsg includes `initial_exempt`, which 11611 rejects (unknown field).
# Enable Feature smoke (#612) must store current token bytes with current launcher.
PIN_FILE="$AUDIT/codeids/11611/wasm.sha256"
PINNED_11611="$AUDIT/codeids/11611/token.wasm"
[[ -f "$PINNED_11611" ]] || "$AUDIT/scripts/fetch-lcd-wasm.sh" 11611
expected_pin="$(tr -d '[:space:]' < "$PIN_FILE" | tr '[:lower:]' '[:upper:]')"
pinned_sha="$(sha256sum "$PINNED_11611" | awk '{print toupper($1)}')"
[[ "$pinned_sha" == "$expected_pin" ]] || {
  echo "FAIL: C1 pin mismatch $pinned_sha != $expected_pin" >&2
  exit 1
}

TOKEN_WASM="${VERIFY601_TOKEN_WASM:-}"
if [[ -z "$TOKEN_WASM" ]]; then
  for cand in \
    "$REPO_ROOT/smartcontracts/artifacts/cl8y_community_tax_token.wasm" \
    "$REPO_ROOT/smartcontracts/target/wasm32-unknown-unknown/release/cl8y_community_tax_token.wasm"; do
    if [[ -f "$cand" ]]; then
      TOKEN_WASM="$cand"
      break
    fi
  done
fi
if [[ -z "$TOKEN_WASM" ]]; then
  echo "601-smoke: building community-tax-token wasm (artifacts missing)"
  (cd "$REPO_ROOT/smartcontracts" && cargo build -p cl8y-community-tax-token \
    --release --target wasm32-unknown-unknown --offline 2>/dev/null \
    || cargo build -p cl8y-community-tax-token --release --target wasm32-unknown-unknown)
  TOKEN_WASM="$REPO_ROOT/smartcontracts/target/wasm32-unknown-unknown/release/cl8y_community_tax_token.wasm"
fi
[[ -f "$TOKEN_WASM" ]] || {
  echo "FAIL: token wasm missing. Run make build-optimized or cargo wasm32." >&2
  exit 1
}
got_pin="$(sha256sum "$TOKEN_WASM" | awk '{print toupper($1)}')"
if [[ "$got_pin" == "$expected_pin" ]]; then
  echo "FAIL: smoke token wasm is still 11611; post-#605 launcher cannot instantiate it" >&2
  exit 1
fi
echo "601-smoke: 11611 pin OK; instantiate wasm=$TOKEN_WASM sha=$got_pin"

MINTABLE_WASM="$REPO_ROOT/smartcontracts/artifacts/cw20_mintable.wasm"
LAUNCHER_WASM="$REPO_ROOT/smartcontracts/artifacts/cl8y_community_token_launcher.wasm"
if [[ ! -f "$LAUNCHER_WASM" ]]; then
  echo "601-smoke: building launcher wasm (artifacts missing)"
  (cd "$REPO_ROOT/smartcontracts" && cargo build -p cl8y-community-token-launcher \
    --release --target wasm32-unknown-unknown --offline 2>/dev/null \
    || cargo build -p cl8y-community-token-launcher --release --target wasm32-unknown-unknown)
  LAUNCHER_WASM="$REPO_ROOT/smartcontracts/target/wasm32-unknown-unknown/release/cl8y_community_token_launcher.wasm"
fi
[[ -f "$LAUNCHER_WASM" ]] || {
  echo "FAIL: launcher wasm missing. Run make build-optimized or cargo wasm32." >&2
  exit 1
}
[[ -f "$MINTABLE_WASM" ]] || {
  echo "FAIL: $MINTABLE_WASM missing (make build-optimized / deploy-local)." >&2
  exit 1
}

terrad_tx() { e2e_terrad_tx "$CONTAINER" "$@"; }

exec_ok() {
  local out tx
  out="$(terrad_tx "$@")"
  tx="$(layer_txhash "$out")"
  [[ -n "$tx" ]] || {
    echo "FAIL: no txhash for: $*" >&2
    printf '%s\n' "$out" >&2
    exit 1
  }
  layer_wait_tx "$tx"
  printf '%s' "$tx"
}

store_wasm() {
  local host="$1" dest="$2"
  layer_docker_cp "$host" "${CONTAINER}:${dest}"
  local out tx json code
  out="$(terrad_tx wasm store "$dest")"
  tx="$(layer_txhash "$out")"
  [[ -n "$tx" ]] || {
    echo "FAIL: store $host produced no txhash" >&2
    printf '%s\n' "$out" >&2
    exit 1
  }
  json="$(terrad_wait_tx_query "$CONTAINER" "$tx" "$TERRAD_NODE")"
  code="$(echo "$json" | terrad_jq_code_id_from_tx_json | head -1 | tr -d '[:space:]')"
  [[ "$code" =~ ^[0-9]+$ ]] || {
    echo "FAIL: could not parse code_id from store $tx" >&2
    exit 1
  }
  printf '%s' "$code"
}

contract_from_tx() {
  local tx="$1"
  echo "$(terrad_wait_tx_query "$CONTAINER" "$tx" "$TERRAD_NODE")" \
    | terrad_jq_contract_address_from_tx_json | head -1
}

lcd_contract_admin() {
  local addr="$1"
  local raw
  raw="$(localterra_lcd_curl "$LCD" "/cosmwasm/wasm/v1/contract/${addr}")"
  echo "$raw" | jq -r '.contract_info.admin // empty'
}

send_cw20_hook() {
  local token="$1" dest="$2" amount="$3" hook_json="$4"
  local b64 msg
  b64="$(printf '%s' "$hook_json" | base64 -w0 2>/dev/null || printf '%s' "$hook_json" | base64 | tr -d '\n')"
  msg="$(jq -nc --arg c "$dest" --arg amt "$amount" --arg m "$b64" \
    '{send:{contract:$c,amount:$amt,msg:$m}}')"
  exec_ok wasm execute "$token" "$msg"
}

echo "601-smoke: storing current token + launcher + mintable UST1 stand-in…"
TOKEN_CODE="$(store_wasm "$TOKEN_WASM" /tmp/cw20-audit-11611.wasm)"
LAUNCHER_CODE="$(store_wasm "$LAUNCHER_WASM" /tmp/cw20-audit-launcher.wasm)"
MINTABLE_CODE="$(store_wasm "$MINTABLE_WASM" /tmp/cw20-audit-mintable.wasm)"
echo "601-smoke: local codes token=$TOKEN_CODE launcher=$LAUNCHER_CODE mintable=$MINTABLE_CODE"

TEST2="$(layer_ensure_test2)"
TREASURY="$TEST2"
STAMP="$(date +%s)"
UST1_INIT="$(jq -nc --arg a "$TEST_ADDRESS" \
  '{name:"SmokeUST1",symbol:"SUST",decimals:6,initial_balances:[{address:$a,amount:"1000000000000"}],mint:{minter:$a}}')"
UST1_TX="$(exec_ok wasm instantiate "$MINTABLE_CODE" "$UST1_INIT" \
  --label "601-ust1-${STAMP}" --admin "$TEST_ADDRESS")"
UST1_ADDR="$(contract_from_tx "$UST1_TX")"
[[ "$UST1_ADDR" == terra1* ]] || {
  echo "FAIL: UST1 instantiate address missing" >&2
  exit 1
}

LAUNCHER_INIT="$(jq -nc \
  --argjson tok "$TOKEN_CODE" --arg ust1 "$UST1_ADDR" --arg a "$TEST_ADDRESS" \
  --arg factory "$VITE_FACTORY_ADDRESS" --arg router "${VITE_ROUTER_ADDRESS:-}" \
  '{
    token_code_id:$tok,
    autolp_code_id:null,
    ust1:$ust1,
    cmm_treasury:$a,
    cmm_governance:$a,
    factory:$factory,
    router: (if $router == "" then null else $router end)
  }')"
LAUNCHER_TX="$(exec_ok wasm instantiate "$LAUNCHER_CODE" "$LAUNCHER_INIT" \
  --label "601-launcher-${STAMP}" --admin "$TEST_ADDRESS")"
LAUNCHER_ADDR="$(contract_from_tx "$LAUNCHER_TX")"
[[ "$LAUNCHER_ADDR" == terra1* ]] || {
  echo "FAIL: launcher instantiate address missing" >&2
  exit 1
}
LAUNCHER_ADMIN="$(lcd_contract_admin "$LAUNCHER_ADDR")"
[[ "$LAUNCHER_ADMIN" == "$TEST_ADDRESS" ]] || {
  echo "FAIL: launcher admin $LAUNCHER_ADMIN != CMM stand-in $TEST_ADDRESS (T592-5)" >&2
  exit 1
}
echo "601-smoke: launcher=$LAUNCHER_ADDR admin=$LAUNCHER_ADMIN (LocalTerra CMM stand-in; columbus-5 CMM is $CMM_PIN)"

SYM="$(layer_terraport_symbol)"
FREE_MSG="$(jq -nc --arg n "FreeTax" --arg s "$SYM" --arg a "$TEST_ADDRESS" \
  --arg treas "$TREASURY" \
  --argjson buy "$BUY_BPS" --argjson sell "$SELL_BPS" \
  '{
    create_token:{
      name:$n, symbol:$s, decimals:6,
      initial_balances:[{address:$a,amount:"1000000000000"}],
      manager:$a, treasury:$treas,
      buy_bps:$buy, sell_bps:$sell,
      max_buy_bps:$buy, max_sell_bps:$sell, max_transfer_bps:0,
      features:[],
      mint:null, transfer_bps:null, sinks:null,
      launch_guards:null,
      autolp_threshold:null, autolp_lp_recipient:null
    }
  }')"
FREE_TX="$(exec_ok wasm execute "$LAUNCHER_ADDR" "$FREE_MSG")"
FREE_JSON="$(terrad_wait_tx_query "$CONTAINER" "$FREE_TX" "$TERRAD_NODE")"
FREE_TOKEN="$(echo "$FREE_JSON" | terrad_jq_contract_address_from_tx_json | head -1)"
[[ "$FREE_TOKEN" == terra1* ]] || {
  echo "FAIL: free-profile CreateToken did not yield a contract" >&2
  exit 1
}
FREE_ADMIN="$(lcd_contract_admin "$FREE_TOKEN")"
[[ "$FREE_ADMIN" == "$TEST_ADDRESS" ]] || {
  echo "FAIL: free token admin $FREE_ADMIN != CMM stand-in (T592-5)" >&2
  exit 1
}
ORIGIN="$(layer_smart "$FREE_TOKEN" '{"get_launcher_origin":{}}')"
ORIGIN_LAUNCHER="$(echo "$ORIGIN" | jq -r '.launcher // empty')"
[[ "$ORIGIN_LAUNCHER" == "$LAUNCHER_ADDR" ]] || {
  echo "FAIL: GetLauncherOrigin.launcher=$ORIGIN_LAUNCHER != $LAUNCHER_ADDR" >&2
  exit 1
}
echo "601-smoke: free token=$FREE_TOKEN origin=$ORIGIN_LAUNCHER"

# Rogue instantiate (Everybody) with a different admin — catalog must not promote.
ROGUE_INIT="$(jq -nc --arg n "RogueTax" --arg s "ROG" --arg a "$TEST_ADDRESS" \
  --arg factory "$VITE_FACTORY_ADDRESS" --arg ust1 "$UST1_ADDR" \
  '{
    name:$n, symbol:$s, decimals:6,
    initial_balances:[{address:$a,amount:"1"}],
    marketing:{},
    manager:$a, treasury:$a,
    buy_bps:0, sell_bps:0,
    max_buy_bps:0, max_sell_bps:0, max_transfer_bps:0,
    factory:$factory, router:null, ust1:$ust1, cmm_treasury:$a,
    features:[], mint:null,
    launch_guards:null
  }')"
ROGUE_OUT="$(layer_terrad_tx_from test2 wasm instantiate "$TOKEN_CODE" "$ROGUE_INIT" \
  --label "601-rogue-${STAMP}" --admin "$TEST2")"
ROGUE_TX="$(layer_txhash "$ROGUE_OUT")"
[[ -n "$ROGUE_TX" ]] || {
  echo "FAIL: rogue instantiate produced no txhash" >&2
  printf '%s\n' "$ROGUE_OUT" >&2
  exit 1
}
layer_wait_tx "$ROGUE_TX"
ROGUE_TOKEN="$(contract_from_tx "$ROGUE_TX")"
ROGUE_ORIGIN="$(layer_smart "$ROGUE_TOKEN" '{"get_launcher_origin":{}}')"
echo "$ROGUE_ORIGIN" | jq -e '.launcher == null' >/dev/null || {
  echo "FAIL: rogue GetLauncherOrigin should be null: $ROGUE_ORIGIN" >&2
  exit 1
}
echo "601-smoke: rogue token=$ROGUE_TOKEN origin=null (catalog filter)"

# Whitelist local token code only (never columbus-5 11611).
WL_MSG="$(jq -nc --argjson c "$TOKEN_CODE" '{add_whitelisted_code_id:{code_id:$c}}')"
if layer_smart "$VITE_FACTORY_ADDRESS" \
  "$(jq -nc --argjson c "$TOKEN_CODE" '{is_code_id_whitelisted:{code_id:$c}}')" \
  | jq -e '.whitelisted == true' >/dev/null 2>&1; then
  echo "601-smoke: local token code already whitelisted"
else
  exec_ok wasm execute "$VITE_FACTORY_ADDRESS" "$WL_MSG" >/dev/null
fi

FACTORY_CFG="$(layer_smart "$VITE_FACTORY_ADDRESS" '{"config":{}}')"
PAIR_FEE="$(echo "$FACTORY_CFG" | jq -r '.pair_creation_fee_uluna // "0"')"
CREATE_MSG="$(jq -nc --arg a "$FREE_TOKEN" --arg b "$VITE_TOKEN_EMBER_ADDRESS" \
  '{create_pair:{asset_infos:[{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')"
fee_args=()
if [[ "$PAIR_FEE" != "0" && -n "$PAIR_FEE" ]]; then
  fee_args=(--amount "${PAIR_FEE}uluna")
fi
CREATE_TX="$(exec_ok wasm execute "$VITE_FACTORY_ADDRESS" "$CREATE_MSG" "${fee_args[@]}")"
PAIR_ADDR="$(contract_from_tx "$CREATE_TX")"
if [[ "$PAIR_ADDR" != terra1* ]]; then
  PAIR_Q="$(jq -nc --arg a "$FREE_TOKEN" --arg b "$VITE_TOKEN_EMBER_ADDRESS" \
    '{pair:{asset_infos:[{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')"
  PAIR_ADDR="$(layer_smart "$VITE_FACTORY_ADDRESS" "$PAIR_Q" | jq -r '.contract_addr // .pair.contract_addr // empty')"
fi
[[ "$PAIR_ADDR" == terra1* ]] || {
  echo "FAIL: CreatePair did not resolve pair address" >&2
  exit 1
}
echo "601-smoke: pair=$PAIR_ADDR"

REG="$(jq -nc --arg p "$PAIR_ADDR" '{register_listed_pair:{pair:$p}}')"
exec_ok wasm execute "$FREE_TOKEN" "$REG" >/dev/null

ALLOW="$(jq -nc --arg s "$PAIR_ADDR" --arg amt "$PROVIDE_RAW" \
  '{increase_allowance:{spender:$s,amount:$amt}}')"
exec_ok wasm execute "$FREE_TOKEN" "$ALLOW" >/dev/null
exec_ok wasm execute "$VITE_TOKEN_EMBER_ADDRESS" "$ALLOW" >/dev/null
CAND_BEFORE="$(layer_cw20_balance "$FREE_TOKEN" "$PAIR_ADDR")"
PROVIDE="$(jq -nc --arg a "$FREE_TOKEN" --arg b "$VITE_TOKEN_EMBER_ADDRESS" --arg amt "$PROVIDE_RAW" \
  '{provide_liquidity:{assets:[{info:{token:{contract_addr:$a}},amount:$amt},{info:{token:{contract_addr:$b}},amount:$amt}],slippage_tolerance:null,receiver:null,deadline:null}}')"
exec_ok wasm execute "$PAIR_ADDR" "$PROVIDE" >/dev/null
CAND_AFTER="$(layer_cw20_balance "$FREE_TOKEN" "$PAIR_ADDR")"
python3 -c '
import sys
b, a, amt = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
if a - b != amt:
    sys.stderr.write(f"FAIL: provide pair credit {b}->{a} != {amt} (T592-1 / P2)\n")
    sys.exit(1)
print("601-smoke: provide TransferFrom 1:1")
' "$CAND_BEFORE" "$CAND_AFTER" "$PROVIDE_RAW"

# TaxPreview max-button (sell extra-debit).
SWAP_HOOK='{"swap":{"max_spread":"1"}}'
HOOK_B64="$(printf '%s' "$SWAP_HOOK" | base64 -w0 2>/dev/null || printf '%s' "$SWAP_HOOK" | base64 | tr -d '\n')"
PREVIEW="$(layer_smart "$FREE_TOKEN" \
  "$(jq -nc --arg f "$TEST_ADDRESS" --arg t "$PAIR_ADDR" --arg amt "$SWAP_RAW" --arg m "$HOOK_B64" \
    '{tax_preview:{from:$f,to:$t,amount:$amt,send_msg:$m}}')")"
PRED_DEBIT="$(echo "$PREVIEW" | jq -r '.debit')"
PRED_CREDIT="$(echo "$PREVIEW" | jq -r '.credit')"
EXPECT_TAX=$((SWAP_RAW * SELL_BPS / 10000))
EXPECT_DEBIT=$((SWAP_RAW + EXPECT_TAX))
[[ "$PRED_DEBIT" == "$EXPECT_DEBIT" && "$PRED_CREDIT" == "$SWAP_RAW" ]] || {
  echo "FAIL: TaxPreview sell debit=$PRED_DEBIT credit=$PRED_CREDIT (want debit=$EXPECT_DEBIT credit=$SWAP_RAW)" >&2
  echo "$PREVIEW" >&2
  exit 1
}
echo "601-smoke: TaxPreview sell extra-debit $PRED_DEBIT (max-button)"

USER_BEFORE="$(layer_cw20_balance "$FREE_TOKEN" "$TEST_ADDRESS")"
PAIR_BEFORE="$(layer_cw20_balance "$FREE_TOKEN" "$PAIR_ADDR")"
SINK_BEFORE="$(layer_cw20_balance "$FREE_TOKEN" "$TREASURY")"
SELL_TX="$(send_cw20_hook "$FREE_TOKEN" "$PAIR_ADDR" "$SWAP_RAW" "$SWAP_HOOK")"
USER_AFTER="$(layer_cw20_balance "$FREE_TOKEN" "$TEST_ADDRESS")"
PAIR_AFTER="$(layer_cw20_balance "$FREE_TOKEN" "$PAIR_ADDR")"
SINK_AFTER="$(layer_cw20_balance "$FREE_TOKEN" "$TREASURY")"
python3 -c '
import sys
ub, ua, pb, pa, sb, sa, amt, tax = (int(x) for x in sys.argv[1:])
if ub - ua != amt + tax:
    sys.stderr.write(f"FAIL: sell user debit {ub}->{ua} != {amt}+{tax} (T592-2)\n")
    sys.exit(1)
if pa - pb != amt:
    sys.stderr.write(f"FAIL: sell pair credit {pb}->{pa} != {amt} (inbound must stay 1:1)\n")
    sys.exit(1)
if sa - sb != tax:
    sys.stderr.write(f"FAIL: sell treasury {sb}->{sa} != {tax}\n")
    sys.exit(1)
print("601-smoke: sell extra-debit + pair inbound 1:1")
' "$USER_BEFORE" "$USER_AFTER" "$PAIR_BEFORE" "$PAIR_AFTER" \
  "$SINK_BEFORE" "$SINK_AFTER" "$SWAP_RAW" "$EXPECT_TAX"

# Buy: EMBER → pair Swap, pair Transfer of tax token uses outbound split.
EMBER_BEFORE="$(layer_cw20_balance "$VITE_TOKEN_EMBER_ADDRESS" "$TEST_ADDRESS")"
python3 -c 'import sys; sys.exit(0 if int(sys.argv[1]) >= int(sys.argv[2]) else 1)' \
  "$EMBER_BEFORE" "$SWAP_RAW" || {
  echo "FAIL: EMBER balance $EMBER_BEFORE < $SWAP_RAW" >&2
  exit 1
}
USER_TOK_BEFORE="$(layer_cw20_balance "$FREE_TOKEN" "$TEST_ADDRESS")"
PAIR_TOK_BEFORE="$(layer_cw20_balance "$FREE_TOKEN" "$PAIR_ADDR")"
BUY_TX="$(send_cw20_hook "$VITE_TOKEN_EMBER_ADDRESS" "$PAIR_ADDR" "$SWAP_RAW" "$SWAP_HOOK")"
USER_TOK_AFTER="$(layer_cw20_balance "$FREE_TOKEN" "$TEST_ADDRESS")"
PAIR_TOK_AFTER="$(layer_cw20_balance "$FREE_TOKEN" "$PAIR_ADDR")"
python3 -c '
import sys
ub, ua, pb, pa, bps = (int(x) for x in sys.argv[1:])
pair_debit = pb - pa
user_credit = ua - ub
if pair_debit <= 0 or user_credit <= 0:
    sys.stderr.write(f"FAIL: buy did not move tax token pair {pb}->{pa} user {ub}->{ua}\n")
    sys.exit(1)
# T592-3: pair debit == declared transfer; user + sinks == debit. Buy tax => user < pair debit.
if user_credit >= pair_debit:
    sys.stderr.write(f"FAIL: buy user credit {user_credit} >= pair debit {pair_debit} (expected outbound split)\n")
    sys.exit(1)
tax = pair_debit - user_credit
expect = pair_debit * bps // 10000
if abs(tax - expect) > 1:
    sys.stderr.write(f"FAIL: buy tax {tax} != ~{expect} ({bps} bps of {pair_debit})\n")
    sys.exit(1)
print(f"601-smoke: buy outbound split pair_debit={pair_debit} user={user_credit} tax={tax}")
' "$USER_TOK_BEFORE" "$USER_TOK_AFTER" "$PAIR_TOK_BEFORE" "$PAIR_TOK_AFTER" "$BUY_BPS"

# SKU unlock via the official dApp path: UST1 Send → launcher → token (#606 / T606-1).
# Do not Send EnableFeature straight to the token here — that hid C-1 (L-1).
SKU_HOOK="$(jq -nc --arg t "$FREE_TOKEN" '{"enable_feature":{"token":$t,"sku":"transfer_tax"}}')"
CMM_UST1_BEFORE="$(layer_cw20_balance "$UST1_ADDR" "$TEST_ADDRESS")"
SKU_TX="$(send_cw20_hook "$UST1_ADDR" "$LAUNCHER_ADDR" "$INVOICE" "$SKU_HOOK")"
FEAT="$(layer_smart "$FREE_TOKEN" '{"get_features":{}}')"
echo "$FEAT" | jq -e '.transfer_tax == true' >/dev/null || {
  echo "FAIL: EnableFeature transfer_tax not set via launcher: $FEAT" >&2
  exit 1
}
LAUNCHER_HELD="$(layer_cw20_balance "$UST1_ADDR" "$LAUNCHER_ADDR")"
TOKEN_HELD="$(layer_cw20_balance "$UST1_ADDR" "$FREE_TOKEN")"
CMM_UST1_AFTER="$(layer_cw20_balance "$UST1_ADDR" "$TEST_ADDRESS")"
python3 -c '
import sys
held_l, held_t, before, after, invoice = (int(x) for x in sys.argv[1:])
if held_l != 0 or held_t != 0:
    sys.stderr.write(f"FAIL: launcher/token kept UST1 {held_l}/{held_t} (T606-6)\n")
    sys.exit(1)
# Payer is also CMM stand-in: net should be 0 after token forwards the invoice.
if after != before:
    sys.stderr.write(f"FAIL: CMM stand-in UST1 {before}->{after} (expected net 0 after +{invoice} forward)\n")
    sys.exit(1)
print("601-smoke: EnableFeature fee forwarded (launcher/token hold 0; CMM stand-in net 0)")
' "$LAUNCHER_HELD" "$TOKEN_HELD" "$CMM_UST1_BEFORE" "$CMM_UST1_AFTER" "$INVOICE"
# 11619+ C605-4: settings buy/sell require variable_rates.
VR_HOOK="$(jq -nc --arg t "$FREE_TOKEN" '{"enable_feature":{"token":$t,"sku":"variable_rates"}}')"
VR_TX="$(send_cw20_hook "$UST1_ADDR" "$LAUNCHER_ADDR" "$INVOICE" "$VR_HOOK")"
FEAT_VR="$(layer_smart "$FREE_TOKEN" '{"get_features":{}}')"
echo "$FEAT_VR" | jq -e '.transfer_tax == true and .variable_rates == true' >/dev/null || {
  echo "FAIL: EnableFeature variable_rates not set via launcher: $FEAT_VR" >&2
  exit 1
}
SET_HOOK='{"update_settings":{"settings":{"buy_bps":400}}}'
SET_TX="$(send_cw20_hook "$UST1_ADDR" "$FREE_TOKEN" "$INVOICE" "$SET_HOOK")"
CFG="$(layer_smart "$FREE_TOKEN" '{"get_config":{}}')"
[[ "$(echo "$CFG" | jq -r '.buy_bps')" == "400" ]] || {
  echo "FAIL: settings batch did not apply buy_bps=400: $CFG" >&2
  exit 1
}
echo "601-smoke: SKU unlock via launcher + settings batch 50 UST1 each (T592-4 / T606-1)"

# MintControl via paid launcher create, then RevokeMint one-way.
MINT_HOOK="$(jq -nc --arg n "MintTax" --arg s "MNT" --arg a "$TEST_ADDRESS" \
  '{
    create_token:{
      name:$n, symbol:$s, decimals:6,
      initial_balances:[{address:$a,amount:"1000000"}],
      manager:$a, treasury:$a,
      buy_bps:0, sell_bps:0,
      max_buy_bps:0, max_sell_bps:0, max_transfer_bps:0,
      features:["mint_control"],
      mint:{minter:$a, cap:null},
      transfer_bps:null, sinks:null,
      launch_guards:null,
      autolp_threshold:null, autolp_lp_recipient:null
    }
  }')"
MINT_CREATE_TX="$(send_cw20_hook "$UST1_ADDR" "$LAUNCHER_ADDR" "$INVOICE" "$MINT_HOOK")"
MINT_JSON="$(terrad_wait_tx_query "$CONTAINER" "$MINT_CREATE_TX" "$TERRAD_NODE")"
MINT_TOKEN="$(echo "$MINT_JSON" | terrad_jq_contract_address_from_tx_json | head -1)"
[[ "$MINT_TOKEN" == terra1* ]] || {
  echo "FAIL: MintControl create did not yield a contract" >&2
  exit 1
}
MINT_OK="$(jq -nc --arg r "$TEST_ADDRESS" '{mint:{recipient:$r,amount:"1"}}')"
exec_ok wasm execute "$MINT_TOKEN" "$MINT_OK" >/dev/null
REV_HOOK='{"update_settings":{"settings":{"revoke_mint":true}}}'
REV_TX="$(send_cw20_hook "$UST1_ADDR" "$MINT_TOKEN" "$INVOICE" "$REV_HOOK")"
set +e
MINT_OUT="$(terrad_tx wasm execute "$MINT_TOKEN" "$MINT_OK" 2>&1)"
MINT_ST=$?
set -e
if [[ "$MINT_ST" -eq 0 ]] && ! layer_execute_rejected "$MINT_OUT"; then
  echo "FAIL: mint succeeded after RevokeMint (T592-6)" >&2
  printf '%s\n' "$MINT_OUT" >&2
  exit 1
fi
echo "601-smoke: MintControl revoke one-way (T592-6)"

# Paid create with one SKU, then Enable Feature a second SKU (#612 / M612-5).
PAID_HOOK="$(jq -nc --arg n "PaidTax" --arg s "PDT" --arg a "$TEST_ADDRESS" \
  --arg treas "$TREASURY" \
  --argjson buy "$BUY_BPS" --argjson sell "$SELL_BPS" \
  '{
    create_token:{
      name:$n, symbol:$s, decimals:6,
      initial_balances:[{address:$a,amount:"1000000000000"}],
      manager:$a, treasury:$treas,
      buy_bps:$buy, sell_bps:$sell,
      max_buy_bps:$buy, max_sell_bps:$sell, max_transfer_bps:100,
      features:["transfer_tax"],
      mint:null, transfer_bps:100, sinks:null,
      launch_guards:null,
      autolp_threshold:null, autolp_lp_recipient:null
    }
  }')"
PAID_CREATE_TX="$(send_cw20_hook "$UST1_ADDR" "$LAUNCHER_ADDR" "$INVOICE" "$PAID_HOOK")"
PAID_JSON="$(terrad_wait_tx_query "$CONTAINER" "$PAID_CREATE_TX" "$TERRAD_NODE")"
PAID_TOKEN="$(echo "$PAID_JSON" | terrad_jq_contract_address_from_tx_json | head -1)"
[[ "$PAID_TOKEN" == terra1* ]] || {
  echo "FAIL: paid create did not yield a contract" >&2
  exit 1
}
PAID_FEAT="$(layer_smart "$PAID_TOKEN" '{"get_features":{}}')"
echo "$PAID_FEAT" | jq -e '.transfer_tax == true and .variable_rates == false' >/dev/null || {
  echo "FAIL: paid create should start with transfer_tax only: $PAID_FEAT" >&2
  exit 1
}
PAID_SKU2_HOOK="$(jq -nc --arg t "$PAID_TOKEN" '{"enable_feature":{"token":$t,"sku":"variable_rates"}}')"
PAID_SKU2_TX="$(send_cw20_hook "$UST1_ADDR" "$LAUNCHER_ADDR" "$INVOICE" "$PAID_SKU2_HOOK")"
PAID_FEAT2="$(layer_smart "$PAID_TOKEN" '{"get_features":{}}')"
echo "$PAID_FEAT2" | jq -e '.transfer_tax == true and .variable_rates == true' >/dev/null || {
  echo "FAIL: second SKU variable_rates not set via launcher: $PAID_FEAT2" >&2
  exit 1
}
echo "601-smoke: paid create transfer_tax + Enable Feature variable_rates via launcher (#612)"

jq -nc \
  --arg token_code "$TOKEN_CODE" \
  --arg launcher_code "$LAUNCHER_CODE" \
  --arg launcher "$LAUNCHER_ADDR" \
  --arg free "$FREE_TOKEN" \
  --arg rogue "$ROGUE_TOKEN" \
  --arg pair "$PAIR_ADDR" \
  --arg mint_tok "$MINT_TOKEN" \
  --arg paid "$PAID_TOKEN" \
  --arg free_tx "$FREE_TX" \
  --arg sell_tx "$SELL_TX" \
  --arg buy_tx "$BUY_TX" \
  --arg sku_tx "$SKU_TX" \
  --arg set_tx "$SET_TX" \
  --arg rev_tx "$REV_TX" \
  --arg paid_tx "$PAID_CREATE_TX" \
  --arg sku2_tx "$PAID_SKU2_TX" \
  --arg pin "$got_pin" \
  '{
    executed: true,
    free_profile_create: true,
    launcher_admin_cmm: true,
    launcher_origin_set: true,
    rogue_origin_null: true,
    provide_one_to_one: true,
    sell_extra_debit: true,
    buy_outbound_split: true,
    sku_unlock_50_ust1: true,
    sku_unlock_via_launcher: true,
    settings_batch_50_ust1: true,
    mintcontrol_revoke_one_way: true,
    paid_create_one_sku: true,
    sku_second_unlock_via_launcher: true,
    local_token_code_id: $token_code,
    local_launcher_code_id: $launcher_code,
    launcher: $launcher,
    free_token: $free,
    rogue_token: $rogue,
    pair: $pair,
    mint_token: $mint_tok,
    paid_token: $paid,
    pin: $pin,
    note: "LocalTerra only. Instantiates current token+launcher (not 11611). Do not whitelist this store id, 11612, 11613, or 8654 on columbus-5."
  }' > "$OUT_JSON"

echo "601-smoke: wrote $OUT_JSON"
echo "PASS: LocalTerra community-tax smoke (O601-3–O601-6)"
