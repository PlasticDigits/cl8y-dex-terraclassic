#!/usr/bin/env bash
# Parse `terrad query tx` JSON for wasm lifecycle events.
#
# Terra Classic terrad v4 (SDK 0.53) exposes ABCI events at top-level `.events`.
# Legacy responses nest them under `.logs[0].events` (terrad v3.x).
#
# GitLab #292 — source this from deploy / e2e shell scripts.

# shellcheck disable=SC2034
TERRAD_TX_EVENTS_JQ_FILTER='(.events // .logs[0].events // [])[]'

terrad_jq_code_id_from_tx_json() {
  jq -r "${TERRAD_TX_EVENTS_JQ_FILTER} | select(.type==\"store_code\") | .attributes[] | select(.key==\"code_id\") | .value"
}

terrad_jq_contract_address_from_tx_json() {
  jq -r "${TERRAD_TX_EVENTS_JQ_FILTER} | select(.type==\"instantiate\") | .attributes[] | select(.key==\"_contract_address\") | .value"
}
