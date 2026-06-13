#!/usr/bin/env bash
# Semi-automated indexer recovery after reorg or cursor reset (GitLab #236, #362).
#
# Usage:
#   ./scripts/indexer-reorg-recover.sh --height 1234567              # dry-run preview
#   ./scripts/indexer-reorg-recover.sh --height 1234567 --apply      # cursor reset only
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

HEIGHT=""
DRY_RUN=1
CLEANUP_DERIVED=0

usage() {
  cat <<EOF
Usage: $0 --height HEIGHT [--cleanup-derived] [--apply]

  --height HEIGHT       Rewind last_indexed_height to HEIGHT-1 (next index = HEIGHT).
  --cleanup-derived     Also delete derived rows for block_height >= HEIGHT (see dry-run).
  --apply               Execute SQL (default is dry-run preview only).
  -h, --help            Show this help.

Environment:
  DATABASE_URL          Postgres connection (or postgres-dev defaults).
  REORG_ALERT_WEBHOOK_URL  (indexer process) optional webhook on reorg halt.

Also: make indexer-reorg-recover HEIGHT=<H> [APPLY=1] [CLEANUP=1]
Recovery runbook: docs/runbooks/indexer-reorg-replay-dedup.md
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --height)
      HEIGHT="${2:-}"
      shift 2
      ;;
    --cleanup-derived)
      CLEANUP_DERIVED=1
      shift
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

psql_query() {
  local query="$1"
  if postgres_psql_init; then
    if [[ "${POSTGRES_PSQL_MODE:-host}" == compose ]]; then
      export PGPASSWORD="${POSTGRES_PASSWORD}"
      postgres_psql -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" -v ON_ERROR_STOP=1 -At -c "$query"
    else
      postgres_psql "$DB_URL" -v ON_ERROR_STOP=1 -At -c "$query"
    fi
  else
    psql "$DB_URL" -v ON_ERROR_STOP=1 -At -c "$query"
  fi
}

psql_exec() {
  local sql="$1"
  if postgres_psql_init; then
    if [[ "${POSTGRES_PSQL_MODE:-host}" == compose ]]; then
      export PGPASSWORD="${POSTGRES_PASSWORD}"
      postgres_psql -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" -v ON_ERROR_STOP=1 -c "$sql"
    else
      postgres_psql "$DB_URL" -v ON_ERROR_STOP=1 -c "$sql"
    fi
  else
    psql "$DB_URL" -v ON_ERROR_STOP=1 -c "$sql"
  fi
}

echo "=== Indexer reorg recovery (GitLab #236 / #362) ==="
echo "DATABASE_URL: ${DB_URL%%@*}@***"
echo "Fork replay start height: ${HEIGHT} (cursor -> ${PREV})"
echo "Mode: $([[ "$DRY_RUN" -eq 1 ]] && echo 'DRY RUN' || echo 'APPLY')"
echo "Cleanup derived rows (block_height >= ${HEIGHT}): $([[ "$CLEANUP_DERIVED" -eq 1 ]] && echo yes || echo no)"
echo ""

CURRENT_HEIGHT="$(psql_query "SELECT COALESCE(value, '0') FROM indexer_state WHERE key = 'last_indexed_height';" 2>/dev/null || echo '?')"
CURRENT_HASH="$(psql_query "SELECT COALESCE(value, '') FROM indexer_state WHERE key = 'last_indexed_block_hash';" 2>/dev/null || echo '')"
echo "Current indexer_state:"
echo "  last_indexed_height: ${CURRENT_HEIGHT}"
echo "  last_indexed_block_hash: ${CURRENT_HASH:-<empty>}"
echo ""

# Row-impact preview (read-only)
echo "--- Row impact preview (block_height >= ${HEIGHT}) ---"
for label_table in \
  "swap_events:swap_events" \
  "liquidity_events:liquidity_events" \
  "limit_order_fills:limit_order_fills" \
  "limit_order_placements:limit_order_placements" \
  "limit_order_cancellations:limit_order_cancellations" \
  "hook_events:hook_events"; do
  label="${label_table%%:*}"
  table="${label_table##*:}"
  count="$(psql_query "SELECT COUNT(*) FROM ${table} WHERE block_height >= ${HEIGHT};" 2>/dev/null || echo 'ERR')"
  echo "  ${label}: ${count} rows"
done

CANDLE_COUNT="$(psql_query "
  SELECT COUNT(*) FROM candles
  WHERE pair_id IN (SELECT DISTINCT pair_id FROM swap_events WHERE block_height >= ${HEIGHT});
" 2>/dev/null || echo 'ERR')"
echo "  candles (pairs with swaps >= ${HEIGHT}): ${CANDLE_COUNT} rows"
echo "  indexer_failed_blocks: $(psql_query 'SELECT COUNT(*) FROM indexer_failed_blocks;' 2>/dev/null || echo 'ERR') rows (truncated on cursor reset)"
echo ""
echo "NOTE: swap_events are kept on cursor-only replay (ON CONFLICT dedup). Use --cleanup-derived"
echo "      when the canonical chain at these heights differs (true reorg)."
echo ""

CURSOR_SQL="
BEGIN;
UPDATE indexer_state SET value = '${PREV}', updated_at = NOW() WHERE key = 'last_indexed_height';
UPDATE indexer_state SET value = '', updated_at = NOW() WHERE key = 'last_indexed_block_hash';
TRUNCATE indexer_failed_blocks;
COMMIT;
"

DERIVED_SQL=""
if [[ "$CLEANUP_DERIVED" -eq 1 ]]; then
  DERIVED_SQL="
BEGIN;
CREATE TEMP TABLE _reorg_affected_pairs ON COMMIT DROP AS
  SELECT DISTINCT pair_id FROM swap_events WHERE block_height >= ${HEIGHT};
DELETE FROM candles WHERE pair_id IN (SELECT pair_id FROM _reorg_affected_pairs);
DELETE FROM hook_events WHERE block_height >= ${HEIGHT};
DELETE FROM limit_order_cancellations WHERE block_height >= ${HEIGHT};
DELETE FROM limit_order_placements WHERE block_height >= ${HEIGHT};
DELETE FROM limit_order_fills WHERE block_height >= ${HEIGHT};
DELETE FROM liquidity_events WHERE block_height >= ${HEIGHT};
DELETE FROM swap_events WHERE block_height >= ${HEIGHT};
COMMIT;
"
fi

echo "--- Cursor reset SQL ---"
echo "$CURSOR_SQL"

if [[ "$CLEANUP_DERIVED" -eq 1 ]]; then
  echo "--- Derived cleanup SQL (--cleanup-derived) ---"
  echo "$DERIVED_SQL"
  echo ""
  echo "WARNING: trader_positions / traders rollups are not height-keyed."
  echo "         After shallow reorg with cleanup, restart indexer and monitor;"
  echo "         deep reorg may require Postgres snapshot restore (see runbook)."
fi

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo ""
  echo "--- DRY RUN complete (pass --apply to execute) ---"
  echo "Suggested flow:"
  echo "  1. Stop indexer"
  echo "  2. $0 --height ${HEIGHT}$([[ "$CLEANUP_DERIVED" -eq 1 ]] && echo ' --cleanup-derived') --apply"
  echo "  3. Restart indexer and verify last_indexed_height advances"
  exit 0
fi

APPLY_SQL="$CURSOR_SQL"
if [[ "$CLEANUP_DERIVED" -eq 1 ]]; then
  APPLY_SQL="
BEGIN;
CREATE TEMP TABLE _reorg_affected_pairs ON COMMIT DROP AS
  SELECT DISTINCT pair_id FROM swap_events WHERE block_height >= ${HEIGHT};
DELETE FROM candles WHERE pair_id IN (SELECT pair_id FROM _reorg_affected_pairs);
DELETE FROM hook_events WHERE block_height >= ${HEIGHT};
DELETE FROM limit_order_cancellations WHERE block_height >= ${HEIGHT};
DELETE FROM limit_order_placements WHERE block_height >= ${HEIGHT};
DELETE FROM limit_order_fills WHERE block_height >= ${HEIGHT};
DELETE FROM liquidity_events WHERE block_height >= ${HEIGHT};
DELETE FROM swap_events WHERE block_height >= ${HEIGHT};
UPDATE indexer_state SET value = '${PREV}', updated_at = NOW() WHERE key = 'last_indexed_height';
UPDATE indexer_state SET value = '', updated_at = NOW() WHERE key = 'last_indexed_block_hash';
TRUNCATE indexer_failed_blocks;
COMMIT;
"
fi

echo ""
if [[ "$CLEANUP_DERIVED" -eq 1 ]]; then
  echo "--- Applying cursor reset + derived cleanup (single transaction) ---"
else
  echo "--- Applying cursor reset ---"
fi
psql_exec "$APPLY_SQL"

echo "Done. Restart the indexer to replay from height ${HEIGHT}."
