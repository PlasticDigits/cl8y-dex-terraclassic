#!/usr/bin/env bash
# Start indexer for strict Playwright (price chart smoke, trade layout, etc.).
# Requires: deploy-dex-local.sh (indexer/.env + frontend-dapp/.env.local), release binary built.
# GitLab #228 — used locally and from CI job `e2e` after deploy.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INDEXER_LOG="${INDEXER_LOG:-/tmp/cl8y-indexer-e2e.log}"
INDEXER_PID_FILE="${INDEXER_PID_FILE:-/tmp/cl8y-indexer-e2e.pid}"
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
  for i in $(seq 1 120); do
    if curl -sf "${INDEXER_URL}/api/v1/overview" >/dev/null 2>&1; then
      echo "[e2e-start-indexer] indexer ready at ${INDEXER_URL}"
      return 0
    fi
    sleep 1
  done
  echo "--- indexer log (${INDEXER_LOG}) ---" >&2
  tail -n 200 "$INDEXER_LOG" >&2 || true
  _fail "Indexer did not become ready at ${INDEXER_URL}"
}

_require_file "$REPO_ROOT/indexer/.env" "run scripts/deploy-dex-local.sh first"
_require_file "$REPO_ROOT/frontend-dapp/.env.local" "run scripts/deploy-dex-local.sh first"
_require_file "$REPO_ROOT/indexer/target/release/cl8y-dex-indexer" "build: (cd indexer && cargo build --release)"

if [[ -f "$INDEXER_PID_FILE" ]]; then
  old_pid="$(cat "$INDEXER_PID_FILE" 2>/dev/null || true)"
  if [[ -n "$old_pid" ]] && kill -0 "$old_pid" 2>/dev/null; then
    if curl -sf "${INDEXER_URL}/api/v1/overview" >/dev/null 2>&1; then
      echo "[e2e-start-indexer] reusing running indexer pid ${old_pid}"
      exit 0
    fi
    echo "[e2e-start-indexer] stopping stale indexer pid ${old_pid}"
    kill "$old_pid" 2>/dev/null || true
    sleep 1
  fi
  rm -f "$INDEXER_PID_FILE"
fi

echo "[e2e-start-indexer] starting indexer (log: ${INDEXER_LOG})"
set -a
# shellcheck disable=SC1091
source "$REPO_ROOT/indexer/.env"
set +a
nohup "$REPO_ROOT/indexer/target/release/cl8y-dex-indexer" >"$INDEXER_LOG" 2>&1 &
echo $! >"$INDEXER_PID_FILE"
_wait_indexer_up
