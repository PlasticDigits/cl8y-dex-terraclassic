#!/usr/bin/env bash
# Stop QA indexer (pidfile) then docker compose (LocalTerra + Postgres).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PIDFILE="${REPO_ROOT}/.indexer-qa.pid"

set -a
if [ -f "${REPO_ROOT}/.env" ]; then
  # shellcheck source=/dev/null
  source "${REPO_ROOT}/.env"
fi
# shellcheck source=/dev/null
source "${REPO_ROOT}/scripts/qa/qa-host.env"
set +a

if [ "${QA_SHARED_HOST:-}" = "1" ]; then
  export COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.yml:docker-compose.qa-shared-host.yml}"
fi

_stop_indexer() {
  if [ ! -f "$PIDFILE" ]; then
    return 0
  fi
  local pid
  pid="$(cat "$PIDFILE" 2>/dev/null || true)"
  if [ -z "${pid}" ]; then
    rm -f "$PIDFILE"
    return 0
  fi
  if kill -0 "$pid" 2>/dev/null; then
    echo "[stop-qa] Stopping indexer (pid ${pid})..."
    kill -TERM "$pid" 2>/dev/null || true
    for _ in $(seq 1 30); do
      if ! kill -0 "$pid" 2>/dev/null; then
        break
      fi
      sleep 1
    done
    if kill -0 "$pid" 2>/dev/null; then
      echo "[stop-qa] Indexer still running; sending SIGKILL..."
      kill -KILL "$pid" 2>/dev/null || true
    fi
  fi
  rm -f "$PIDFILE"
}

_stop_indexer

docker compose down

echo "[stop-qa] Done."
