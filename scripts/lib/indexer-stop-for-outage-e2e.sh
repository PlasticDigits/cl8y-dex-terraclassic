#!/usr/bin/env bash
# Stop all listeners on the indexer HTTP port before outage Playwright (GitLab #219).
# Idempotent; safe to call from global-setup after test-e2e-indexer-outage.sh sanity stop.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=scripts/lib/indexer-port.sh
source "$REPO_ROOT/scripts/lib/indexer-port.sh"

INDEXER_URL="${VITE_INDEXER_URL:-http://127.0.0.1:3001}"
INDEXER_URL="${INDEXER_URL%/}"
INDEXER_PORT="$(indexer_port_from_url "$INDEXER_URL")"

indexer_stop_port_listeners "$INDEXER_PORT"
indexer_clear_stale_qa_pidfile_if_dead "$REPO_ROOT" "$INDEXER_PORT"

for _ in $(seq 1 30); do
  if ! curl -sf "${INDEXER_URL}/api/v1/overview" >/dev/null 2>&1; then
    echo "[indexer-stop-for-outage-e2e] indexer unreachable at ${INDEXER_URL}"
    exit 0
  fi
  sleep 1
done

echo "ERROR: indexer still up at ${INDEXER_URL} before outage E2E" >&2
exit 1
