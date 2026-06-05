#!/usr/bin/env bash
# Poll terrad until a broadcast tx is queryable (replaces fixed sleep 3 in deploy scripts).
# GitLab #325 — see scripts/bots/swarm.py _poll_tx_inclusion for the Python equivalent.

terrad_tx_query_succeeded() {
  local json="$1"
  local code
  code="$(printf '%s' "$json" | jq -r '.tx_response.code // .code // empty' 2>/dev/null || true)"
  [[ -n "$code" && "$code" = "0" ]]
}

# Args: container tx_hash node [timeout_sec]
terrad_wait_tx_inclusion() {
  local container="$1"
  local tx_hash="$2"
  local node="$3"
  local timeout="${4:-90}"
  local elapsed=0
  local result code

  while [ "$elapsed" -lt "$timeout" ]; do
    if result="$(docker exec "$container" terrad query tx "$tx_hash" --node "$node" --output json 2>/dev/null)"; then
      if terrad_tx_query_succeeded "$result"; then
        return 0
      fi
      code="$(printf '%s' "$result" | jq -r '.tx_response.code // .code // empty' 2>/dev/null || true)"
      if [[ -n "$code" && "$code" != "0" ]]; then
        echo "[terrad-wait-tx] tx ${tx_hash} failed with code=${code}" >&2
        return 1
      fi
    fi
    sleep 1
    elapsed=$((elapsed + 1))
  done

  echo "[terrad-wait-tx] timeout waiting for tx ${tx_hash} after ${timeout}s" >&2
  return 1
}

# Args: container tx_hash node [timeout_sec] — prints query JSON on success.
terrad_wait_tx_query() {
  local container="$1"
  local tx_hash="$2"
  local node="$3"
  local timeout="${4:-90}"
  local elapsed=0
  local result code

  while [ "$elapsed" -lt "$timeout" ]; do
    if result="$(docker exec "$container" terrad query tx "$tx_hash" --node "$node" --output json 2>/dev/null)"; then
      if terrad_tx_query_succeeded "$result"; then
        printf '%s' "$result"
        return 0
      fi
      code="$(printf '%s' "$result" | jq -r '.tx_response.code // .code // empty' 2>/dev/null || true)"
      if [[ -n "$code" && "$code" != "0" ]]; then
        echo "[terrad-wait-tx] tx ${tx_hash} failed with code=${code}" >&2
        return 1
      fi
    fi
    sleep 1
    elapsed=$((elapsed + 1))
  done

  echo "[terrad-wait-tx] timeout querying tx ${tx_hash} after ${timeout}s" >&2
  return 1
}
