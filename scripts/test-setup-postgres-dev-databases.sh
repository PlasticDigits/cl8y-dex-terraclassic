#!/usr/bin/env bash
# Unit + optional live Docker checks for setup-postgres role bootstrap (GitLab #245).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

_fail() {
  echo "FAIL: $*" >&2
  exit 1
}

_assert_file_contains() {
  local file=$1 pattern=$2 msg=$3
  grep -q "$pattern" "$file" || _fail "$msg"
}

_assert_file_contains "$REPO_ROOT/scripts/setup-postgres-dev-databases.sh" \
  'postgres-bootstrap-role.sh' \
  'setup-postgres must source postgres-bootstrap-role.sh'

_assert_file_contains "$REPO_ROOT/scripts/lib/postgres-bootstrap-role.sh" \
  'postgres_bootstrap_app_role' \
  'bootstrap helper must define postgres_bootstrap_app_role'

_assert_file_contains "$REPO_ROOT/scripts/lib/postgres-dev.env" \
  'POSTGRES_SUPERUSER' \
  'postgres-dev.env must define POSTGRES_SUPERUSER'

_assert_file_contains "$REPO_ROOT/skills/AGENTS_LOCAL_POSTGRES_DEV.md" \
  '#245' \
  'AGENTS_LOCAL_POSTGRES_DEV must cross-link GitLab #245'

_assert_file_contains "$REPO_ROOT/docs/testing.md" \
  'cl8y_legal' \
  'docs/testing.md must document cl8y_legal stack prereq'

echo "OK: setup-postgres static checks"

if [ "${SKIP_LIVE_POSTGRES_BOOTSTRAP_TEST:-0}" = "1" ]; then
  echo "SKIP: live postgres bootstrap test (SKIP_LIVE_POSTGRES_BOOTSTRAP_TEST=1)"
  exit 0
fi

if ! command -v docker >/dev/null 2>&1 || ! command -v psql >/dev/null 2>&1 || ! command -v timeout >/dev/null 2>&1; then
  echo "SKIP: live postgres bootstrap test (docker, psql, or timeout unavailable)"
  exit 0
fi

LIVE_PORT="${POSTGRES_BOOTSTRAP_TEST_PORT:-55432}"

CONTAINER=""
cleanup() {
  if [ -n "$CONTAINER" ]; then
    docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

if ss -ltn 2>/dev/null | grep -q ":${LIVE_PORT} "; then
  echo "SKIP: live postgres bootstrap test (port ${LIVE_PORT} in use)"
  exit 0
fi

CONTAINER="$(docker run -d --rm -e POSTGRES_PASSWORD=postgres -p "127.0.0.1:${LIVE_PORT}:5432" postgres:16-alpine)"

for _ in $(seq 1 30); do
  if docker exec "$CONTAINER" pg_isready -U postgres >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
docker exec "$CONTAINER" pg_isready -U postgres >/dev/null 2>&1 \
  || _fail "temp postgres did not become ready"

INDEXER_ENV_BACKUP=""
if [ -f "$REPO_ROOT/indexer/.env" ]; then
  INDEXER_ENV_BACKUP="$(mktemp)"
  cp "$REPO_ROOT/indexer/.env" "$INDEXER_ENV_BACKUP"
fi

restore_indexer_env() {
  if [ -n "$INDEXER_ENV_BACKUP" ] && [ -f "$INDEXER_ENV_BACKUP" ]; then
    cp "$INDEXER_ENV_BACKUP" "$REPO_ROOT/indexer/.env"
    rm -f "$INDEXER_ENV_BACKUP"
  fi
}
trap 'restore_indexer_env; cleanup' EXIT

# Shared network namespace: psql to 127.0.0.1:5432 inside the sidecar reaches the temp Postgres.
setup_log="$(mktemp)"
docker run --rm --network "container:${CONTAINER}" \
  -v "$REPO_ROOT:/repo:rw" \
  -w /repo \
  -e POSTGRES_HOST=127.0.0.1 \
  -e POSTGRES_PORT=5432 \
  -e POSTGRES_USER=cl8y_legal \
  -e POSTGRES_PASSWORD=cl8y_legal \
  -e POSTGRES_SUPERUSER=postgres \
  -e POSTGRES_SUPERUSER_PASSWORD=postgres \
  -e POSTGRES_DB=dex_indexer_bootstrap_test \
  -e POSTGRES_TEST_DB=dex_indexer_bootstrap_test2 \
  postgres:16-alpine \
  sh -c 'apk add --no-cache bash >/dev/null && ./scripts/setup-postgres-dev-databases.sh' \
  >"$setup_log" 2>&1 || true

grep -q 'bootstrapping role cl8y_legal' "$setup_log" \
  || { cat "$setup_log" >&2; _fail 'expected bootstrap log line'; }
rm -f "$setup_log"

docker exec -e PGPASSWORD=cl8y_legal "$CONTAINER" psql -U cl8y_legal -d postgres -c '\q' >/dev/null 2>&1 \
  || _fail 'cl8y_legal login failed after bootstrap'

exists="$(
  docker exec -e PGPASSWORD=cl8y_legal "$CONTAINER" psql -U cl8y_legal -d postgres -tAc \
    "SELECT 1 FROM pg_database WHERE datname='dex_indexer_bootstrap_test'" 2>/dev/null || true
)"
[ "$exists" = "1" ] || _fail 'dex_indexer_bootstrap_test was not created'

echo "OK: live postgres bootstrap via superuser"
