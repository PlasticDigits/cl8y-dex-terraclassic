#!/usr/bin/env bash
# Shared terrad tx helper for Playwright E2E seed scripts (test1 keyring).
# Retries on account sequence mismatch when LocalTerra has concurrent traffic (bot swarm).
# shellcheck shell=bash

e2e_terrad_tx() {
  local container="$1"
  shift
  local max_attempts="${E2E_TERRAD_TX_MAX_ATTEMPTS:-12}"
  local attempt=1
  local out=""

  while (( attempt <= max_attempts )); do
    if out=$(docker exec "$container" terrad tx "$@" \
      --from test1 \
      --keyring-backend test \
      --chain-id localterra \
      --gas auto \
      --gas-adjustment 1.3 \
      --fees 500000000uluna \
      --node http://127.0.0.1:26657 \
      --broadcast-mode sync \
      -y --output json 2>&1); then
      echo "$out"
      return 0
    fi
    if [[ "$out" == *"account sequence mismatch"* ]]; then
      sleep "$attempt"
      ((attempt++))
      continue
    fi
    echo "$out" >&2
    return 1
  done

  echo "e2e_terrad_tx: account sequence mismatch after ${max_attempts} attempts (reduce bot swarm load or retry)." >&2
  return 1
}
