#!/usr/bin/env bash
# Live least-privilege regression on a disposable PostgreSQL cluster (#1396).
set -euo pipefail
umask 077
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
fail() { echo "FAIL: $*" >&2; exit 1; }
for tool in docker psql openssl; do
  command -v "$tool" >/dev/null || { echo "SKIP: role ACL test ($tool unavailable)"; exit 0; }
done
docker info >/dev/null 2>&1 || { echo 'SKIP: role ACL test (Docker daemon unavailable)'; exit 0; }
export POSTGRES_SUPERUSER=postgres POSTGRES_USER=dex_indexer_runtime
export POSTGRES_MIGRATOR_USER=dex_indexer_migrator POSTGRES_QA_USER=dex_indexer_qa
export POSTGRES_DB=dex_indexer POSTGRES_TEST_DB=dex_indexer_test POSTGRES_HOST=127.0.0.1
export POSTGRES_SUPERUSER_PASSWORD="$(openssl rand -hex 24)"
export POSTGRES_PASSWORD="$(openssl rand -hex 24)"
export POSTGRES_MIGRATOR_PASSWORD="$(openssl rand -hex 24)"
export POSTGRES_QA_PASSWORD="$(openssl rand -hex 24)"
# Prevent inherited DSNs directing this test toward another database.
unset DATABASE_URL MIGRATION_DATABASE_URL TEST_DATABASE_URL TEST_DATABASE_ADMIN_URL
CONTAINER=''
BACKUP="$(mktemp -d)"
export POSTGRES_CONFIG_FILE="$BACKUP/no-operator-env"
export INDEXER_DATABASE_ENV_DIR="$BACKUP/indexer"
cleanup() {
  rm -rf "$BACKUP"
  if [ -n "$CONTAINER" ]; then docker rm -f "$CONTAINER" >/dev/null 2>&1 || true; fi
}
trap cleanup EXIT
CONTAINER="$(docker run -d --rm --tmpfs /var/lib/postgresql/data:rw,size=256m -e POSTGRES_INITDB_ARGS=--no-sync -e POSTGRES_PASSWORD="$POSTGRES_SUPERUSER_PASSWORD" -p 127.0.0.1::5432 postgres:16-alpine)"
POSTGRES_PORT="$(docker port "$CONTAINER" 5432/tcp | sed 's/.*://')"
export POSTGRES_PORT
for ((i=0; i<30; i++)); do
  docker exec "$CONTAINER" pg_isready -U postgres >/dev/null 2>&1 && break
  sleep 1
done
docker exec "$CONTAINER" pg_isready -U postgres >/dev/null 2>&1 || { docker logs --tail 20 "$CONTAINER" >&2; fail 'disposable Postgres unavailable'; }
setup_log="$BACKUP/setup.log"
./scripts/setup-postgres-dev-databases.sh >"$setup_log" 2>&1 || { cat "$setup_log" >&2; fail 'initial setup'; }
./scripts/setup-postgres-dev-databases.sh >>"$setup_log" 2>&1 || { cat "$setup_log" >&2; fail 'idempotent setup'; }
for secret in "$POSTGRES_PASSWORD" "$POSTGRES_MIGRATOR_PASSWORD" "$POSTGRES_QA_PASSWORD" "$POSTGRES_SUPERUSER_PASSWORD"; do
  if rg -Fq "$secret" "$setup_log"; then fail 'setup logged a password'; fi
done
[ "$(stat -c %a "$INDEXER_DATABASE_ENV_DIR/.env")" = 600 ] && [ "$(stat -c %a "$INDEXER_DATABASE_ENV_DIR/.env.migrate")" = 600 ] && [ "$(stat -c %a "$INDEXER_DATABASE_ENV_DIR/.env.qa")" = 600 ] || fail 'private file modes'
! rg -q '^(TEST_DATABASE_URL|TEST_DATABASE_ADMIN_URL|MIGRATION_DATABASE_URL)=' "$INDEXER_DATABASE_ENV_DIR/.env" || fail 'privileged DSN in runtime env'
admin() { PGPASSWORD="$POSTGRES_SUPERUSER_PASSWORD" psql -X -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U postgres -v ON_ERROR_STOP=1 "$@"; }
runtime() { PGPASSWORD="$POSTGRES_PASSWORD" psql -X -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" -d dex_indexer -v ON_ERROR_STOP=1 "$@"; }
reject() { if runtime -c "$1" >/dev/null 2>&1; then fail "runtime unexpectedly allowed: $2"; fi; }
admin -d postgres -c 'CREATE DATABASE unrelated_database' >/dev/null
admin -d postgres -c 'REVOKE CONNECT ON DATABASE unrelated_database FROM PUBLIC' >/dev/null
admin -d dex_indexer >/dev/null <<'SQL'
SET ROLE dex_indexer_migrator;
CREATE TABLE role_acl_fixture (id bigserial PRIMARY KEY, value text);
CREATE TABLE _sqlx_migrations (version bigint PRIMARY KEY, checksum bytea);
CREATE TABLE indexer_schema_version (singleton boolean PRIMARY KEY, version bigint);
INSERT INTO indexer_schema_version VALUES (true, 106);
CREATE SCHEMA unrelated;
CREATE TABLE unrelated.secret (id int);
RESET ROLE;
SQL
./scripts/setup-postgres-dev-databases.sh >>"$setup_log" 2>&1 || fail 'existing-schema grants'
runtime >/dev/null <<'SQL'
INSERT INTO role_acl_fixture(value) VALUES ('ok');
UPDATE role_acl_fixture SET value='updated';
SELECT * FROM role_acl_fixture;
SELECT * FROM indexer_schema_version;
DELETE FROM role_acl_fixture;
SQL
reject 'CREATE TABLE forbidden(id int)' 'table DDL'
reject 'CREATE TEMP TABLE forbidden(id int)' 'temporary table DDL'
reject 'CREATE SCHEMA forbidden' 'schema DDL'
reject 'ALTER TABLE role_acl_fixture ADD COLUMN forbidden int' 'ALTER'
reject 'TRUNCATE role_acl_fixture' 'TRUNCATE'
reject 'CREATE ROLE forbidden' 'role creation'
reject 'ALTER ROLE dex_indexer_runtime SUPERUSER' 'role privilege escalation'
reject 'CREATE EXTENSION hstore' 'extension creation'
reject "COPY (SELECT 1) TO PROGRAM 'true'" 'server program execution'
reject "SELECT pg_read_file('/etc/passwd')" 'server file access'
reject 'DROP SCHEMA unrelated CASCADE' 'schema DROP'
reject 'CREATE DATABASE forbidden' 'database creation'
reject 'SET ROLE dex_indexer_migrator' 'owner role inheritance'
reject 'SELECT * FROM _sqlx_migrations' 'migration ledger reads'
reject 'INSERT INTO _sqlx_migrations VALUES (999, NULL)' 'migration ledger writes'
reject 'UPDATE indexer_schema_version SET version=999' 'schema readiness spoof'
reject 'SELECT * FROM unrelated.secret' 'unrelated schema access'
for db in dex_indexer_test postgres template1 unrelated_database; do
  if PGPASSWORD="$POSTGRES_PASSWORD" psql -X -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" -d "$db" -c 'SELECT 1' >/dev/null 2>&1; then fail "runtime connected to unrelated $db"; fi
done
# Owner credential remains separate and can apply migration-style DDL.
PGPASSWORD="$POSTGRES_MIGRATOR_PASSWORD" psql -X -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_MIGRATOR_USER" -d dex_indexer -v ON_ERROR_STOP=1 -c 'CREATE TABLE migration_fixture (id int)' >/dev/null
# Foreign public relations must fail before runtime receives table grants.
admin -d dex_indexer -c 'CREATE TABLE foreign_owner_fixture (id int)' >/dev/null
if ./scripts/setup-postgres-dev-databases.sh >"$BACKUP/foreign.log" 2>&1; then fail 'granted unrelated table access'; fi
reject 'SELECT * FROM foreign_owner_fixture' 'foreign owner table access'
admin -d dex_indexer -c 'DROP TABLE foreign_owner_fixture' >/dev/null
# Dedicated QA role can create a disposable DB, but cannot connect to runtime DB.
if PGPASSWORD="$POSTGRES_QA_PASSWORD" psql -X -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_QA_USER" -d dex_indexer -c 'SELECT 1' >/dev/null 2>&1; then fail 'QA reached runtime DB'; fi
PGPASSWORD="$POSTGRES_QA_PASSWORD" psql -X -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_QA_USER" -d dex_indexer_test -v ON_ERROR_STOP=1 -c 'CREATE DATABASE dex_indexer_test_disposable' >/dev/null
# Legacy ownership is a hard, reviewable failure; no role reset, migration edits or DB drop.
admin -d postgres -c 'CREATE DATABASE legacy_owner_db' >/dev/null
if POSTGRES_DB=legacy_owner_db ./scripts/setup-postgres-dev-databases.sh >"$BACKUP/legacy.log" 2>&1; then fail 'silently adopted legacy owner'; fi
[ "$(admin -d postgres -tAc "SELECT pg_get_userbyid(datdba) FROM pg_database WHERE datname='legacy_owner_db'")" = postgres ] || fail 'legacy owner mutated'
echo 'PASS: runtime DML, denied DDL/role escalation/extensions/programs/files/ledger/unrelated access, migration/QA separation, private env files, idempotency and legacy-owner fail-closed'
