#!/usr/bin/env bash
# Bootstrap the app Postgres role via superuser when the stack only ships postgres:postgres.
# Sourced by scripts/setup-postgres-dev-databases.sh (GitLab #245 infra note).
# shellcheck shell=bash

# Returns 0 when psql can connect with the given credentials.
postgres_can_connect() {
  local user=$1 password=$2
  if command -v timeout >/dev/null 2>&1; then
    timeout 5 env PGPASSWORD="$password" psql -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$user" -d postgres -c '\q' \
      >/dev/null 2>&1
    return $?
  fi
  PGPASSWORD="$password" psql -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$user" -d postgres -c '\q' \
    >/dev/null 2>&1
}

# Ensure POSTGRES_USER exists and accepts POSTGRES_PASSWORD. Uses POSTGRES_SUPERUSER when
# the app role is missing or unreachable. Idempotent: never resets an existing role password.
postgres_bootstrap_app_role() {
  if postgres_can_connect "$POSTGRES_USER" "$POSTGRES_PASSWORD"; then
    return 0
  fi

  if [ "$POSTGRES_USER" = "$POSTGRES_SUPERUSER" ]; then
    echo "[setup-postgres] WARN: cannot connect as ${POSTGRES_USER}@${POSTGRES_HOST}:${POSTGRES_PORT}; skipping database ensure" >&2
    return 1
  fi

  if ! postgres_can_connect "$POSTGRES_SUPERUSER" "$POSTGRES_SUPERUSER_PASSWORD"; then
    echo "[setup-postgres] WARN: cannot connect as ${POSTGRES_USER} and superuser ${POSTGRES_SUPERUSER} is unavailable; skipping database ensure" >&2
    echo "[setup-postgres] WARN: stack prereq — create role ${POSTGRES_USER} (LOGIN) or set POSTGRES_SUPERUSER / POSTGRES_SUPERUSER_PASSWORD for bootstrap (see skills/AGENTS_LOCAL_POSTGRES_DEV.md)" >&2
    return 1
  fi

  local role_exists
  role_exists="$(
    PGPASSWORD="$POSTGRES_SUPERUSER_PASSWORD" psql -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" \
      -U "$POSTGRES_SUPERUSER" -d postgres -tAc \
      "SELECT 1 FROM pg_roles WHERE rolname='${POSTGRES_USER}'" 2>/dev/null || true
  )"

  if [ "$role_exists" = "1" ]; then
    echo "[setup-postgres] WARN: role ${POSTGRES_USER} exists but password auth failed; fix credentials or reset password as superuser" >&2
    return 1
  fi

  echo "[setup-postgres] bootstrapping role ${POSTGRES_USER} via superuser ${POSTGRES_SUPERUSER}..."
  local escaped_password="${POSTGRES_PASSWORD//\'/\'\'}"
  PGPASSWORD="$POSTGRES_SUPERUSER_PASSWORD" psql -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" \
    -U "$POSTGRES_SUPERUSER" -d postgres -v ON_ERROR_STOP=1 \
    -c "CREATE ROLE \"${POSTGRES_USER}\" WITH LOGIN CREATEDB PASSWORD '${escaped_password}';"

  if ! postgres_can_connect "$POSTGRES_USER" "$POSTGRES_PASSWORD"; then
    echo "[setup-postgres] WARN: created role ${POSTGRES_USER} but still cannot connect; skipping database ensure" >&2
    return 1
  fi

  echo "[setup-postgres] role ${POSTGRES_USER} ready"
  return 0
}
