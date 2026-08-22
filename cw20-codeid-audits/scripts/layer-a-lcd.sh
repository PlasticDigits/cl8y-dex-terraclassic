#!/usr/bin/env bash
# Layer A-lcd: store + instantiate + 1:1 Transfer of pinned LCD wasm on LocalTerra.
# GitLab #589 / #590. Fail closed — never a silent skip.
#
# Usage: layer-a-lcd.sh <code_id>
# Requires: make has-localterra, codeids/<id>/token.wasm (fetch-lcd-wasm.sh first).
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

WASM="$AUDIT_ROOT/codeids/$ID/token.wasm"
PIN_FILE="$AUDIT_ROOT/codeids/$ID/wasm.sha256"
OUT_JSON="$AUDIT_ROOT/codeids/$ID/layer-a-lcd.json"
DEST_WASM="/tmp/cw20-audit-${ID}.wasm"

if [[ ! -f "$WASM" ]]; then
  echo "FAIL: $WASM missing — run fetch-lcd-wasm.sh $ID first" >&2
  exit 1
fi

expected_pin="$(tr -d '[:space:]' < "$PIN_FILE" | tr '[:lower:]' '[:upper:]')"
got_pin="$(sha256sum "$WASM" | awk '{print toupper($1)}')"
if [[ "$got_pin" != "$expected_pin" ]]; then
  echo "FAIL: C1 pin mismatch $got_pin != $expected_pin" >&2
  exit 1
fi

layer_require_localterra

ENV_LOCAL="$(layer_find_env_local || true)"
if [[ -z "$ENV_LOCAL" ]]; then
  echo "FAIL: frontend-dapp/.env.local missing (run make deploy-local)." >&2
  exit 1
fi
layer_load_env_local "$ENV_LOCAL"
LCD="${VITE_TERRA_LCD_URL:-http://localhost:1317}"
LCD="${LCD%/}"

terrad_tx() {
  e2e_terrad_tx "$CONTAINER" "$@"
}

echo "A-lcd: storing LCD wasm code_id=$ID pin=${got_pin:0:16}…"
layer_docker_cp "$WASM" "${CONTAINER}:${DEST_WASM}"
STORE_OUT="$(terrad_tx wasm store "$DEST_WASM")"
STORE_TX="$(layer_txhash "$STORE_OUT")"
[[ -n "$STORE_TX" ]] || {
  echo "FAIL: wasm store produced no txhash:" >&2
  printf '%s\n' "$STORE_OUT" >&2
  exit 1
}
STORE_JSON="$(terrad_wait_tx_query "$CONTAINER" "$STORE_TX" "$TERRAD_NODE")"
LOCAL_CODE_ID="$(echo "$STORE_JSON" | terrad_jq_code_id_from_tx_json | head -1 | tr -d '[:space:]')"
[[ -n "$LOCAL_CODE_ID" && "$LOCAL_CODE_ID" =~ ^[0-9]+$ ]] || {
  echo "FAIL: could not parse local store code_id from tx $STORE_TX" >&2
  exit 1
}
echo "A-lcd: local store code_id=$LOCAL_CODE_ID tx=$STORE_TX"

STAMP="$(date +%s)"
SYM="$(layer_terraport_symbol)"
INIT="$(jq -nc --arg n "Audit${ID}" --arg s "$SYM" --arg a "$TEST_ADDRESS" \
  '{name:$n,symbol:$s,decimals:6,initial_balances:[{address:$a,amount:"1000000000000"}],mint:{minter:$a}}')"
INST_OUT="$(terrad_tx wasm instantiate "$LOCAL_CODE_ID" "$INIT" \
  --label "audit-${ID}-a-lcd-${STAMP}" --admin "$TEST_ADDRESS" || true)"
INST_TX="$(layer_txhash "$INST_OUT")"
if [[ -z "$INST_TX" ]]; then
  echo "A-lcd: instantiate without marketing failed; retrying with marketing:{}" >&2
  INIT="$(jq -nc --arg n "Audit${ID}" --arg s "$SYM" --arg a "$TEST_ADDRESS" \
    '{name:$n,symbol:$s,decimals:6,initial_balances:[{address:$a,amount:"1000000000000"}],mint:{minter:$a},marketing:{}}')"
  INST_OUT="$(terrad_tx wasm instantiate "$LOCAL_CODE_ID" "$INIT" \
    --label "audit-${ID}-a-lcd-${STAMP}" --admin "$TEST_ADDRESS")"
  INST_TX="$(layer_txhash "$INST_OUT")"
fi
[[ -n "$INST_TX" ]] || {
  echo "FAIL: instantiate produced no txhash (LCD wasm may need Terra-only init):" >&2
  printf '%s\n' "$INST_OUT" >&2
  exit 1
}
layer_wait_tx "$INST_TX"
TOKEN_ADDR="$(echo "$(terrad_wait_tx_query "$CONTAINER" "$INST_TX" "$TERRAD_NODE")" \
  | terrad_jq_contract_address_from_tx_json | head -1)"
[[ "$TOKEN_ADDR" == terra1* ]] || {
  echo "FAIL: instantiate did not yield contract address (tx $INST_TX)" >&2
  exit 1
}
echo "A-lcd: instantiated $TOKEN_ADDR"

RECIPIENT="$(localterra_docker_exec "$CONTAINER" terrad keys show test2 -a --keyring-backend test 2>/dev/null || true)"
if [[ "$RECIPIENT" != terra1* ]]; then
  RECIPIENT="${VITE_FACTORY_ADDRESS:-}"
fi
[[ "$RECIPIENT" == terra1* ]] || {
  echo "FAIL: no transfer recipient (test2 / factory)." >&2
  exit 1
}

BAL_BEFORE_SENDER="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
BAL_BEFORE_RECV="$(layer_cw20_balance "$TOKEN_ADDR" "$RECIPIENT")"

XFER="$(jq -nc --arg r "$RECIPIENT" --arg amt "$TRANSFER_RAW" '{transfer:{recipient:$r,amount:$amt}}')"
XFER_OUT="$(terrad_tx wasm execute "$TOKEN_ADDR" "$XFER")"
XFER_TX="$(layer_txhash "$XFER_OUT")"
[[ -n "$XFER_TX" ]] || {
  echo "FAIL: transfer produced no txhash:" >&2
  printf '%s\n' "$XFER_OUT" >&2
  exit 1
}
layer_wait_tx "$XFER_TX"

BAL_AFTER_SENDER="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
BAL_AFTER_RECV="$(layer_cw20_balance "$TOKEN_ADDR" "$RECIPIENT")"

python3 - "$BAL_BEFORE_SENDER" "$BAL_AFTER_SENDER" "$BAL_BEFORE_RECV" "$BAL_AFTER_RECV" "$TRANSFER_RAW" <<'PY'
import sys
bs, as_, br, ar, amt = (int(x) for x in sys.argv[1:])
if bs - as_ != amt:
    sys.stderr.write(f"FAIL: sender debit {bs}->{as_} != {amt}\n")
    sys.exit(1)
if ar - br != amt:
    sys.stderr.write(f"FAIL: recipient credit {br}->{ar} != {amt} (FoT / tax)\n")
    sys.exit(1)
print("A-lcd: 1:1 Transfer holds")
PY

ALLOW="$(jq -nc --arg s "$RECIPIENT" --arg amt "$TRANSFER_RAW" \
  '{increase_allowance:{spender:$s,amount:$amt}}')"
ALLOW_OUT="$(terrad_tx wasm execute "$TOKEN_ADDR" "$ALLOW")"
ALLOW_TX="$(layer_txhash "$ALLOW_OUT")"
[[ -n "$ALLOW_TX" ]] || {
  echo "FAIL: increase_allowance produced no txhash" >&2
  exit 1
}
layer_wait_tx "$ALLOW_TX"

mkdir -p "$(dirname "$OUT_JSON")"
jq -nc \
  --arg id "$ID" \
  --arg pin "$got_pin" \
  --arg local "$LOCAL_CODE_ID" \
  --arg token "$TOKEN_ADDR" \
  --arg store "$STORE_TX" \
  --arg inst "$INST_TX" \
  --arg xfer "$XFER_TX" \
  --arg recv "$RECIPIENT" \
  --argjson one true \
  '{
    executed: true,
    one_to_one: $one,
    columbus5_code_id: $id,
    local_code_id: $local,
    token: $token,
    pin: $pin,
    store_tx: $store,
    instantiate_tx: $inst,
    transfer_tx: $xfer,
    recipient: $recv,
    note: "LocalTerra copy of LCD wasm. Do not AddWhitelistedCodeId this columbus-5 id on mainnet from this run."
  }' > "$OUT_JSON"

echo "A-lcd: wrote $OUT_JSON"
echo "PASS: Layer A-lcd executed pinned wasm $ID (local code $LOCAL_CODE_ID)"
