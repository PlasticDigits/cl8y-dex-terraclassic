#!/usr/bin/env bash
# Docker helpers that must not poison host `indexer/target` with root-owned Cargo files.
#
# NEVER bind-mount `indexer/` (or the repo) into a root container and run `cargo`.
# Default container uid is 0, so cargo writes `target/debug/.cargo-build-lock` as root
# and host `cargo test` / rust-analyzer fail with permission denied.
#
# Host TCP to published Postgres/LCD can hang (userland-proxy, VPN). Use
# `make setup-indexer-postgres`, `scripts/lib/postgres-psql.sh`, or
# `scripts/lib/localterra-host-curl.sh`. Do not move cargo into Docker to "reach" Postgres.
#
# If cargo in Docker is unavoidable:
#   docker run --rm --user "$(id -u):$(id -g)" \
#     -e CARGO_HOME=/tmp/cargo -e CARGO_TARGET_DIR=/tmp/target \
#     -v "$PWD/indexer:/work" -w /work rust:1.96-bookworm cargo test --lib
# Or overlay target with a named volume (see smartcontracts/scripts/optimize.sh).
#
# shellcheck shell=bash

docker_indexer_cmd() {
  if groups 2>/dev/null | grep -qw docker || [[ -w /var/run/docker.sock ]]; then
    docker "$@"
    return $?
  fi
  sg docker -c "docker $(printf '%q ' "$@")"
}

# sqlx migrate via the compose Postgres network without mounting indexer/ (so target/ stays host-owned).
docker_sqlx_migrate_on_compose_network() {
  local net="${1:?}"
  local db_url="${2:?}"
  local migrations_dir="${3:?}"
  local sqlx_bin
  sqlx_bin="$(command -v sqlx)" || return 1
  docker_indexer_cmd run --rm \
    --network "$net" \
    -v "${migrations_dir}:/migrations:ro" \
    -v "${sqlx_bin}:/usr/local/bin/sqlx:ro" \
    -w /tmp \
    -e DATABASE_URL="$db_url" \
    ubuntu:24.04 \
    bash -c 'apt-get update -qq && apt-get install -y -qq libssl3 ca-certificates >/dev/null && sqlx migrate run --source /migrations --no-dotenv'
}
