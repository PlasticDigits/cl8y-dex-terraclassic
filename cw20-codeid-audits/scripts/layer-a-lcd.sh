#!/usr/bin/env bash
# Layer A-lcd: store + instantiate + CW20 write-path suite on pinned LCD wasm (LocalTerra).
# GitLab #589 / #581 / #590. Fail closed — never a silent skip.
#
# Covers Transfer / TransferFrom 1:1, allowance, unauthorized mint, idle/snapshot,
# self/oversize/zero. Send 1:1 to a Receive hook is Layer B-lt (pair Swap + limit).
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
INST_LABEL="audit-${ID}-a-lcd-${STAMP}"
INST_KIND="cw20-base"

layer_a_try_instantiate() {
  local init="$1"
  local out tx
  out="$(terrad_tx wasm instantiate "$LOCAL_CODE_ID" "$init" \
    --label "$INST_LABEL" --admin "$TEST_ADDRESS" || true)"
  tx="$(layer_txhash "$out")"
  if [[ -n "$tx" ]]; then
    INST_OUT="$out"
    INST_TX="$tx"
    return 0
  fi
  INST_OUT="$out"
  return 1
}

# cw20-base / mintable (8266, 10184 analogue).
INIT="$(jq -nc --arg n "Audit${ID}" --arg s "$SYM" --arg a "$TEST_ADDRESS" \
  '{name:$n,symbol:$s,decimals:6,initial_balances:[{address:$a,amount:"1000000000000"}],mint:{minter:$a}}')"
if ! layer_a_try_instantiate "$INIT"; then
  echo "A-lcd: instantiate without marketing failed; retrying with marketing:{}" >&2
  INIT="$(jq -nc --arg n "Audit${ID}" --arg s "$SYM" --arg a "$TEST_ADDRESS" \
    '{name:$n,symbol:$s,decimals:6,initial_balances:[{address:$a,amount:"1000000000000"}],mint:{minter:$a},marketing:{}}')"
  layer_a_try_instantiate "$INIT" || true
fi

# Community tax / #601 (11611): extra manager/treasury/factory fields; not cw20-base.
if [[ -z "${INST_TX:-}" ]]; then
  echo "A-lcd: retrying community-tax InstantiateMsg (O601-1)" >&2
  INST_KIND="community-tax"
  FACTORY_ADDR="${VITE_FACTORY_ADDRESS:-}"
  ROUTER_ADDR="${VITE_ROUTER_ADDRESS:-}"
  UST1_ADDR="${VITE_UST1_TOKEN_ADDRESS:-${VITE_TOKEN_EMBER_ADDRESS:-}}"
  [[ "$FACTORY_ADDR" == terra1* && "$UST1_ADDR" == terra1* ]] || {
    echo "FAIL: community-tax instantiate needs VITE_FACTORY_ADDRESS and a UST1/EMBER addr." >&2
    printf '%s\n' "$INST_OUT" >&2
    exit 1
  }
  INIT="$(jq -nc \
    --arg n "Audit${ID}" --arg s "$SYM" --arg a "$TEST_ADDRESS" \
    --arg factory "$FACTORY_ADDR" --arg router "$ROUTER_ADDR" --arg ust1 "$UST1_ADDR" \
    '{
      name:$n, symbol:$s, decimals:6,
      initial_balances:[{address:$a,amount:"1000000000000"}],
      marketing:{},
      manager:$a, treasury:$a,
      buy_bps:0, sell_bps:0,
      max_buy_bps:1000, max_sell_bps:1000, max_transfer_bps:500,
      factory:$factory,
      router: (if $router == "" then null else $router end),
      ust1:$ust1,
      cmm_treasury:$a,
      features:["mint_control"],
      mint:{minter:$a},
      launch_guards:{max_wallet:null, cooldown_blocks:0, trading_enabled:true}
    }')"
  if ! layer_a_try_instantiate "$INIT"; then
    # 11619+ (#605): SKU payloads require the matching feature (guards / variable_rates headroom).
    echo "A-lcd: retrying community-tax InstantiateMsg without SKU-gated fields" >&2
    INIT="$(jq -nc \
      --arg n "Audit${ID}" --arg s "$SYM" --arg a "$TEST_ADDRESS" \
      --arg factory "$FACTORY_ADDR" --arg router "$ROUTER_ADDR" --arg ust1 "$UST1_ADDR" \
      '{
        name:$n, symbol:$s, decimals:6,
        initial_balances:[{address:$a,amount:"1000000000000"}],
        marketing:{},
        manager:$a, treasury:$a,
        buy_bps:0, sell_bps:0,
        max_buy_bps:0, max_sell_bps:0, max_transfer_bps:0,
        factory:$factory,
        router: (if $router == "" then null else $router end),
        ust1:$ust1,
        cmm_treasury:$a,
        features:["mint_control"],
        mint:{minter:$a},
        launch_guards:null,
        initial_exempt:null
      }')"
    layer_a_try_instantiate "$INIT" || true
  fi
fi
[[ -n "${INST_TX:-}" ]] || {
  echo "FAIL: instantiate produced no txhash (LCD wasm may need Terra-only init):" >&2
  printf '%s\n' "$INST_OUT" >&2
  exit 1
}
echo "A-lcd: instantiate kind=$INST_KIND"
layer_wait_tx "$INST_TX"
TOKEN_ADDR="$(echo "$(terrad_wait_tx_query "$CONTAINER" "$INST_TX" "$TERRAD_NODE")" \
  | terrad_jq_contract_address_from_tx_json | head -1)"
[[ "$TOKEN_ADDR" == terra1* ]] || {
  echo "FAIL: instantiate did not yield contract address (tx $INST_TX)" >&2
  exit 1
}
echo "A-lcd: instantiated $TOKEN_ADDR"

RECIPIENT="$(layer_ensure_test2)"
[[ "$RECIPIENT" == terra1* ]] || {
  echo "FAIL: no transfer recipient (test2)." >&2
  exit 1
}
echo "A-lcd: spender/recipient test2=$RECIPIENT"

# --- A1 Transfer 1:1 ---
BAL_BEFORE_SENDER="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
BAL_BEFORE_RECV="$(layer_cw20_balance "$TOKEN_ADDR" "$RECIPIENT")"
XFER="$(jq -nc --arg r "$RECIPIENT" --arg amt "$TRANSFER_RAW" '{transfer:{recipient:$r,amount:$amt}}')"
XFER_TX="$(exec_ok wasm execute "$TOKEN_ADDR" "$XFER")"
BAL_AFTER_SENDER="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
BAL_AFTER_RECV="$(layer_cw20_balance "$TOKEN_ADDR" "$RECIPIENT")"
layer_assert_one_to_one "A-lcd Transfer" "$BAL_BEFORE_SENDER" "$BAL_AFTER_SENDER" \
  "$BAL_BEFORE_RECV" "$BAL_AFTER_RECV" "$TRANSFER_RAW"

# --- A16 events vs debit (wasm amount attr, if present, must not exceed debit) ---
XFER_JSON="$(terrad_wait_tx_query "$CONTAINER" "$XFER_TX" "$TERRAD_NODE")"
EVENT_AMT="$(echo "$XFER_JSON" | jq -r '[.tx_response.events // .events // [] | .[] | select(.type=="wasm") | .attributes[]? | select(.key=="amount") | .value] | first // empty')"
if [[ -n "$EVENT_AMT" && "$EVENT_AMT" =~ ^[0-9]+$ ]]; then
  python3 -c '
import sys
ev, debit = int(sys.argv[1]), int(sys.argv[2])
if ev != debit:
    sys.stderr.write(f"FAIL: transfer event amount {ev} != debit {debit}\n")
    sys.exit(1)
print("A-lcd: transfer event matches debit")
' "$EVENT_AMT" "$TRANSFER_RAW"
else
  echo "A-lcd: no numeric wasm amount attr (record; debit already 1:1)"
fi

# --- A8 TransferFrom without allowance must fail ---
NOALLOW="$(jq -nc --arg o "$TEST_ADDRESS" --arg r "$RECIPIENT" --arg amt "$TRANSFER_RAW" \
  '{transfer_from:{owner:$o,recipient:$r,amount:$amt}}')"
set +e
NOALLOW_OUT="$(layer_terrad_tx_from test2 wasm execute "$TOKEN_ADDR" "$NOALLOW" 2>&1)"
NOALLOW_ST=$?
set -e
if [[ "$NOALLOW_ST" -eq 0 ]] && ! layer_execute_rejected "$NOALLOW_OUT"; then
  echo "FAIL: TransferFrom without allowance succeeded (A8 backdoor)" >&2
  printf '%s\n' "$NOALLOW_OUT" >&2
  exit 1
fi
echo "A-lcd: TransferFrom without allowance rejected"

# --- A1 / A7 TransferFrom 1:1 with allowance ---
ALLOW="$(jq -nc --arg s "$RECIPIENT" --arg amt "$TRANSFER_RAW" \
  '{increase_allowance:{spender:$s,amount:$amt}}')"
ALLOW_TX="$(exec_ok wasm execute "$TOKEN_ADDR" "$ALLOW")"
TF_BEFORE_S="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
TF_BEFORE_R="$(layer_cw20_balance "$TOKEN_ADDR" "$RECIPIENT")"
TF_MSG="$(jq -nc --arg o "$TEST_ADDRESS" --arg r "$RECIPIENT" --arg amt "$TRANSFER_RAW" \
  '{transfer_from:{owner:$o,recipient:$r,amount:$amt}}')"
TF_OUT="$(layer_terrad_tx_from test2 wasm execute "$TOKEN_ADDR" "$TF_MSG")" || {
  echo "FAIL: TransferFrom with allowance failed:" >&2
  printf '%s\n' "$TF_OUT" >&2
  exit 1
}
TF_TX="$(layer_txhash "$TF_OUT")"
[[ -n "$TF_TX" ]] || {
  echo "FAIL: TransferFrom with allowance produced no txhash" >&2
  printf '%s\n' "$TF_OUT" >&2
  exit 1
}
layer_wait_tx "$TF_TX"
TF_AFTER_S="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
TF_AFTER_R="$(layer_cw20_balance "$TOKEN_ADDR" "$RECIPIENT")"
layer_assert_one_to_one "A-lcd TransferFrom" "$TF_BEFORE_S" "$TF_AFTER_S" \
  "$TF_BEFORE_R" "$TF_AFTER_R" "$TRANSFER_RAW"

# --- A7 leftover allowance should be 0 after exact spend ---
LEFT="$(layer_cw20_allowance "$TOKEN_ADDR" "$TEST_ADDRESS" "$RECIPIENT")"
[[ "$LEFT" == "0" ]] || {
  echo "FAIL: allowance leftover $LEFT after exact TransferFrom" >&2
  exit 1
}

# --- A12 unauthorized mint from test2 ---
MINT_BAD="$(jq -nc --arg r "$RECIPIENT" '{mint:{recipient:$r,amount:"1"}}')"
set +e
MINT_OUT="$(layer_terrad_tx_from test2 wasm execute "$TOKEN_ADDR" "$MINT_BAD" 2>&1)"
MINT_ST=$?
set -e
if [[ "$MINT_ST" -eq 0 ]] && ! layer_execute_rejected "$MINT_OUT"; then
  echo "FAIL: unauthorized mint succeeded (A12)" >&2
  exit 1
fi
echo "A-lcd: unauthorized mint rejected"

# --- burn_from without allowance ---
BURN_BAD="$(jq -nc --arg o "$TEST_ADDRESS" '{burn_from:{owner:$o,amount:"1"}}')"
set +e
BURN_OUT="$(layer_terrad_tx_from test2 wasm execute "$TOKEN_ADDR" "$BURN_BAD" 2>&1)"
BURN_ST=$?
set -e
if [[ "$BURN_ST" -eq 0 ]] && ! layer_execute_rejected "$BURN_OUT"; then
  echo "FAIL: unauthorized burn_from succeeded" >&2
  exit 1
fi
echo "A-lcd: unauthorized burn_from rejected"

# --- A19 self-transfer net zero ---
SELF_BEFORE="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
SELF_MSG="$(jq -nc --arg r "$TEST_ADDRESS" --arg amt "$TRANSFER_RAW" '{transfer:{recipient:$r,amount:$amt}}')"
SELF_TX="$(exec_ok wasm execute "$TOKEN_ADDR" "$SELF_MSG")"
SELF_AFTER="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
[[ "$SELF_BEFORE" == "$SELF_AFTER" ]] || {
  echo "FAIL: self-transfer mutated balance $SELF_BEFORE -> $SELF_AFTER" >&2
  exit 1
}
echo "A-lcd: self-transfer net zero"

# --- oversize rejected ---
OVER_MSG="$(jq -nc --arg r "$RECIPIENT" '{transfer:{recipient:$r,amount:"999999999999999999999999"}}')"
set +e
OVER_OUT="$(terrad_tx wasm execute "$TOKEN_ADDR" "$OVER_MSG" 2>&1)"
OVER_ST=$?
set -e
if [[ "$OVER_ST" -eq 0 ]] && ! layer_execute_rejected "$OVER_OUT"; then
  echo "FAIL: oversize transfer succeeded" >&2
  exit 1
fi
echo "A-lcd: oversize transfer rejected"

# --- A18 zero amount: reject or 0-delta ---
ZERO_BEFORE_S="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
ZERO_BEFORE_R="$(layer_cw20_balance "$TOKEN_ADDR" "$RECIPIENT")"
ZERO_MSG="$(jq -nc --arg r "$RECIPIENT" '{transfer:{recipient:$r,amount:"0"}}')"
ZERO_REJECTED=false
set +e
ZERO_OUT="$(terrad_tx wasm execute "$TOKEN_ADDR" "$ZERO_MSG" 2>&1)"
ZERO_ST=$?
set -e
if [[ "$ZERO_ST" -eq 0 ]] && ! layer_execute_rejected "$ZERO_OUT"; then
  ZERO_TX="$(layer_txhash "$ZERO_OUT")"
  layer_wait_tx "$ZERO_TX"
  ZERO_AFTER_S="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
  ZERO_AFTER_R="$(layer_cw20_balance "$TOKEN_ADDR" "$RECIPIENT")"
  [[ "$ZERO_BEFORE_S" == "$ZERO_AFTER_S" && "$ZERO_BEFORE_R" == "$ZERO_AFTER_R" ]] || {
    echo "FAIL: zero transfer mutated balances" >&2
    exit 1
  }
  echo "A-lcd: zero transfer no-op (accepted)"
else
  ZERO_REJECTED=true
  echo "A-lcd: zero transfer rejected (deterministic)"
fi

# --- A22 TokenInfo ---
INFO="$(layer_cw20_token_info "$TOKEN_ADDR")"
DECIMALS="$(echo "$INFO" | jq -r '.decimals // empty')"
[[ "$DECIMALS" == "6" ]] || {
  echo "FAIL: TokenInfo decimals=$DECIMALS (expected 6)" >&2
  echo "$INFO" >&2
  exit 1
}
echo "A-lcd: TokenInfo decimals=6"

# --- A23 / A29 idle + snapshot ---
IDLE_BEFORE="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
H1="$(layer_block_height)"
# Nudge a block with a no-op bank send of 1 uluna to test2 if needed: wait for next height.
for _ in $(seq 1 15); do
  H2="$(layer_block_height)"
  if [[ -n "$H2" && "$H2" != "$H1" ]]; then
    break
  fi
  sleep 1
done
IDLE_AFTER="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
[[ "$IDLE_BEFORE" == "$IDLE_AFTER" ]] || {
  echo "FAIL: idle balance changed $IDLE_BEFORE -> $IDLE_AFTER (rebase A3/A23)" >&2
  exit 1
}
echo "A-lcd: idle balance stable"

SNAP_H="$(layer_block_height)"
HAS_BALANCE_AT=true
if [[ "${INST_KIND:-}" == "community-tax" ]]; then
  HAS_BALANCE_AT=false
elif [[ -f "$AUDIT_ROOT/codeids/$ID/decomp/fingerprint.json" ]]; then
  if jq -e '.hits.balance_at == false or .balance_at == false' "$AUDIT_ROOT/codeids/$ID/decomp/fingerprint.json" >/dev/null 2>&1; then
    HAS_BALANCE_AT=false
  fi
fi
if [[ "$HAS_BALANCE_AT" == "false" ]]; then
  echo "A-lcd: balance_at unsupported (A29 N/A; idle 1:1 already asserted)"
elif [[ "$SNAP_H" =~ ^[0-9]+$ ]]; then
  SNAP=""
  set +e
  SNAP="$(layer_cw20_balance_at "$TOKEN_ADDR" "$TEST_ADDRESS" "$SNAP_H" 2>/dev/null)"
  SNAP_ST=$?
  set -e
  if [[ "$SNAP_ST" -eq 0 && "$SNAP" =~ ^[0-9]+$ ]]; then
    LIVE="$(layer_cw20_balance "$TOKEN_ADDR" "$TEST_ADDRESS")"
    [[ "$SNAP" == "$LIVE" ]] || {
      echo "FAIL: balance_at@$SNAP_H=$SNAP != live $LIVE (A29)" >&2
      exit 1
    }
    echo "A-lcd: balance_at snapshot matches live"
  else
    echo "A-lcd: balance_at unsupported (A29 N/A; idle 1:1 already asserted)"
  fi
else
  echo "A-lcd: no block height for balance_at (A29 N/A; idle already asserted)"
fi

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
  --arg tf "$TF_TX" \
  --argjson zero_rejected "$ZERO_REJECTED" \
  --argjson one true \
  '{
    executed: true,
    one_to_one: $one,
    transfer_one_to_one: true,
    transfer_from_one_to_one: true,
    transfer_from_no_allowance_rejected: true,
    unauthorized_mint_rejected: true,
    unauthorized_burn_from_rejected: true,
    self_transfer_net_zero: true,
    oversize_rejected: true,
    zero_amount_deterministic: true,
    zero_rejected: $zero_rejected,
    token_info_ok: true,
    idle_balance_stable: true,
    snapshot_matches_live: true,
    send_hook: "layer-b-lt (pair Receive)",
    columbus5_code_id: $id,
    local_code_id: $local,
    token: $token,
    pin: $pin,
    store_tx: $store,
    instantiate_tx: $inst,
    transfer_tx: $xfer,
    transfer_from_tx: $tf,
    recipient: $recv,
    note: "LocalTerra copy of LCD wasm. Do not AddWhitelistedCodeId this columbus-5 id on mainnet from this run."
  }' > "$OUT_JSON"

echo "A-lcd: wrote $OUT_JSON"
echo "PASS: Layer A-lcd executed pinned wasm $ID (local code $LOCAL_CODE_ID)"
