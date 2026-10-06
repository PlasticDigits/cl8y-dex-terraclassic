#!/usr/bin/env bash
# Explicit administrator bootstrap; never promote or reset an existing role.
# shellcheck shell=bash
postgres_can_connect() {
  local user=$1 password=$2
  PGPASSWORD="$password" postgres_psql -h "$POSTGRES_PSQL_HOST" -p "$POSTGRES_PSQL_PORT" \
    -U "$user" -d postgres -c '\q' >/dev/null 2>&1
}

postgres_bootstrap_app_role() {
  if ! postgres_can_connect "$POSTGRES_SUPERUSER" "$POSTGRES_SUPERUSER_PASSWORD"; then
    echo '[setup-postgres] ERROR: explicit administrator credentials are required' >&2
    return 1
  fi
  local role password createdb
  for role in "$POSTGRES_USER" "$POSTGRES_MIGRATOR_USER" "$POSTGRES_QA_USER"; do
    case "$role" in
      "$POSTGRES_USER") password="$POSTGRES_PASSWORD" ;;
      "$POSTGRES_MIGRATOR_USER") password="$POSTGRES_MIGRATOR_PASSWORD" ;;
      "$POSTGRES_QA_USER") password="$POSTGRES_QA_PASSWORD" ;;
    esac
    createdb=false
    [ "$role" != "$POSTGRES_QA_USER" ] || createdb=true
    PGPASSWORD="$POSTGRES_SUPERUSER_PASSWORD" postgres_psql -h "$POSTGRES_PSQL_HOST" -p "$POSTGRES_PSQL_PORT" \
      -U "$POSTGRES_SUPERUSER" -d postgres -v ON_ERROR_STOP=1 -v role="$role" -v password="$password" -v createdb="$createdb" <<'SQL'
SELECT format('CREATE ROLE %I LOGIN NOINHERIT NOSUPERUSER %s NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD %L', :'role', CASE WHEN :'createdb'::boolean THEN 'CREATEDB' ELSE 'NOCREATEDB' END, :'password')
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = :'role') \gexec
SELECT NOT (rolsuper OR rolcreaterole OR rolreplication OR rolbypassrls) AND rolcreatedb = :'createdb'::boolean
       AND NOT EXISTS (SELECT 1 FROM pg_auth_members WHERE member = pg_roles.oid) AS safe
FROM pg_roles WHERE rolname = :'role' \gset
\if :safe
\else
  \echo 'ERROR: existing role is privileged or has inherited memberships; explicit reviewed rotation required'
  DO $$ BEGIN RAISE EXCEPTION 'unsafe database provisioning refused'; END $$;
\endif
SQL
  done
}
