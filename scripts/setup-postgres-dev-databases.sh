#!/usr/bin/env bash
# Ensure local dev Postgres databases exist (idempotent). Called during deploy / wait-healthy.
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

export PGPASSWORD="$POSTGRES_PASSWORD"

ensure_db() {
  local db=$1
  local exists
  exists="$(psql -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" -d postgres -tAc \
    "SELECT 1 FROM pg_database WHERE datname='${db}'" 2>/dev/null || true)"
  if [ "$exists" = "1" ]; then
    echo "[setup-postgres] database ${db} already exists"
    return 0
  fi
  echo "[setup-postgres] creating database ${db} (owner ${POSTGRES_USER})..."
  psql -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" -d postgres -v ON_ERROR_STOP=1 \
    -c "CREATE DATABASE \"${db}\" OWNER \"${POSTGRES_USER}\";"
}

if ! command -v psql >/dev/null 2>&1; then
  echo "[setup-postgres] WARN: psql not found; skipping database ensure" >&2
  exit 0
fi

if ! psql -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" -d postgres -c '\q' 2>/dev/null; then
  echo "[setup-postgres] WARN: cannot connect as ${POSTGRES_USER}@${POSTGRES_HOST}:${POSTGRES_PORT}; skipping" >&2
  exit 0
fi

ensure_db "$POSTGRES_DB"
ensure_db "$POSTGRES_TEST_DB"

echo "[setup-postgres] DATABASE_URL=${DATABASE_URL}"
echo "[setup-postgres] TEST_DATABASE_URL=${TEST_DATABASE_URL}"
