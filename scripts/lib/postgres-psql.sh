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
  postgres_docker_compose exec -T postgres pg_isready -U "${POSTGRES_SUPERUSER:-postgres}" >/dev/null 2>&1
}

# Initialize psql invocation mode. Returns 0 on success.
postgres_psql_init() {
  POSTGRES_PSQL_MODE="${POSTGRES_PSQL_MODE:-}"

  if command -v psql >/dev/null 2>&1; then
    POSTGRES_PSQL_MODE=host
    POSTGRES_PSQL_HOST="${POSTGRES_HOST:-127.0.0.1}"
    POSTGRES_PSQL_PORT="${POSTGRES_PORT:-5432}"
    return 0
  fi

  if ! command -v docker >/dev/null 2>&1; then
    echo "[setup-postgres] ERROR: psql not found and docker is unavailable" >&2
    echo "[setup-postgres]        install postgresql-client or start Docker + postgres compose service" >&2
    return 1
  fi

  if ! postgres_compose_postgres_ready; then
    echo "[setup-postgres] ERROR: psql not found and compose postgres is not ready" >&2
    echo "[setup-postgres]        run: docker compose up -d postgres  (or make setup-indexer-postgres)" >&2
    return 1
  fi

  POSTGRES_PSQL_MODE=compose
  POSTGRES_PSQL_HOST=localhost
  POSTGRES_PSQL_PORT=5432
  echo "[setup-postgres] using docker compose exec psql (host psql not installed)"
  return 0
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
