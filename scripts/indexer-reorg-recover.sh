#!/usr/bin/env bash
# Semi-automated indexer recovery after reorg or cursor reset (GitLab #236, #362).
#
# Usage:
#   ./scripts/indexer-reorg-recover.sh --height 1234567          # dry-run (default)
#   ./scripts/indexer-reorg-recover.sh --height 1234567 --apply  # execute cursor reset
#
# Stops at SQL preview unless --apply is passed. Always review derived-table impact
# before rewinding — see docs/runbooks/indexer-reorg-replay-dedup.md.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/postgres-dev.env
source "${REPO_ROOT}/scripts/lib/postgres-dev.env"
# shellcheck source=scripts/lib/postgres-psql.sh
source "${REPO_ROOT}/scripts/lib/postgres-psql.sh"

HEIGHT=""
DRY_RUN=1

usage() {
  cat <<EOF
Usage: $0 --height HEIGHT [--apply]

  --height HEIGHT   Rewind last_indexed_height to HEIGHT-1 (next index = HEIGHT).
  --apply           Execute SQL (default is dry-run preview only).
  -h, --help        Show this help.

Environment: DATABASE_URL or postgres-dev defaults (see scripts/lib/postgres-dev.env).
             Optional REORG_ALERT_WEBHOOK_URL on the indexer for halt paging (GitLab #362).

Also: make indexer-reorg-recover HEIGHT=<H> [APPLY=1]
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

echo "=== Indexer reorg recovery (GitLab #236 / #362) ==="
echo "DATABASE_URL: ${DB_URL%%@*}@***"
echo "Will set last_indexed_height -> ${PREV} (next block to index: ${HEIGHT})"
echo "Will clear last_indexed_block_hash (reorg check skipped until re-indexed)"
echo "Will TRUNCATE indexer_failed_blocks"
echo ""
echo "WARNING: Candles (ohlcv_candles) and trader aggregates are time-based;"
echo "         rows for heights >= ${HEIGHT} may leave derived state inconsistent."
echo "         See docs/runbooks/indexer-reorg-replay-dedup.md"
echo ""

preview_row_impact() {
  if ! postgres_psql_init; then
    echo "--- Row impact preview: SKIPPED (psql unavailable) ---"
    return 0
  fi

  local psql_args=()
  if [[ "${POSTGRES_PSQL_MODE:-host}" == compose ]]; then
    psql_args=(-U "${POSTGRES_USER}" -d "${POSTGRES_DB}")
    export PGPASSWORD="${POSTGRES_PASSWORD}"
  else
    psql_args=("$DB_URL")
  fi

  echo "--- Row impact preview (block_height >= ${HEIGHT}; swaps replay safely via ON CONFLICT) ---"
  local tables=(
    "swap_events"
    "liquidity_events"
    "limit_order_fills"
    "hook_events"
  )
  for table in "${tables[@]}"; do
    local count
    count="$(postgres_psql "${psql_args[@]}" -tAc \
      "SELECT COUNT(*) FROM ${table} WHERE block_height >= ${HEIGHT};" 2>/dev/null | tr -d '[:space:]' || true)"
    if [[ -n "$count" ]]; then
      echo "  ${table}: ${count} row(s)"
    else
      echo "  ${table}: (query failed or table missing)"
    fi
  done
  echo ""
  echo "  ohlcv_candles: time-bucketed — review manually if swaps were deleted or reorg was deep"
  echo "  traders / token_volume_stats: aggregate tables — may need refresh after deep reorg"
  echo ""
}

preview_row_impact

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
if postgres_psql_init && [[ "${POSTGRES_PSQL_MODE:-host}" == compose ]]; then
  export PGPASSWORD="${POSTGRES_PASSWORD}"
  postgres_psql -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" -v ON_ERROR_STOP=1 -c "$SQL"
else
  psql "$DB_URL" -v ON_ERROR_STOP=1 -c "$SQL"
fi
echo "Done. Restart the indexer to replay from height ${HEIGHT}."
