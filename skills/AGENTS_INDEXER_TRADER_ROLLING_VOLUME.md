# Agent playbook: trader rolling raw volume `NUMERIC(38, 0)` (#1277)

Audience: third-party agents changing `traders.volume_24h` / `7d` / `30d` / `total_volume`, `refresh_rolling_volumes`, or `upsert_trader` raw add.

**Issue:** [Forgejo **#1277**](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1277)  
**ADR:** [`docs/adr/0005-trader-rolling-volume-numeric.md`](../docs/adr/0005-trader-rolling-volume-numeric.md)  
**Slices:** [`design/1277-trader-rolling-volume-numeric.md`](../design/1277-trader-rolling-volume-numeric.md)  
**Sibling types:** [#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548) pair/global `(38, 0)`; [#676](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/676) positions `(78, 18)` — **not** this path  
**Decay:** [`AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md`](./AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md) (**D2** still applies)

## Problem class

`LEAST(SUM(offer_amount), 10^38-1)` does not fit `NUMERIC(38, 18)` (`|x| < 10^20`). One large 18-dec sender fails the whole rolling `UPDATE`; #577 idle zero-out shares that transaction. `insert_swap` commits first; overflowing `upsert_trader` leaves a swap with no (or stale) `traders` row. Replay skips via `trade_exists`. Rolling SQL is UPDATE-only. Overflow fails the whole upsert, so skip-class `volume_usd` never lands until #553 refresh after heal. `--cleanup-derived` leftover-lifetime rows (totals set, no remaining `swap_events`) would double-count on `upsert_trader` unless heal zeros them (not `DELETE`).

## Invariants (V1277-1–V1277-8)

| ID | Rule |
|----|------|
| **V1277-1** | `traders.volume_24h` / `volume_7d` / `volume_30d` / `total_volume` are **`NUMERIC(38, 0)`**. Do not switch these to `NUMERIC(78, 18)`. |
| **V1277-2** | Keep `LEAST(SUM(…), POWER(10,38)-1)` on rolling sums. Cap `upsert_trader` `total_volume + $2` the same way. Do not clamp raw volume to the USD `10^20` cap. |
| **V1277-3** | `total_volume_usd` stays `NUMERIC(38, 18)` / `10^20` USD cap. Heal INSERT / lifetime UPDATE / leftover-zero do **not** rewrite it. Idle **D2** zeros rolling columns only. Unpriced stays NULL (**R5**). Same heal transaction: **copy** `refresh_trader_total_volume_usd` SQL onto `&mut Transaction`, then leftover-USD `NULL` when no remaining priced `swap_events`. Do **not** call `refresh_trader_total_volume_usd(&pool)` inside `heal_trader_lifetime_from_swaps`. Do not change P522-Q for senders who still have priced swaps. |
| **V1277-4** | After overflow-skip, one-shot + poller heal from `swap_events`: `INSERT` missing senders (`ON CONFLICT DO NOTHING`); `UPDATE` existing including register-then-overflow; leftover-zero `UPDATE` (`total_trades = 0`, `total_volume = 0`, `first_trade_at` / `last_trade_at` `NULL`) when leftover-lifetime and **no** remaining `swap_events` — **do not `DELETE`** (keep tier / registered / P&L). Do **not** zero idle traders who still have old `swap_events` (**D2**). Block replay alone does not heal. In `poller.rs`, `heal_trader_lifetime_from_swaps` **before** `refresh_all_volume_windows(..., true)` (#1269 slot, not the #676 slot after refresh). Mismatch-gated with `COALESCE` like `#676` `positions_trade_count_diverges` (not `s.sender IS NOT NULL`): leftover-lifetime ghosts **trip**; registered / zero-lifetime ghosts **do not**. Scan is `GROUP BY sender` on `swap_events` (`idx_swaps_sender`), not “`traders` is small”. Heal writes + USD are **one transaction**. |
| **V1277-5** | `refresh_rolling_volumes` stays UPDATE-only once heal has inserted senders. |
| **V1277-6** | Unscoped rolling + `total_volume` **and** pair-scoped `total_volume` JSON uses **`bd_plain_string`** (no `1e+19`). Unscoped GET does not live-`SUM(swap_events)` for rolling windows (guardrail 6). |
| **V1277-7** | `make verify-issue-1277` after `make setup-indexer-postgres`. Canonical I10 is **not** live `ALTER TABLE traders` on `TEST_DATABASE_URL`: (1) overflow assert on a **temp** `(38, 18)` destination — assignment of `LEAST(10^38-1)` / `10^21` must fail the old type; (2) leftover-state heal — `swap_events` with no `traders` row **and** a second sender with stale `total_trades` (register-then-overflow); (3) leftover-lifetime ghost — `traders` row with `total_trades > 0` and **no** `swap_events` zeros raw lifetime + leftover USD, simulated `upsert_trader` does not double-count; registered `total_trades = 0` with no swaps must **not** trip the gate. Then heal (including USD priced + leftover-NULL in one tx) → rolling refresh `Ok` → unscoped board + capped `total_volume`. Also **A1**: running sum ≥ `10^38` → `LEAST` = `10^38-1` (I6 `10^21` never hits the cap). Crates: `indexer_volume_window_decay`, `volume_usd_catalog`, `api_traders`, plus the new overflow/skip test (`--test-threads=1`). |
| **V1277-8** | Coolify = sqlx migrate + indexer restart. No narrowing down-migration: rollback is not `ALTER` back to `(38, 18)` once values exceed `10^20`. |

## Do / don’t

- **Do** widen (`USING …::numeric(38, 0)`), then heal (INSERT + lifetime UPDATE + leftover-zero), then #553 USD UPDATE, then leftover-USD `NULL`, in the same migration file. Migration SQL is canonical; Rust **copies** it (including the #553 priced SQL) with a keep-in-sync comment (`backfill_swap_volume_usd`) and runs those statements in **one** transaction. sqlx migrations cannot call Rust. Do **not** call `refresh_trader_total_volume_usd(&pool)` from the helper.
- **Do** keep rolling refresh and idle zero-out in one transaction (**D2** / **A6**).
- **Do** call `heal_trader_lifetime_from_swaps` in `poller.rs` **before** `refresh_all_volume_windows(..., true)`.
- **Do** gate with `COALESCE` like #676. Leftover-lifetime ghosts trip; registered zeros do not.
- **Don’t** wrap `insert_swap` + `upsert_trader` in one ingest transaction on this ticket.
- **Don’t** UPSERT senders inside the rolling window SQL.
- **Don’t** `DELETE` leftover `traders` rows.
- **Don’t** call `refresh_trader_total_volume_usd(&pool)` **inside** `heal_trader_lifetime_from_swaps` (copy the #553 SQL onto `&mut Transaction`) **or** after a committed heal (crash window). Catalog `volume.rs` may still use the pool-level function.
- **Don’t** bind-mount `indexer/` into root Docker for cargo (`make test-indexer-target-ownership`).
- **Don’t** fold this into #1276.
- **Don’t** `ALTER TABLE traders` on the shared integration pool for I10.

## Key files

| Area | Path |
|------|------|
| Migration | `indexer/migrations/20260920120000_traders_raw_volume_numeric_38_0.sql` (name may shift; must `ALTER` + `USING`, then heal including leftover-zero, then #553 USD, then leftover-USD `NULL`) |
| Rolling SQL + upsert + heal copy | [`indexer/src/db/queries/traders.rs`](../indexer/src/db/queries/traders.rs) |
| Poller order | [`indexer/src/indexer/poller.rs`](../indexer/src/indexer/poller.rs) — heal **before** `refresh_all_volume_windows(..., true)` |
| Ingest order | [`indexer/src/indexer/parser.rs`](../indexer/src/indexer/parser.rs) `insert_swap` then `update_trader_on_swap` |
| Aggregator | [`indexer/src/indexer/volume_aggregator.rs`](../indexer/src/indexer/volume_aggregator.rs) |
| JSON | [`indexer/src/api/traders.rs`](../indexer/src/api/traders.rs) both `TraderResponse` constructors |
| Reorg | [`docs/runbooks/indexer-reorg-replay-dedup.md`](../docs/runbooks/indexer-reorg-replay-dedup.md) — leftover-zero `UPDATE` (not `DELETE`) in the heal tx after `--cleanup-derived` |
| Tests | [`indexer/tests/indexer_volume_window_decay.rs`](../indexer/tests/indexer_volume_window_decay.rs), [`volume_usd_catalog.rs`](../indexer/tests/volume_usd_catalog.rs), [`api_traders.rs`](../indexer/tests/api_traders.rs), new overflow/skip test |

## Regression

```bash
make setup-indexer-postgres
make verify-issue-1277
# equivalent crates (target may land with the implement MR):
# cd indexer && cargo test --test indexer_volume_window_decay --test volume_usd_catalog --test api_traders --test indexer_trader_rolling_volume_numeric -- --test-threads=1
make verify-issue-577
```

## Related

- [`AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md`](./AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md) — **D2** / **D5**
- [`AGENTS_FRONTEND_TRADER_VOLUME_USD.md`](./AGENTS_FRONTEND_TRADER_VOLUME_USD.md) — lifetime USD (**T553**); rolling stays raw API-only
- [`AGENTS_INDEXER_TRADER_POSITIONS_DECIMALS.md`](./AGENTS_INDEXER_TRADER_POSITIONS_DECIMALS.md) — skip-class precedent, different type; `COALESCE` mismatch-gate analog (leftover extras trip); positions `DELETE` is **not** the trader leftover path
