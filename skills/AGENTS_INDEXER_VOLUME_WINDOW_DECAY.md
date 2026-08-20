# Agent playbook: indexer trailing-window volume decay (GitLab #577)

Audience: third-party agents touching token/trader/pair/global volume rollups, `/overview`, Charts 24h USD, or the volume aggregator loop.

**Issue:** [GitLab **#577**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/577) (related [#576](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/576) Charts copy — out of scope here)  
**Invariants table:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **Trailing window decay #577**)  
**Ops:** [`docs/runbooks/overview-global-stats-brin.md`](../docs/runbooks/overview-global-stats-brin.md) § Rollup freshness  
**Sibling:** [`AGENTS_INDEXER_VOLUME_PAGINATION.md`](./AGENTS_INDEXER_VOLUME_PAGINATION.md) (**V1–V5**); this skill owns **D1–D7**.

## Problem class

Charts **24h Volume (USD)** is a trailing window from `global_stats_24h`. Pair list volume already zeros idle pairs. Token windows (`token_volume_stats`) and trader rolling columns (`traders.volume_24h` / `7d` / `30d`) used to **stick** after `swap_events` left the cutoff, so APIs looked cumulative. A dead aggregator also froze a non-zero `/overview` 24h total because `get_global_stats` ignored `updated_at`.

## Invariants (D1–D7)

| ID | Rule |
|----|------|
| **D1** | After `refresh_token_volumes`, an asset whose **offer-side** swaps are all older than the window has that `"window"` row at **0** (volume, volume_usd, trade_count, unique_traders). 7d/30d analogous. **Offer-side only** (`GROUP BY offer_asset_id`) — do not sum both legs. |
| **D2** | Trader whose last swap is older than **30d** → `volume_24h` = `volume_7d` = `volume_30d` = **0**. Never zero `total_volume` / `total_volume_usd` / `total_trades` (lifetime, [#553](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553)). |
| **D3** | Pair with only 48h-old swaps → `pair_volume_24h.volume_quote = 0` after refresh. `sort=volume_24h` must not rank it as live. |
| **D4** | Aging a previously counted 24h swap to 25h then refreshing **decreases** `global_stats_24h` volume/trades (not only “extra old row ignored”). |
| **D5** | Indexer **startup** (`poller` → `refresh_all_volume_windows(..., true)`) refreshes token + trader + pair + global **without** waiting 5 min. Loop still sleeps 300s first. |
| **D6** | If `global_stats_24h.updated_at` is older than **15 minutes**, `get_global_stats` emits a **tracing warning** and **keeps serving the rollup**. No Prometheus (indexer has tracing only, [#200](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/200)). **Forbidden:** live 30d `swap_events` scan on `/overview` GET because the row is stale (**V5** / [#281](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/281) / [#333](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/333)). Existing uninitialized-zero + recent-swaps live fallback is unchanged. |
| **D7** | This skill + invariants + runbook + `make verify-issue-577`. |

Trailing windows use **`Utc::now() − window`**. No calendar-day reset. Decay tests mutate stored `block_timestamp` (V3) — do not `sleep` 24h. Future LCD timestamps can inflate a window until they age out; do **not** clamp ingest with `Utc::now()`.

## Do / don’t

- **Do** bind `"window"` / cutoffs as SQL parameters (never concatenate user strings).
- **Do** wrap each refresh function’s INSERT+zero-out in **one transaction**.
- **Do** keep `LEAST(…, POWER(10,38)-1)` / USD cap on sums; zero-out writes `0` not NULL.
- **Do** key zero-out by `pair_id` / `(asset_id, window)` / `traders.address` + `idx_swaps_sender` / `idx_swaps_offer_asset`.
- **Don’t** live-`SUM(swap_events)` on every `/overview` GET when the rollup is stale-but-nonzero.
- **Don’t** zero lifetime trader fields “to fix 24h”.
- **Don’t** change USD ingest (`volume_usd_for_swap` / P522-Q).
- **Don’t** add a retail Charts `stats_updated_at` box (integrator-only if ever added).
- **Don’t** treat Charts copy (`$2.7K` on a **live** trailing window) as this issue — that is [#576](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/576).

## Key files

| Area | Path |
|------|------|
| Token / pair / global refresh + stale warn | [`indexer/src/db/queries/volume.rs`](../indexer/src/db/queries/volume.rs) |
| Trader rolling zero-out | [`indexer/src/db/queries/traders.rs`](../indexer/src/db/queries/traders.rs) |
| Loop + startup helper | [`indexer/src/indexer/volume_aggregator.rs`](../indexer/src/indexer/volume_aggregator.rs) `refresh_all_volume_windows` |
| Startup call | [`indexer/src/indexer/poller.rs`](../indexer/src/indexer/poller.rs) |
| Tests | [`indexer/tests/indexer_volume_window_decay.rs`](../indexer/tests/indexer_volume_window_decay.rs), [`indexer_overview_global_stats.rs`](../indexer/tests/indexer_overview_global_stats.rs), [`indexer_pair_volume_pagination.rs`](../indexer/tests/indexer_pair_volume_pagination.rs), [`api_traders.rs`](../indexer/tests/api_traders.rs), [`api_tokens.rs`](../indexer/tests/api_tokens.rs) |

## Regression checklist

1. `make setup-indexer-postgres` if `indexer/.env` is missing
2. `make verify-issue-577`
3. Operator: `SELECT updated_at FROM global_stats_24h WHERE id = 1` advances on a running indexer (~5 min). Age > 15 min → warning in indexer logs; Charts must not stay frozen at a dead aggregator’s last non-zero refresh once the loop (or restart) runs.

## Related

- [`AGENTS_INDEXER_VOLUME_PAGINATION.md`](./AGENTS_INDEXER_VOLUME_PAGINATION.md) — rollup read path + pagination (**V1–V5**)
- [`AGENTS_FRONTEND_CHARTS_OVERVIEW.md`](./AGENTS_FRONTEND_CHARTS_OVERVIEW.md) — retail USD box reads the global rollup ([#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548))
- [`AGENTS_FRONTEND_TRADER_VOLUME_USD.md`](./AGENTS_FRONTEND_TRADER_VOLUME_USD.md) — Charts leaderboard uses **lifetime** `total_volume_usd` ([#553](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553)); still keep rolling columns correct for API / QA
- [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) — `/protocol` 24h/7d/30d from the same rollup ([#550](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550))
- [`AGENTS_LOCAL_POSTGRES_DEV.md`](./AGENTS_LOCAL_POSTGRES_DEV.md) — Postgres for integration tests
