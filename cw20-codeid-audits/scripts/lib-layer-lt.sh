#!/usr/bin/env bash
# Shared LocalTerra helpers for Layer A-lcd / B-lt (GitLab #589 / #590).
# Never run against columbus-5. Terraport ticker is [a-zA-Z-]{3,12} — digits fail instantiate.
# shellcheck shell=bash

TEST_ADDRESS="${TEST_ADDRESS:-terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v}"
TERRAD_NODE="${TERRAD_NODE:-http://127.0.0.1:26657}"
TRANSFER_RAW="${LAYER_LT_TRANSFER_RAW:-1000000}"

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
  local sibling
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

layer_wait_tx() {
  local tx="$1"
  terrad_wait_tx_query "$CONTAINER" "$tx" "$TERRAD_NODE" >/dev/null
}
