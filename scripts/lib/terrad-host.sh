#!/usr/bin/env bash
# Host-side terrad helpers for mainnet/testnet (no Docker LocalTerra).
# Pair with scripts/lib/terrad-tx-events.sh for code_id / contract address parsing.
# shellcheck shell=bash

# Defaults match columbus-5 operator docs; override via env.
TERRAD_HOST_CHAIN_ID="${TERRAD_HOST_CHAIN_ID:-columbus-5}"
TERRAD_HOST_NODE="${TERRAD_HOST_NODE:-https://terra-classic-rpc.publicnode.com:443}"
TERRAD_HOST_KEY="${TERRAD_HOST_KEY:-cl8ydeploy}"
TERRAD_HOST_HOME="${TERRAD_HOST_HOME:-$HOME/.terra}"
# Empty → auto-detect (prefer keyring-file if $KEY.info exists; else os).
TERRAD_HOST_KEYRING_BACKEND="${TERRAD_HOST_KEYRING_BACKEND:-}"
TERRAD_HOST_GAS_ADJUSTMENT="${TERRAD_HOST_GAS_ADJUSTMENT:-1.4}"
# Classic tax-module floor (FCD /v1/txs/gas_prices). Prefer gas-prices so fee = gas_wanted × price.
# Fixed --fees is wrong for mixed store/execute workloads (5 LUNC too low for store; flat 100 LUNC overpays executes).
TERRAD_HOST_GAS_PRICES="${TERRAD_HOST_GAS_PRICES:-28.325uluna}"
# Optional escape hatch: if set, use --fees instead of --gas-prices.
TERRAD_HOST_FEES="${TERRAD_HOST_FEES:-}"
TERRAD_HOST_BROADCAST_MODE="${TERRAD_HOST_BROADCAST_MODE:-sync}"
# Optional passphrase for encrypted keyring-file (or OS keyring prompts). Prefer a TTY prompt
# when possible; for long soft-launch runs set TERRAD_HOST_KEYRING_PASS so --gas auto does not
# block on dozens of interactive unlocks. Unset after deploy. Never commit this value.
TERRAD_HOST_KEYRING_PASS="${TERRAD_HOST_KEYRING_PASS:-}"

# Resolve keyring backend once. Wrong backend → terrad treats the key *name* as an address and
# fails with: "decoding bech32 failed: invalid separator index -1".
terrad_host_resolve_keyring_backend() {
  if [[ -n "${TERRAD_HOST_KEYRING_BACKEND:-}" ]]; then
    return 0
  fi
  local home="${TERRAD_HOST_HOME:-$HOME/.terra}"
  local key="${TERRAD_HOST_KEY:-cl8ydeploy}"
  if [[ -f "$home/keyring-file/${key}.info" ]]; then
    TERRAD_HOST_KEYRING_BACKEND=file
  elif [[ -f "$home/keyring-test/${key}.info" ]]; then
    TERRAD_HOST_KEYRING_BACKEND=test
  else
    TERRAD_HOST_KEYRING_BACKEND=os
  fi
}

terrad_host_common_flags() {
  terrad_host_resolve_keyring_backend
  echo --chain-id "$TERRAD_HOST_CHAIN_ID" \
    --node "$TERRAD_HOST_NODE" \
    --keyring-backend "$TERRAD_HOST_KEYRING_BACKEND" \
    --home "$TERRAD_HOST_HOME"
}

# Query-only flags: classic `terrad query` rejects --keyring-backend.
terrad_host_query_flags() {
  echo --chain-id "$TERRAD_HOST_CHAIN_ID" \
    --node "$TERRAD_HOST_NODE" \
    --home "$TERRAD_HOST_HOME"
}

# Fee flags: gas-prices (default) or explicit TERRAD_HOST_FEES.
terrad_host_fee_flags() {
  if [[ -n "${TERRAD_HOST_FEES:-}" ]]; then
    echo --fees "$TERRAD_HOST_FEES"
  else
    echo --gas-prices "$TERRAD_HOST_GAS_PRICES"
  fi
}

# Run terrad, feeding keyring passphrase on stdin when TERRAD_HOST_KEYRING_PASS is set.
# File backend + --gas auto may unlock more than once per command.
#
# Important: soft-launch runs under `set -o pipefail`. An infinite password feeder gets
# SIGPIPE (exit 141) when terrad closes stdin — that must NOT override terrad's real exit
# status (otherwise unlock looks like failure with empty stderr after a correct passphrase).
terrad_host_exec() {
  if [[ -z "${TERRAD_HOST_KEYRING_PASS:-}" ]]; then
    terrad "$@"
    return $?
  fi
  local st pipefail_was=0
  # Disable pipefail for this pipeline only (caller's shell options restored after).
  [[ -o pipefail ]] && pipefail_was=1
  set +o pipefail
  {
    local _i
    # Enough lines for --gas auto (simulate + sign) without an infinite writer.
    for _i in $(seq 1 64); do
      printf '%s\n' "$TERRAD_HOST_KEYRING_PASS" || break
    done
  } | terrad "$@"
  st=${PIPESTATUS[1]}
  [[ "$pipefail_was" -eq 1 ]] && set -o pipefail
  return "$st"
}

# Broadcast a tx from TERRAD_HOST_KEY. Prints full JSON to stdout.
# Retries transient RPC failures (connection reset / EOF / i/o timeout) and
# account-sequence races (broadcast often succeeded before the reset).
# Usage: terrad_host_tx wasm store ... | jq -r .txhash
terrad_host_tx() {
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "[DRY_RUN] terrad tx $*" >&2
    echo '{"txhash":"DRY_RUN_TX","code":0}'
    return 0
  fi
  local attempts="${TERRAD_HOST_TX_RETRIES:-6}"
  local delay="${TERRAD_HOST_TX_RETRY_DELAY_SEC:-4}"
  local i out st err
  for i in $(seq 1 "$attempts"); do
    set +e
    # shellcheck disable=SC2046
    out="$(terrad_host_exec tx "$@" \
      --from "$TERRAD_HOST_KEY" \
      $(terrad_host_common_flags) \
      --gas auto \
      --gas-adjustment "$TERRAD_HOST_GAS_ADJUSTMENT" \
      $(terrad_host_fee_flags) \
      --broadcast-mode "$TERRAD_HOST_BROADCAST_MODE" \
      -y --output json 2>/tmp/terrad-host-tx.err)"
    st=$?
    set -e
    if [[ "$st" -eq 0 && -n "$(printf '%s' "$out" | jq -r '.txhash // empty' 2>/dev/null)" ]]; then
      printf '%s' "$out"
      return 0
    fi
    err="$(cat /tmp/terrad-host-tx.err 2>/dev/null || true)"
    # Connection dropped after broadcast: next attempt often hits sequence mismatch.
    if echo "$err$out" | grep -qiE 'connection reset|EOF|i/o timeout|broken pipe|TLS handshake|temporarily unavailable|503|502|429|account sequence mismatch|incorrect account sequence'; then
      echo "[terrad-host] transient RPC/sequence error (attempt $i/$attempts), retry in ${delay}s" >&2
      echo "  ${err:-$out}" | head -c 500 >&2
      echo >&2
      sleep "$delay"
      delay=$((delay + 2))
      continue
    fi
    printf '%s\n' "$err" >&2
    printf '%s' "$out"
    return "$st"
  done
  echo "[terrad-host] exhausted $attempts RPC retries" >&2
  cat /tmp/terrad-host-tx.err >&2 || true
  return 1
}

# Query helpers (read-only; ignore DRY_RUN for real queries unless forced).
terrad_host_query() {
  if [[ "${DRY_RUN:-0}" == "1" && "${DRY_RUN_ALLOW_QUERY:-0}" != "1" ]]; then
    echo '{}'
    return 0
  fi
  # shellcheck disable=SC2046
  terrad query "$@" $(terrad_host_query_flags) --output json
}

terrad_host_wait_tx_inclusion() {
  local tx_hash="$1"
  local timeout="${2:-120}"
  local elapsed=0
  local result code

  if [[ "$tx_hash" == "DRY_RUN_TX" ]]; then
    return 0
  fi

  # shellcheck source=terrad-wait-tx.sh
  source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/terrad-wait-tx.sh"

  while [ "$elapsed" -lt "$timeout" ]; do
    if result="$(terrad query tx "$tx_hash" --node "$TERRAD_HOST_NODE" --output json 2>/dev/null)"; then
      if terrad_tx_query_succeeded "$result"; then
        return 0
      fi
      code="$(printf '%s' "$result" | jq -r '.tx_response.code // .code // empty' 2>/dev/null || true)"
      if [[ -n "$code" && "$code" != "0" ]]; then
        echo "[terrad-host] tx ${tx_hash} failed with code=${code}" >&2
        printf '%s\n' "$result" >&2
        return 1
      fi
    fi
    sleep 1
    elapsed=$((elapsed + 1))
  done

  echo "[terrad-host] timeout waiting for tx ${tx_hash} after ${timeout}s" >&2
  return 1
}

terrad_host_wait_tx_query() {
  local tx_hash="$1"
  local timeout="${2:-120}"
  local elapsed=0
  local result code

  if [[ "$tx_hash" == "DRY_RUN_TX" ]]; then
    echo '{"txhash":"DRY_RUN_TX","code":0,"events":[]}'
    return 0
  fi

  # shellcheck source=terrad-wait-tx.sh
  source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/terrad-wait-tx.sh"

  while [ "$elapsed" -lt "$timeout" ]; do
    if result="$(terrad query tx "$tx_hash" --node "$TERRAD_HOST_NODE" --output json 2>/dev/null)"; then
      if terrad_tx_query_succeeded "$result"; then
        printf '%s' "$result"
        return 0
      fi
      code="$(printf '%s' "$result" | jq -r '.tx_response.code // .code // empty' 2>/dev/null || true)"
      if [[ -n "$code" && "$code" != "0" ]]; then
        echo "[terrad-host] tx ${tx_hash} failed with code=${code}" >&2
        return 1
      fi
    fi
    sleep 1
    elapsed=$((elapsed + 1))
  done

  echo "[terrad-host] timeout querying tx ${tx_hash} after ${timeout}s" >&2
  return 1
}

terrad_host_code_id_from_store_tx() {
  local tx_hash="$1"
  local json
  json="$(terrad_host_wait_tx_query "$tx_hash")"
  # shellcheck source=terrad-tx-events.sh
  source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/terrad-tx-events.sh"
  if [[ "$tx_hash" == "DRY_RUN_TX" ]]; then
    echo "${DRY_RUN_CODE_ID:-999001}"
    return 0
  fi
  printf '%s' "$json" | terrad_jq_code_id_from_tx_json | head -1
}

terrad_host_contract_address_from_instantiate_tx() {
  local tx_hash="$1"
  local json
  json="$(terrad_host_wait_tx_query "$tx_hash")"
  # shellcheck source=terrad-tx-events.sh
  source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/terrad-tx-events.sh"
  if [[ "$tx_hash" == "DRY_RUN_TX" ]]; then
    # Unique per call so dry-run catalogs stay distinguishable
    echo "terra1dryrun$(printf '%s' "${DRY_RUN_LABEL:-contract}${RANDOM}" | sha256sum | awk '{print substr($1,1,38)}')"
    return 0
  fi
  printf '%s' "$json" | terrad_jq_contract_address_from_tx_json | head -1
}

terrad_host_key_address() {
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "${TERRAD_HOST_EXPECTED_ADDR:-terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv}"
    return 0
  fi
  terrad_host_resolve_keyring_backend
  local addr err errfile
  if [[ "$TERRAD_HOST_KEYRING_BACKEND" == "file" && -z "${TERRAD_HOST_KEYRING_PASS:-}" && ! -t 0 ]]; then
    echo "ERROR: keyring-file is encrypted and stdin is not a TTY." >&2
    echo "  Unlock once for this shell, then re-run:" >&2
    echo "    read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS" >&2
    return 1
  fi
  errfile="$(mktemp)"
  # shellcheck disable=SC2034
  if ! addr="$(terrad_host_exec keys show "$TERRAD_HOST_KEY" \
    --keyring-backend "$TERRAD_HOST_KEYRING_BACKEND" \
    --home "$TERRAD_HOST_HOME" \
    -a 2>"$errfile")"; then
    err="$(cat "$errfile" 2>/dev/null || true)"
    rm -f "$errfile"
    echo "ERROR: cannot resolve key '$TERRAD_HOST_KEY' (backend=$TERRAD_HOST_KEYRING_BACKEND home=$TERRAD_HOST_HOME)" >&2
    [[ -n "$err" ]] && printf '%s\n' "$err" >&2
    if echo "$err" | grep -qi 'incorrect passphrase\|failed passphrase\|password must'; then
      echo "  Hint: wrong TERRAD_HOST_KEYRING_PASS (file keyring passphrase)." >&2
    elif echo "$err" | grep -qi 'bech32\|not a valid name'; then
      echo "  Hint: key name was treated as an address — wrong --keyring-backend." >&2
      echo "  Try: TERRAD_HOST_KEYRING_BACKEND=file" >&2
    elif [[ -z "$err" ]]; then
      echo "  Hint: empty terrad stderr often means passphrase pipe/pipefail mismatch; pull latest terrad-host.sh." >&2
    fi
    return 1
  fi
  rm -f "$errfile"
  printf '%s' "$addr"
}
