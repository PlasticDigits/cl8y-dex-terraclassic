#!/usr/bin/env bash
# Stop QA indexer (pidfile) then docker compose (LocalTerra + Postgres).
# When QA_FRESH_VOLUMES is set, also removes named volumes (localterra-data, postgres-data).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PIDFILE="${REPO_ROOT}/.indexer-qa.pid"

# shellcheck source=scripts/qa/lib/qa-env.sh
source "$REPO_ROOT/scripts/qa/lib/qa-env.sh"
qa_load_env

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

_vol_args="$(qa_compose_down_volume_args)"
if [ -n "$_vol_args" ]; then
  echo "[stop-qa] Removing QA Docker volumes (${_vol_args})..."
  docker compose down -v
else
  docker compose down
fi

echo "[stop-qa] Done."
