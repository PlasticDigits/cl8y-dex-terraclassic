#!/usr/bin/env bash
# Local server/recovery launcher: remove inherited database capabilities.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT/indexer"
runtime_env="${INDEXER_RUNTIME_ENV:-$ROOT/indexer/.env}"
if [[ -f "$runtime_env" ]]; then
  set -a
  source "$runtime_env"
  set +a
fi
[[ $# -gt 0 ]] || set -- cargo run
exec env -u MIGRATION_DATABASE_URL -u MIGRATOR_DATABASE_URL -u DATABASE_ADMIN_URL \
  -u TEST_DATABASE_URL -u TEST_DATABASE_ADMIN_URL -u TEST_DATABASE_RUNTIME_PASSWORD \
  -u POSTGRES_PASSWORD -u POSTGRES_SUPERUSER_PASSWORD -u POSTGRES_MIGRATOR_PASSWORD \
  -u POSTGRES_QA_PASSWORD -u INDEXER_MIGRATION_BIN "$@"
