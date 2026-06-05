#!/usr/bin/env bash
# QA server: LocalTerra + Postgres → build + deploy-dex-local → indexer → tunnel instructions.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/qa/lib/qa-env.sh
source "$REPO_ROOT/scripts/qa/lib/qa-env.sh"
# shellcheck source=scripts/lib/qa-phase-timing.sh
source "$REPO_ROOT/scripts/lib/qa-phase-timing.sh"
# shellcheck source=scripts/lib/deploy-up-to-date.sh
source "$REPO_ROOT/scripts/lib/deploy-up-to-date.sh"

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

qa_load_env
qa_timing_begin_session

if qa_is_fresh_volumes; then
  chmod +x "$REPO_ROOT/scripts/qa/lib/print-fresh-volumes-banner.sh" 2>/dev/null || true
  "$REPO_ROOT/scripts/qa/lib/print-fresh-volumes-banner.sh"
fi

if [ "${QA_SHARED_HOST:-}" = "1" ]; then
  echo "[start-qa] QA_SHARED_HOST=1 — using ${COMPOSE_FILE}"
fi

qa_timing_phase_start "infra"
echo "==> Tearing down prior QA stack (indexer + compose) if present..."
"$REPO_ROOT/scripts/qa/stop-qa.sh"

echo "==> Starting Docker Compose (localterra + postgres)..."
if ! docker compose up -d localterra postgres; then
  echo "[start-qa] ERROR: docker compose up failed." >&2
  docker compose ps -a 2>&1 || true
  exit 1
fi

echo "==> Waiting for LocalTerra (${TERRA_RPC_URL})..."
_qa_localterra_container="$(docker compose ps -q localterra 2>/dev/null | head -1 || true)"
for i in $(seq 1 60); do
  if [ -n "$_qa_localterra_container" ] && docker exec "$_qa_localterra_container" curl -m 3 -sf http://127.0.0.1:26657/status >/dev/null 2>&1; then
    echo "LocalTerra is ready!"
    break
  elif curl -m 3 -sf "${TERRA_RPC_URL}/status" >/dev/null 2>&1; then
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
  if docker compose exec -T postgres pg_isready -U "${POSTGRES_USER:-cl8y_legal}" >/dev/null 2>&1; then
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
qa_timing_phase_end

if [ "${QA_FETCH_CI_ARTIFACTS:-0}" = "1" ]; then
  qa_timing_phase_start "fetch-ci-artifacts"
  chmod +x "$REPO_ROOT/scripts/qa/fetch-qa-ci-artifacts.sh"
  "$REPO_ROOT/scripts/qa/fetch-qa-ci-artifacts.sh" || true
  qa_timing_phase_end
fi

qa_timing_phase_start "deploy"
export TERRA_RPC_URL TERRA_LCD_URL DEX_TERRA_RPC_PORT DEX_TERRA_LCD_PORT

DEPLOY_SKIPPED=0
if deploy_up_to_date "$REPO_ROOT"; then
  echo "[start-qa] deploy stamp matches HEAD + factory LCD probe — skipping make deploy-local"
  DEPLOY_SKIPPED=1
elif compgen -G "$REPO_ROOT/smartcontracts/artifacts/cl8y_dex_*.wasm" >/dev/null; then
  echo "[start-qa] wasm artifacts present — make deploy-local-no-build"
  make deploy-local-no-build
else
  echo "[start-qa] no fresh wasm artifacts — make deploy-local (optimizer + deploy)"
  make deploy-local
fi
qa_timing_phase_end

echo "==> Verifying deployed contracts match current tree (qa-verify-deploy)..."
chmod +x "$REPO_ROOT/scripts/qa/verify-deploy.sh" "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
"$REPO_ROOT/scripts/qa/verify-deploy.sh"

qa_timing_phase_start "indexer"
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
if [ -z "$INDEXER_BIN" ] || [ ! -x "$INDEXER_BIN" ]; then
  _default_indexer="${REPO_ROOT}/indexer/target/release/cl8y-dex-indexer"
  if [ -x "$_default_indexer" ]; then
    INDEXER_BIN="$_default_indexer"
  fi
fi

if [ -z "$INDEXER_BIN" ] || [ ! -x "$INDEXER_BIN" ]; then
  echo "[start-qa] building indexer release binary (set INDEXER_QA_BIN to skip)..."
  (cd "$REPO_ROOT/indexer" && cargo build --release)
  INDEXER_BIN="${REPO_ROOT}/indexer/target/release/cl8y-dex-indexer"
fi

if [ -n "$INDEXER_BIN" ] && [ -x "$INDEXER_BIN" ]; then
  nohup sh -c "cd \"$REPO_ROOT/indexer\" && set -a && [ -f .env ] && . ./.env && set +a && exec \"$INDEXER_BIN\"" >>"$LOGFILE" 2>&1 &
else
  nohup sh -c "cd \"$REPO_ROOT/indexer\" && set -a && [ -f .env ] && . ./.env && set +a && exec cargo run --release" >>"$LOGFILE" 2>&1 &
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
qa_timing_phase_end
qa_timing_session_end

echo ""
echo "========================================================================"
echo "  start-qa finished successfully on this host."
if [ "$DEPLOY_SKIPPED" = "1" ]; then
  echo "  (deploy skipped — stamp matched HEAD)"
fi
echo "========================================================================"
chmod +x "$REPO_ROOT/scripts/qa/print-qa-tunnel-instructions.sh" 2>/dev/null || true
"$REPO_ROOT/scripts/qa/print-qa-tunnel-instructions.sh"
