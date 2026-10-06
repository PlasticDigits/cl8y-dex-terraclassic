#!/usr/bin/env bash
# One-shot local migrations: privileged credentials never enter the server env.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MIGRATION_ENV="${INDEXER_MIGRATION_ENV:-$ROOT/indexer/.env.migrate}"
if [[ ! -f "$MIGRATION_ENV" ]]; then
  echo 'Missing isolated migration env; run scripts/setup-postgres-dev-databases.sh first.' >&2
  exit 1
fi
(
  # This file is private local configuration, not a server dotenv file.
  source "$MIGRATION_ENV"
  : "${DATABASE_URL:?Migration env must contain DATABASE_URL}"
  cd "$ROOT/indexer"
  command=(cargo run --locked -- migrate)
  if [[ -n "${INDEXER_MIGRATION_BIN:-}" ]]; then
    command=("$INDEXER_MIGRATION_BIN" migrate)
  fi
  env -i PATH="$PATH" HOME="$HOME" DATABASE_URL="$DATABASE_URL" \
    INDEXER_RUNTIME_ROLE="${INDEXER_RUNTIME_ROLE:-dex_indexer_runtime}" \
    "${command[@]}"
)
