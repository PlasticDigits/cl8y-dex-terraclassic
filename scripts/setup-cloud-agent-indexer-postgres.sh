#!/usr/bin/env bash
# Lightweight Postgres + indexer/.env for Cursor Cloud Agent indexer integration tests.
# No LocalTerra, wasm build, or deploy (GitLab #335 follow-up MR !91 / #324).
#
# Usage (repo root):
#   ./scripts/setup-cloud-agent-indexer-postgres.sh
#   make setup-indexer-postgres
#
# After success:
#   make test-indexer-integration
#   make verify-issue-324
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/lib/cloud-agent-docker.sh
source "$REPO_ROOT/scripts/lib/cloud-agent-docker.sh"

echo "[setup-indexer-postgres] repo: $REPO_ROOT"
cloud_agent_ensure_dockerd

echo "[setup-indexer-postgres] starting postgres service (LocalTerra not started)…"
cloud_agent_docker_compose up -d postgres

echo "[setup-indexer-postgres] waiting for Postgres…"
for i in $(seq 1 30); do
  if command -v pg_isready >/dev/null 2>&1 \
    && pg_isready -h "${POSTGRES_HOST:-127.0.0.1}" -U "${POSTGRES_USER:-cl8y_legal}" >/dev/null 2>&1; then
    break
  fi
  if cloud_agent_docker_compose exec -T postgres pg_isready -U "${POSTGRES_USER:-cl8y_legal}" >/dev/null 2>&1; then
    break
  fi
  if [ "$i" -eq 30 ]; then
    echo "[setup-indexer-postgres] ERROR: Postgres did not become ready in time" >&2
    exit 1
  fi
  sleep 2
done

chmod +x scripts/setup-postgres-dev-databases.sh scripts/lib/upsert-dotenv.sh \
  scripts/lib/postgres-psql.sh scripts/lib/postgres-bootstrap-role.sh
./scripts/setup-postgres-dev-databases.sh

echo ""
echo "[setup-indexer-postgres] OK — indexer/.env has DATABASE_URL + TEST_DATABASE_URL"
echo "  Serialized integration tests:"
echo "    make test-indexer-integration"
echo "  #324 cache tier integration (MR !91 gap):"
echo "    make verify-issue-324"
echo "  Single binary:"
echo "    cd indexer && cargo test --test api_route_solve -- --test-threads=1"
