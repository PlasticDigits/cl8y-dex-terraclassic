#!/usr/bin/env bash
# Ensure local dev Postgres databases exist (idempotent). Called during deploy / wait-healthy.
# Bootstraps POSTGRES_USER via POSTGRES_SUPERUSER when the role is missing (GitLab #245).
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

# shellcheck source=scripts/lib/postgres-bootstrap-role.sh
source "$REPO_ROOT/scripts/lib/postgres-bootstrap-role.sh"

sync_indexer_database_env() {
  local indexer_env="$REPO_ROOT/indexer/.env"
  # shellcheck source=scripts/lib/upsert-dotenv.sh
  source "$REPO_ROOT/scripts/lib/upsert-dotenv.sh"

  if [ ! -f "$indexer_env" ]; then
    cat >"$indexer_env" <<'EOF'
# Local indexer env — database URLs maintained by scripts/setup-postgres-dev-databases.sh
# Run scripts/deploy-dex-local.sh (or make deploy-local) for factory/router/LCD vars.
EOF
  fi

  upsert_dotenv_var "$indexer_env" "DATABASE_URL" "$DATABASE_URL"
  upsert_dotenv_var "$indexer_env" "TEST_DATABASE_URL" "$TEST_DATABASE_URL"

  echo "[setup-postgres] DATABASE_URL=${DATABASE_URL}"
  echo "[setup-postgres] TEST_DATABASE_URL=${TEST_DATABASE_URL}"
  echo "[setup-postgres] synced indexer/.env (DATABASE_URL + TEST_DATABASE_URL)"
  echo "[setup-postgres] integration tests: cd indexer && cargo test --tests -j 1 -- --test-threads=1"
}

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
  sync_indexer_database_env
  exit 0
fi

if ! postgres_bootstrap_app_role; then
  sync_indexer_database_env
  exit 0
fi

ensure_db "$POSTGRES_DB"
ensure_db "$POSTGRES_TEST_DB"
sync_indexer_database_env
