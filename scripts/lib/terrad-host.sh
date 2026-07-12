#!/usr/bin/env bash
# Host-side terrad helpers for mainnet/testnet (no Docker LocalTerra).
# Pair with scripts/lib/terrad-tx-events.sh for code_id / contract address parsing.
# shellcheck shell=bash

# Defaults match columbus-5 operator docs; override via env.
TERRAD_HOST_CHAIN_ID="${TERRAD_HOST_CHAIN_ID:-columbus-5}"
TERRAD_HOST_NODE="${TERRAD_HOST_NODE:-https://terra-classic-rpc.publicnode.com:443}"
TERRAD_HOST_KEY="${TERRAD_HOST_KEY:-cl8ydeploy}"
TERRAD_HOST_KEYRING_BACKEND="${TERRAD_HOST_KEYRING_BACKEND:-os}"
TERRAD_HOST_GAS_ADJUSTMENT="${TERRAD_HOST_GAS_ADJUSTMENT:-1.4}"
# Classic tax-module floor (FCD /v1/txs/gas_prices). Prefer gas-prices so fee = gas_wanted × price.
# Fixed --fees is wrong for mixed store/execute workloads (5 LUNC too low for store; flat 100 LUNC overpays executes).
TERRAD_HOST_GAS_PRICES="${TERRAD_HOST_GAS_PRICES:-28.325uluna}"
# Optional escape hatch: if set, use --fees instead of --gas-prices.
TERRAD_HOST_FEES="${TERRAD_HOST_FEES:-}"
TERRAD_HOST_BROADCAST_MODE="${TERRAD_HOST_BROADCAST_MODE:-sync}"

terrad_host_common_flags() {
  echo --chain-id "$TERRAD_HOST_CHAIN_ID" \
    --node "$TERRAD_HOST_NODE" \
    --keyring-backend "$TERRAD_HOST_KEYRING_BACKEND"
}

# Fee flags: gas-prices (default) or explicit TERRAD_HOST_FEES.
terrad_host_fee_flags() {
  if [[ -n "${TERRAD_HOST_FEES:-}" ]]; then
    echo --fees "$TERRAD_HOST_FEES"
  else
    echo --gas-prices "$TERRAD_HOST_GAS_PRICES"
  fi
}

# Broadcast a tx from TERRAD_HOST_KEY. Prints full JSON to stdout.
# Usage: terrad_host_tx wasm store ... | jq -r .txhash
terrad_host_tx() {
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "[DRY_RUN] terrad tx $*" >&2
    echo '{"txhash":"DRY_RUN_TX","code":0}'
    return 0
  fi
  # shellcheck disable=SC2046
  terrad tx "$@" \
    --from "$TERRAD_HOST_KEY" \
    $(terrad_host_common_flags) \
    --gas auto \
    --gas-adjustment "$TERRAD_HOST_GAS_ADJUSTMENT" \
    $(terrad_host_fee_flags) \
    --broadcast-mode "$TERRAD_HOST_BROADCAST_MODE" \
    -y --output json
}

# Query helpers (read-only; ignore DRY_RUN for real queries unless forced).
terrad_host_query() {
  if [[ "${DRY_RUN:-0}" == "1" && "${DRY_RUN_ALLOW_QUERY:-0}" != "1" ]]; then
    echo '{}'
    return 0
  fi
  terrad query "$@" $(terrad_host_common_flags) --output json
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
  terrad keys show "$TERRAD_HOST_KEY" \
    --keyring-backend "$TERRAD_HOST_KEYRING_BACKEND" \
    -a
}
