# Design 1277 — trader rolling raw volume `NUMERIC(38, 0)`

**Issue:** [#1277](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1277)  
**Revision:** 2026-09-20-r2 (addresses `DESIGN: REVISE`: heal after `ALTER`, skip-class fixture, Coolify migrate, unsafe down-migration, `bd_plain_string`)  
**Canonical decision:** [ADR 0005](../docs/adr/0005-trader-rolling-volume-numeric.md)  
**Playbook:** [`skills/AGENTS_INDEXER_TRADER_ROLLING_VOLUME.md`](../skills/AGENTS_INDEXER_TRADER_ROLLING_VOLUME.md)

Ordinary leftover ops. Not a [#297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297) deploy/spend/custody/policy expansion. Not a founder card.

## Outcome

`volume_aggregator` commits trader 24h/7d/30d windows when a sender’s trailing `SUM(swap_events.offer_amount)` is ≥ `10^20` (about 100 human 18-dec tokens). Unscoped leaderboard `sort=volume_24h` ranks that integer. Senders whose `upsert_trader` overflowed after a committed swap — replay will not retry — appear on the board after migrate+heal with honest lifetime `total_trades` / `total_volume`. USD lifetime and #577 decay stay the previous contracts.

## Context

PostgreSQL `NUMERIC(38, 18)` stores `|x| < 10^20`. `refresh_rolling_volumes` already `LEAST`s the raw sum to `10^38-1` (the #548 integer cap for `NUMERIC(38, 0)` destinations). Assigning that value into `(38, 18)` overflows. Token / pair / global raw rollups widened in `20260817120000_*`. Trader rolling columns were renamed from `volume_*_usd` in `20260310000002_*` and never widened. `traders.total_volume` is the same leftover type; `upsert_trader` adds `offer_amount` with no `LEAST`.

`insert_swap` uses the pool (auto-commit). `trader_tracker::update_trader_on_swap` then `upsert_trader` runs after a successful insert. On numeric overflow the swap row stays. Replay takes `trade_exists` / `inserted.is_none()` and returns before trader upsert — the same skip class #676 repaired for positions. Rolling SQL only `UPDATE`s matching `traders.address`; missing senders stay off the unscoped board; undercounted `total_trades` / `total_volume` stay wrong even after a successful window stamp.

`TraderResponse` currently serializes raw volumes with `BigDecimal::to_string()` (`1e+19` risk). Issue **R1** already requires plain decimal JSON.

## Non-goals

- Indexer Coolify auto-deploy / `/health` git SHA ([#1276](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1276)).
- Re-opening #577 decay math or #576 trailing-window copy.
- Pair-scoped leaderboard GET `SUM(offer_amount)` ([#666](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/666)).
- `trader_positions` / P&L `NUMERIC(78, 18)` ([#676](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/676)).
- USD formula / P522-Q ingest / `refresh_trader_total_volume_usd` semantics ([#553](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553)).
- Frontend compact-format of raw rolling fields (API-only today).
- Wrapping `insert_swap` + `upsert_trader` in one ingest transaction (would roll back a committed swap class and change poller error semantics for an overflow this type change removes).
- UPSERT inside `refresh_rolling_volumes` (heal owns identity + lifetime totals; rolling SQL stays a window stamp + **D2** zero-out).

## Component / state / interface changes

| Area | Change |
|------|--------|
| Schema | New sqlx migration: `ALTER TABLE traders ALTER COLUMN volume_24h TYPE NUMERIC(38, 0)` (same for `volume_7d`, `volume_30d`, `total_volume`). Comment: raw `offer_amount` sums, not USD. `total_volume_usd` unchanged. |
| Heal SQL (same migration, after `ALTER`) | `INSERT INTO traders (address, total_trades, total_volume, first_trade_at, last_trade_at, …)` from `GROUP BY sender` on `swap_events` for addresses **not** in `traders`. Then `UPDATE` existing rows: `total_trades = COUNT(*)`, `total_volume = LEAST(SUM(offer_amount), 10^38-1)`, `first_trade_at` / `last_trade_at` from min/max `block_timestamp`. Do **not** set `total_volume_usd`. Preserve tier / registered / P&L columns. |
| `upsert_trader` | `total_volume = LEAST(traders.total_volume + $2, POWER(10::numeric, 38) - 1)` (and the insert branch `LEAST($2, …)`). USD `LEAST` stays `10^20 - 10^-18`. |
| `refresh_rolling_volumes` | Keep current two-statement tx: window `UPDATE … FROM SUM` + idle zero `UPDATE`. **D2** still applies. No `INSERT`. |
| Poller startup | Call the same heal SQL (Rust helper wrapping the migration statements) after migrate, idempotent, `traders` is small — same leftover class as `repair_positions_if_trade_count_mismatch` / `backfill_missing_swap_amm_fees`. |
| HTTP | `TraderResponse` / pair-scoped `total_volume`: `bd_plain_string`. Rolling zeros on pair-scoped rows stay `"0"`. P&L fields already mixed; this ticket owns raw **volume** strings only. |
| Verify | `make verify-issue-1277` → `scripts/qa/verify-issue-1277.sh` (Postgres only). |

## Affected invariants

Keep **D2** / **D5** ([#577](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/577)), **T553** USD cap, **P676** positions type isolation, **C1–C9** overview USD. Add **V1277-1–V1277-8** (ADR 0005). Trailing-window decay row in [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) gains a pointer that rolling raw volume is `(38, 0)` and must not fail the aggregator.

## Alternatives

| Option | Why not |
|--------|---------|
| Clamp rolling raw to USD `10^20` | Truncates honest ~100 human 18-dec volume (issue **A3**). |
| `NUMERIC(78, 18)` like #676 | Wrong unit class; integer offer sums match `offer_amount` / `pair_volume_24h.volume_quote`. |
| Widen without heal | Unblocks the 5-minute loop for wallets that already have a row; does not insert missing senders; does not recompute undercounted lifetime totals; replay skip remains. |
| UPSERT senders inside rolling refresh | Would insert missing addresses but would not repair `total_trades` / `total_volume`. Mixes window stamp with identity. Heal + UPDATE-only is enough. |
| Single ingest transaction around swap + trader | Larger poller semantics change; overflow class is removed by the type + cap. Crash-between-insert-and-upsert remains a heal/startup concern, not an ingest rewrite. |

## Complexity added / removed

**Added:** one migration, one idempotent heal query, one `LEAST` on lifetime raw add, JSON helper swap, skip-class test, poller startup call.  
**Removed:** aggregator error path for this overflow; operator need to rebuild traders by hand after 18-dec volume.  
Net: small. `traders` cardinality is low versus `swap_events`.

## Migration

1. `ALTER TYPE` the four raw columns to `NUMERIC(38, 0)` (rewrite is cheap; table is small).
2. Immediately run heal SQL in the same file (source of truth for Coolify apply).
3. Optional `COMMENT ON COLUMN` that these are raw offer sums.
4. No down SQL that narrows the type. Once any stored value is ≥ `10^20`, `ALTER … TYPE NUMERIC(38, 18)` fails. Rollback = revert indexer binary **keeping** the wide columns, or a new forward migration.

Suggested filename: `indexer/migrations/20260920120000_traders_raw_volume_numeric_38_0.sql`.

## Observability

Existing #577 **D5** logs: 5-minute loop `Failed to refresh rolling trader volumes` at error; startup at warn. After this change that line must not fire for a `10^21` in-window sender. Heal logs `traders_healed` count at info on startup (0 is success). No Prometheus ([#200](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/200)). Operator SQL: `\d traders` shows `numeric(38,0)` on the four raw columns and `numeric(38,18)` on `total_volume_usd`.

## Failure modes

| Failure | Handling |
|---------|----------|
| Hostile `offer_amount` at `10^38` | `LEAST` to `10^38-1`; no wrap to negative (**A1**). |
| Heal runs before `ALTER` | Forbidden ordering in the migration file. |
| Heal rewrites USD | Forbidden. Call `refresh_trader_total_volume_usd` only via existing #553 path if Coolify still needs a catalog re-stamp. |
| Statement timeout mid rolling tx | Keep one tx for window UPDATE + idle zero (**A6**); retry next loop. Heal is a separate statement/tx. |
| Process crash between `insert_swap` and `upsert_trader` after deploy | Startup heal inserts/recomputes. Rolling UPDATE then stamps windows. |
| Narrowing rollback | Forbidden once values exceed `10^20`. |

## Ordered implementation slices

1. **Schema + heal SQL** in one sqlx migration (`ALTER` then heal). No app code yet still leaves rolling UPDATE failing until the process loads the new type — ship with slice 2.
2. **`upsert_trader` `LEAST`** on raw lifetime add.
3. **`heal_trader_lifetime_from_swaps` helper** used by migration (SQL) and poller startup (Rust). Idempotent.
4. **`bd_plain_string`** on unscoped/pair-scoped raw volume JSON fields this response owns.
5. **Tests** (I1–I10, skip-class I10) + **`make verify-issue-1277`**.
6. **Docs:** invariants row, decay skill pointer, runbook sentence, `docs/testing.md`, `AGENTS.md` verify line.

Dependencies: none on open issues (`#548` / `#553` / `#577` / `#676` already landed). Slice 5 depends on 1–4. Coolify apply is migrate+restart after merge, not a separate issue.

## Tests

Reuse V3: mutate `block_timestamp`, no 24h sleeps. Postgres via `make setup-indexer-postgres`. `--test-threads=1`.

| ID | Case | Expect |
|----|------|--------|
| I1 | 24h swap `offer_amount = 10^21` | `refresh_rolling_volumes` `Ok`; `volume_24h = 10^21` |
| I2 | Same swap aged 25h | `volume_24h = 0`; `volume_7d = 10^21` |
| I3 | Aged 31d | all three rolling 0; `total_volume` still includes the swap (**D2**) |
| I4 | Two senders: 6-dec seed + 18-dec `10^21` | both refresh; 6-dec matches decay fixture |
| I5 | Empty DB / no swaps | zeros, no error |
| I6 | `upsert_trader` `10^21` then refresh | no overflow on either path; `LEAST` at `10^38-1` if the running sum exceeds |
| I7 | Leaderboard `sort=volume_24h` | 18-dec sender ranks by raw integer; JSON plain string (`bd_plain_string`, not `1e+19`) |
| I8 | `refresh_all_volume_windows(..., false)` | no `Failed to refresh rolling trader volumes` for I1 |
| I9 | Priced 6-dec USD | `total_volume_usd` still `(38, 18)`-legal; I1 raw does not smash USD |
| I10 | **Skip class:** `insert_swap` of `10^21` succeeds; force `upsert_trader` to fail with numeric overflow (narrow columns in a `serial` test, then restore); replay `trade_exists` / second insert returns `None`; run widen+heal; `refresh_rolling_volumes` is `Ok`; wallet is on the unscoped board with matching `total_trades` / capped `total_volume` | Reconstructs the Coolify leftover. If live `ALTER` on the shared pool is too brittle, equivalent: insert `swap_events` with no `traders` row (and a second sender with stale `total_trades`), call heal, then refresh + board assert — plus a dedicated overflow assertion on a temp `(38, 18)` column. Prefer the live narrow/restore under `#[serial]` with a drop guard so types are restored on panic. |

Issue I1–I9 remain. Attack rows **A1–A9** from the issue stay as review checks; **A3**/**A4** are forbidden product choices, not tests to implement as features.

## Rollout

1. Merge indexer change.
2. Coolify Postgres runs sqlx migrate (widen + heal). Table `traders` is small; expected seconds.
3. Indexer restart loads new binary (`LEAST` upsert, `bd_plain_string`, startup heal).
4. Next aggregator cycle (or startup `refresh_all_volume_windows(..., true)`) stamps rolling windows.

Do not treat env-only restart as enough: old binary + new columns is OK; new binary + old columns still overflows. Migrate first.

## Rollback

- Revert the indexer image **without** narrowing columns.
- Leave `(38, 0)` in place. A second forward migration may drop the startup heal if needed.
- **Do not** `ALTER … TYPE NUMERIC(38, 18)` after any `10^21` value exists.

## Integration completion criteria

- `\d traders`: four raw columns `numeric(38,0)`; `total_volume_usd` `numeric(38,18)`.
- I1 + I10 green; `make verify-issue-577` still green.
- Unscoped `GET /api/v1/traders/leaderboard?sort=volume_24h` for the I1 wallet: `volume_24h` is `"1000000000000000000000"` (plain digits).
- Coolify: migrate applied, indexer up, no rolling-trader error in the 5-minute log for that dataset.
- ADR 0005 + this file + skill **V1277-1–V1277-8** + invariants/runbook pointers land with the implement MR (design branch may already contain the docs).
