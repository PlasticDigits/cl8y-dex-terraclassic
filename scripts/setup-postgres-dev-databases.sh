#!/usr/bin/env bash
# Provision local role separation without resetting data or adopting legacy ownership.
set -euo pipefail
umask 077
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
set -a
POSTGRES_CONFIG_FILE="${POSTGRES_CONFIG_FILE:-$REPO_ROOT/.env}"
if [ -f "$POSTGRES_CONFIG_FILE" ]; then source "$POSTGRES_CONFIG_FILE"; fi
INDEXER_DATABASE_ENV_DIR="${INDEXER_DATABASE_ENV_DIR:-$REPO_ROOT/indexer}"
source "$REPO_ROOT/scripts/lib/postgres-dev.env"
set +a
source "$REPO_ROOT/scripts/lib/postgres-bootstrap-role.sh"
source "$REPO_ROOT/scripts/lib/postgres-psql.sh"
source "$REPO_ROOT/scripts/lib/upsert-dotenv.sh"

for key in POSTGRES_USER POSTGRES_MIGRATOR_USER POSTGRES_QA_USER POSTGRES_SUPERUSER POSTGRES_DB POSTGRES_TEST_DB; do
  [[ "${!key}" =~ ^[a-z_][a-z0-9_]*$ ]] || { echo "[setup-postgres] ERROR: invalid identifier $key" >&2; exit 1; }
done
[[ "$POSTGRES_HOST" =~ ^[a-zA-Z0-9_.:-]+$ ]] && [[ "$POSTGRES_PORT" =~ ^[0-9]+$ ]] \
  || { echo '[setup-postgres] ERROR: invalid host or port' >&2; exit 1; }
[ "$POSTGRES_USER" != "$POSTGRES_MIGRATOR_USER" ] && [ "$POSTGRES_USER" != "$POSTGRES_QA_USER" ] && [ "$POSTGRES_MIGRATOR_USER" != "$POSTGRES_QA_USER" ] \
  || { echo '[setup-postgres] ERROR: runtime, migrator and QA roles must differ' >&2; exit 1; }
passwords_seen=()
for key in POSTGRES_PASSWORD POSTGRES_MIGRATOR_PASSWORD POSTGRES_QA_PASSWORD POSTGRES_SUPERUSER_PASSWORD; do
  case "${!key}" in ''|postgres|cl8y_legal|changeme) echo "[setup-postgres] ERROR: set a nondefault $key via private env or bao-exec" >&2; exit 1 ;; esac
  secret_value="${!key}"
  [[ ${#secret_value} -ge 16 ]] || { echo "[setup-postgres] ERROR: $key must have at least 16 characters" >&2; exit 1; }
  for previous in "${passwords_seen[@]}"; do
    [[ "$previous" != "$secret_value" ]] || { echo '[setup-postgres] ERROR: database capabilities require distinct passwords' >&2; exit 1; }
  done
  passwords_seen+=("$secret_value")
done
unset passwords_seen previous secret_value
postgres_psql_init
postgres_bootstrap_app_role
admin_psql() {
  PGPASSWORD="$POSTGRES_SUPERUSER_PASSWORD" postgres_psql -h "$POSTGRES_PSQL_HOST" -p "$POSTGRES_PSQL_PORT" \
    -U "$POSTGRES_SUPERUSER" -v ON_ERROR_STOP=1 "$@"
}
ensure_db() {
  local db="$1" owner="$2" user="$3"
  admin_psql -d postgres -v db="$db" -v owner="$owner" -v user="$user" <<'SQL'
SELECT format('CREATE DATABASE %I OWNER %I', :'db', :'owner')
WHERE NOT EXISTS (SELECT 1 FROM pg_database WHERE datname = :'db') \gexec
SELECT pg_get_userbyid(datdba) = :'owner' AS owned FROM pg_database WHERE datname = :'db' \gset
\if :owned
\else
  \echo 'ERROR: existing database has a legacy owner; reviewed ownership transition required, no automatic adoption'
  DO $$ BEGIN RAISE EXCEPTION 'unsafe database provisioning refused'; END $$;
\endif
SELECT format('REVOKE ALL ON DATABASE %I FROM PUBLIC', :'db') \gexec
SELECT format('GRANT CONNECT ON DATABASE %I TO %I', :'db', :'user') \gexec
SQL
}
ensure_db "$POSTGRES_DB" "$POSTGRES_MIGRATOR_USER" "$POSTGRES_USER"
# QA login owns only its schema; administrator owns and authenticates the QA database.
ensure_db "$POSTGRES_TEST_DB" "$POSTGRES_SUPERUSER" "$POSTGRES_QA_USER"
for role in "$POSTGRES_USER" "$POSTGRES_MIGRATOR_USER" "$POSTGRES_QA_USER"; do
  case "$role" in
    "$POSTGRES_USER") password="$POSTGRES_PASSWORD"; database="$POSTGRES_DB" ;;
    "$POSTGRES_MIGRATOR_USER") password="$POSTGRES_MIGRATOR_PASSWORD"; database="$POSTGRES_DB" ;;
    "$POSTGRES_QA_USER") password="$POSTGRES_QA_PASSWORD"; database="$POSTGRES_TEST_DB" ;;
  esac
  if ! PGPASSWORD="$password" postgres_psql -h "$POSTGRES_PSQL_HOST" -p "$POSTGRES_PSQL_PORT" -U "$role" -d "$database" -c '\q' >/dev/null 2>&1; then
    echo "[setup-postgres] ERROR: credentials for $role do not authenticate; no existing password was reset" >&2
    exit 1
  fi
done
admin_psql -d "$POSTGRES_DB" -v runtime="$POSTGRES_USER" -v migrator="$POSTGRES_MIGRATOR_USER" <<'SQL'
BEGIN;
SELECT NOT EXISTS (
  SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
  WHERE n.nspname = 'public' AND c.relkind IN ('r','p','v','m','S','f')
    AND pg_get_userbyid(c.relowner) <> :'migrator'
) AS owned \gset
\if :owned
\else
  \echo 'ERROR: public relations have another owner; reviewed ownership transition required'
  DO $$ BEGIN RAISE EXCEPTION 'unsafe database provisioning refused'; END $$;
\endif
REVOKE ALL ON SCHEMA public FROM PUBLIC;
SELECT format('ALTER SCHEMA public OWNER TO %I', :'migrator') \gexec
SELECT format('REVOKE ALL ON SCHEMA public FROM %I', :'runtime') \gexec
SELECT format('GRANT USAGE ON SCHEMA public TO %I', :'runtime') \gexec
REVOKE ALL ON ALL TABLES IN SCHEMA public FROM PUBLIC;
SELECT format('GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO %I', :'runtime') \gexec
SELECT format('GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO %I', :'runtime') \gexec
SELECT format('REVOKE ALL ON TABLE public._sqlx_migrations FROM %I', :'runtime') WHERE to_regclass('public._sqlx_migrations') IS NOT NULL \gexec
SELECT format('REVOKE ALL ON TABLE public.indexer_schema_version FROM %I', :'runtime') WHERE to_regclass('public.indexer_schema_version') IS NOT NULL \gexec
SELECT format('GRANT SELECT ON TABLE public.indexer_schema_version TO %I', :'runtime') WHERE to_regclass('public.indexer_schema_version') IS NOT NULL \gexec
COMMIT;
SQL
admin_psql -d "$POSTGRES_TEST_DB" -v qa="$POSTGRES_QA_USER" -v db="$POSTGRES_TEST_DB" <<'SQL'
REVOKE ALL ON SCHEMA public FROM PUBLIC;
SELECT format('ALTER SCHEMA public OWNER TO %I', :'qa') \gexec
SELECT format('COMMENT ON DATABASE %I IS %L', :'db', 'cl8y-dex:isolated-qa:v1') \gexec
SQL
# Control databases must not expose implicit PUBLIC connection to runtime.
admin_psql -d postgres <<'SQL'
REVOKE CONNECT ON DATABASE postgres FROM PUBLIC;
REVOKE CONNECT ON DATABASE template1 FROM PUBLIC;
SQL
# PostgreSQL has no per-role deny ACL. Refuse implicit PUBLIC access to another DB;
# changing an unrelated database's PUBLIC ACL requires a reviewed administrator action.
admin_psql -d postgres -v runtime="$POSTGRES_USER" -v db="$POSTGRES_DB" <<'SQL'
SELECT NOT EXISTS (
  SELECT 1 FROM pg_database WHERE NOT datistemplate AND datallowconn AND datname <> :'db'
  AND has_database_privilege(:'runtime', oid, 'CONNECT')
) AS isolated \gset
\if :isolated
\else
  \echo 'ERROR: runtime has CONNECT to an unrelated database; administrator must review PUBLIC/explicit ACLs'
  DO $$ BEGIN RAISE EXCEPTION 'unsafe database provisioning refused'; END $$;
\endif
SQL
mkdir -p "$INDEXER_DATABASE_ENV_DIR"
touch "$INDEXER_DATABASE_ENV_DIR/.env"
upsert_dotenv_var "$INDEXER_DATABASE_ENV_DIR/.env" DATABASE_URL "$DATABASE_URL"
# Remove privileged values from old combined files before writing scoped files.
sed -i '/^\(TEST_DATABASE_URL\|TEST_DATABASE_ADMIN_URL\|MIGRATION_DATABASE_URL\)=/d' "$INDEXER_DATABASE_ENV_DIR/.env"
printf 'DATABASE_URL="%s"\nINDEXER_RUNTIME_ROLE=%s\n' "$MIGRATION_DATABASE_URL" "$POSTGRES_USER" > "$INDEXER_DATABASE_ENV_DIR/.env.migrate"
printf 'TEST_DATABASE_URL="%s"\nTEST_DATABASE_ADMIN_URL="%s"\nQA_DATABASE_NAME=%s\nRUN_MODE=qa\n' "$TEST_DATABASE_URL" "$TEST_DATABASE_URL" "$POSTGRES_TEST_DB" > "$INDEXER_DATABASE_ENV_DIR/.env.qa"
chmod 600 "$INDEXER_DATABASE_ENV_DIR/.env" "$INDEXER_DATABASE_ENV_DIR/.env.migrate" "$INDEXER_DATABASE_ENV_DIR/.env.qa"
echo '[setup-postgres] provisioned runtime, migrator and isolated QA roles; wrote private scoped env files (credentials redacted)'
