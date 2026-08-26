# Overview global 24h stats — rollup and BRIN maintenance

Operators running [`indexer/`](../../indexer/) should use this runbook for **`GET /api/v1/overview`** performance and the **`idx_swaps_block_timestamp_brin`** index. Implementation: GitLab [**#333**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/333) (follow-up to [**#281**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/281)).

Human invariants: [`docs/indexer-invariants.md`](../indexer-invariants.md). Agent playbook: [`skills/AGENTS_INDEXER_VOLUME_PAGINATION.md`](../../skills/AGENTS_INDEXER_VOLUME_PAGINATION.md).

## Steady-state read path

1. **60s whole-response cache** in [`overview.rs`](../../indexer/src/api/overview.rs).
2. On cache miss, [`get_global_stats`](../../indexer/src/db/queries/volume.rs) reads the single row in **`global_stats_24h`** (O(1) primary key fetch), including additive 7d/30d USD, active-pair, unique-trader, **pool TVL / Δ%**, and **treasury fee** columns ([#550](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550) / [#569](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/569) / [#586](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/586)).
3. Cheap `COUNT(*)` census: `token_count` is unique pair-leg assets ([#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548) **C6**); `tokens_added_30d` / `pairs_added_30d` from `created_at >= now() - 30 days` (indexer first-seen, not on-chain genesis). Supporting indexes: `idx_assets_created_at`, `idx_pairs_created_at`.
4. The rollup is refreshed every **~5 minutes** by [`volume_aggregator.rs`](../../indexer/src/indexer/volume_aggregator.rs) (volume SQL **then** protocol TVL **then** protocol fees) and once at indexer startup in [`poller.rs`](../../indexer/src/indexer/poller.rs). Hub USD refresh also recomputes TVL so UST1/USTR marks exist. **Do not** `SUM` / `COUNT(DISTINCT)` 30d `swap_events` on the request path. **Do not** join `pair_reserves` or walk `global_liquidity_snapshots` on GET. **Do not** `SUM` `protocol_fee_events` / fills / wrap events on GET — fee totals and `GET /api/v1/protocol/fees` read rollup / child tables only.

Expect up to one refresh interval of lag vs a live `swap_events` aggregate. Pair count still comes from `SELECT COUNT(*) FROM pairs` on each cache miss. **`token_count`** is unique pair-leg assets (GitLab [#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548)). Charts displays **USD-only** 24h volume; `total_volume_24h` remains raw for API clients (`global_stats_24h.total_volume` is `NUMERIC(38, 0)` so 18-decimal CW20 sums fit; `NUMERIC(38, 18)` overflows at `10^20`). After catalog backfill (`20260817120000_backfill_swap_volume_usd_catalog.sql`), refresh this rollup (migration already does).

### Rollup freshness (GitLab #577 **D6**)

`get_global_stats` reads `global_stats_24h.updated_at`. If it is **older than 15 minutes** (three missed ~5 min aggregator cycles), the indexer emits:

```
global_stats_24h.updated_at is stale; serving last rollup (no live 30d swap_events scan)
```

It **keeps serving the last rollup**. Do **not** fall back to a live 30d `SUM(swap_events)` on the request path — that is a DoS vector ([#281](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/281) / [#333](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/333) **V5**). There is no Prometheus `/metrics` endpoint ([#200](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/200)); the warning is the operator signal.

Check freshness on a running indexer:

```sql
SELECT id, total_trades, total_volume_usd, updated_at,
       NOW() - updated_at AS age
FROM global_stats_24h
WHERE id = 1;
```

`updated_at` should advance about every **5 minutes** (and immediately on indexer restart — token + trader windows too, **D5**). If `age` exceeds 15 minutes, the aggregator loop is stuck or the process is down; restart the indexer. A successful refresh **can** drop 24h volume/trades to 0 when `swap_events` leave the trailing cutoff ([#577](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/577)).

Token `token_volume_stats` and trader `volume_24h` / `7d` / `30d` use the same trailing cutoffs and **zero idle rows** on refresh. Lifetime `traders.total_volume*` is unchanged. Skill: [`AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md`](../../skills/AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md).

### Actively traded pairs (GitLab #550)

`active_pairs_24h` is the count of distinct `pair_id` with ≥1 `swap_events` row in the last 24h, materialized on the rollup. Dust swaps count; there is no USD floor. This is **not** unique traders (`unique_traders_24h` is a separate rollup column) and **not** TVL.

### Protocol pool TVL (GitLab #569)

`total_liquidity_usd` is humanized USD of **priced factory `pair_reserves`** (constant-product legs), using the same catalog as volume (P522-Q + hub USD). It is **not** CoinGecko `liquidity_in_usd` (that field is still mislabeled 24h volume), not `total_volume_*`, not LP supply, and not resting limit-order escrow. The same refresh writes per-pair **`pair_liquidity_usd`** for `GET /api/v1/pairs/{addr}` ([#664](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/664)) and the `/pool` list JOIN ([#655](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/655)). GET paths read the stamp only — they do not re-sum reserves.

Refresh lives in [`protocol_tvl.rs`](../../indexer/src/indexer/protocol_tvl.rs) and is invoked from `refresh_global_stats` (so a 24h-only volume `INSERT` cannot leave TVL stale) and after hub USD refresh (UST1/USTR marks). History is `global_liquidity_snapshots` (retain ≥ 35 days; prune older). Δ% looks up the snapshot **nearest** to `now()-24h` / `now()-30d` within ±30 minutes. No snapshot, `then = 0`, or overflow → JSON `null` (UI em-dash). After `--fresh` / a young indexer, Δ% stays empty until real snapshots accrue — do not backfill from `liquidity_events` or zeros.

Flash LP that is added and withdrawn inside one snapshot interval can move **current** TVL; Δ% uses snapshots, not mempool. v1 does not add an extra anti-flash filter.

`OVERVIEW_GLOBAL_STATS_LIVE=1` still live-aggregates **volume totals** over `swap_events`. Liquidity, fee, and **volume Δ%** columns stay O(1) from the rollup (live mode must not walk 30d snapshots, `SUM` `protocol_fee_events`, or 60d-SUM volume priors).

### Protocol volume Δ% + UTC-day series (GitLab #652)

`volume_change_{24h,7d,30d}_pct` is **UPDATE-only** on the existing `global_stats_24h` id=1 row after the volume INSERT (`refresh_volume_change_pct` in [`volume.rs`](../../indexer/src/db/queries/volume.rs)). Same `flow_change_pct` as fees vs prior equal windows (`[48h,24h)`, `[14d,7d)`, `[60d,30d)`). Idle current + positive prior → `−100`. Activity + all unpriced → `NULL` (not a fake `$0` then `−100%`). Overflow / `prior ≤ 0` → `NULL`. Never Infinity.

`GET /api/v1/protocol/volume/daily?days=` allowlists `7` \| `30` (else **400**). 60s cache keyed by allowlisted `days` only. Reads `protocol_daily_volume` — **not** `defillama_daily_stats` (Protocol catalog includes gems / wrap / window swaps). Idle day `"0"`; activity+unpriced `null`; missing row treated as idle `"0"`; prune ≥ 35 days. Do not N+1 Llama `GET /defillama/daily` and do not add `from`/`to` to Llama.

### Protocol fees (GitLab #586)

`total_fees_{24h,7d,30d}_usd` and the matching `fees_change_*_pct` / `*_event_count` columns are **UPDATE-only** on the existing `global_stats_24h` id=1 row (`refresh_protocol_fee_stats`). A volume-zero INSERT must not create a fee-only stub. Source / token mix lives in `protocol_fee_stats_by_source` and `protocol_fee_stats_by_token` (top 8 + `other`). Idle windows store `"0"` with `event_count=0`; activity with all unpriced fees stores `NULL` (UI `—`). Δ% vs the prior equal window is `NULL` when `then ≤ 0`.

`GET /api/v1/protocol/fees?window=` is allowlisted (`24h` \| `7d` \| `30d`) and 60s-cached. Do **not** overload `window=` with “ust1-window”. A 15-minute stale rollup still **serves** last fee columns — do not fall back to a live 60d event scan.

Hybrid L7: `swap_amm` is pool `commission_amount` only; `book_take` is fill commission — never also `book_commission_amount`. Unwrap uses mapper **`fee`** (legacy `fee_amount`), not InstantWithdraw `tax_amount` ([#590](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/590)). Wrap execute is mapper `notify_deposit` — last-value parse on a flattened wrap+swap stream must not drop it ([#613](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/613), **I613-1–I613-8**). UST1 mint/redeem uses pinned `UST1_WINDOW_ADDRESS` `fee_amount` only — never `ust1_out × fee_total_bps` ([#614](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614), **PFee-13** / **I614-1–I614-8**). Columbus-5 window **11618** (same address as 11566) emits `fee_amount` + `fee_asset`; 11566 bps-only attrs fail closed. Unconfigured `ust1_window_configured` omits those source rows. Playbook: [`AGENTS_INDEXER_UST1_WINDOW_FEES.md`](../../skills/AGENTS_INDEXER_UST1_WINDOW_FEES.md).

Do **not** run a 24h-only `INSERT` that omits 7d/30d / `active_pairs_24h` / `unique_traders_24h` / **liquidity** / **fee** columns — those columns would stay stale or zero. Use the indexer aggregator (`refresh_global_stats` in [`volume.rs`](../../indexer/src/db/queries/volume.rs)) or restart the indexer. The live SQL matches that function: one `swap_events` pass with `FILTER` windows (`$1` = 24h, `$2` = 7d, `$3` = 30d). Liquidity and fees are follow-on updates of the same row.

### New tokens / pairs (30d)

`tokens_added_30d` / `pairs_added_30d` count rows whose indexer `created_at` is within 30 days. After a DB rebuild (`--fresh`) every row looks new. UI copy must not claim on-chain genesis this month.

### Debug: live aggregate

Set `OVERVIEW_GLOBAL_STATS_LIVE=1` on the indexer to force the pre-#333 live query over `swap_events` (uses BRIN when the planner chooses it). Use only for parity checks — not in production under load.

## Rollup refresh after reorg / replay

Rollup refresh recomputes from `swap_events WHERE block_timestamp >= now() - 24h`. It inherits the same ingestion safety as pair volume rollup:

- **C3 reorg guard** halts forward progress on hash mismatch ([`indexer-reorg-replay-dedup.md`](./indexer-reorg-replay-dedup.md)).
- **Swap dedup** (`ON CONFLICT DO NOTHING`) prevents double-counting on replay.

After a deep reorg recovery that deletes or rewinds swap rows, run a manual refresh or wait for the next aggregator cycle:

Do **not** run a 24h-only `INSERT` that omits 7d/30d / `active_pairs_24h` / `unique_traders_24h` / **liquidity** / **fee** columns — those columns would stay stale or zero. Use the indexer aggregator (`refresh_global_stats` in [`volume.rs`](../../indexer/src/db/queries/volume.rs)) or restart the indexer. The live SQL matches that function: one `swap_events` pass with `FILTER` windows (`$1` = 24h, `$2` = 7d, `$3` = 30d). Liquidity and fees are follow-on updates of the same `global_stats_24h` row.

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
| `token_volume_stats` | Per-asset rolling windows for token endpoints; idle windows **zeroed** on refresh ([#577](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/577) **D1**, offer-side only) |

Human invariants: [`docs/indexer-invariants.md`](../indexer-invariants.md) (**Protocol global stats #550**, **Trailing window decay #577**). Agent playbooks: [`skills/AGENTS_FRONTEND_PROTOCOL_STATS.md`](../../skills/AGENTS_FRONTEND_PROTOCOL_STATS.md), [`skills/AGENTS_INDEXER_VOLUME_PAGINATION.md`](../../skills/AGENTS_INDEXER_VOLUME_PAGINATION.md), [`skills/AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md`](../../skills/AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md).
