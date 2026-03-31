#!/usr/bin/env bash
# QA server: LocalTerra + Postgres → build + deploy-dex-local → indexer → tunnel instructions.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PIDFILE="${REPO_ROOT}/.indexer-qa.pid"
LOGFILE="${REPO_ROOT}/.indexer-qa.log"

if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
  _QA_HI=$'\033[93;1m'
  _QA_RST=$'\033[0m'
else
  _QA_HI=''
  _QA_RST=''
fi
printf '%b\n' "${_QA_HI}┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓${_QA_RST}"
printf '%b\n' "${_QA_HI}┃  REMINDER: SSH tunnel + laptop steps print at END; reprint: make qa-tunnel-help ┃${_QA_RST}"
printf '%b\n' "${_QA_HI}┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛${_QA_RST}"
echo ""

set -a
if [ -f "$REPO_ROOT/.env" ]; then
  # shellcheck source=/dev/null
  source "$REPO_ROOT/.env"
fi
# shellcheck source=/dev/null
source "$REPO_ROOT/scripts/qa/qa-host.env"
set +a

if [ "${QA_SHARED_HOST:-}" = "1" ]; then
  export COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.yml:docker-compose.qa-shared-host.yml}"
  echo "[start-qa] QA_SHARED_HOST=1 — using ${COMPOSE_FILE}"
fi

echo "==> Tearing down prior QA stack (indexer + compose) if present..."
"$REPO_ROOT/scripts/qa/stop-qa.sh"

echo "==> Starting Docker Compose (localterra + postgres)..."
if ! docker compose up -d localterra postgres; then
  echo "[start-qa] ERROR: docker compose up failed." >&2
  docker compose ps -a 2>&1 || true
  exit 1
fi

echo "==> Waiting for LocalTerra (${TERRA_RPC_URL})..."
for i in $(seq 1 60); do
  if curl -sf "${TERRA_RPC_URL}/status" >/dev/null 2>&1; then
    echo "LocalTerra is ready!"
    break
  fi
  if [ "$i" -eq 60 ]; then
    echo "ERROR: LocalTerra did not start in time." >&2
    exit 1
  fi
  sleep 2
done

echo "==> Waiting for Postgres..."
for i in $(seq 1 30); do
  if docker compose exec -T postgres pg_isready -U postgres >/dev/null 2>&1; then
    echo "Postgres is ready!"
    break
  fi
  if [ "$i" -eq 30 ]; then
    echo "ERROR: Postgres did not start in time." >&2
    exit 1
  fi
  sleep 2
done

docker compose ps

echo "==> Build optimized wasm + deploy to LocalTerra (deploy-dex-local)..."
export TERRA_RPC_URL TERRA_LCD_URL DEX_TERRA_RPC_PORT DEX_TERRA_LCD_PORT
make deploy-local

echo "==> Starting indexer (release, background)..."
if [ -f "$PIDFILE" ]; then
  old="$(cat "$PIDFILE" 2>/dev/null || true)"
  if [ -n "${old}" ] && kill -0 "${old}" 2>/dev/null; then
    echo "[start-qa] WARN: stale pidfile; stop-qa should have cleared it. Stopping pid ${old}..." >&2
    kill -TERM "${old}" 2>/dev/null || true
    sleep 2
  fi
  rm -f "$PIDFILE"
fi

INDEXER_BIN="${INDEXER_QA_BIN:-}"
if [ -n "$INDEXER_BIN" ] && [ -x "$INDEXER_BIN" ]; then
  nohup env DATABASE_URL="${DATABASE_URL:-postgres://postgres:postgres@127.0.0.1:5432/dex_indexer}" \
    sh -c "cd \"$REPO_ROOT/indexer\" && exec \"$INDEXER_BIN\"" >>"$LOGFILE" 2>&1 &
else
  nohup env DATABASE_URL="${DATABASE_URL:-postgres://postgres:postgres@127.0.0.1:5432/dex_indexer}" \
    sh -c "cd \"$REPO_ROOT/indexer\" && exec cargo run --release" >>"$LOGFILE" 2>&1 &
fi
echo $! >"$PIDFILE"
echo "[start-qa] Indexer pid $(cat "$PIDFILE"); log: $LOGFILE"

echo "==> Waiting for indexer /health (first release build can take several minutes)..."
ok=0
for i in $(seq 1 120); do
  if curl -sf "http://127.0.0.1:${API_PORT}/health" >/dev/null 2>&1; then
    ok=1
    break
  fi
  sleep 3
done
if [ "$ok" != 1 ]; then
  echo "[start-qa] ERROR: indexer health check failed (http://127.0.0.1:${API_PORT}/health)" >&2
  echo "  See: $LOGFILE" >&2
  exit 1
fi
echo "==> Indexer healthy."

echo ""
echo "========================================================================"
echo "  start-qa finished successfully on this host."
echo "========================================================================"
chmod +x "$REPO_ROOT/scripts/qa/print-qa-tunnel-instructions.sh" 2>/dev/null || true
"$REPO_ROOT/scripts/qa/print-qa-tunnel-instructions.sh"
