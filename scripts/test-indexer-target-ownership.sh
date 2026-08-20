#!/usr/bin/env bash
# Static regression: docker paths must not bind-mount indexer/ for cargo or sqlx as root.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

_fail() {
  echo "FAIL: $*" >&2
  exit 1
}

CHARTS="$REPO_ROOT/scripts/test-charts-integration.sh"
LIB="$REPO_ROOT/scripts/lib/docker-indexer-bind-mount.sh"

grep -q 'docker_sqlx_migrate_on_compose_network' "$CHARTS" \
  || _fail 'test-charts-integration.sh must call docker_sqlx_migrate_on_compose_network'
grep -q '/migrations:ro' "$LIB" \
  || _fail 'docker-indexer-bind-mount.sh must mount migrations read-only, not indexer/'
if grep -qE -- '-v .*indexer:/indexer' "$CHARTS"; then
  _fail 'test-charts-integration.sh must not bind-mount indexer/ (root-owned target/ lock files)'
fi
grep -q 'CARGO_TARGET_DIR=/tmp/target' "$LIB" \
  || _fail 'lib must document CARGO_TARGET_DIR=/tmp/target for unavoidable docker cargo'
grep -q 'root-owned' "$REPO_ROOT/AGENTS.md" \
  || _fail 'AGENTS.md must document root-owned indexer/target'
grep -qE 'Never bind-mount `indexer/' "$REPO_ROOT/AGENTS.md" \
  || _fail 'AGENTS.md must say never bind-mount indexer/ and run cargo'

echo "OK: indexer target ownership static checks"
