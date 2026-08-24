#!/usr/bin/env bash
# Layer B tax-on: named community-tax DEX suite (GitLab #623).
#
# Separate from generic B-lt. Do **not** RegisterListedPair or extra-debit
# inside layer-b-lt.sh (C623-1 / C589-5). This script answers the tax-on
# question after register: extra-debit, outbound split, official-router
# trader, limit Place 1:1, AutoLP floor.
#
# Inputs (first match):
#   1. Seed deploy pins (VITE_TOKEN_COMMUNITY_TAX_ADDRESS + pair + AutoLP)
#   2. Ephemeral store/instantiate (nonzero bps, AutoLP on)
#
# Usage:
#   LAYER_B_TAX_ON=1 ./cw20-codeid-audits/scripts/layer-b-tax-on.sh
# Refuses to run without LocalTerra. Never stub PASS. Never whitelist
# columbus-5 11611/11619/8654 from this evidence.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AUDIT_ROOT="$(cd "${CW20_AUDIT_ROOT:-$SCRIPT_DIR/..}" && pwd)"
REPO_ROOT="$(cd "$AUDIT_ROOT/.." && pwd)"
cd "$REPO_ROOT"

OUT_JSON="${LAYER_B_TAX_ON_JSON:-$AUDIT_ROOT/harness/layer-b-tax-on.json}"
mkdir -p "$(dirname "$OUT_JSON")"

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
source "$SCRIPT_DIR/lib-layer-lt.sh"
# shellcheck source=cw20-codeid-audits/scripts/lib-tax-on.sh
source "$SCRIPT_DIR/lib-tax-on.sh"

layer_require_localterra || exit 1

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
[[ -n "${VITE_ROUTER_ADDRESS:-}" ]] || {
  echo "FAIL: VITE_ROUTER_ADDRESS unset (official-router hop required)." >&2
  exit 1
}

TEST2="$(layer_ensure_test2)"
[[ "$TEST2" == terra1* ]] || {
  echo "FAIL: test2 address missing" >&2
  exit 1
}

SELL_TX=""
BUY_TX=""
BUY_USER=""
ROUTER_TX=""
SPOOF_TX=""
LIMIT_TX=""
SKIM_TX=""
REGISTER_TX=""
SOURCE="ephemeral"
LOCAL_TOKEN_CODE=""
TOKEN_ADDR=""
PAIR_ADDR=""
AUTOLP_ADDR=""
TREASURY=""
BUY_BPS="$TAX_ON_BUY_BPS"
SELL_BPS="$TAX_ON_SELL_BPS"
HOP2_TOKEN=""
HOP2_PAIR=""

resolve_second_hop() {
  local ember="$VITE_TOKEN_EMBER_ADDRESS"
  local cand pair
  for cand in \
    "${VITE_TOKEN_CORAL_ADDRESS:-}" \
    "${VITE_TOKEN_JADE_ADDRESS:-}" \
    "${VITE_TOKEN_ONYX_ADDRESS:-}" \
    "${VITE_USTC_C_TOKEN_ADDRESS:-}" \
    "${VITE_LUNC_C_TOKEN_ADDRESS:-}"; do
    [[ "$cand" == terra1* ]] || continue
    pair="$(tax_on_resolve_pair "$VITE_FACTORY_ADDRESS" "$ember" "$cand")"
    if [[ "$pair" == terra1* ]]; then
      HOP2_TOKEN="$cand"
      HOP2_PAIR="$pair"
      return 0
    fi
  done
  # First factory page is enough on LocalTerra (EMBER gem pairs are created early).
  local raw
  raw="$(layer_smart "$VITE_FACTORY_ADDRESS" '{"pairs":{"start_after":null,"limit":30}}')"
  while IFS=$'\t' read -r a0 a1 pair; do
    [[ "$pair" == terra1* ]] || continue
    [[ "$pair" != "$PAIR_ADDR" ]] || continue
    if [[ "$a0" == "$ember" && "$a1" == terra1* && "$a1" != "$TOKEN_ADDR" ]]; then
      HOP2_TOKEN="$a1"
      HOP2_PAIR="$pair"
      return 0
    fi
    if [[ "$a1" == "$ember" && "$a0" == terra1* && "$a0" != "$TOKEN_ADDR" ]]; then
      HOP2_TOKEN="$a0"
      HOP2_PAIR="$pair"
      return 0
    fi
  done < <(echo "$raw" | jq -r '.pairs[]? | [
      (.asset_infos[0].token.contract_addr // ""),
      (.asset_infos[1].token.contract_addr // ""),
      .contract_addr
    ] | @tsv')
  return 1
}

fund_trader() {
  # Prefer a wallet that is not the token treasury so extra-debit is visible.
  if [[ "$TREASURY" == "$TEST_ADDRESS" ]]; then
    printf '%s' "$TEST2"
  else
    printf '%s' "$TEST_ADDRESS"
  fi
}

ensure_trader_funded() {
  local token="$1" trader="$2" need="$3"
  local have
  have="$(layer_cw20_balance "$token" "$trader")"
  if python3 -c "import sys; sys.exit(0 if int(sys.argv[1]) >= int(sys.argv[2]) else 1)" \
    "$have" "$need"; then
    return 0
  fi
  [[ "$trader" != "$TEST_ADDRESS" ]] || {
    echo "FAIL: test1 tax balance $have < $need" >&2
    exit 1
  }
  local xfer
  xfer="$(jq -nc --arg r "$trader" --arg amt "$need" '{transfer:{recipient:$r,amount:$amt}}')"
  tax_on_exec_ok wasm execute "$token" "$xfer" >/dev/null
}

use_seed_pins() {
  TOKEN_ADDR="${VITE_TOKEN_COMMUNITY_TAX_ADDRESS:-}"
  PAIR_ADDR="${VITE_PAIR_COMMUNITY_TAX_EMBER:-}"
  [[ "$TOKEN_ADDR" == terra1* && "$PAIR_ADDR" == terra1* ]] || return 1
  local cfg alp
  cfg="$(layer_smart "$TOKEN_ADDR" '{"get_config":{}}')"
  TREASURY="$(echo "$cfg" | jq -r '.treasury // empty')"
  BUY_BPS="$(echo "$cfg" | jq -r '.buy_bps')"
  SELL_BPS="$(echo "$cfg" | jq -r '.sell_bps')"
  AUTOLP_ADDR="$(echo "$cfg" | jq -r '.autolp // empty')"
  if [[ "$AUTOLP_ADDR" != terra1* ]]; then
    AUTOLP_ADDR=""
  fi
  LOCAL_TOKEN_CODE="${VITE_COMMUNITY_TAX_CODE_ID:-}"
  if [[ "$BUY_BPS" == "0" && "$SELL_BPS" == "0" ]]; then
    echo "FAIL: seed token buy/sell bps are 0 — tax-on suite needs nonzero rates." >&2
    exit 1
  fi
  SOURCE="seed"
  echo "tax-on: using seed pins token=$TOKEN_ADDR pair=$PAIR_ADDR autolp=${AUTOLP_ADDR:-none}"
}

instantiate_ephemeral() {
  local token_wasm autolp_wasm mintable_wasm
  token_wasm="$(tax_on_find_wasm cl8y_community_tax_token cl8y-community-tax-token)"
  autolp_wasm="$(tax_on_find_wasm cl8y_community_tax_autolp cl8y-community-tax-autolp)"
  mintable_wasm="$(tax_on_find_wasm cw20_mintable cw20-mintable || true)"
  if [[ -z "$mintable_wasm" || ! -f "$mintable_wasm" ]]; then
    mintable_wasm="$REPO_ROOT/smartcontracts/artifacts/cw20_mintable.wasm"
    local primary
    primary="$(cd "$REPO_ROOT/.." 2>/dev/null && pwd)/cl8y-dex-terraclassic/smartcontracts/artifacts/cw20_mintable.wasm"
    [[ -f "$mintable_wasm" ]] || mintable_wasm="$primary"
  fi
  [[ -f "$mintable_wasm" ]] || {
    echo "FAIL: cw20_mintable.wasm missing (UST1 stand-in)." >&2
    exit 1
  }

  echo "tax-on: storing current token + AutoLP (local ids only)…"
  LOCAL_TOKEN_CODE="$(tax_on_store_wasm "$token_wasm" /tmp/cw20-audit-tax-on-token.wasm)"
  local autolp_code
  autolp_code="$(tax_on_store_wasm "$autolp_wasm" /tmp/cw20-audit-tax-on-autolp.wasm)"
  echo "tax-on: local codes token=$LOCAL_TOKEN_CODE autolp=$autolp_code"

  tax_on_whitelist_local "$VITE_FACTORY_ADDRESS" "$LOCAL_TOKEN_CODE"

  local ust1="${VITE_UST1_TOKEN_ADDRESS:-}"
  if [[ "$ust1" != terra1* ]]; then
    local stamp mint_code ust1_init ust1_tx
    stamp="$(date +%s)"
    mint_code="$(tax_on_store_wasm "$mintable_wasm" /tmp/cw20-audit-tax-on-mintable.wasm)"
    ust1_init="$(jq -nc --arg a "$TEST_ADDRESS" \
      '{name:"TaxOnUST1",symbol:"TU1",decimals:6,initial_balances:[{address:$a,amount:"1000000000000"}],mint:{minter:$a}}')"
    ust1_tx="$(tax_on_exec_ok wasm instantiate "$mint_code" "$ust1_init" \
      --label "623-ust1-${stamp}" --admin "$TEST_ADDRESS")"
    ust1="$(tax_on_contract_from_tx "$ust1_tx")"
  fi
  [[ "$ust1" == terra1* ]] || {
    echo "FAIL: UST1 stand-in missing" >&2
    exit 1
  }

  TREASURY="$TEST2"
  local stamp sym init tx
  stamp="$(date +%s)"
  sym="$(layer_terraport_symbol)"
  init="$(tax_on_token_init_msg "TaxOn" "$sym" "$TEST_ADDRESS" "$TREASURY" \
    "$VITE_FACTORY_ADDRESS" "$VITE_ROUTER_ADDRESS" "$ust1" \
    "$TAX_ON_BUY_BPS" "$TAX_ON_SELL_BPS")"
  tx="$(tax_on_exec_ok wasm instantiate "$LOCAL_TOKEN_CODE" "$init" \
    --label "623-tax-${stamp}" --admin "$TEST_ADDRESS")"
  TOKEN_ADDR="$(tax_on_contract_from_tx "$tx")"
  [[ "$TOKEN_ADDR" == terra1* ]] || {
    echo "FAIL: ephemeral tax token instantiate address missing" >&2
    exit 1
  }
  BUY_BPS="$TAX_ON_BUY_BPS"
  SELL_BPS="$TAX_ON_SELL_BPS"
  echo "tax-on: ephemeral token=$TOKEN_ADDR treasury=$TREASURY"

  PAIR_ADDR="$(tax_on_create_pair "$VITE_FACTORY_ADDRESS" "$TOKEN_ADDR" "$VITE_TOKEN_EMBER_ADDRESS")"
  echo "tax-on: pair=$PAIR_ADDR"

  local alp_init alp_tx
  alp_init="$(jq -nc \
    --arg tok "$TOKEN_ADDR" --arg mgr "$TEST_ADDRESS" \
    --arg factory "$VITE_FACTORY_ADDRESS" --arg router "$VITE_ROUTER_ADDRESS" \
    --arg pair "$PAIR_ADDR" --arg lp "$TEST_ADDRESS" \
    '{
      token:$tok, manager:$mgr, factory:$factory,
      router:$router, pair:$pair, quote_token:null,
      threshold:"1", lp_recipient:$lp,
      skim_max_spread:null, skim_min_return:null
    }')"
  alp_tx="$(tax_on_exec_ok wasm instantiate "$autolp_code" "$alp_init" \
    --label "623-autolp-${stamp}" --admin "$TEST_ADDRESS")"
  AUTOLP_ADDR="$(tax_on_contract_from_tx "$alp_tx")"
  [[ "$AUTOLP_ADDR" == terra1* ]] || {
    echo "FAIL: AutoLP instantiate address missing" >&2
    exit 1
  }
  echo "tax-on: AutoLP=$AUTOLP_ADDR"
}

if use_seed_pins; then
  :
else
  instantiate_ephemeral
fi

# --- RegisterListedPair (if not already) ---
if tax_on_is_listed_pair "$TOKEN_ADDR" "$PAIR_ADDR"; then
  echo "tax-on: RegisterListedPair already set"
else
  REGISTER_TX="$(tax_on_exec_ok wasm execute "$TOKEN_ADDR" \
    "$(jq -nc --arg p "$PAIR_ADDR" '{register_listed_pair:{pair:$p}}')")"
  tax_on_is_listed_pair "$TOKEN_ADDR" "$PAIR_ADDR" || {
    echo "FAIL: pair not protocol-exempt after RegisterListedPair" >&2
    exit 1
  }
fi

# Non-factory register must fail (T592-9 / D8).
FAKE_REG_OUT="$(tax_on_try_tx wasm execute "$TOKEN_ADDR" \
  "$(jq -nc --arg p "$VITE_TOKEN_EMBER_ADDRESS" '{register_listed_pair:{pair:$p}}')")"
if ! layer_execute_rejected "$FAKE_REG_OUT"; then
  echo "FAIL: RegisterListedPair accepted a non-factory address (T592-9)" >&2
  exit 1
fi
echo "tax-on: non-factory RegisterListedPair rejected (T592-9)"

# --- Provide 1:1 after register ---
tax_on_provide_1to1 "$TOKEN_ADDR" "$VITE_TOKEN_EMBER_ADDRESS" "$PAIR_ADDR" "$TAX_ON_PROVIDE_RAW"

TRADER="$(fund_trader)"
NEED=$((TAX_ON_SWAP_RAW * 8 + TAX_ON_LIMIT_RAW * 2 + TAX_ON_SKIM_RAW * 3))
ensure_trader_funded "$TOKEN_ADDR" "$TRADER" "$NEED"
TRADER_KEY="test1"
if [[ "$TRADER" == "$TEST2" ]]; then
  TRADER_KEY="test2"
fi

# --- TaxPreview sell ---
EXPECT_TAX="$(tax_on_preview_sell "$TOKEN_ADDR" "$TRADER" "$PAIR_ADDR" "$TAX_ON_SWAP_RAW" "$SELL_BPS")"
echo "tax-on: TaxPreview sell extra-debit $((TAX_ON_SWAP_RAW + EXPECT_TAX)) (tax=$EXPECT_TAX)"

SWAP_HOOK='{"swap":{"max_spread":"1"}}'

# --- Pair-direct sell ---
USER_BEFORE="$(layer_cw20_balance "$TOKEN_ADDR" "$TRADER")"
PAIR_BEFORE="$(layer_cw20_balance "$TOKEN_ADDR" "$PAIR_ADDR")"
SINK_BEFORE="$(layer_cw20_balance "$TOKEN_ADDR" "$TREASURY")"
if [[ "$TRADER_KEY" == "test1" ]]; then
  SELL_TX="$(tax_on_send_cw20_hook "$TOKEN_ADDR" "$PAIR_ADDR" "$TAX_ON_SWAP_RAW" "$SWAP_HOOK")"
else
  SELL_TX="$(tax_on_send_cw20_hook_from test2 "$TOKEN_ADDR" "$PAIR_ADDR" "$TAX_ON_SWAP_RAW" "$SWAP_HOOK")"
fi
USER_AFTER="$(layer_cw20_balance_changed "$TOKEN_ADDR" "$TRADER" "$USER_BEFORE")"
PAIR_AFTER="$(layer_cw20_balance_changed "$TOKEN_ADDR" "$PAIR_ADDR" "$PAIR_BEFORE")"
SINK_AFTER="$(layer_cw20_balance "$TOKEN_ADDR" "$TREASURY")"
tax_on_assert_sell "$USER_BEFORE" "$USER_AFTER" "$PAIR_BEFORE" "$PAIR_AFTER" \
  "$SINK_BEFORE" "$SINK_AFTER" "$TAX_ON_SWAP_RAW" "$EXPECT_TAX"

# --- Pair-direct buy (EMBER → tax token outbound split) ---
# Buyer must not be treasury (tax sink returns to the same wallet → 1:1) and
# must not be manager-directory exempt (#609). Reuse pick_trader / TRADER.
# Seed leftover: test1 is manager (+ was treasury); buy from test1 failed
# `user credit >= pair debit` even with buy_bps=500 (#625 leftover #1).
BUY_USER="$TRADER"
[[ "$BUY_USER" != "$TREASURY" ]] || {
  echo "FAIL: buy wallet $BUY_USER is the token treasury — outbound split is invisible" >&2
  exit 1
}
ensure_trader_funded "$VITE_TOKEN_EMBER_ADDRESS" "$BUY_USER" "$TAX_ON_SWAP_RAW"
EMBER_HAVE="$(layer_cw20_balance "$VITE_TOKEN_EMBER_ADDRESS" "$BUY_USER")"
python3 -c 'import sys; sys.exit(0 if int(sys.argv[1]) >= int(sys.argv[2]) else 1)' \
  "$EMBER_HAVE" "$TAX_ON_SWAP_RAW" || {
  echo "FAIL: EMBER balance $EMBER_HAVE < $TAX_ON_SWAP_RAW on $BUY_USER" >&2
  exit 1
}
USER_TOK_BEFORE="$(layer_cw20_balance "$TOKEN_ADDR" "$BUY_USER")"
PAIR_TOK_BEFORE="$(layer_cw20_balance "$TOKEN_ADDR" "$PAIR_ADDR")"
if [[ "$TRADER_KEY" == "test1" ]]; then
  BUY_TX="$(tax_on_send_cw20_hook "$VITE_TOKEN_EMBER_ADDRESS" "$PAIR_ADDR" "$TAX_ON_SWAP_RAW" "$SWAP_HOOK")"
else
  BUY_TX="$(tax_on_send_cw20_hook_from test2 "$VITE_TOKEN_EMBER_ADDRESS" "$PAIR_ADDR" "$TAX_ON_SWAP_RAW" "$SWAP_HOOK")"
fi
USER_TOK_AFTER="$(layer_cw20_balance_changed "$TOKEN_ADDR" "$BUY_USER" "$USER_TOK_BEFORE")"
PAIR_TOK_AFTER="$(layer_cw20_balance_changed "$TOKEN_ADDR" "$PAIR_ADDR" "$PAIR_TOK_BEFORE")"
tax_on_assert_buy_split "$USER_TOK_BEFORE" "$USER_TOK_AFTER" \
  "$PAIR_TOK_BEFORE" "$PAIR_TOK_AFTER" "$BUY_BPS"

# --- Spoofed pair-direct trader does not move a third wallet ---
# Factory is not the trader or treasury; pair-direct must ignore Swap.trader.
VICTIM="$VITE_FACTORY_ADDRESS"
SPOOF_HOOK="$(jq -nc --arg t "$VICTIM" '{swap:{max_spread:"1",trader:$t}}')"
VICTIM_BEFORE="$(layer_cw20_balance "$TOKEN_ADDR" "$VICTIM")"
SPOOF_USER_BEFORE="$(layer_cw20_balance "$TOKEN_ADDR" "$TRADER")"
if [[ "$TRADER_KEY" == "test1" ]]; then
  SPOOF_TX="$(tax_on_send_cw20_hook "$TOKEN_ADDR" "$PAIR_ADDR" "$TAX_ON_SWAP_RAW" "$SPOOF_HOOK")"
else
  SPOOF_TX="$(tax_on_send_cw20_hook_from test2 "$TOKEN_ADDR" "$PAIR_ADDR" "$TAX_ON_SWAP_RAW" "$SPOOF_HOOK")"
fi
VICTIM_AFTER="$(layer_cw20_balance "$TOKEN_ADDR" "$VICTIM")"
SPOOF_USER_AFTER="$(layer_cw20_balance_changed "$TOKEN_ADDR" "$TRADER" "$SPOOF_USER_BEFORE")"
python3 -c '
import sys
vb, va, ub, ua, amt, tax = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), int(sys.argv[5]), int(sys.argv[6])
if vb != va:
    sys.stderr.write(f"FAIL: spoofed trader victim moved {vb}->{va}\n")
    sys.exit(1)
if ub - ua != amt + tax:
    sys.stderr.write(f"FAIL: spoof pair-direct from debit {ub}->{ua} != {amt}+{tax}\n")
    sys.exit(1)
print("tax-on: pair-direct ignores spoofed trader (from extra-debited)")
' "$VICTIM_BEFORE" "$VICTIM_AFTER" "$SPOOF_USER_BEFORE" "$SPOOF_USER_AFTER" \
  "$TAX_ON_SWAP_RAW" "$EXPECT_TAX"

# --- Official-router ≥2hop: extra-debit trader; pair→router 1:1 ---
resolve_second_hop || {
  echo "FAIL: no EMBER second hop (CORAL/JADE/ONYX/cUSTC/cLUNC pair). Redeploy gems." >&2
  exit 1
}
echo "tax-on: 2hop TAX→EMBER→$(echo "$HOP2_TOKEN" | cut -c1-16)… via $HOP2_PAIR"
ROUTER_HOOK="$(jq -nc --arg tax "$TOKEN_ADDR" --arg ember "$VITE_TOKEN_EMBER_ADDRESS" --arg hop2 "$HOP2_TOKEN" \
  '{
    execute_swap_operations:{
      operations:[
        {terra_swap:{offer_asset_info:{token:{contract_addr:$tax}},ask_asset_info:{token:{contract_addr:$ember}}}},
        {terra_swap:{offer_asset_info:{token:{contract_addr:$ember}},ask_asset_info:{token:{contract_addr:$hop2}}}}
      ],
      max_spread:"1"
    }
  }')"
R_USER_B="$(layer_cw20_balance "$TOKEN_ADDR" "$TRADER")"
R_PAIR_B="$(layer_cw20_balance "$TOKEN_ADDR" "$PAIR_ADDR")"
R_ROUTER_B="$(layer_cw20_balance "$TOKEN_ADDR" "$VITE_ROUTER_ADDRESS")"
R_SINK_B="$(layer_cw20_balance "$TOKEN_ADDR" "$TREASURY")"
if [[ "$TRADER_KEY" == "test1" ]]; then
  ROUTER_TX="$(tax_on_send_cw20_hook "$TOKEN_ADDR" "$VITE_ROUTER_ADDRESS" "$TAX_ON_SWAP_RAW" "$ROUTER_HOOK")"
else
  ROUTER_TX="$(tax_on_send_cw20_hook_from test2 "$TOKEN_ADDR" "$VITE_ROUTER_ADDRESS" "$TAX_ON_SWAP_RAW" "$ROUTER_HOOK")"
fi
R_USER_A="$(layer_cw20_balance_changed "$TOKEN_ADDR" "$TRADER" "$R_USER_B")"
R_PAIR_A="$(layer_cw20_balance_changed "$TOKEN_ADDR" "$PAIR_ADDR" "$R_PAIR_B")"
R_ROUTER_A="$(layer_cw20_balance "$TOKEN_ADDR" "$VITE_ROUTER_ADDRESS")"
R_SINK_A="$(layer_cw20_balance "$TOKEN_ADDR" "$TREASURY")"
python3 -c '
import sys
ub, ua, pb, pa, rb, ra, sb, sa, amt, tax = (int(x) for x in sys.argv[1:])
if ub - ua != amt + tax:
    sys.stderr.write(f"FAIL: router hop trader debit {ub}->{ua} != {amt}+{tax} (T592-13)\n")
    sys.exit(1)
if pa - pb != amt:
    sys.stderr.write(f"FAIL: router hop pair credit {pb}->{pa} != {amt} (inbound 1:1)\n")
    sys.exit(1)
if ra != rb:
    sys.stderr.write(f"FAIL: router TAX net {rb}->{ra} (pair→router must stay 1:1; router must not keep tax)\n")
    sys.exit(1)
if sa - sb != tax:
    sys.stderr.write(f"FAIL: router hop treasury {sb}->{sa} != {tax}\n")
    sys.exit(1)
print("tax-on: official-router ≥2hop extra-debits trader; pair credit 1:1")
' "$R_USER_B" "$R_USER_A" "$R_PAIR_B" "$R_PAIR_A" "$R_ROUTER_B" "$R_ROUTER_A" \
  "$R_SINK_B" "$R_SINK_A" "$TAX_ON_SWAP_RAW" "$EXPECT_TAX"

# --- Missing router trader fail-closes (probe token: config.router = test2) ---
echo "tax-on: missing-trader probe (router=test2)…"
PROBE_SYM="$(layer_terraport_symbol)"
UST1_FOR_PROBE="${VITE_UST1_TOKEN_ADDRESS:-$VITE_TOKEN_EMBER_ADDRESS}"
if [[ "$SOURCE" == "ephemeral" ]]; then
  PROBE_CODE="$LOCAL_TOKEN_CODE"
else
  PROBE_WASM="$(tax_on_find_wasm cl8y_community_tax_token cl8y-community-tax-token)"
  PROBE_CODE="$(tax_on_store_wasm "$PROBE_WASM" /tmp/cw20-audit-tax-on-probe.wasm)"
  tax_on_whitelist_local "$VITE_FACTORY_ADDRESS" "$PROBE_CODE"
fi
PROBE_INIT="$(tax_on_token_init_msg "ProbeTax" "$PROBE_SYM" "$TEST_ADDRESS" "$TEST_ADDRESS" \
  "$VITE_FACTORY_ADDRESS" "$TEST2" "$UST1_FOR_PROBE" "$TAX_ON_BUY_BPS" "$TAX_ON_SELL_BPS")"
PROBE_TX="$(tax_on_exec_ok wasm instantiate "$PROBE_CODE" "$PROBE_INIT" \
  --label "623-probe-$(date +%s)" --admin "$TEST_ADDRESS")"
PROBE_TOKEN="$(tax_on_contract_from_tx "$PROBE_TX")"
PROBE_PAIR="$(tax_on_create_pair "$VITE_FACTORY_ADDRESS" "$PROBE_TOKEN" "$VITE_TOKEN_EMBER_ADDRESS")"
tax_on_register_listed "$PROBE_TOKEN" "$PROBE_PAIR"
PROBE_FUND="$(jq -nc --arg r "$TEST2" --arg amt "$TAX_ON_SWAP_RAW" '{transfer:{recipient:$r,amount:$amt}}')"
tax_on_exec_ok wasm execute "$PROBE_TOKEN" "$PROBE_FUND" >/dev/null
MISSING_OUT="$(tax_on_try_tx_from test2 wasm execute "$PROBE_TOKEN" \
  "$(jq -nc --arg c "$PROBE_PAIR" --arg amt "$TAX_ON_SWAP_RAW" --arg m "$(
    printf '%s' "$SWAP_HOOK" | base64 -w0 2>/dev/null || printf '%s' "$SWAP_HOOK" | base64 | tr -d '\n'
  )" '{send:{contract:$c,amount:$amt,msg:$m}}')")"
if ! layer_execute_rejected "$MISSING_OUT"; then
  echo "FAIL: router Send+Swap without trader succeeded (T592-13 fail-closed)" >&2
  printf '%s\n' "$MISSING_OUT" >&2
  exit 1
fi
echo "tax-on: missing router trader fail-closes"

# --- Place-limit Send 1:1 after register (must not extra-debit Place) ---
PAIR_INFO="$(layer_smart "$PAIR_ADDR" '{"pair":{}}')"
ASSET0="$(echo "$PAIR_INFO" | jq -r '.asset_infos[0].token.contract_addr // empty')"
if [[ "$ASSET0" == "$TOKEN_ADDR" ]]; then
  LIMIT_SIDE="ask"
  LIMIT_PRICE="10"
else
  LIMIT_SIDE="bid"
  LIMIT_PRICE="0.1"
fi
LIMIT_HOOK="$(jq -nc --arg side "$LIMIT_SIDE" --arg amt "$TAX_ON_LIMIT_RAW" --arg price "$LIMIT_PRICE" \
  '{place_limit_order_batch:{side:$side,orders:[{price:$price,amount:$amt,max_adjust_steps:32}]}}')"
FEE_CFG="$(layer_smart "$PAIR_ADDR" '{"get_fee_config":{}}')"
PAIR_TREASURY="$(echo "$FEE_CFG" | jq -r '.fee_config.treasury // .treasury // empty')"
if [[ "$PAIR_TREASURY" != terra1* ]]; then
  PAIR_TREASURY="${VITE_TREASURY_ADDRESS:-}"
fi
LIM_USER_B="$(layer_cw20_balance "$TOKEN_ADDR" "$TRADER")"
LIM_PAIR_B="$(layer_cw20_balance "$TOKEN_ADDR" "$PAIR_ADDR")"
TAX_SINK_B="$(layer_cw20_balance "$TOKEN_ADDR" "$TREASURY")"
LIM_Treas_B="0"
if [[ "$PAIR_TREASURY" == terra1* ]]; then
  LIM_Treas_B="$(layer_cw20_balance "$TOKEN_ADDR" "$PAIR_TREASURY")"
fi
if [[ "$TRADER_KEY" == "test1" ]]; then
  LIMIT_TX="$(tax_on_send_cw20_hook "$TOKEN_ADDR" "$PAIR_ADDR" "$TAX_ON_LIMIT_RAW" "$LIMIT_HOOK")"
else
  LIMIT_TX="$(tax_on_send_cw20_hook_from test2 "$TOKEN_ADDR" "$PAIR_ADDR" "$TAX_ON_LIMIT_RAW" "$LIMIT_HOOK")"
fi
LIM_PAIR_A="$(layer_cw20_balance_changed "$TOKEN_ADDR" "$PAIR_ADDR" "$LIM_PAIR_B")"
LIM_USER_A="$(layer_cw20_balance "$TOKEN_ADDR" "$TRADER")"
LIM_Treas_A="0"
if [[ "$PAIR_TREASURY" == terra1* && "$PAIR_TREASURY" != "$TRADER" ]]; then
  LIM_Treas_A="$(layer_cw20_balance "$TOKEN_ADDR" "$PAIR_TREASURY")"
else
  LIM_Treas_B="0"
  PAIR_TREASURY=""
fi
# Pair→EOA limit refund uses buy tax (T592-7). Count the token sink so
# conservation holds; Place itself must still not extra-debit.
# Maker fee can land after the pair CW20 moves; poll until conservation holds.
for _i in 1 2 3 4 5 6 7 8 9 10; do
  LIM_USER_A="$(layer_cw20_balance "$TOKEN_ADDR" "$TRADER")"
  LIM_PAIR_A="$(layer_cw20_balance "$TOKEN_ADDR" "$PAIR_ADDR")"
  if [[ "$PAIR_TREASURY" == terra1* ]]; then
    LIM_Treas_A="$(layer_cw20_balance "$TOKEN_ADDR" "$PAIR_TREASURY")"
  fi
  TAX_SINK_A="$(layer_cw20_balance "$TOKEN_ADDR" "$TREASURY")"
  if python3 -c '
import sys
ub, ua, pb, pa, tb, ta, amt = (int(x) for x in sys.argv[1:8])
debit = ub - ua
credited = (pa - pb) + (ta - tb)
sys.exit(0 if debit > 0 and debit <= amt and credited == debit else 1)
' "$LIM_USER_B" "$LIM_USER_A" "$LIM_PAIR_B" "$LIM_PAIR_A" \
    "$LIM_Treas_B" "$LIM_Treas_A" "$TAX_ON_LIMIT_RAW"; then
    break
  fi
  sleep 0.4
done
tax_on_assert_limit_1to1 "$LIM_USER_B" "$LIM_USER_A" "$LIM_PAIR_B" "$LIM_PAIR_A" \
  "$LIM_Treas_B" "$LIM_Treas_A" "$TAX_ON_LIMIT_RAW" "$TAX_SINK_B" "$TAX_SINK_A" \
  "$([[ "$TREASURY" == "$TRADER" ]] && echo 1 || echo 0)"

# --- AutoLP: factory pair, skim floor, fake pair rejected ---
[[ "$AUTOLP_ADDR" == terra1* ]] || {
  echo "FAIL: AutoLP address missing (tax-on suite requires AutoLP on)" >&2
  exit 1
}
ALP_CFG="$(layer_smart "$AUTOLP_ADDR" '{"get_config":{}}')"
echo "$ALP_CFG" | jq -e --arg p "$PAIR_ADDR" '.pair == $p' >/dev/null || {
  echo "FAIL: AutoLP GetConfig.pair != factory tax pair (M610-1): $ALP_CFG" >&2
  exit 1
}
echo "tax-on: AutoLP factory pair set"

# Fake / inverted pair rejected on UpdateConfig (M610-7).
FAKE_ALP="$(tax_on_try_tx wasm execute "$AUTOLP_ADDR" \
  "$(jq -nc --arg p "$VITE_TOKEN_EMBER_ADDRESS" '{update_config:{pair:$p}}')")"
if ! layer_execute_rejected "$FAKE_ALP"; then
  echo "FAIL: AutoLP accepted a random CW20 as pair (M610-1/7)" >&2
  exit 1
fi
echo "tax-on: AutoLP fake pair rejected"

# Leftover #625: a prior tax-on hostile probe leaves skim_min_return=1e15 on
# the seed AutoLP. Zero clears it (contract treats 0 as None / M610-3).
CLEAR_MIN="$(jq -nc '{update_config:{skim_min_return:"0"}}')"
tax_on_exec_ok wasm execute "$AUTOLP_ADDR" "$CLEAR_MIN" >/dev/null
echo "tax-on: cleared leftover skim_min_return"

# Deep-pool skim should succeed at default 100 bps floor.
SKIM_SEED="$(jq -nc --arg r "$AUTOLP_ADDR" --arg amt "$TAX_ON_SKIM_RAW" \
  '{transfer:{recipient:$r,amount:$amt}}')"
tax_on_exec_ok wasm execute "$TOKEN_ADDR" "$SKIM_SEED" >/dev/null
ALP_BEFORE="$(layer_cw20_balance "$TOKEN_ADDR" "$AUTOLP_ADDR")"
SKIM_TX="$(tax_on_exec_ok wasm execute "$AUTOLP_ADDR" '{"skim_to_lp":{}}')"
ALP_AFTER="$(layer_cw20_balance "$TOKEN_ADDR" "$AUTOLP_ADDR")"
python3 -c '
import sys
b, a = int(sys.argv[1]), int(sys.argv[2])
if a >= b:
    sys.stderr.write(f"FAIL: SkimToLp did not consume AutoLP tax {b}->{a}\n")
    sys.exit(1)
print(f"tax-on: SkimToLp succeeded at floor ({b}->{a})")
' "$ALP_BEFORE" "$ALP_AFTER"

# Hostile min_return: skim reverts, tax remains (M610-4).
HOSTILE="$(jq -nc '{update_config:{skim_min_return:"1000000000000000"}}')"
tax_on_exec_ok wasm execute "$AUTOLP_ADDR" "$HOSTILE" >/dev/null
tax_on_exec_ok wasm execute "$TOKEN_ADDR" "$SKIM_SEED" >/dev/null
HOSTILE_BEFORE="$(layer_cw20_balance "$TOKEN_ADDR" "$AUTOLP_ADDR")"
HOSTILE_OUT="$(tax_on_try_tx wasm execute "$AUTOLP_ADDR" '{"skim_to_lp":{}}')"
if ! layer_execute_rejected "$HOSTILE_OUT"; then
  echo "FAIL: hostile SkimToLp succeeded (expected floor revert)" >&2
  exit 1
fi
HOSTILE_AFTER="$(layer_cw20_balance "$TOKEN_ADDR" "$AUTOLP_ADDR")"
[[ "$HOSTILE_AFTER" == "$HOSTILE_BEFORE" ]] || {
  echo "FAIL: hostile skim changed AutoLP $HOSTILE_BEFORE->$HOSTILE_AFTER (tax must stay)" >&2
  exit 1
}
echo "tax-on: hostile SkimToLp reverted; tax remains on AutoLP"
tax_on_exec_ok wasm execute "$AUTOLP_ADDR" "$CLEAR_MIN" >/dev/null
echo "tax-on: restored skim_min_return so leftover re-runs are not stuck"

jq -nc \
  --arg source "$SOURCE" \
  --arg token "$TOKEN_ADDR" \
  --arg pair "$PAIR_ADDR" \
  --arg autolp "$AUTOLP_ADDR" \
  --arg local "${LOCAL_TOKEN_CODE}" \
  --arg trader "$TRADER" \
  --arg buy_user "$BUY_USER" \
  --arg hop2 "$HOP2_TOKEN" \
  --arg sell "$SELL_TX" \
  --arg buy "$BUY_TX" \
  --arg router "$ROUTER_TX" \
  --arg spoof "$SPOOF_TX" \
  --arg limit "$LIMIT_TX" \
  --arg skim "$SKIM_TX" \
  --arg probe "$PROBE_TOKEN" \
  '{
    executed: true,
    source: $source,
    pair_direct_sell: true,
    pair_direct_buy: true,
    router_trader: true,
    spoof_trader_negative: true,
    missing_router_trader_fail_closed: true,
    limit_one_to_one: true,
    autolp_floor: true,
    autolp_fake_pair_rejected: true,
    provide_one_to_one: true,
    token: $token,
    pair: $pair,
    autolp: $autolp,
    local_token_code_id: $local,
    trader: $trader,
    buy_user: $buy_user,
    hop2_token: $hop2,
    sell_tx: $sell,
    buy_tx: $buy,
    router_tx: $router,
    spoof_tx: $spoof,
    limit_tx: $limit,
    skim_tx: $skim,
    probe_token: $probe,
    note: "LocalTerra tax-on suite only. Do not AddWhitelistedCodeId columbus-5 11611/11619/8654 or this local store id from this evidence. B-lt stays the honest-CW20 gate."
  }' > "$OUT_JSON"

echo "tax-on: wrote $OUT_JSON"
echo "PASS: Layer B tax-on executed ($SOURCE) pair=$PAIR_ADDR"
