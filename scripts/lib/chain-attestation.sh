#!/usr/bin/env bash
# Shared policy: fail closed, attest each selected RPC, pin per-run code identity.
# shellcheck shell=bash
CHAIN_ATTESTATION_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

terrad_host_attest() {
  local node="${1:-$TERRAD_HOST_NODE}" receipt baseline="${TERRAD_HOST_ATTESTED_IDENTITIES:-}"
  [[ -n "$baseline" ]] || baseline='{}'
  local expected="${TERRAD_HOST_EXPECTED_CHAIN_ID:-$TERRAD_HOST_CHAIN_ID}"
  if [[ "$expected" != "$TERRAD_HOST_CHAIN_ID" ]]; then
    echo "[chain-attestation] FAIL: expected chain differs from signing chain" >&2
    return 1
  fi
  local -a pin_flags=()
  if [[ -n "${TERRAD_HOST_IDENTITY_PINS:-}" ]]; then
    pin_flags=(--pins "$TERRAD_HOST_IDENTITY_PINS")
  fi
  receipt="$(python3 "$CHAIN_ATTESTATION_LIB_DIR/chain-attestation.py" \
    --expected-chain "$expected" \
    --lcd "${TERRAD_HOST_LCD_URL:-${LCD_URL:-https://terra-classic-lcd.publicnode.com}}" \
    --rpc "$node" --fallbacks "${TERRAD_HOST_RPC_FALLBACKS:-}" \
    --max-age "${TERRAD_HOST_MAX_BLOCK_AGE_SEC:-120}" \
    --max-lag "${TERRAD_HOST_MAX_HEIGHT_LAG:-5}" \
    --timeout "${TERRAD_HOST_ATTEST_TIMEOUT_SEC:-10}" \
    --contracts "${TERRAD_HOST_IDENTITY_CONTRACTS:-}" \
    --baseline "$baseline" "${pin_flags[@]}")" || return 1
  local selected_index selected_node
  selected_index="$(jq -er '.rpc_candidate_index | select(type == "number" and . >= 0 and . == floor)' <<<"$receipt")" || return 1
  if (( selected_index == 0 )); then
    selected_node="$node"
  else
    local -a fallback_nodes=()
    IFS=',' read -r -a fallback_nodes <<<"${TERRAD_HOST_RPC_FALLBACKS:-}"
    (( selected_index <= ${#fallback_nodes[@]} )) || return 1
    selected_node="${fallback_nodes[$((selected_index - 1))]}"
    selected_node="${selected_node#"${selected_node%%[![:space:]]*}"}"
    selected_node="${selected_node%"${selected_node##*[![:space:]]}"}"
    [[ -n "$selected_node" ]] || return 1
  fi
  TERRAD_HOST_NODE="$selected_node"
  TERRAD_HOST_ATTESTED_IDENTITIES="$(jq -c '.identities' <<<"$receipt")" || return 1
  export TERRAD_HOST_ATTESTED_IDENTITIES
  echo "[chain-attestation] PASS: $expected height=$(jq -r '.height' <<<"$receipt") contracts=$(jq '.identities | length' <<<"$receipt")" >&2
}
