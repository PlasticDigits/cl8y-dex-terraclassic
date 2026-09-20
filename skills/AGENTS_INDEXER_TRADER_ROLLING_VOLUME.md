# Agent playbook: trader rolling raw volume `NUMERIC(38, 0)` (#1277)

Audience: third-party agents changing `traders.volume_24h` / `7d` / `30d` / `total_volume`, `refresh_rolling_volumes`, or `upsert_trader` raw add.

**Issue:** [Forgejo **#1277**](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1277)  
**ADR:** [`docs/adr/0005-trader-rolling-volume-numeric.md`](../docs/adr/0005-trader-rolling-volume-numeric.md)  
**Slices:** [`design/1277-trader-rolling-volume-numeric.md`](../design/1277-trader-rolling-volume-numeric.md)  
**Sibling types:** [#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548) pair/global `(38, 0)`; [#676](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/676) positions `(78, 18)` — **not** this path  
**Decay:** [`AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md`](./AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md) (**D2** still applies)

## Problem class

`LEAST(SUM(offer_amount), 10^38-1)` does not fit `NUMERIC(38, 18)` (`|x| < 10^20`). One large 18-dec sender fails the whole rolling `UPDATE`; #577 idle zero-out shares that transaction. `insert_swap` commits first; overflowing `upsert_trader` leaves a swap with no (or stale) `traders` row. Replay skips via `trade_exists`. Rolling SQL is UPDATE-only.

## Invariants (V1277-1–V1277-8)

| ID | Rule |
|----|------|
| **V1277-1** | `traders.volume_24h` / `volume_7d` / `volume_30d` / `total_volume` are **`NUMERIC(38, 0)`**. Do not switch these to `NUMERIC(78, 18)`. |
| **V1277-2** | Keep `LEAST(SUM(…), POWER(10,38)-1)` on rolling sums. Cap `upsert_trader` `total_volume + $2` the same way. Do not clamp raw volume to the USD `10^20` cap. |
| **V1277-3** | `total_volume_usd` stays `NUMERIC(38, 18)` / `10^20` USD cap. Heal does **not** rewrite it. Idle **D2** zeros rolling columns only. |
| **V1277-4** | After overflow-skip, one-shot + poller heal from `swap_events`: `INSERT` missing senders; recompute `total_trades` / `total_volume` with the integer cap. Block replay alone does not heal. |
| **V1277-5** | `refresh_rolling_volumes` stays UPDATE-only once heal has inserted senders. |
| **V1277-6** | Unscoped (and pair-scoped `total_volume`) JSON uses **`bd_plain_string`** (no `1e+19`). |
| **V1277-7** | `make verify-issue-1277` after `make setup-indexer-postgres`. Include the skip-class fixture (swap inserted, upsert overflowed, replay skipped, migrate+heal, refresh `Ok`, wallet on unscoped board). |
| **V1277-8** | Coolify = sqlx migrate + indexer restart. Rollback is not `ALTER` back to `(38, 18)` once values exceed `10^20`. |

## Do / don’t

- **Do** widen, then heal, in the same migration file.
- **Do** keep rolling refresh and idle zero-out in one transaction (**D2** / **A6**).
- **Don’t** wrap `insert_swap` + `upsert_trader` in one ingest transaction on this ticket.
- **Don’t** UPSERT senders inside the rolling window SQL.
- **Don’t** bind-mount `indexer/` into root Docker for cargo (`make test-indexer-target-ownership`).
- **Don’t** fold this into #1276.

## Key files

| Area | Path |
|------|------|
| Migration | `indexer/migrations/20260920120000_traders_raw_volume_numeric_38_0.sql` (name may shift; must `ALTER` then heal) |
| Rolling SQL + upsert + heal helper | [`indexer/src/db/queries/traders.rs`](../indexer/src/db/queries/traders.rs) |
| Ingest order | [`indexer/src/indexer/parser.rs`](../indexer/src/indexer/parser.rs) `insert_swap` then `update_trader_on_swap` |
| Aggregator | [`indexer/src/indexer/volume_aggregator.rs`](../indexer/src/indexer/volume_aggregator.rs) |
| JSON | [`indexer/src/api/traders.rs`](../indexer/src/api/traders.rs) |
| Tests | [`indexer/tests/indexer_volume_window_decay.rs`](../indexer/tests/indexer_volume_window_decay.rs), [`volume_usd_catalog.rs`](../indexer/tests/volume_usd_catalog.rs), [`api_traders.rs`](../indexer/tests/api_traders.rs), new skip/overflow test |

## Regression

```bash
make setup-indexer-postgres
make verify-issue-1277
make verify-issue-577
```

## Related

- [`AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md`](./AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md) — **D2** / **D5**
- [`AGENTS_FRONTEND_TRADER_VOLUME_USD.md`](./AGENTS_FRONTEND_TRADER_VOLUME_USD.md) — lifetime USD (**T553**); rolling stays raw API-only
- [`AGENTS_INDEXER_TRADER_POSITIONS_DECIMALS.md`](./AGENTS_INDEXER_TRADER_POSITIONS_DECIMALS.md) — skip-class precedent, different type
