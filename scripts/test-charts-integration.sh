#!/usr/bin/env bash
# One-command local charts integration tests: Postgres ensure → migrate → seed → Vitest.
# Fixture pair: terra1paircontractabc (see frontend-dapp/src/test/chartsIntegrationConstants.ts).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

set -a
if [ -f "$REPO_ROOT/.env" ]; then
  # shellcheck source=/dev/null
  source "$REPO_ROOT/.env"
fi
# shellcheck source=scripts/lib/postgres-dev.env
source "$REPO_ROOT/scripts/lib/postgres-dev.env"
set +a

VITE_INDEXER_URL="${VITE_INDEXER_URL:-http://127.0.0.1:3001}"
VITE_TERRA_LCD_URL="${VITE_TERRA_LCD_URL:-http://localhost:1317}"
VITE_TERRA_RPC_URL="${VITE_TERRA_RPC_URL:-http://localhost:26657}"
CHARTS_INT_DATABASE_URL="${CHARTS_INT_DATABASE_URL:-$DATABASE_URL}"
export DATABASE_URL="$CHARTS_INT_DATABASE_URL"
export PGPASSWORD="$POSTGRES_PASSWORD"

_charts_db_name() {
  local url="${1:?}"
  local name="${url##*/}"
  name="${name%%\?*}"
  echo "$name"
}

CHARTS_DB="$(_charts_db_name "$DATABASE_URL")"

_fail() {
  echo "ERROR: $*" >&2
  exit 1
}

_require_cmd() {
  local cmd=$1
  local hint=$2
  command -v "$cmd" >/dev/null 2>&1 || _fail "$hint"
}

_ensure_charts_db() {
  local exists
  exists="$(psql -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" -d postgres -tAc \
    "SELECT 1 FROM pg_database WHERE datname='${CHARTS_DB}'" 2>/dev/null || true)"
  if [ "$exists" = "1" ]; then
    echo "[test-charts-integration] database ${CHARTS_DB} already exists"
    return 0
  fi
  echo "[test-charts-integration] creating database ${CHARTS_DB} (owner ${POSTGRES_USER})..."
  psql -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" -d postgres -v ON_ERROR_STOP=1 \
    -c "CREATE DATABASE \"${CHARTS_DB}\" OWNER \"${POSTGRES_USER}\";"
}

_require_postgres() {
  _require_cmd psql "psql not found. Install PostgreSQL client tools."
  if ! pg_isready -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" >/dev/null 2>&1; then
    _fail "Postgres unreachable at ${POSTGRES_USER}@${POSTGRES_HOST}:${POSTGRES_PORT}. Start Postgres (e.g. docker compose up -d postgres) or see skills/AGENTS_LOCAL_POSTGRES_DEV.md"
  fi
  if ! psql -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" -d postgres -c '\q' 2>/dev/null; then
    _fail "Cannot connect to Postgres as ${POSTGRES_USER}@${POSTGRES_HOST}:${POSTGRES_PORT}. Check credentials in scripts/lib/postgres-dev.env or repo .env"
  fi
}

_require_indexer() {
  if ! curl -sf "${VITE_INDEXER_URL%/}/health" >/dev/null 2>&1; then
    _fail "Indexer HTTP not reachable at ${VITE_INDEXER_URL}. Start the indexer first (make start-qa or make indexer-dev) and ensure it uses DATABASE_URL=${DATABASE_URL}"
  fi
}

_run_migrations() {
  _require_cmd sqlx "sqlx-cli not found. Install: cargo install sqlx-cli --version 0.8.6 --no-default-features --features rustls,postgres --locked"
  echo "[test-charts-integration] running migrations against ${DATABASE_URL}"
  (cd "$REPO_ROOT/indexer" && sqlx migrate run)
}

_seed_fixtures() {
  echo "[test-charts-integration] seeding charts integration fixtures (idempotent)"
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$REPO_ROOT/indexer/scripts/seed-charts-integration.sql"
}

_run_vitest() {
  echo "[test-charts-integration] running frontend integration tests (VITE_INDEXER_URL=${VITE_INDEXER_URL})"
  bash "$REPO_ROOT/scripts/with-node.sh" --cwd frontend-dapp -- \
    env \
      VITE_INDEXER_URL="$VITE_INDEXER_URL" \
      VITE_TERRA_LCD_URL="$VITE_TERRA_LCD_URL" \
      VITE_TERRA_RPC_URL="$VITE_TERRA_RPC_URL" \
      npm run test:integration
}

echo "[test-charts-integration] DATABASE_URL=${DATABASE_URL}"
_require_postgres
_ensure_charts_db
_run_migrations
_seed_fixtures
_require_indexer
_run_vitest
