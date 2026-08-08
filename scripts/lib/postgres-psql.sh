#!/usr/bin/env bash
# Host psql with docker compose exec fallback (Cloud Agent VMs often lack postgresql-client).
# Sourced after scripts/lib/postgres-dev.env. Sets POSTGRES_PSQL_MODE=host|compose.
# shellcheck shell=bash

# Run docker compose (sg docker when the shell is not in the docker group).
postgres_docker_compose() {
  if groups 2>/dev/null | grep -qw docker; then
    docker compose "$@"
  else
    sg docker -c "docker compose $(printf '%q ' "$@")"
  fi
}

# Returns 0 when the compose postgres service is running and accepting connections.
postgres_compose_postgres_ready() {
  postgres_docker_compose ps -q postgres 2>/dev/null | grep -q . || return 1
  postgres_docker_compose exec -T postgres pg_isready -U "${POSTGRES_USER:-cl8y_legal}" >/dev/null 2>&1
}

# Probe host TCP psql quickly. Linux userland-proxy can hang host clients even when
# compose exec works (same class of quirk as LCD host curl — prefer compose fallback).
postgres_host_psql_reachable() {
  local host="${1:-127.0.0.1}"
  local port="${2:-5432}"
  local user="${3:-cl8y_legal}"
  local password="${4:-}"
  local db="${5:-postgres}"
  if ! command -v psql >/dev/null 2>&1; then
    return 1
  fi
  # timeout avoids indefinite hang on broken userland-proxy; 3s is enough for local docker.
  if command -v timeout >/dev/null 2>&1; then
    PGPASSWORD="$password" timeout 3s psql -h "$host" -p "$port" -U "$user" -d "$db" -c '\q' >/dev/null 2>&1
  else
    PGPASSWORD="$password" psql -h "$host" -p "$port" -U "$user" -d "$db" -c '\q' >/dev/null 2>&1
  fi
}

# Initialize psql invocation mode. Returns 0 on success.
postgres_psql_init() {
  POSTGRES_PSQL_MODE="${POSTGRES_PSQL_MODE:-}"
  local host="${POSTGRES_HOST:-127.0.0.1}"
  local port="${POSTGRES_PORT:-5432}"

  if [ -n "$POSTGRES_PSQL_MODE" ]; then
    POSTGRES_PSQL_HOST="${POSTGRES_PSQL_HOST:-$host}"
    POSTGRES_PSQL_PORT="${POSTGRES_PSQL_PORT:-$port}"
    return 0
  fi

  if command -v psql >/dev/null 2>&1; then
    # Probe app role, then superuser (bootstrap containers often have only postgres until
    # postgres_bootstrap_app_role runs), then fall through to compose when both fail.
    if postgres_host_psql_reachable "$host" "$port" "${POSTGRES_USER:-cl8y_legal}" "${POSTGRES_PASSWORD:-}" postgres \
      || postgres_host_psql_reachable "$host" "$port" "${POSTGRES_USER:-cl8y_legal}" "${POSTGRES_PASSWORD:-}" "${POSTGRES_DB:-dex_indexer}" \
      || postgres_host_psql_reachable "$host" "$port" "${POSTGRES_SUPERUSER:-postgres}" "${POSTGRES_SUPERUSER_PASSWORD:-postgres}" postgres; then
      POSTGRES_PSQL_MODE=host
      POSTGRES_PSQL_HOST="$host"
      POSTGRES_PSQL_PORT="$port"
      return 0
    fi
    echo "[setup-postgres] WARN: host psql cannot reach ${host}:${port}; trying docker compose exec" >&2
  fi

  if command -v docker >/dev/null 2>&1 && postgres_compose_postgres_ready; then
    POSTGRES_PSQL_MODE=compose
    POSTGRES_PSQL_HOST=localhost
    POSTGRES_PSQL_PORT=5432
    echo "[setup-postgres] using docker compose exec psql (host TCP unreachable or psql missing)"
    return 0
  fi

  # Last resort: prefer host psql when present (role bootstrap / sidecar tests without docker CLI).
  if command -v psql >/dev/null 2>&1; then
    POSTGRES_PSQL_MODE=host
    POSTGRES_PSQL_HOST="$host"
    POSTGRES_PSQL_PORT="$port"
    echo "[setup-postgres] WARN: using host psql without successful probe (compose unavailable)" >&2
    return 0
  fi

  echo "[setup-postgres] ERROR: psql host unreachable and docker compose postgres is not ready" >&2
  echo "[setup-postgres]        install postgresql-client or start Docker + postgres compose service" >&2
  return 1
}

# Invoke psql against local dev Postgres (host TCP or compose exec).
postgres_psql() {
  local mode="${POSTGRES_PSQL_MODE:-host}"
  if [ "$mode" = compose ]; then
    postgres_docker_compose exec -T -e PGPASSWORD="${PGPASSWORD:-}" postgres psql "$@"
  else
    psql "$@"
  fi
}
