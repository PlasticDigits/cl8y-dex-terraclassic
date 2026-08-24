#!/usr/bin/env bash
# Start indexer for strict Playwright (price chart smoke, trade layout, etc.).
# Requires: deploy-dex-local.sh (indexer/.env + frontend-dapp/.env.local), release binary built.
# GitLab #228 — used locally; reference automation job `e2e` after deploy (see docs/testing.md § CI).
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

# shellcheck source=scripts/lib/indexer-cors-playwright.sh
source "$REPO_ROOT/scripts/lib/indexer-cors-playwright.sh"
# shellcheck source=scripts/lib/indexer-port.sh
source "$REPO_ROOT/scripts/lib/indexer-port.sh"
CORS_CHANGED=0
if indexer_cors_apply_env_file "$REPO_ROOT/indexer/.env"; then
  CORS_CHANGED=1
fi

INDEXER_PORT="$(indexer_port_from_url "$INDEXER_URL")"
if [[ "$CORS_CHANGED" -eq 0 ]] && curl -sf "${INDEXER_URL}/api/v1/overview" >/dev/null 2>&1; then
  indexer_sync_pidfile_with_port "$INDEXER_PID_FILE" "$INDEXER_PORT"
  echo "[e2e-start-indexer] reusing indexer already listening on :${INDEXER_PORT}"
  exit 0
fi
if [[ -f "$INDEXER_PID_FILE" ]]; then
  old_pid="$(cat "$INDEXER_PID_FILE" 2>/dev/null || true)"
  if [[ -n "$old_pid" ]] && kill -0 "$old_pid" 2>/dev/null; then
    if [[ "$CORS_CHANGED" -eq 0 ]] && curl -sf "${INDEXER_URL}/api/v1/overview" >/dev/null 2>&1; then
      echo "[e2e-start-indexer] reusing running indexer pid ${old_pid}"
      exit 0
    fi
    echo "[e2e-start-indexer] stopping indexer pid ${old_pid} (stale or CORS_ORIGINS updated)"
    kill "$old_pid" 2>/dev/null || true
    sleep 1
  fi
  rm -f "$INDEXER_PID_FILE"
fi
# Worktree / leftover QA may have started the API under another pid file.
if [[ "$CORS_CHANGED" -eq 1 ]]; then
  echo "[e2e-start-indexer] restarting :${INDEXER_PORT} so CORS_ORIGINS includes Playwright Vite"
  indexer_stop_port_listeners "$INDEXER_PORT"
fi

echo "[e2e-start-indexer] starting indexer (log: ${INDEXER_LOG})"
set -a
# shellcheck disable=SC1091
source "$REPO_ROOT/indexer/.env"
set +a
nohup "$REPO_ROOT/indexer/target/release/cl8y-dex-indexer" >"$INDEXER_LOG" 2>&1 &
echo $! >"$INDEXER_PID_FILE"
_wait_indexer_up
