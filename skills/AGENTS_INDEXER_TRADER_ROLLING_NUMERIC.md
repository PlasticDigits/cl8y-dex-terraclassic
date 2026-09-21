# Agent playbook: trader rolling raw volume NUMERIC(38, 0) (GitLab #1277)

Audience: third-party agents touching `traders.volume_24h` / `7d` / `30d` / `total_volume`, `refresh_rolling_volumes`, `upsert_trader`, or the volume aggregator loop.

**Issue:** [#1277](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1277)  
**Invariants:** **R1277-1–R1277-8** in [`docs/indexer-invariants.md`](../docs/indexer-invariants.md)  
**Sibling decay:** [`AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md`](./AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md) (**D1–D7**, [#577](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/577))  
**Sibling USD:** [`AGENTS_FRONTEND_TRADER_VOLUME_USD.md`](./AGENTS_FRONTEND_TRADER_VOLUME_USD.md) (**#553**) — `total_volume_usd` stays `(38, 18)`  
**Not:** [#1276](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1276) health/SHA ([`AGENTS_INDEXER_HEALTH_GIT_SHA.md`](./AGENTS_INDEXER_HEALTH_GIT_SHA.md) **H1276-8**), [#676](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/676) positions `(78, 18)`, pair-scoped GET sums ([#666](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/666))

## Problem class

`volume_aggregator` logged **Failed to refresh rolling trader volumes** whenever a sender’s trailing-window `SUM(swap_events.offer_amount)` could not fit **`NUMERIC(38, 18)`** (`|x| < 10^20`, ~100 human 18-dec CW20). Pair/global raw rollups were widened to **`NUMERIC(38, 0)`** in `20260817120000_*` (#548). Trader rolling columns were renamed from `volume_*_usd` in `20260310000002_*` and never widened. `#577` decay kept `LEAST(…, POWER(10,38)-1)`, which does **not** fit `(38, 18)`. One overflowing sender failed the whole rolling `UPDATE` (single tx), so every other trader’s 24h/7d/30d froze and idle zero-out never ran.

## Invariants (R1277-1–R1277-8)

| ID | Rule |
|----|------|
| **R1277-1** | `traders.volume_24h` / `volume_7d` / `volume_30d` / `total_volume` are **`NUMERIC(38, 0)`** raw `offer_amount` integers. Do not store rolling volume as `(38, 18)` or `(78, 18)`. |
| **R1277-2** | Keep `LEAST(SUM(…), POWER(10::numeric, 38) - 1)` on rolling sums **after** the destination is `(38, 0)`. **Forbidden:** clamp rolling raw volume to the USD cap `10^20 - 10^-18` (truncates honest 18-dec volume — issue **A3**). |
| **R1277-3** | Idle 30d+ traders still zero **rolling** columns only (#577 **D2**). Never zero or rewrite `total_volume` / `total_volume_usd` / `total_trades`. |
| **R1277-4** | `upsert_trader` caps additive lifetime raw at `10^38-1` (INSERT and `ON CONFLICT`). A single 18-dec `10^21` swap must not overflow ingest. |
| **R1277-5** | `total_volume_usd` remains `NUMERIC(38, 18)` with the existing `10^20` USD cap. Unpriced stays NULL. Rolling refresh must not smash USD. |
| **R1277-6** | Unscoped rolling JSON uses **`bd_plain_string`** (no `1e+19`). Pair-scoped leaderboard GET `SUM(offer_amount)` is out of scope for stored columns (issue **A9**). |
| **R1277-7** | Token / pair / global refresh in `refresh_all_volume_windows` still run when trader refresh would previously have been the only failure. After this fix, trader refresh must not be that failure. |
| **R1277-8** | Decay tests stay **V3** (mutate `block_timestamp`). No live `SUM(swap_events)` on unscoped trader GET. Do not fold this into #1276 or reopen #577. |

## Do / don’t

- **Do** treat rolling + lifetime **raw** volume as integer offer sums, matching `swap_events.offer_amount` / `pair_volume_24h.volume_quote` / `global_stats_24h.total_volume`.
- **Do** keep rolling UPDATE + idle zero-out in **one transaction** (issue **A6** — no torn 24h vs 30d).
- **Do** bind window cutoffs as timestamps (no user `window` string concat — **A5**).
- **Don’t** reuse `#553` USD `LEAST` SQL on raw rolling columns.
- **Don’t** zero `total_volume_usd` to “heal” overflow (**A4** / #553 / #577 **A5**).
- **Don’t** clamp ingest timestamps with `Utc::now()` (same as #577 **A1** / this **A8**).
- **Don’t** switch rolling volume to `NUMERIC(78, 18)` (#676 is inventory/P&L).
- **Don’t** add `total_volume_usd` to `SQL_TRADER_LIFETIME_DIVERGES` (USD-only skew must not re-trip the poller — leftover [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) **M1305-4**).

Coolify leftover migrate `20260921130000` + D5 heal no-op: [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) ([`AGENTS_POST_MERGE_OPS_1305.md`](./AGENTS_POST_MERGE_OPS_1305.md); `make verify-issue-1305`). If the #1276 checkbox is off, leftover 2 is a **manual** Coolify indexer deploy of `729b097f+`. Healthy `GET /health` is not `_sqlx_migrations` evidence. Widen `…000` + 30d `…001` stay sister [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300).

## Canonical code

| File | Role |
|------|------|
| [`20260921120000_traders_rolling_volume_numeric_38_0.sql`](../indexer/migrations/20260921120000_traders_rolling_volume_numeric_38_0.sql) | Widen raw columns |
| [`20260921130000_traders_lifetime_heal_from_swaps.sql`](../indexer/migrations/20260921130000_traders_lifetime_heal_from_swaps.sql) | One-shot lifetime heal from `swap_events` |
| [`traders.rs`](../indexer/src/db/queries/traders.rs) | `refresh_rolling_volumes` + `upsert_trader` cap + `heal_trader_lifetime_from_swaps` |
| [`poller.rs`](../indexer/src/indexer/poller.rs) | Mismatch-gated heal **before** `refresh_all_volume_windows` (**D5**) |
| [`volume_aggregator.rs`](../indexer/src/indexer/volume_aggregator.rs) | Isolated fail log (`rolling trader volumes`) |
| [`api/traders.rs`](../indexer/src/api/traders.rs) | `bd_plain_string` on raw volume JSON |
| [`indexer_trader_rolling_numeric.rs`](../indexer/tests/indexer_trader_rolling_numeric.rs) | `10^21` refresh + decay + upsert cap + leaderboard |

## Regression

```bash
make verify-issue-1277
```

```bash
cd indexer && cargo test --test indexer_trader_rolling_numeric --test indexer_volume_window_decay --test volume_usd_catalog --test api_traders -- --test-threads=1 --quiet
```

Needs `make setup-indexer-postgres` when `indexer/.env` is missing.

## Related

- [`AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md`](./AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md) — trailing-window zero-out (**D1–D7**)
- [`AGENTS_FRONTEND_CHARTS_OVERVIEW.md`](./AGENTS_FRONTEND_CHARTS_OVERVIEW.md) — catalog USD ingest (#548) widened **global/pair** raw, not trader rolling
- [`AGENTS_INDEXER_TRADER_POSITIONS_DECIMALS.md`](./AGENTS_INDEXER_TRADER_POSITIONS_DECIMALS.md) — `(78, 18)` positions, not rolling volume
- [`AGENTS_INDEXER_HEALTH_GIT_SHA.md`](./AGENTS_INDEXER_HEALTH_GIT_SHA.md) — **H1276-8** keeps this ticket out of `/health`
- [`docs/adr/0006-indexer-health-git-sha.md`](../docs/adr/0006-indexer-health-git-sha.md) — #1277 remains a non-goal of #1276
