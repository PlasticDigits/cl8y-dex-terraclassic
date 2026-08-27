# Agent playbook: Trader positions 18-decimal storage (GitLab #676)

Audience: third-party agents touching `GET /api/v1/traders/{addr}/positions`, `trader_positions`, or `position_tracker`.

**Issue:** [GitLab **#676**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/676)  
**Invariants:** **P676-1–P676-8** in [`docs/indexer-invariants.md`](../docs/indexer-invariants.md)  
**Related:** [#551](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/551) human scale, [#529](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/529) 6-vs-18 contract band, [#557](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/557) plain digit strings

## Problem class

`/positions` undercounted or omitted mixed-decimal pairs while `/trades` was complete:

| Pair | Decimals | Symptom |
|------|----------|---------|
| CL8Y-cb / cUSTC | 18 / 6 | `trade_count` 2 vs 7 swaps |
| UST1 / USTR | 6 / 18 | pair **absent** vs 3 swaps |
| 6 / 6 pairs | 6 / 6 | matched |

Root cause: `trader_positions` used `NUMERIC(38, 18)` (`|x| < 10^20`). ~100 human 18-dec tokens is `10^20` raw. Postgres rejected the upsert **after** `swap_events` insert. Replay then skipped the position update (`ON CONFLICT DO NOTHING` on the swap). Same overflow class as `20260817120000` (raw volume) and candle human `NUMERIC(78, 18)`.

## Invariants (P676-1–P676-8)

| ID | Rule |
|----|------|
| **P676-1** | `trader_positions` amount columns and `traders` P&L / fee columns are **`NUMERIC(78, 18)`**. Do not revert to `NUMERIC(38, 18)`. |
| **P676-2** | Stored units stay **raw** (#551). Do not humanize in the indexer. |
| **P676-3** | Every ingested `swap_events` row for `(sender, pair_id)` increments that position’s `trade_count` by 1 — 6/6, 6/18, and 18/6. |
| **P676-4** | A pair with swaps must appear on `GET /positions` (unless the pair row is missing). Do not drop a row because a raw amount is large. |
| **P676-5** | Positions JSON amounts use **`bd_plain_string`** (no `1e+19`). dApp `parseNumericParts` rejects scientific notation. |
| **P676-6** | After overflow-skip or reorg, rebuild from `swap_events` (`rebuild_all_positions_from_swaps` / poller `repair_positions_if_trade_count_mismatch`). Do not expect block replay alone to heal — duplicate swaps return early. |
| **P676-7** | Migration `20260827120000_*` widens types and one-shot SQL-replays existing swaps. Poller repair is the ongoing source of truth when counts still diverge. |
| **P676-8** | Human display stays **P551-1–P551-6**. This ticket does not change avg-entry scale or header USD (#560). |

## Do / don’t

- **Do** persist 100+ human USTR / CL8Y as raw `10^20+`.
- **Do** compare per-pair `trade_count` to `COUNT(*)` from `swap_events` for the same wallet.
- **Do** run `cl8y-dex-indexer rebuild-positions` after a deep reorg that truncates derived tables (see [`docs/runbooks/indexer-reorg-replay-dedup.md`](../docs/runbooks/indexer-reorg-replay-dedup.md)).
- **Don’t** clamp or skip position upserts when `|raw| ≥ 10^20`.
- **Don’t** treat a missing `/positions` row as “flat” when `/trades` lists that pair.
- **Don’t** sum `traders.total_realized_pnl` across pairs (#551).

## Canonical code

| File | Role |
|------|------|
| [`position_tracker.rs`](../indexer/src/indexer/position_tracker.rs) | Ingest + rebuild + mismatch repair |
| [`20260827120000_trader_positions_numeric_78.sql`](../indexer/migrations/20260827120000_trader_positions_numeric_78.sql) | Widen + one-shot SQL replay |
| [`poller.rs`](../indexer/src/indexer/poller.rs) | Startup repair when counts diverge |
| [`traders.rs`](../indexer/src/api/traders.rs) | `bd_plain_string` on position JSON |
| [`position_tracker_18dec.rs`](../indexer/tests/position_tracker_18dec.rs) | 6/18 + 18/6 persist + rebuild |

## Regression

```bash
make verify-issue-676
```

```bash
cd indexer && cargo test --test position_tracker_18dec -- --test-threads=1 --quiet
cd indexer && cargo test --test position_tracker_clamp -- --test-threads=1 --quiet
cd indexer && cargo test --lib position_tracker -- --quiet
```

## Related

- [`AGENTS_FRONTEND_PORTFOLIO.md`](./AGENTS_FRONTEND_PORTFOLIO.md) — `/portfolio` shell
- [`AGENTS_FRONTEND_PORTFOLIO_PNL.md`](./AGENTS_FRONTEND_PORTFOLIO_PNL.md) — human scale (**P551**)
- [`AGENTS_FRONTEND_HUB_PNL.md`](./AGENTS_FRONTEND_HUB_PNL.md) — header USD (**P560**)
- [`AGENTS_LIMIT_PRICE_DECIMALS.md`](./AGENTS_LIMIT_PRICE_DECIMALS.md) — contract 6-vs-18 band (#529)
- [`AGENTS_FRONTEND_TAPE_AMOUNTS.md`](./AGENTS_FRONTEND_TAPE_AMOUNTS.md) — tape plain strings (#557)
