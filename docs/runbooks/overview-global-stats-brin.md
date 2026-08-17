# Overview global 24h stats — rollup and BRIN maintenance

Operators running [`indexer/`](../../indexer/) should use this runbook for **`GET /api/v1/overview`** performance and the **`idx_swaps_block_timestamp_brin`** index. Implementation: GitLab [**#333**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/333) (follow-up to [**#281**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/281)).

Human invariants: [`docs/indexer-invariants.md`](../indexer-invariants.md). Agent playbook: [`skills/AGENTS_INDEXER_VOLUME_PAGINATION.md`](../../skills/AGENTS_INDEXER_VOLUME_PAGINATION.md).

## Steady-state read path

1. **60s whole-response cache** in [`overview.rs`](../../indexer/src/api/overview.rs).
2. On cache miss, [`get_global_stats`](../../indexer/src/db/queries/volume.rs) reads the single row in **`global_stats_24h`** (O(1) primary key fetch), including additive 7d/30d USD, active-pair, and unique-trader columns ([#550](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550)).
3. Cheap `COUNT(*)` census: `token_count` is unique pair-leg assets ([#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548) **C6**); `tokens_added_30d` / `pairs_added_30d` from `created_at >= now() - 30 days` (indexer first-seen, not on-chain genesis). Supporting indexes: `idx_assets_created_at`, `idx_pairs_created_at`.
4. The rollup is refreshed every **~5 minutes** by [`volume_aggregator.rs`](../../indexer/src/indexer/volume_aggregator.rs) and once at indexer startup in [`poller.rs`](../../indexer/src/indexer/poller.rs). **Do not** `SUM` / `COUNT(DISTINCT)` 30d `swap_events` on the request path.

Expect up to one refresh interval of lag vs a live `swap_events` aggregate. Pair count still comes from `SELECT COUNT(*) FROM pairs` on each cache miss. **`token_count`** is unique pair-leg assets (GitLab [#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548)). Charts displays **USD-only** 24h volume; `total_volume_24h` remains raw for API clients (`global_stats_24h.total_volume` is `NUMERIC(38, 0)` so 18-decimal CW20 sums fit; `NUMERIC(38, 18)` overflows at `10^20`). After catalog backfill (`20260817120000_backfill_swap_volume_usd_catalog.sql`), refresh this rollup (migration already does).

### Actively traded pairs (GitLab #550)

`active_pairs_24h` is the count of distinct `pair_id` with ≥1 `swap_events` row in the last 24h, materialized on the rollup. Dust swaps count; there is no USD floor. This is **not** unique traders (`unique_traders_24h` is a separate rollup column) and **not** TVL.

### New tokens / pairs (30d)

`tokens_added_30d` / `pairs_added_30d` count rows whose indexer `created_at` is within 30 days. After a DB rebuild (`--fresh`) every row looks new. UI copy must not claim on-chain genesis this month.

### Debug: live aggregate

Set `OVERVIEW_GLOBAL_STATS_LIVE=1` on the indexer to force the pre-#333 live query over `swap_events` (uses BRIN when the planner chooses it). Use only for parity checks — not in production under load.

## Rollup refresh after reorg / replay

Rollup refresh recomputes from `swap_events WHERE block_timestamp >= now() - 24h`. It inherits the same ingestion safety as pair volume rollup:

- **C3 reorg guard** halts forward progress on hash mismatch ([`indexer-reorg-replay-dedup.md`](./indexer-reorg-replay-dedup.md)).
- **Swap dedup** (`ON CONFLICT DO NOTHING`) prevents double-counting on replay.

After a deep reorg recovery that deletes or rewinds swap rows, run a manual refresh or wait for the next aggregator cycle:

Do **not** run a 24h-only `INSERT` that omits 7d/30d / `active_pairs_24h` / `unique_traders_24h` — those columns would stay stale or zero. Use the indexer aggregator (`refresh_global_stats` in [`volume.rs`](../../indexer/src/db/queries/volume.rs)) or restart the indexer. The live SQL matches that function: one `swap_events` pass with `FILTER` windows (`$1` = 24h, `$2` = 7d, `$3` = 30d).

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
| `global_stats_24h` | Cross-pair 24h `offer_amount` / `volume_usd` / trade count plus 7d/30d USD, `active_pairs_24h`, `unique_traders_24h` for `/overview` ([#550](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550)) |
| `pair_volume_24h` | Per-pair quote volume for `GET /pairs?sort=volume_24h` |
| `token_volume_stats` | Per-asset rolling windows for token endpoints |

Human invariants: [`docs/indexer-invariants.md`](../indexer-invariants.md) (**Protocol global stats #550**). Agent playbooks: [`skills/AGENTS_FRONTEND_PROTOCOL_STATS.md`](../../skills/AGENTS_FRONTEND_PROTOCOL_STATS.md), [`skills/AGENTS_INDEXER_VOLUME_PAGINATION.md`](../../skills/AGENTS_INDEXER_VOLUME_PAGINATION.md).
