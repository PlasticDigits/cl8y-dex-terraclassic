#!/usr/bin/env bash
# Shared LocalTerra helpers for Layer A-lcd / B-lt (GitLab #589 / #590).
# Never run against columbus-5. Terraport ticker is [a-zA-Z-]{3,12} — digits fail instantiate.
# shellcheck shell=bash

TEST_ADDRESS="${TEST_ADDRESS:-terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v}"
TERRAD_NODE="${TERRAD_NODE:-http://127.0.0.1:26657}"
TRANSFER_RAW="${LAYER_LT_TRANSFER_RAW:-1000000}"
LIQ_RAW="${LAYER_LT_LIQ_RAW:-100000000}"
SWAP_RAW="${LAYER_LT_SWAP_RAW:-1000000}"
LIMIT_RAW="${LAYER_LT_LIMIT_RAW:-1000000}"

# Terraport 8266 ticker is [a-zA-Z-]{3,12} — digits fail instantiate (A-lcd).
layer_terraport_symbol() {
  python3 -c 'import random, string; print("AUD" + "".join(random.choice(string.ascii_uppercase) for _ in range(3)))'
}

layer_find_env_local() {
  if [[ -n "${VERIFY_ENV_LOCAL:-}" && -f "${VERIFY_ENV_LOCAL}" ]]; then
    printf '%s' "$VERIFY_ENV_LOCAL"
    return 0
  fi
  if [[ -f "$REPO_ROOT/frontend-dapp/.env.local" ]]; then
    printf '%s' "$REPO_ROOT/frontend-dapp/.env.local"
    return 0
  fi
  local sibling primary
  # Git worktrees sit beside the primary clone (…/cl8y-dex-terraclassic-wt-*).
  primary="$(cd "$REPO_ROOT/.." 2>/dev/null && pwd)/cl8y-dex-terraclassic/frontend-dapp/.env.local"
  if [[ -f "$primary" ]]; then
    printf '%s' "$primary"
    return 0
  fi
  sibling="$(cd "$REPO_ROOT/../.." 2>/dev/null && pwd)/frontend-dapp/.env.local"
  if [[ -f "$sibling" ]]; then
    printf '%s' "$sibling"
    return 0
  fi
  return 1
}

layer_load_env_local() {
  local env_local="$1"
  set -a
  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ "$line" =~ ^[[:space:]]*# ]] && continue
    [[ "$line" =~ ^VITE_[A-Z0-9_]+= ]] || continue
    local key="${line%%=*}"
    local val="${line#*=}"
    export "$key=$val"
  done <"$env_local"
  set +a
}

layer_docker_cp() {
  local src="$1" dest="$2"
  if [[ -n "${LOCALTERRA_DOCKER_VIA_SG:-}" ]] && command -v sg >/dev/null 2>&1; then
    sg docker -c "docker cp $(printf '%q' "$src") $(printf '%q' "$dest")"
  else
    docker cp "$src" "$dest"
  fi
}

layer_txhash() {
  local raw="$1"
  echo "$raw" | sed -n '/^{/,$p' | tail -1 | jq -r '.txhash // .tx_response.txhash // empty' 2>/dev/null \
    || echo "$raw" | tr '\n' ' ' | grep -oE '\{.*\}' | tail -1 | jq -r '.txhash // .tx_response.txhash // empty' 2>/dev/null \
    || true
}

layer_require_localterra() {
  if ! timeout 20 make -s -C "$REPO_ROOT" has-localterra >/dev/null 2>&1; then
    echo "FAIL: LocalTerra is down. Provision: make setup-cloud-localterra" >&2
    return 1
  fi
  CONTAINER="$(localterra_container_id "$REPO_ROOT")"
  [[ -n "$CONTAINER" ]] || {
    echo "FAIL: localterra container id empty." >&2
    return 1
  }
  local status chain
  status="$(localterra_docker_exec "$CONTAINER" terrad status --node "$TERRAD_NODE" --output json 2>/dev/null || true)"
  chain="$(echo "$status" | jq -r '.node_info.network // .default_node_info.network // empty' 2>/dev/null || true)"
  if [[ "$chain" != "localterra" ]]; then
    echo "FAIL: chain_id=${chain:-unknown} — A-lcd/B-lt execute only on LocalTerra (never columbus-5)." >&2
    return 1
  fi
}

layer_cw20_balance() {
  local contract="$1" addr="$2"
  local raw payload
  raw="$(lcd_smart_query_raw "$LCD" "$contract" "$(jq -nc --arg a "$addr" '{balance:{address:$a}}')")"
  payload="$(lcd_decode_smart_data "$raw")"
  echo "$payload" | jq -r '.balance // "0"'
}

# Re-query until the CW20 balance differs from `expect_ne` (LCD can lag terrad).
layer_cw20_balance_changed() {
  local contract="$1" addr="$2" expect_ne="$3"
  local i bal
      for i in $(seq 1 40); do
    bal="$(layer_cw20_balance "$contract" "$addr")"
    if [[ "$bal" != "$expect_ne" ]]; then
      printf '%s' "$bal"
      return 0
    fi
    sleep 0.3
  done
  printf '%s' "$bal"
}

layer_wait_tx() {
  local tx="$1"
  terrad_wait_tx_query "$CONTAINER" "$tx" "$TERRAD_NODE" >/dev/null
}

# Broadcast as a named LocalTerra key (test1 / test2). Prints tx JSON on success.
layer_terrad_tx_from() {
  local from="$1"
  shift
  local max_attempts="${E2E_TERRAD_TX_MAX_ATTEMPTS:-12}"
  local attempt=1
  local out=""
  while ((attempt <= max_attempts)); do
    if out="$(localterra_docker_exec "$CONTAINER" terrad tx "$@" \
      --from "$from" \
      --keyring-backend test \
      --chain-id localterra \
      --gas auto \
      --gas-adjustment 1.3 \
      --fees 500000000uluna \
      --node "$TERRAD_NODE" \
      --broadcast-mode sync \
      -y --output json 2>&1)"; then
      printf '%s\n' "$out"
      return 0
    fi
    if [[ "$out" == *"account sequence mismatch"* ]]; then
      sleep "$attempt"
      ((attempt++))
      continue
    fi
    printf '%s\n' "$out"
    return 1
  done
  printf '%s\n' "$out"
  return 1
}

layer_addr_of() {
  local name="$1"
  localterra_docker_exec "$CONTAINER" terrad keys show "$name" -a --keyring-backend test 2>/dev/null || true
}

# Ensure a second LocalTerra key exists and has uluna (TransferFrom / SendFrom spender).
# Does not print or store seed material (C6).
layer_ensure_test2() {
  local addr
  addr="$(layer_addr_of test2)"
  if [[ "$addr" != terra1* ]]; then
    echo "A/B-lt: adding LocalTerra key test2 (ephemeral; not persisted)" >&2
    localterra_docker_exec "$CONTAINER" terrad keys add test2 --keyring-backend test --output json >/dev/null
    addr="$(layer_addr_of test2)"
  fi
  [[ "$addr" == terra1* ]] || {
    echo "FAIL: could not create test2 key" >&2
    return 1
  }
  local uluna
  uluna="$(localterra_docker_exec "$CONTAINER" terrad query bank balances "$addr" \
    --node "$TERRAD_NODE" --output json 2>/dev/null \
    | jq -r '.balances[]? | select(.denom=="uluna") | .amount' || true)"
  uluna="${uluna:-0}"
  if python3 -c "import sys; sys.exit(0 if int(sys.argv[1]) >= 10000000000 else 1)" "$uluna"; then
    printf '%s' "$addr"
    return 0
  fi
  echo "A/B-lt: funding test2 $addr with uluna from test1" >&2
  local out tx
  out="$(layer_terrad_tx_from test1 bank send test1 "$addr" "50000000000uluna")"
  tx="$(layer_txhash "$out")"
  [[ -n "$tx" ]] || {
    echo "FAIL: bank send to test2 produced no txhash:" >&2
    printf '%s\n' "$out" >&2
    return 1
  }
  layer_wait_tx "$tx"
  printf '%s' "$addr"
}

layer_b64_json() {
  jq -nc "$1" | base64 -w0 2>/dev/null || jq -nc "$1" | base64 | tr -d '\n'
}

layer_assert_one_to_one() {
  python3 -c '
import sys
label, before_s, after_s, before_r, after_r, amt = sys.argv[1:]
bs, as_, br, ar, a = (int(x) for x in (before_s, after_s, before_r, after_r, amt))
if bs - as_ != a:
    sys.stderr.write(f"FAIL: {label} sender debit {bs}->{as_} != {a}\n")
    sys.exit(1)
if ar - br != a:
    sys.stderr.write(f"FAIL: {label} recipient credit {br}->{ar} != {a} (FoT / tax)\n")
    sys.exit(1)
print(f"{label}: 1:1 holds")
' "$@"
}

layer_assert_debit() {
  python3 -c '
import sys
label, before, after, amt = sys.argv[1:]
b, a, n = int(before), int(after), int(amt)
if b - a != n:
    sys.stderr.write(f"FAIL: {label} debit {b}->{a} != {n}\n")
    sys.exit(1)
print(f"{label}: debit 1:1")
' "$@"
}

layer_cw20_allowance() {
  local contract="$1" owner="$2" spender="$3"
  local raw payload
  raw="$(lcd_smart_query_raw "$LCD" "$contract" \
    "$(jq -nc --arg o "$owner" --arg s "$spender" '{allowance:{owner:$o,spender:$s}}')")"
  payload="$(lcd_decode_smart_data "$raw")"
  echo "$payload" | jq -r '.allowance // "0"'
}

layer_cw20_token_info() {
  local contract="$1"
  local raw
  raw="$(lcd_smart_query_raw "$LCD" "$contract" '{"token_info":{}}')"
  lcd_decode_smart_data "$raw"
}

layer_cw20_balance_at() {
  local contract="$1" addr="$2" height="$3"
  local raw payload
  raw="$(lcd_smart_query_raw "$LCD" "$contract" \
    "$(jq -nc --arg a "$addr" --argjson h "$height" '{balance_at:{address:$a,height:$h}}')")"
  payload="$(lcd_decode_smart_data "$raw")"
  echo "$payload" | jq -r '.balance // "0"'
}

layer_block_height() {
  local status
  status="$(localterra_docker_exec "$CONTAINER" terrad status --node "$TERRAD_NODE" --output json 2>/dev/null || true)"
  echo "$status" | jq -r '.sync_info.latest_block_height // .SyncInfo.latest_block_height // empty'
}

layer_smart() {
  local contract="$1" msg="$2"
  local raw
  raw="$(lcd_smart_query_raw "$LCD" "$contract" "$msg")"
  lcd_decode_smart_data "$raw"
}

# Return 0 when the wasm execute was rejected (no txhash, or included with code != 0).
layer_execute_rejected() {
  local out="$1"
  local tx
  tx="$(layer_txhash "$out")"
  if [[ -z "$tx" ]]; then
    return 0
  fi
  if terrad_wait_tx_query "$CONTAINER" "$tx" "$TERRAD_NODE" >/dev/null 2>&1; then
    return 1
  fi
  return 0
}
