# Agent playbook: indexer volume rollup & pagination (GitLab #243)

## When to use

You are changing **pair list 24h volume**, **block timestamp resolution**, or **pagination caps** on pair/token/CG list endpoints.

Gap analysis: [`gaps/GAP_1780200149.md`](../gaps/GAP_1780200149.md) — **M4**, **M8**.

## Invariants (V1–V4)

| ID | Invariant | Enforcement |
|----|-----------|-------------|
| **V1** | Pair list `sort=volume_24h` reads **`pair_volume_24h`** rollup, not live 24h `swap_events` scan | [`db/queries/pairs.rs`](../indexer/src/db/queries/pairs.rs) LEFT JOIN |
| **V2** | Rollup refreshed ~**5 min** by [`volume_aggregator`](../indexer/src/indexer/volume_aggregator.rs) + initial refresh in [`poller.rs`](../indexer/src/indexer/poller.rs) | [`volume::refresh_pair_volumes`](../indexer/src/db/queries/volume.rs) |
| **V3** | Block time: tx RFC3339 → LCD header; **never** `Utc::now()` | [`block_indexer::resolve_block_time`](../indexer/src/indexer/block_indexer.rs) |
| **V4** | Deep list **`offset` > 10_000** → **400** on pairs, tokens, `/cg/pairs` | [`api/pairs.rs`](../indexer/src/api/pairs.rs), [`api/tokens.rs`](../indexer/src/api/tokens.rs), [`api/cg.rs`](../indexer/src/api/cg.rs) |
| **V5** | `/overview` global 24h aggregate: **`global_stats_24h`** rollup (~5 min refresh) + **60s** whole-response cache; BRIN `idx_swaps_block_timestamp_brin` for optional live fallback | [`overview.rs`](../indexer/src/api/overview.rs), [`volume::get_global_stats`](../indexer/src/db/queries/volume.rs), [`volume::refresh_global_stats`](../indexer/src/db/queries/volume.rs); [**#333**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/333), [**#281**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/281) |

Human doc: [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (indexing + API cap rows).

## Key files

| Area | Path |
|------|------|
| Pair rollup migration | [`indexer/migrations/20260531143000_pair_volume_24h_rollup.sql`](../indexer/migrations/20260531143000_pair_volume_24h_rollup.sql) |
| Global overview rollup migration | [`indexer/migrations/20260605120000_global_stats_24h_rollup.sql`](../indexer/migrations/20260605120000_global_stats_24h_rollup.sql) |
| Rollup refresh | [`indexer/src/db/queries/volume.rs`](../indexer/src/db/queries/volume.rs) (`refresh_pair_volumes`, `refresh_global_stats`) |
| BRIN ops runbook | [`docs/runbooks/overview-global-stats-brin.md`](../docs/runbooks/overview-global-stats-brin.md) |
| Pair list query | [`indexer/src/db/queries/pairs.rs`](../indexer/src/db/queries/pairs.rs) |
| Block time | [`indexer/src/indexer/block_indexer.rs`](../indexer/src/indexer/block_indexer.rs) |
| Tests | [`indexer/tests/indexer_pair_volume_pagination.rs`](../indexer/tests/indexer_pair_volume_pagination.rs) (pair rollup match, EXPLAIN no `swap_events`), [`indexer_overview_global_stats.rs`](../indexer/tests/indexer_overview_global_stats.rs) (global rollup match, EXPLAIN no `swap_events`, BRIN index + overview cache), [`indexer_ingestion_hardening.rs`](../indexer/tests/indexer_ingestion_hardening.rs) |

## Tests

```bash
cd indexer && cargo test --lib indexer::block_indexer
docker compose up -d postgres
./scripts/setup-postgres-dev-databases.sh
cd indexer && cargo test --test indexer_pair_volume_pagination --test indexer_overview_global_stats --test indexer_ingestion_hardening --test api_pairs --test api_tokens --test api_cg -j 1 -- --test-threads=1
```

## Do not regress

- **`pair_volume_24h` / `global_stats_24h` seed** in integration tests: call `refresh_pair_volumes` and `refresh_global_stats` after swap seed ([`tests/common/mod.rs`](../indexer/tests/common/mod.rs)).
- Backward-compatible JSON shapes (arrays for tokens/CG pairs; pair list paginated object unchanged).
- Ingestion hardening **C1–C3** from [`AGENTS_INDEXER_INGESTION_HARDENING.md`](./AGENTS_INDEXER_INGESTION_HARDENING.md).
