#!/usr/bin/env bash
# Layer B-lt: whitelist locally stored LCD wasm, CreatePair vs EMBER, provide, swap, limit escrow.
# GitLab #589 / #581 / #590. Fail closed — LAYER_B_LT=1 must not PASS as a stub.
#
# Whitelists only the LocalTerra store code_id from layer-a-lcd.json — never columbus-5 8266.
# Usage: layer-b-lt.sh <code_id>
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AUDIT_ROOT="$(cd "${CW20_AUDIT_ROOT:-$SCRIPT_DIR/..}" && pwd)"
REPO_ROOT="$(cd "$AUDIT_ROOT/.." && pwd)"
cd "$REPO_ROOT"

ID="${1:-}"
if [[ -z "$ID" || ! "$ID" =~ ^[0-9]+$ ]]; then
  echo "Usage: $0 <code_id>" >&2
  exit 1
fi

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

A_JSON="$AUDIT_ROOT/codeids/$ID/layer-a-lcd.json"
OUT_JSON="$AUDIT_ROOT/codeids/$ID/layer-b-lt.json"
[[ -f "$A_JSON" ]] || {
  echo "FAIL: $A_JSON missing — run layer-a-lcd.sh $ID first" >&2
  exit 1
}

layer_require_localterra

ENV_LOCAL="$(layer_find_env_local || true)"
[[ -n "$ENV_LOCAL" ]] || {
  echo "FAIL: frontend-dapp/.env.local missing." >&2
  exit 1
}
layer_load_env_local "$ENV_LOCAL"
LCD="${VITE_TERRA_LCD_URL:-http://localhost:1317}"
LCD="${LCD%/}"
[[ -n "${VITE_FACTORY_ADDRESS:-}" ]] || {
  echo "FAIL: VITE_FACTORY_ADDRESS unset." >&2
  exit 1
}
[[ -n "${VITE_TOKEN_EMBER_ADDRESS:-}" ]] || {
  echo "FAIL: VITE_TOKEN_EMBER_ADDRESS unset." >&2
  exit 1
}

LOCAL_CODE_ID="$(jq -r '.local_code_id' "$A_JSON")"
TOKEN_ADDR="$(jq -r '.token' "$A_JSON")"
[[ "$LOCAL_CODE_ID" =~ ^[0-9]+$ ]] || {
  echo "FAIL: layer-a-lcd.json local_code_id missing." >&2
  exit 1
}
[[ "$TOKEN_ADDR" == terra1* ]] || {
  echo "FAIL: layer-a-lcd.json token missing." >&2
  exit 1
}

terrad_tx() {
  e2e_terrad_tx "$CONTAINER" "$@"
}

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

send_cw20_hook() {
  local token="$1" pair="$2" amount="$3" hook_json="$4"
  local b64 msg
  b64="$(printf '%s' "$hook_json" | base64 -w0 2>/dev/null || printf '%s' "$hook_json" | base64 | tr -d '\n')"
  msg="$(jq -nc --arg c "$pair" --arg amt "$amount" --arg m "$b64" \
    '{send:{contract:$c,amount:$amt,msg:$m}}')"
  exec_ok wasm execute "$token" "$msg"
}

is_local_code_whitelisted() {
  local raw
  # Prefer O(1) IsCodeIdWhitelisted (factory 1.9+). LocalTerra may still be older:
  # unknown variant → fall back to GetWhitelistedCodeIds.
  raw="$(layer_smart "$VITE_FACTORY_ADDRESS" \
    "$(jq -nc --argjson c "$LOCAL_CODE_ID" '{is_code_id_whitelisted:{code_id:$c}}')" 2>/dev/null || true)"
  if echo "$raw" | jq -e '.whitelisted == true' >/dev/null 2>&1; then
    return 0
  fi
  raw="$(layer_smart "$VITE_FACTORY_ADDRESS" '{"get_whitelisted_code_ids":{}}')"
  echo "$raw" | jq -e --argjson c "$LOCAL_CODE_ID" '.code_ids | index($c) != null' >/dev/null 2>&1
}

echo "B-lt: whitelist local code_id=$LOCAL_CODE_ID on factory (not columbus-5 $ID)…"
WL_TX=""
if is_local_code_whitelisted; then
  echo "B-lt: local code_id already whitelisted"
else
  WL_MSG="$(jq -nc --argjson c "$LOCAL_CODE_ID" '{add_whitelisted_code_id:{code_id:$c}}')"
  WL_TX="$(exec_ok wasm execute "$VITE_FACTORY_ADDRESS" "$WL_MSG")"
  is_local_code_whitelisted || {
    echo "FAIL: local code_id $LOCAL_CODE_ID not whitelisted after AddWhitelistedCodeId" >&2
    exit 1
  }
fi

FACTORY_CFG="$(layer_smart "$VITE_FACTORY_ADDRESS" '{"config":{}}')"
PAIR_FEE="$(echo "$FACTORY_CFG" | jq -r '.pair_creation_fee_uluna // "0"')"

factory_pair_addr() {
  local a="$1" b="$2"
  local q raw
  q="$(jq -nc --arg a "$a" --arg b "$b" \
    '{pair:{asset_infos:[{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')"
  raw="$(lcd_smart_query_raw "$LCD" "$VITE_FACTORY_ADDRESS" "$q" 2>/dev/null || true)"
  lcd_decode_smart_data "$raw" 2>/dev/null | jq -r '.contract_addr // .pair.contract_addr // empty' || true
}

PAIR_ADDR="$(factory_pair_addr "$TOKEN_ADDR" "$VITE_TOKEN_EMBER_ADDRESS")"
if [[ "$PAIR_ADDR" != terra1* ]]; then
  PAIR_ADDR="$(factory_pair_addr "$VITE_TOKEN_EMBER_ADDRESS" "$TOKEN_ADDR")"
fi

CREATE_TX=""
if [[ "$PAIR_ADDR" != terra1* ]]; then
  echo "B-lt: CreatePair candidate vs EMBER…"
  CREATE_MSG="$(jq -nc --arg a "$TOKEN_ADDR" --arg b "$VITE_TOKEN_EMBER_ADDRESS" \
    '{create_pair:{asset_infos:[{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')"
  fee_args=()
  if [[ "$PAIR_FEE" != "0" && -n "$PAIR_FEE" ]]; then
    fee_args=(--amount "${PAIR_FEE}uluna")
  fi
  CREATE_OUT="$(terrad_tx wasm execute "$VITE_FACTORY_ADDRESS" "$CREATE_MSG" "${fee_args[@]}")"
  CREATE_TX="$(layer_txhash "$CREATE_OUT")"
  [[ -n "$CREATE_TX" ]] || {
    echo "FAIL: create_pair produced no txhash:" >&2
    printf '%s\n' "$CREATE_OUT" >&2
    exit 1
  }
  layer_wait_tx "$CREATE_TX"
  PAIR_ADDR="$(echo "$(terrad_wait_tx_query "$CONTAINER" "$CREATE_TX" "$TERRAD_NODE")" \
    | terrad_jq_contract_address_from_tx_json | head -1)"
  if [[ "$PAIR_ADDR" != terra1* ]]; then
    PAIR_ADDR="$(factory_pair_addr "$TOKEN_ADDR" "$VITE_TOKEN_EMBER_ADDRESS")"
  fi
  if [[ "$PAIR_ADDR" != terra1* ]]; then
    PAIR_ADDR="$(factory_pair_addr "$VITE_TOKEN_EMBER_ADDRESS" "$TOKEN_ADDR")"
  fi
fi
[[ "$PAIR_ADDR" == terra1* ]] || {
  echo "FAIL: could not resolve pair for candidate vs EMBER." >&2
  exit 1
}
echo "B-lt: pair=$PAIR_ADDR"

EMBER_BAL="$(layer_cw20_balance "$VITE_TOKEN_EMBER_ADDRESS" "$TEST_ADDRESS")"
python3 -c '
import sys
need, have = int(sys.argv[1]), int(sys.argv[2])
if have < need:
    sys.stderr.write(f"FAIL: EMBER balance {have} < provide {need}\n")
    sys.exit(1)
' "$LIQ_RAW" "$EMBER_BAL"

# --- provide liquidity (TransferFrom 1:1 into pair) ---
CAND_PAIR_BEFORE="$(layer_cw20_balance "$TOKEN_ADDR" "$PAIR_ADDR")"
EMBER_PAIR_BEFORE="$(layer_cw20_balance "$VITE_TOKEN_EMBER_ADDRESS" "$PAIR_ADDR")"
POOL_BEFORE="$(layer_smart "$PAIR_ADDR" '{"pool":{}}')"
ALLOW_C="$(jq -nc --arg s "$PAIR_ADDR" --arg amt "$LIQ_RAW" \
  '{increase_allowance:{spender:$s,amount:$amt}}')"
exec_ok wasm execute "$TOKEN_ADDR" "$ALLOW_C" >/dev/null
exec_ok wasm execute "$VITE_TOKEN_EMBER_ADDRESS" "$ALLOW_C" >/dev/null
PROVIDE="$(jq -nc --arg a "$TOKEN_ADDR" --arg b "$VITE_TOKEN_EMBER_ADDRESS" --arg amt "$LIQ_RAW" \
  '{provide_liquidity:{assets:[{info:{token:{contract_addr:$a}},amount:$amt},{info:{token:{contract_addr:$b}},amount:$amt}],slippage_tolerance:null,receiver:null,deadline:null}}')"
PROVIDE_TX="$(exec_ok wasm execute "$PAIR_ADDR" "$PROVIDE")"
CAND_PAIR_AFTER="$(layer_cw20_balance "$TOKEN_ADDR" "$PAIR_ADDR")"
EMBER_PAIR_AFTER="$(layer_cw20_balance "$VITE_TOKEN_EMBER_ADDRESS" "$PAIR_ADDR")"
python3 -c '
import sys
cb, ca, eb, ea, amt = (int(x) for x in sys.argv[1:])
if ca - cb != amt:
    sys.stderr.write(f"FAIL: candidate pair credit {cb}->{ca} != {amt} (P2 FoT)\n")
    sys.exit(1)
if ea - eb != amt:
    sys.stderr.write(f"FAIL: EMBER pair credit {eb}->{ea} != {amt} (P2 FoT)\n")
    sys.exit(1)
print("B-lt: provide TransferFrom 1:1 into pair")
' "$CAND_PAIR_BEFORE" "$CAND_PAIR_AFTER" "$EMBER_PAIR_BEFORE" "$EMBER_PAIR_AFTER" "$LIQ_RAW"

# --- P2: pool reserve *delta* equals declared provide (escrow leftovers may sit in CW20) ---
POOL_AFTER="$(layer_smart "$PAIR_ADDR" '{"pool":{}}')"
python3 - "$POOL_BEFORE" "$POOL_AFTER" "$TOKEN_ADDR" "$VITE_TOKEN_EMBER_ADDRESS" "$LIQ_RAW" <<'PY'
import json, sys
before, after = json.loads(sys.argv[1]), json.loads(sys.argv[2])
tok, emb, amt = sys.argv[3], sys.argv[4], int(sys.argv[5])

def addr(asset):
    info = asset.get("info") or {}
    token = info.get("token") or {}
    return token.get("contract_addr") or ""

def amt_of(asset):
    return int(asset.get("amount") or 0)

def reserves(pool):
    return {addr(a): amt_of(a) for a in (pool.get("assets") or [])}

b, a = reserves(before), reserves(after)
if a.get(tok, 0) - b.get(tok, 0) != amt:
    sys.stderr.write(f"FAIL: P2 candidate reserve delta {b.get(tok)}->{a.get(tok)} != {amt}\n")
    sys.exit(1)
if a.get(emb, 0) - b.get(emb, 0) != amt:
    sys.stderr.write(f"FAIL: P2 EMBER reserve delta {b.get(emb)}->{a.get(emb)} != {amt}\n")
    sys.exit(1)
print("B-lt: P2 reserve delta matches provide")
PY

# --- B7 / A1 Send: round-trip swap ---
SWAP_HOOK='{"swap":{"max_spread":"1"}}'
CAND_USER_BEFORE="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
EMBER_USER_BEFORE="$(layer_cw20_balance "$VITE_TOKEN_EMBER_ADDRESS" "$TEST_ADDRESS")"
SWAP_AB_TX="$(send_cw20_hook "$TOKEN_ADDR" "$PAIR_ADDR" "$SWAP_RAW" "$SWAP_HOOK")"
CAND_USER_MID="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
EMBER_USER_MID="$(layer_cw20_balance "$VITE_TOKEN_EMBER_ADDRESS" "$TEST_ADDRESS")"
layer_assert_debit "B-lt Send swap candidate" "$CAND_USER_BEFORE" "$CAND_USER_MID" "$SWAP_RAW"
python3 -c '
import sys
b, a = int(sys.argv[1]), int(sys.argv[2])
if a <= b:
    sys.stderr.write(f"FAIL: EMBER did not increase on swap ({b}->{a}); honeypot B7\n")
    sys.exit(1)
print("B-lt: swap candidate→EMBER credited output")
' "$EMBER_USER_BEFORE" "$EMBER_USER_MID"

EMBER_OUT=$((EMBER_USER_MID - EMBER_USER_BEFORE))
# Sell a slice of the received EMBER back (leave 1 if possible).
BACK="$EMBER_OUT"
if (( BACK > 1 )); then
  BACK=$((BACK / 2))
  if (( BACK < 1 )); then BACK=1; fi
fi
[[ "$BACK" -ge 1 ]] || {
  echo "FAIL: no EMBER output to reverse-swap" >&2
  exit 1
}
CAND_BEFORE_BACK="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
EMBER_BEFORE_BACK="$(layer_cw20_balance "$VITE_TOKEN_EMBER_ADDRESS" "$TEST_ADDRESS")"
SWAP_BA_TX="$(send_cw20_hook "$VITE_TOKEN_EMBER_ADDRESS" "$PAIR_ADDR" "$BACK" "$SWAP_HOOK")"
CAND_AFTER_BACK="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
EMBER_AFTER_BACK="$(layer_cw20_balance "$VITE_TOKEN_EMBER_ADDRESS" "$TEST_ADDRESS")"
layer_assert_debit "B-lt Send swap EMBER" "$EMBER_BEFORE_BACK" "$EMBER_AFTER_BACK" "$BACK"
python3 -c '
import sys
b, a = int(sys.argv[1]), int(sys.argv[2])
if a <= b:
    sys.stderr.write(f"FAIL: candidate did not increase on reverse swap ({b}->{a}); sell-side honeypot B7\n")
    sys.exit(1)
print("B-lt: round-trip swap succeeded (B7)")
' "$CAND_BEFORE_BACK" "$CAND_AFTER_BACK"

# --- B6 / A1 Send: limit escrow 1:1 ---
PAIR_INFO="$(layer_smart "$PAIR_ADDR" '{"pair":{}}')"
ASSET0="$(echo "$PAIR_INFO" | jq -r '.asset_infos[0].token.contract_addr // empty')"
if [[ "$ASSET0" == "$TOKEN_ADDR" ]]; then
  LIMIT_SIDE="ask"
  LIMIT_PRICE="10"
else
  LIMIT_SIDE="bid"
  LIMIT_PRICE="0.1"
fi
LIMIT_HOOK="$(jq -nc --arg side "$LIMIT_SIDE" --arg amt "$LIMIT_RAW" --arg price "$LIMIT_PRICE" \
  '{place_limit_order_batch:{side:$side,orders:[{price:$price,amount:$amt,max_adjust_steps:32}]}}')"
FEE_CFG="$(layer_smart "$PAIR_ADDR" '{"get_fee_config":{}}')"
TREASURY="$(echo "$FEE_CFG" | jq -r '.fee_config.treasury // .treasury // empty')"
if [[ "$TREASURY" != terra1* ]]; then
  TREASURY="${VITE_TREASURY_ADDRESS:-}"
fi
USER_LIM_BEFORE="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
PAIR_LIM_BEFORE="$(layer_cw20_balance "$TOKEN_ADDR" "$PAIR_ADDR")"
TREAS_LIM_BEFORE="0"
if [[ "$TREASURY" == terra1* ]]; then
  TREAS_LIM_BEFORE="$(layer_cw20_balance "$TOKEN_ADDR" "$TREASURY")"
fi
LIMIT_TX="$(send_cw20_hook "$TOKEN_ADDR" "$PAIR_ADDR" "$LIMIT_RAW" "$LIMIT_HOOK")"
echo "B-lt: limit tx=$LIMIT_TX side=$LIMIT_SIDE price=$LIMIT_PRICE treasury=${TREASURY:-none}" >&2
# Inclusion is not enough — LCD can lag. Wait until pair CW20 moves or timeout.
PAIR_LIM_AFTER="$(layer_cw20_balance_changed "$TOKEN_ADDR" "$PAIR_ADDR" "$PAIR_LIM_BEFORE")"
USER_LIM_AFTER="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
TREAS_LIM_AFTER="0"
if [[ "$TREASURY" == terra1* && "$TREASURY" != "$TEST_ADDRESS" ]]; then
  TREAS_LIM_AFTER="$(layer_cw20_balance "$TOKEN_ADDR" "$TREASURY")"
else
  TREAS_LIM_BEFORE="0"
  TREASURY=""
fi
python3 -c '
import sys
ub, ua, pb, pa, tb, ta, amt = (int(x) for x in sys.argv[1:])
debit = ub - ua
credited = (pa - pb) + (ta - tb)
if debit <= 0:
    sys.stderr.write(f"FAIL: limit Send user debit {ub}->{ua} not positive\n")
    sys.exit(1)
# Honest CW20: declared Send amount is conserved between user debit and pair+treasury
# (maker fee may leave the pair for fee_config.treasury; batch refund may lower net debit).
if credited != debit:
    sys.stderr.write(
        f"FAIL: limit escrow pair+treasury credit {credited} != user debit {debit} (L1 FoT)\n"
    )
    sys.stderr.write(
        f"  user {ub}->{ua} pair {pb}->{pa} treasury {tb}->{ta} declared {amt}\n"
    )
    sys.exit(1)
if debit > amt:
    sys.stderr.write(f"FAIL: limit Send debit {debit} > declared {amt}\n")
    sys.exit(1)
print(f"B-lt: limit Send escrow 1:1 (B6/L1) debit={debit} pair_d={pa-pb} treas_d={ta-tb}")
' "$USER_LIM_BEFORE" "$USER_LIM_AFTER" "$PAIR_LIM_BEFORE" "$PAIR_LIM_AFTER" \
  "$TREAS_LIM_BEFORE" "$TREAS_LIM_AFTER" "$LIMIT_RAW"

# --- A1 SendFrom: spender test2 sends into pair Swap ---
TEST2="$(layer_ensure_test2)"
SEND_FROM_TX=""
if [[ "$TEST2" == terra1* ]]; then
  SF_ALLOW="$(jq -nc --arg s "$TEST2" --arg amt "$TRANSFER_RAW" \
    '{increase_allowance:{spender:$s,amount:$amt}}')"
  exec_ok wasm execute "$TOKEN_ADDR" "$SF_ALLOW" >/dev/null
  SF_B64="$(printf '%s' "$SWAP_HOOK" | base64 -w0 2>/dev/null || printf '%s' "$SWAP_HOOK" | base64 | tr -d '\n')"
  SF_MSG="$(jq -nc --arg o "$TEST_ADDRESS" --arg c "$PAIR_ADDR" --arg amt "$TRANSFER_RAW" --arg m "$SF_B64" \
    '{send_from:{owner:$o,contract:$c,amount:$amt,msg:$m}}')"
  SF_BEFORE="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
  SF_OUT="$(layer_terrad_tx_from test2 wasm execute "$TOKEN_ADDR" "$SF_MSG")"
  SEND_FROM_TX="$(layer_txhash "$SF_OUT")"
  [[ -n "$SEND_FROM_TX" ]] || {
    echo "FAIL: SendFrom swap produced no txhash" >&2
    printf '%s\n' "$SF_OUT" >&2
    exit 1
  }
  layer_wait_tx "$SEND_FROM_TX"
  SF_AFTER="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
  layer_assert_debit "B-lt SendFrom swap" "$SF_BEFORE" "$SF_AFTER" "$TRANSFER_RAW"
else
  echo "FAIL: test2 address missing for SendFrom" >&2
  exit 1
fi

jq -nc \
  --arg id "$ID" \
  --arg local "$LOCAL_CODE_ID" \
  --arg token "$TOKEN_ADDR" \
  --arg pair "$PAIR_ADDR" \
  --arg wl "$WL_TX" \
  --arg create "${CREATE_TX}" \
  --arg provide "$PROVIDE_TX" \
  --arg swap_ab "$SWAP_AB_TX" \
  --arg swap_ba "$SWAP_BA_TX" \
  --arg limit "$LIMIT_TX" \
  --arg send_from "$SEND_FROM_TX" \
  '{
    executed: true,
    one_to_one_into_pair: true,
    provide_liquidity: true,
    p2_reserves_match: true,
    send_one_to_one: true,
    round_trip_swap: true,
    limit_escrow_one_to_one: true,
    send_from_one_to_one: true,
    columbus5_code_id: $id,
    local_code_id: $local,
    token: $token,
    pair: $pair,
    whitelist_tx: $wl,
    create_pair_tx: $create,
    provide_tx: $provide,
    swap_candidate_to_ember_tx: $swap_ab,
    swap_ember_to_candidate_tx: $swap_ba,
    limit_send_tx: $limit,
    send_from_tx: $send_from,
    note: "Test factory only. Do not AddWhitelistedCodeId columbus-5 8266 from this evidence."
  }' > "$OUT_JSON"

echo "B-lt: wrote $OUT_JSON"
echo "PASS: Layer B-lt executed pinned wasm $ID on LocalTerra pair $PAIR_ADDR"
