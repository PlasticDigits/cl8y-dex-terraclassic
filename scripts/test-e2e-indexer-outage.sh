#!/usr/bin/env bash
# Playwright market-data-down specs: indexer up (sanity) → stop → E2E_INDEXER_OUTAGE=1.
# Requires: LocalTerra + scripts/deploy-dex-local.sh, Postgres reachable, indexer built.
# CI: see job frontend-e2e-indexer-outage in .github/workflows/test.yml (GitLab #219).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

INDEXER_LOG="${INDEXER_LOG:-/tmp/cl8y-indexer-e2e-outage.log}"
INDEXER_PID_FILE="${INDEXER_PID_FILE:-/tmp/cl8y-indexer-e2e-outage.pid}"
INDEXER_URL="${VITE_INDEXER_URL:-http://127.0.0.1:3001}"
INDEXER_URL="${INDEXER_URL%/}"

_fail() {
  echo "ERROR: $*" >&2
  exit 1
}

_require_file() {
  [[ -f "$1" ]] || _fail "Missing $1 ($2)"
}

_wait_indexer_up() {
  local i
  for i in $(seq 1 90); do
    if curl -sf "${INDEXER_URL}/api/v1/overview" >/dev/null 2>&1; then
      echo "[test-e2e-indexer-outage] indexer ready at ${INDEXER_URL}"
      return 0
    fi
    sleep 1
  done
  echo "--- indexer log (${INDEXER_LOG}) ---" >&2
  tail -n 200 "$INDEXER_LOG" >&2 || true
  _fail "Indexer did not become ready at ${INDEXER_URL}"
}

_wait_indexer_down() {
  local i
  for i in $(seq 1 30); do
    if ! curl -sf "${INDEXER_URL}/api/v1/overview" >/dev/null 2>&1; then
      echo "[test-e2e-indexer-outage] indexer stopped (unreachable at ${INDEXER_URL})"
      return 0
    fi
    sleep 1
  done
  _fail "Indexer still responding at ${INDEXER_URL} after stop"
}

_start_indexer() {
  _require_file "$REPO_ROOT/indexer/.env" "run scripts/deploy-dex-local.sh first"
  _require_file "$REPO_ROOT/indexer/target/release/cl8y-dex-indexer" "build indexer: (cd indexer && cargo build --release)"

  if [[ -f "$INDEXER_PID_FILE" ]]; then
    old_pid="$(cat "$INDEXER_PID_FILE" 2>/dev/null || true)"
    if [[ -n "$old_pid" ]] && kill -0 "$old_pid" 2>/dev/null; then
      echo "[test-e2e-indexer-outage] stopping previous indexer pid ${old_pid}"
      kill "$old_pid" 2>/dev/null || true
      sleep 1
    fi
    rm -f "$INDEXER_PID_FILE"
  fi

  echo "[test-e2e-indexer-outage] starting indexer (log: ${INDEXER_LOG})"
  set -a
  # shellcheck disable=SC1091
  source "$REPO_ROOT/indexer/.env"
  set +a
  nohup "$REPO_ROOT/indexer/target/release/cl8y-dex-indexer" >"$INDEXER_LOG" 2>&1 &
  echo $! >"$INDEXER_PID_FILE"
  _wait_indexer_up
}

_stop_indexer() {
  if [[ -f "$INDEXER_PID_FILE" ]]; then
    pid="$(cat "$INDEXER_PID_FILE")"
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
    rm -f "$INDEXER_PID_FILE"
  fi
  _wait_indexer_down
}

_run_playwright() {
  local pair
  pair="$(bash "$REPO_ROOT/scripts/lib/e2e-trade-pair-from-deploy.sh")"
  echo "[test-e2e-indexer-outage] E2E_TRADE_PAIR=${pair}"
  bash "$REPO_ROOT/scripts/with-node.sh" --cwd frontend-dapp -- \
    env \
      E2E_INDEXER_OUTAGE=1 \
      E2E_TRADE_PAIR="$pair" \
      VITE_NETWORK=local \
      VITE_INDEXER_URL="$INDEXER_URL" \
      npm run test:e2e:indexer-outage
}

_require_file "$REPO_ROOT/frontend-dapp/.env.local" "run scripts/deploy-dex-local.sh first"

trap '_stop_indexer 2>/dev/null || true' EXIT

_start_indexer
_stop_indexer
_run_playwright

echo "[test-e2e-indexer-outage] done"
