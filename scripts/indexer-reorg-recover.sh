#!/usr/bin/env bash
# Semi-automated indexer recovery after reorg or cursor reset (GitLab #236, #362).
#
# Usage:
#   ./scripts/indexer-reorg-recover.sh --height 1234567              # dry-run preview
#   ./scripts/indexer-reorg-recover.sh --height 1234567 --apply
#   ./scripts/indexer-reorg-recover.sh --height 1234567 --cleanup-derived --apply
#
# Stops at SQL preview unless --apply is passed. Always review derived-table impact
# before rewinding — see docs/runbooks/indexer-reorg-replay-dedup.md.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/postgres-dev.env
source "${REPO_ROOT}/scripts/lib/postgres-dev.env"
# shellcheck source=scripts/lib/postgres-psql.sh
source "${REPO_ROOT}/scripts/lib/postgres-psql.sh"
cd "${REPO_ROOT}"
export PGPASSWORD="${POSTGRES_PASSWORD}"
postgres_psql_init || exit 1

HEIGHT=""
DRY_RUN=1
CLEANUP_DERIVED=0

usage() {
  cat <<EOF
Usage: $0 --height HEIGHT [--cleanup-derived] [--apply]

  --height HEIGHT       Rewind last_indexed_height to HEIGHT-1 (next index = HEIGHT).
  --cleanup-derived     Delete height-indexed rows at block_height >= HEIGHT before cursor reset.
  --apply               Execute SQL (default is dry-run preview only).
  -h, --help            Show this help.

Environment: DATABASE_URL or postgres-dev defaults (see scripts/lib/postgres-dev.env).
             REORG_ALERT_WEBHOOK_URL — optional indexer halt webhook (see runbook).
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
    --cleanup-derived)
      CLEANUP_DERIVED=1
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
DB_URL="${DATABASE_URL:-postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@${POSTGRES_PSQL_HOST:-127.0.0.1}:${POSTGRES_PSQL_PORT:-5432}/${POSTGRES_DB}}"
# Honour DATABASE_URL database name when using compose exec (Cloud Agent has no host psql).
DB_NAME="${POSTGRES_DB}"
if [[ -n "${DATABASE_URL:-}" ]]; then
  DB_NAME="${DATABASE_URL##*/}"
  DB_NAME="${DB_NAME%%\?*}"
fi

psql_query() {
  if [ "${POSTGRES_PSQL_MODE}" = compose ]; then
    postgres_psql -U "${POSTGRES_USER}" -d "${DB_NAME}" -v ON_ERROR_STOP=1 -At "$@"
  else
    psql "${DB_URL}" -v ON_ERROR_STOP=1 -At "$@"
  fi
}

echo "=== Indexer reorg recovery (GitLab #236 / #362) ==="
echo "DATABASE_URL: ${DB_URL%%@*}@***"
echo "Fork replay start height: ${HEIGHT} (cursor -> ${PREV})"
echo ""

CURRENT_HEIGHT="$(psql_query -c "SELECT COALESCE(value, '0') FROM indexer_state WHERE key = 'last_indexed_height';" 2>/dev/null || echo "?")"
CURRENT_HASH="$(psql_query -c "SELECT COALESCE(value, '') FROM indexer_state WHERE key = 'last_indexed_block_hash';" 2>/dev/null || echo "")"
echo "Current indexer_state:"
echo "  last_indexed_height:      ${CURRENT_HEIGHT}"
echo "  last_indexed_block_hash:  ${CURRENT_HASH:-<empty>}"
echo ""

echo "--- Row impact preview (block_height >= ${HEIGHT}) ---"
IMPACT_TABLES=(
  swap_events
  limit_order_fills
  limit_order_placements
  limit_order_cancellations
  liquidity_events
  hook_events
)
for table in "${IMPACT_TABLES[@]}"; do
  count="$(psql_query -c "SELECT COUNT(*)::text FROM ${table} WHERE block_height >= ${HEIGHT};" 2>/dev/null || true)"
  if [[ -z "$count" ]]; then
    echo "${table}: (unavailable — migrations not applied or Postgres unreachable)"
  else
    echo "${table}: ${count}"
  fi
done
echo ""
echo "Aggregate/mirror tables without per-block rows (ohlcv_candles, traders,"
echo "trader_positions, token_volume_stats, pair_volume_24h, global_stats_24h,"
echo "pair_reserves, resting_limit_orders) may be stale until indexer catch-up"
echo "and scheduled refresh cycles complete. Use --cleanup-derived for height-indexed"
echo "rows above, or restore Postgres from snapshot for deep reorgs."
echo ""

CLEANUP_SQL=""
if [[ "$CLEANUP_DERIVED" -eq 1 ]]; then
  CLEANUP_SQL="
BEGIN;
DELETE FROM hook_events WHERE block_height >= ${HEIGHT};
DELETE FROM liquidity_events WHERE block_height >= ${HEIGHT};
DELETE FROM limit_order_cancellations WHERE block_height >= ${HEIGHT};
DELETE FROM limit_order_placements WHERE block_height >= ${HEIGHT};
DELETE FROM limit_order_fills WHERE block_height >= ${HEIGHT};
DELETE FROM swap_events WHERE block_height >= ${HEIGHT};
COMMIT;
"
  echo "--- Derived cleanup SQL (--cleanup-derived) ---"
  echo "$CLEANUP_SQL"
  echo ""
fi

CURSOR_SQL="
BEGIN;
UPDATE indexer_state SET value = '${PREV}', updated_at = NOW() WHERE key = 'last_indexed_height';
UPDATE indexer_state SET value = '', updated_at = NOW() WHERE key = 'last_indexed_block_hash';
TRUNCATE indexer_failed_blocks;
COMMIT;
"

echo "--- Cursor reset SQL ---"
echo "$CURSOR_SQL"

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo ""
  echo "--- DRY RUN (pass --apply to execute) ---"
  echo "Suggested order: stop indexer → run with --apply → restart indexer."
  if [[ "$CLEANUP_DERIVED" -eq 0 ]]; then
    echo "Shallow reorg (≤ few blocks): swap replay is dedup-safe; add --cleanup-derived if"
    echo "limit-order or candle aggregates look wrong after catch-up."
  fi
  exit 0
fi

if [[ "$CLEANUP_DERIVED" -eq 1 ]]; then
  echo ""
  echo "--- Applying derived cleanup ---"
  psql_query -c "$CLEANUP_SQL"
fi

echo ""
echo "--- Applying cursor reset ---"
psql_query -c "$CURSOR_SQL"
echo "Done. Restart the indexer to replay from height ${HEIGHT}."
