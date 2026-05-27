#!/usr/bin/env bash
# Full QA bring-up with empty LocalTerra + Postgres volumes (alias for QA_FRESH_VOLUMES=1 start-qa).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export QA_FRESH_VOLUMES=1
exec "$REPO_ROOT/scripts/qa/start-qa.sh"
