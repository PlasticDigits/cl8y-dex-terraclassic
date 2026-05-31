#!/usr/bin/env bash
# Semi-automated indexer recovery after reorg or cursor reset (GitLab #236).
#
# Usage:
#   ./scripts/indexer-reorg-recover.sh --height 1234567 [--dry-run]
#
# Stops at SQL preview unless --apply is passed. Always review derived-table impact
# before rewinding — see docs/runbooks/indexer-reorg-replay-dedup.md.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/postgres-dev.env
source "${REPO_ROOT}/scripts/lib/postgres-dev.env"

HEIGHT=""
DRY_RUN=1

usage() {
  cat <<EOF
Usage: $0 --height HEIGHT [--apply]

  --height HEIGHT   Rewind last_indexed_height to HEIGHT-1 (next index = HEIGHT).
  --apply           Execute SQL (default is dry-run preview only).
  -h, --help        Show this help.

Environment: DATABASE_URL or postgres-dev defaults (see scripts/lib/postgres-dev.env).
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --height)
      HEIGHT="${2:-}"
      shift 2
      ;;
    --apply)
      DRY_RUN=0
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ -z "$HEIGHT" ]] || ! [[ "$HEIGHT" =~ ^[0-9]+$ ]] || [[ "$HEIGHT" -lt 1 ]]; then
  echo "ERROR: --height must be a positive integer (fork replay start height)." >&2
  exit 1
fi

PREV=$((HEIGHT - 1))
DB_URL="${DATABASE_URL:-postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@127.0.0.1:5432/${POSTGRES_DB}}"

echo "=== Indexer reorg recovery (GitLab #236) ==="
echo "DATABASE_URL: ${DB_URL%%@*}@***"
echo "Will set last_indexed_height -> ${PREV} (next block to index: ${HEIGHT})"
echo "Will clear last_indexed_block_hash (reorg check skipped until re-indexed)"
echo "Will TRUNCATE indexer_failed_blocks"
echo ""
echo "WARNING: Candles, positions, and trader aggregates for heights >= ${HEIGHT}"
echo "         may be inconsistent until you delete affected rows or full re-backfill."
echo "         See docs/runbooks/indexer-reorg-replay-dedup.md"
echo ""

SQL="
BEGIN;
UPDATE indexer_state SET value = '${PREV}', updated_at = NOW() WHERE key = 'last_indexed_height';
UPDATE indexer_state SET value = '', updated_at = NOW() WHERE key = 'last_indexed_block_hash';
TRUNCATE indexer_failed_blocks;
COMMIT;
"

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "--- DRY RUN (pass --apply to execute) ---"
  echo "$SQL"
  echo ""
  echo "After apply: stop indexer, run this script with --apply, restart indexer."
  exit 0
fi

echo "--- Applying cursor reset ---"
psql "$DB_URL" -v ON_ERROR_STOP=1 -c "$SQL"
echo "Done. Restart the indexer to replay from height ${HEIGHT}."
