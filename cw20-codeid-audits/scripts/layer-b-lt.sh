#!/usr/bin/env bash
# Layer B-lt: whitelist the *locally stored* LCD wasm, CreatePair vs EMBER, 1:1 Send into pair.
# GitLab #589 / #590. Fail closed — LAYER_B_LT=1 must not PASS as a stub.
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

terrad_smart() {
  local contract="$1" msg="$2"
  local raw
  raw="$(lcd_smart_query_raw "$LCD" "$contract" "$msg")"
  lcd_decode_smart_data "$raw"
}

is_local_code_whitelisted() {
  local raw
  raw="$(terrad_smart "$VITE_FACTORY_ADDRESS" \
    "$(jq -nc --argjson c "$LOCAL_CODE_ID" '{is_code_id_whitelisted:{code_id:$c}}')")"
  echo "$raw" | jq -e '.whitelisted == true' >/dev/null 2>&1
}

echo "B-lt: whitelist local code_id=$LOCAL_CODE_ID on factory (not columbus-5 $ID)…"
WL_TX=""
if is_local_code_whitelisted; then
  echo "B-lt: local code_id already whitelisted"
else
  WL_MSG="$(jq -nc --argjson c "$LOCAL_CODE_ID" '{add_whitelisted_code_id:{code_id:$c}}')"
  WL_OUT="$(terrad_tx wasm execute "$VITE_FACTORY_ADDRESS" "$WL_MSG")"
  WL_TX="$(layer_txhash "$WL_OUT")"
  [[ -n "$WL_TX" ]] || {
    echo "FAIL: AddWhitelistedCodeId produced no txhash:" >&2
    printf '%s\n' "$WL_OUT" >&2
    exit 1
  }
  layer_wait_tx "$WL_TX"
  is_local_code_whitelisted || {
    echo "FAIL: local code_id $LOCAL_CODE_ID not whitelisted after AddWhitelistedCodeId" >&2
    exit 1
  }
fi

FACTORY_CFG="$(terrad_smart "$VITE_FACTORY_ADDRESS" '{"config":{}}')"
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

BAL_BEFORE="$(layer_cw20_balance "$TOKEN_ADDR" "$PAIR_ADDR")"
SEND_MSG="$(jq -nc --arg c "$PAIR_ADDR" --arg amt "$TRANSFER_RAW" \
  '{transfer:{recipient:$c,amount:$amt}}')"
SEND_OUT="$(terrad_tx wasm execute "$TOKEN_ADDR" "$SEND_MSG")"
SEND_TX="$(layer_txhash "$SEND_OUT")"
[[ -n "$SEND_TX" ]] || {
  echo "FAIL: transfer to pair produced no txhash" >&2
  exit 1
}
layer_wait_tx "$SEND_TX"
BAL_AFTER="$(layer_cw20_balance "$TOKEN_ADDR" "$PAIR_ADDR")"
python3 - "$BAL_BEFORE" "$BAL_AFTER" "$TRANSFER_RAW" <<'PY'
import sys
b, a, amt = (int(x) for x in sys.argv[1:])
if a - b != amt:
    sys.stderr.write(f"FAIL: pair credit {b}->{a} != {amt} (P2 FoT desync)\n")
    sys.exit(1)
print("B-lt: 1:1 Transfer into pair holds")
PY

jq -nc \
  --arg id "$ID" \
  --arg local "$LOCAL_CODE_ID" \
  --arg token "$TOKEN_ADDR" \
  --arg pair "$PAIR_ADDR" \
  --arg wl "$WL_TX" \
  --arg create "${CREATE_TX}" \
  --arg send "$SEND_TX" \
  '{
    executed: true,
    one_to_one_into_pair: true,
    columbus5_code_id: $id,
    local_code_id: $local,
    token: $token,
    pair: $pair,
    whitelist_tx: $wl,
    create_pair_tx: $create,
    transfer_to_pair_tx: $send,
    note: "Test factory only. Do not AddWhitelistedCodeId columbus-5 8266 from this evidence."
  }' > "$OUT_JSON"

echo "B-lt: wrote $OUT_JSON"
echo "PASS: Layer B-lt executed pinned wasm $ID on LocalTerra pair $PAIR_ADDR"
