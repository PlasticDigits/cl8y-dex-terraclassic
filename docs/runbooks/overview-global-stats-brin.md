# Overview global 24h stats — rollup and BRIN maintenance

Operators running [`indexer/`](../../indexer/) should use this runbook for **`GET /api/v1/overview`** performance and the **`idx_swaps_block_timestamp_brin`** index. Implementation: GitLab [**#333**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/333) (follow-up to [**#281**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/281)).

Human invariants: [`docs/indexer-invariants.md`](../indexer-invariants.md). Agent playbook: [`skills/AGENTS_INDEXER_VOLUME_PAGINATION.md`](../../skills/AGENTS_INDEXER_VOLUME_PAGINATION.md).

## Steady-state read path

1. **60s whole-response cache** in [`overview.rs`](../../indexer/src/api/overview.rs).
2. On cache miss, [`get_global_stats`](../../indexer/src/db/queries/volume.rs) reads the single row in **`global_stats_24h`** (O(1) primary key fetch).
3. The rollup is refreshed every **~5 minutes** by [`volume_aggregator.rs`](../../indexer/src/indexer/volume_aggregator.rs) and once at indexer startup in [`poller.rs`](../../indexer/src/indexer/poller.rs).

Expect up to one refresh interval of lag vs a live `swap_events` aggregate. Pair count still comes from `SELECT COUNT(*) FROM pairs` on each cache miss.

### Debug: live aggregate

Set `OVERVIEW_GLOBAL_STATS_LIVE=1` on the indexer to force the pre-#333 live query over `swap_events` (uses BRIN when the planner chooses it). Use only for parity checks — not in production under load.

## Rollup refresh after reorg / replay

Rollup refresh recomputes from `swap_events WHERE block_timestamp >= now() - 24h`. It inherits the same ingestion safety as pair volume rollup:

- **C3 reorg guard** halts forward progress on hash mismatch ([`indexer-reorg-replay-dedup.md`](./indexer-reorg-replay-dedup.md)).
- **Swap dedup** (`ON CONFLICT DO NOTHING`) prevents double-counting on replay.

After a deep reorg recovery that deletes or rewinds swap rows, run a manual refresh or wait for the next aggregator cycle:

```sql
-- Optional immediate refresh (same SQL as the indexer task)
INSERT INTO global_stats_24h (id, total_volume, total_volume_usd, total_trades, updated_at)
SELECT 1,
       COALESCE(SUM(offer_amount), 0),
       COALESCE(SUM(volume_usd), 0),
       COUNT(*),
       NOW()
FROM swap_events
WHERE block_timestamp >= NOW() - INTERVAL '24 hours'
ON CONFLICT (id) DO UPDATE SET
  total_volume = EXCLUDED.total_volume,
  total_volume_usd = EXCLUDED.total_volume_usd,
  total_trades = EXCLUDED.total_trades,
  updated_at = EXCLUDED.updated_at;
```

## BRIN index tuning (production safety net)

Migration `20260604120100_swap_events_block_timestamp_brin.sql` creates the default BRIN index. The **rollup is the primary bounded path**; BRIN supports the optional live-query fallback and operator diagnostics.

### When to tune

- After initial production backfill when `swap_events` exceeds ~1M rows.
- When `EXPLAIN (ANALYZE, BUFFERS)` on the live 24h aggregate (with `OVERVIEW_GLOBAL_STATS_LIVE=1`) shows sequential scans or poor buffer hit ratio on recent pages.
- After large manual replay: run `ANALYZE swap_events` and summarize new BRIN ranges.

### Recommended production settings

On a production clone with realistic row counts, capture a baseline plan, then apply:

```sql
-- Example starting point for high-insert swap log (tune from EXPLAIN on your clone)
ALTER INDEX idx_swaps_block_timestamp_brin SET (pages_per_range = 64);
ALTER INDEX idx_swaps_block_timestamp_brin SET (autosummarize = on);

ANALYZE swap_events;
SELECT brin_summarize_new_values('idx_swaps_block_timestamp_brin');
```

`pages_per_range` trade-off:

| Value | Effect |
|-------|--------|
| **32** | Finer summaries, larger index, better selectivity on narrow time windows |
| **64–128** | Common production starting range for monotonic `block_timestamp` |
| **256+** | Smaller index, coarser ranges — may seq-scan more heap pages on 24h window |

### Before / after verification (production-scale fixture)

Run on a **clone** with production-like `swap_events` cardinality (not empty dev DBs):

```sql
EXPLAIN (ANALYZE, BUFFERS)
SELECT COALESCE(SUM(offer_amount), 0),
       COALESCE(SUM(volume_usd), 0),
       COUNT(*)
FROM swap_events
WHERE block_timestamp >= NOW() - INTERVAL '24 hours';
```

**Before tuning (typical on large tables with default BRIN):** plan may show `Seq Scan on swap_events` or `Bitmap Index Scan` on BRIN with high heap fetches.

**After tuning + `brin_summarize_new_values`:** expect `Bitmap Index Scan` on `idx_swaps_block_timestamp_brin` with reduced heap pages read for the 24h window. Record actual `Buffers:` and timing in your change ticket.

Steady-state `/overview` cache miss should show only:

```sql
EXPLAIN (FORMAT TEXT)
SELECT total_volume, total_volume_usd, total_trades FROM global_stats_24h WHERE id = 1;
```

→ `Index Scan` or `Seq Scan` on the single-row `global_stats_24h` table (no `swap_events`).

### Scheduled maintenance

Add to your Postgres maintenance (e.g. nightly or after bulk replay):

```sql
ANALYZE swap_events;
SELECT brin_summarize_new_values('idx_swaps_block_timestamp_brin');
```

Re-run `EXPLAIN (ANALYZE, BUFFERS)` on the live aggregate periodically if you rely on `OVERVIEW_GLOBAL_STATS_LIVE=1` in staging.

## Related tables

| Table | Purpose |
|-------|---------|
| `global_stats_24h` | Cross-pair 24h `offer_amount` / `volume_usd` / trade count for `/overview` |
| `pair_volume_24h` | Per-pair quote volume for `GET /pairs?sort=volume_24h` |
| `token_volume_stats` | Per-asset rolling windows for token endpoints |
