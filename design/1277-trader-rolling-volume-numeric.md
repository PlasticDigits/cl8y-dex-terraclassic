# Design 1277 — trader rolling raw volume `NUMERIC(38, 0)`

**Issue:** [#1277](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1277)  
**Revision:** 2026-09-20-r3 (addresses `DESIGN: REVISE`: poller #1269 slot, full heal SQL, migration-canonical SQL, mismatch-gate, canonical I10, JSON/V1277-8, required #553 USD after heal, reorg trader heal)  
**Canonical decision:** [ADR 0005](../docs/adr/0005-trader-rolling-volume-numeric.md)  
**Playbook:** [`skills/AGENTS_INDEXER_TRADER_ROLLING_VOLUME.md`](../skills/AGENTS_INDEXER_TRADER_ROLLING_VOLUME.md)

Ordinary leftover ops. Not a [#297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297) deploy/spend/custody/policy expansion. Not a founder card.

## Outcome

`volume_aggregator` commits trader 24h/7d/30d windows when a sender’s trailing `SUM(swap_events.offer_amount)` is ≥ `10^20` (about 100 human 18-dec tokens). Unscoped leaderboard `sort=volume_24h` ranks that integer. Senders whose `upsert_trader` overflowed after a committed swap — replay will not retry — appear on the board after migrate+heal with honest lifetime `total_trades` / `total_volume`. Priced skip-class wallets get `total_volume_usd` from the existing #553 refresh after heal; unpriced stays NULL (**R5**). #577 decay stays the previous contract. Unscoped trader GET keeps the rollup (issue guardrail 6: no live `SUM(swap_events)` on unscoped trader GET).

## Context

PostgreSQL `NUMERIC(38, 18)` stores `|x| < 10^20`. `refresh_rolling_volumes` already `LEAST`s the raw sum to `10^38-1` (the #548 integer cap for `NUMERIC(38, 0)` destinations). Assigning that value into `(38, 18)` overflows. Token / pair / global raw rollups widened in `20260817120000_*`. Trader rolling columns were renamed from `volume_*_usd` in `20260310000002_*` and never widened. `traders.total_volume` is the same leftover type; `upsert_trader` adds `offer_amount` with no `LEAST`.

`insert_swap` uses the pool (auto-commit). `trader_tracker::update_trader_on_swap` then `upsert_trader` runs after a successful insert. On numeric overflow the swap row stays. Replay takes `trade_exists` / `inserted.is_none()` and returns before trader upsert — the same skip class #676 repaired for positions. Rolling SQL only `UPDATE`s matching `traders.address`; missing senders stay off the unscoped board; undercounted `total_trades` / `total_volume` stay wrong even after a successful window stamp. `upsert_trader` failure aborts the whole statement, so that swap’s `volume_usd` never lands on the trader; `refresh_trader_total_volume_usd` is UPDATE-only and is **not** on the 5-minute aggregator.

`TraderResponse` currently serializes raw volumes with `BigDecimal::to_string()` (`1e+19` risk) on **both** unscoped and pair-scoped constructors (`indexer/src/api/traders.rs`). Issue **R1** already requires plain decimal JSON.

Live `indexer/src/indexer/poller.rs` order: `#1269` `backfill_missing_swap_amm_fees` → `refresh_all_volume_windows(..., true)` (**D5**) → `#676` `repair_positions_if_trade_count_mismatch`. Missing senders must exist **before** the window stamp.

## Non-goals

- Indexer Coolify auto-deploy / `/health` git SHA ([#1276](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1276)).
- Re-opening #577 decay math or #576 trailing-window copy.
- Pair-scoped leaderboard GET `SUM(offer_amount)` ([#666](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/666) / issue **A9**). Stored unscoped rolling columns stay the rollup (guardrail 6).
- `trader_positions` / P&L `NUMERIC(78, 18)` ([#676](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/676)).
- USD formula / P522-Q ingest / changing `refresh_trader_total_volume_usd` semantics ([#553](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553)). This ticket **calls** that function after heal; it does not change it.
- Frontend compact-format of raw rolling fields (API-only today).
- Wrapping `insert_swap` + `upsert_trader` in one ingest transaction (would roll back a committed swap class and change poller error semantics for an overflow this type change removes).
- UPSERT inside `refresh_rolling_volumes` (heal owns identity + lifetime totals; rolling SQL stays a window stamp + **D2** zero-out).
- Clamping raw volume to the USD `10^20` cap.
- Switching these columns to `(78, 18)`.

## Component / state / interface changes

| Area | Change |
|------|--------|
| Schema | New sqlx migration: `ALTER TABLE traders ALTER COLUMN volume_24h TYPE NUMERIC(38, 0) USING volume_24h::numeric(38, 0)` (same `USING` for `volume_7d`, `volume_30d`, `total_volume`). Assignment cast rounds; existing values are integer offer sums so the round is a no-op. Comment: raw `offer_amount` sums, not USD. `total_volume_usd` unchanged. |
| Heal SQL (same migration, after `ALTER`) | Canonical SQL below. `INSERT` lists only identity + lifetime raw columns (no ellipsis). `ON CONFLICT (address) DO NOTHING` so concurrent `upsert_trader_tier` cannot abort Coolify migrate. Then `UPDATE` existing senders (covers register-then-overflow: `upsert_trader_tier` row with `total_trades = 0`). Do **not** set `volume_24h` / `volume_7d` / `volume_30d` or `total_volume_usd` in either statement. Preserve tier / registered / P&L via defaults / untouched columns. |
| #553 USD after heal | Same migration file, **after** heal INSERT/UPDATE: copy of `refresh_trader_total_volume_usd` SQL. Required, not optional. Unpriced stays NULL (**R5**). Heal INSERT/UPDATE still does not `SET` USD (**V1277-3**). |
| `upsert_trader` | `total_volume = LEAST(traders.total_volume + $2, POWER(10::numeric, 38) - 1)` (and the insert branch `LEAST($2, …)`). USD `LEAST` stays `10^20 - 10^-18`. |
| `refresh_rolling_volumes` | Keep current two-statement tx: window `UPDATE … FROM SUM` + idle zero `UPDATE`. **D2** still applies. No `INSERT`. |
| Poller startup | In `indexer/src/indexer/poller.rs`, call `heal_trader_lifetime_from_swaps` **before** `refresh_all_volume_windows(..., true)` (the `#1269` slot, after `backfill_missing_swap_amm_fees`). Do **not** copy the `#676` slot after refresh — that leaves post-deploy skip-class wallets off `sort=volume_24h` until the 5-minute loop. Mismatch-gated (missing sender **or** `total_trades` / capped `total_volume` diverge). When the gate fires, run heal SQL then `refresh_trader_total_volume_usd`. |
| HTTP | Unscoped `TraderResponse` raw volumes **and** pair-scoped `total_volume`: `bd_plain_string`. Rolling zeros on pair-scoped rows stay `"0"`. P&L fields already mixed; this ticket owns raw **volume** strings only. |
| Verify | `make verify-issue-1277` → `scripts/qa/verify-issue-1277.sh` (Postgres only). Target may land with the implement MR. Crates: `indexer_volume_window_decay`, `volume_usd_catalog`, `api_traders`, plus the new overflow/skip test; `--test-threads=1`. |

### Heal SQL (canonical; migration file is source of truth)

Defaults from `20260310000001` + later ADDs cover rolling (`volume_24h/7d/30d` DEFAULT 0), tier (`tier_id` 0 / `tier_name` `'Default'` / `registered` false), P&L, `created_at` / `updated_at`. Do not list those on INSERT. Do not write `volume_24h` / `volume_7d` / `volume_30d` or `total_volume_usd`.

`upsert_trader` (`traders.rs` 41–53) stamps `first_trade_at` / `last_trade_at` with ingest `NOW()`, not `block_timestamp`. Heal INSERT uses `min/max(block_timestamp)` (chain time). The UPDATE below **scopes** timestamp rewrite to mismatched rows (missing senders are the INSERT; existing rows where `total_trades` or capped `total_volume` diverge, including register-then-overflow). Healthy traders keep ingest `NOW()`. Do **not** rewrite every trader’s timestamps on migrate or on a gated boot.

```sql
-- After ALTER … TYPE NUMERIC(38, 0) USING …::numeric(38, 0)

INSERT INTO traders (address, total_trades, total_volume, first_trade_at, last_trade_at)
SELECT
  se.sender,
  COUNT(*)::bigint,
  LEAST(SUM(se.offer_amount), POWER(10::numeric, 38) - 1),
  MIN(se.block_timestamp),
  MAX(se.block_timestamp)
FROM swap_events se
GROUP BY se.sender
ON CONFLICT (address) DO NOTHING;

UPDATE traders t
SET
  total_trades = sub.cnt,
  total_volume = sub.vol,
  first_trade_at = CASE
    WHEN t.total_trades IS DISTINCT FROM sub.cnt
      OR t.total_volume IS DISTINCT FROM sub.vol
      OR t.first_trade_at IS NULL
    THEN sub.first_ts
    ELSE t.first_trade_at
  END,
  last_trade_at = CASE
    WHEN t.total_trades IS DISTINCT FROM sub.cnt
      OR t.total_volume IS DISTINCT FROM sub.vol
      OR t.last_trade_at IS NULL
    THEN sub.last_ts
    ELSE t.last_trade_at
  END,
  updated_at = NOW()
FROM (
  SELECT
    sender,
    COUNT(*)::bigint AS cnt,
    LEAST(SUM(offer_amount), POWER(10::numeric, 38) - 1) AS vol,
    MIN(block_timestamp) AS first_ts,
    MAX(block_timestamp) AS last_ts
  FROM swap_events
  GROUP BY sender
) sub
WHERE t.address = sub.sender;

-- Required #553 stamp (copy of refresh_trader_total_volume_usd). Unpriced stays NULL (R5).
UPDATE traders t
SET total_volume_usd = sub.usd,
    updated_at = NOW()
FROM (
  SELECT
    sender,
    LEAST(SUM(volume_usd), POWER(10::numeric, 20) - POWER(10::numeric, -18)) AS usd
  FROM swap_events
  WHERE volume_usd IS NOT NULL AND volume_usd > 0
  GROUP BY sender
) sub
WHERE t.address = sub.sender;
```

Rust `heal_trader_lifetime_from_swaps` **copies** the INSERT + lifetime UPDATE (keep-in-sync comment, same pattern as `volume.rs` `backfill_swap_volume_usd` / #548). sqlx migrations cannot call Rust. After those writes, call existing `refresh_trader_total_volume_usd` (do not duplicate USD formula).

Mismatch-gate (`trader_lifetime_diverges_from_swaps`), analogous to `#676` `positions_trade_count_diverges` — **not** an unconditional rewrite, and **not** “`traders` is small”. The scan is `GROUP BY sender` over `swap_events` (`idx_swaps_sender` exists). Skip writes when aligned:

```sql
SELECT EXISTS (
  SELECT 1
  FROM (
    SELECT sender,
           COUNT(*)::bigint AS cnt,
           LEAST(SUM(offer_amount), POWER(10::numeric, 38) - 1) AS vol
    FROM swap_events
    GROUP BY sender
  ) s
  FULL OUTER JOIN traders t ON t.address = s.sender
  WHERE s.sender IS NOT NULL
    AND (
      t.address IS NULL
      OR t.total_trades IS DISTINCT FROM s.cnt
      OR t.total_volume IS DISTINCT FROM s.vol
    )
);
```

Ghost traders with no swaps (`s.sender IS NULL`) do not trip the gate. Migration always runs heal (one-shot after `ALTER`). Poller uses the gate; when it fires, heal then #553 USD refresh.

## Affected invariants

Keep **D2** / **D5** ([#577](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/577)), **T553** USD cap, **P676** positions type isolation, **C1–C9** overview USD. Add **V1277-1–V1277-8** (ADR 0005). Trailing-window decay row in [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) gains a pointer that rolling raw volume is `(38, 0)` and must not fail the aggregator. Unscoped GET stays on the rollup (guardrail 6). **R5:** unpriced `total_volume_usd` stays NULL.

## Alternatives

| Option | Why not |
|--------|---------|
| Clamp rolling raw to USD `10^20` | Truncates honest ~100 human 18-dec volume (issue **A3**). |
| `NUMERIC(78, 18)` like #676 | Wrong unit class; integer offer sums match `offer_amount` / `pair_volume_24h.volume_quote`. |
| Widen without heal | Unblocks the 5-minute loop for wallets that already have a row; does not insert missing senders; does not recompute undercounted lifetime totals; replay skip remains. |
| UPSERT senders inside rolling refresh | Would insert missing addresses but would not repair `total_trades` / `total_volume`. Mixes window stamp with identity. Heal + UPDATE-only is enough. |
| Single ingest transaction around swap + trader | Larger poller semantics change; overflow class is removed by the type + cap. Crash-between-insert-and-upsert remains a heal/startup concern, not an ingest rewrite. |
| Unconditional poller `GROUP BY sender` every boot | #676 is mismatch-gated (`positions_trade_count_diverges` then rebuild). Copy that: skip writes when lifetime already matches. The gate itself still scans `swap_events`; it does not rewrite `traders` when aligned. |
| Heal after `refresh_all_volume_windows` (#676 slot) | **D5** window stamp would miss skip-class senders until the 5-minute loop. Poller order is the #1269 slot (before first rollup). |

## Complexity added / removed

**Added:** one migration (`ALTER` + heal + #553 USD UPDATE), Rust copy of heal SQL with keep-in-sync comment, mismatch-gate, one `LEAST` on lifetime raw add, JSON helper swap, I10 + **A1** fixtures, poller call in the #1269 slot.  
**Removed:** aggregator error path for this overflow; operator need to rebuild traders by hand after 18-dec volume.  
Net: small. Heal cost is `GROUP BY sender` on `swap_events` (`idx_swaps_sender`), not “`traders` is small”. The `ALTER` rewrite of `traders` is a small table; the leftover scan is not.

## Migration

1. `ALTER … TYPE NUMERIC(38, 0) USING <col>::numeric(38, 0)` on the four raw columns.
2. Immediately run heal INSERT + lifetime UPDATE in the same file (source of truth for Coolify apply).
3. Immediately run the #553 `total_volume_usd` UPDATE in the same file (required).
4. Optional `COMMENT ON COLUMN` that these are raw offer sums.
5. No down SQL that narrows the type. Once any stored value is ≥ `10^20`, `ALTER … TYPE NUMERIC(38, 18)` fails. Rollback = revert indexer binary **keeping** the wide columns, or a new forward migration.

Suggested filename: `indexer/migrations/20260920120000_traders_raw_volume_numeric_38_0.sql`.

## Observability

Existing #577 **D5** logs: 5-minute loop `Failed to refresh rolling trader volumes` at error; startup at warn. After this change that line must not fire for a `10^21` in-window sender. Heal logs `traders_healed` count at info when the gate fires (aligned skip logs nothing or debug). No Prometheus ([#200](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/200)). Operator SQL: `\d traders` shows `numeric(38,0)` on the four raw columns and `numeric(38,18)` on `total_volume_usd`.

## Failure modes

| Failure | Handling |
|---------|----------|
| Hostile `offer_amount` at `10^38` | `LEAST` to `10^38-1`; no wrap to negative (**A1** fixture). |
| Heal runs before `ALTER` | Forbidden ordering in the migration file. |
| Heal INSERT/UPDATE sets USD | Forbidden (**V1277-3**). Required follow-on is the existing #553 UPDATE / `refresh_trader_total_volume_usd` (unpriced stays NULL, **R5**). |
| Concurrent `upsert_trader_tier` during Coolify migrate | `ON CONFLICT (address) DO NOTHING` on INSERT; UPDATE then repairs `total_trades = 0`. |
| Statement timeout mid rolling tx | Keep one tx for window UPDATE + idle zero (**A6**); retry next loop. Heal is a separate statement/tx. |
| Process crash between `insert_swap` and `upsert_trader` after deploy | Startup heal in the #1269 slot inserts/recomputes **before** `refresh_all_volume_windows(..., true)`. |
| `--cleanup-derived` then re-ingest | Additive `upsert_trader` would double-count leftover lifetime. Startup heal recomputes from remaining `swap_events` before the poll loop. |
| Narrowing rollback | Forbidden once values exceed `10^20` (**V1277-8**). |

## Ordered implementation slices

1. **Schema + canonical heal SQL + #553 USD UPDATE** in one sqlx migration (`ALTER` with `USING`, then INSERT/UPDATE heal, then USD stamp). No app code yet still leaves rolling UPDATE failing until the process loads the new type — ship with slice 2.
2. **`upsert_trader` `LEAST`** on raw lifetime add.
3. **`heal_trader_lifetime_from_swaps` + `trader_lifetime_diverges_from_swaps`** in `traders.rs`. Migration SQL is canonical; Rust copies it with a keep-in-sync comment (`volume.rs` `backfill_swap_volume_usd`). Poller calls the helper **before** `refresh_all_volume_windows(..., true)` in `poller.rs`. When the gate fires, call `refresh_trader_total_volume_usd` after heal writes.
4. **`bd_plain_string`** on unscoped rolling + `total_volume` **and** pair-scoped `total_volume`.
5. **Tests** (I1–I10, I7b, **A1**) + **`make verify-issue-1277`** (crates named below; Makefile target may land with the implement MR).
6. **Docs:** invariants row **V1277-1–V1277-8**, decay skill pointer, reorg runbook trader-heal sentence, `docs/testing.md`, `AGENTS.md` verify line.

Dependencies: none on open issues (`#548` / `#553` / `#577` / `#676` already landed). Slice 5 depends on 1–4. Coolify apply is migrate + indexer restart after merge, not a separate issue.

## Tests

Reuse V3: mutate `block_timestamp`, no 24h sleeps. Postgres via `make setup-indexer-postgres`. `--test-threads=1`. Do **not** `ALTER TABLE traders` on the shared `TEST_DATABASE_URL` pool (`#[serial]` with #577 / #553).

| ID | Case | Expect |
|----|------|--------|
| I1 | 24h swap `offer_amount = 10^21` | `refresh_rolling_volumes` `Ok`; `volume_24h = 10^21` |
| I2 | Same swap aged 25h | `volume_24h = 0`; `volume_7d = 10^21` |
| I3 | Aged 31d | all three rolling 0; `total_volume` still includes the swap (**D2**) |
| I4 | Two senders: 6-dec seed + 18-dec `10^21` | both refresh; 6-dec matches decay fixture |
| I5 | Empty DB / no swaps | zeros, no error |
| I6 | `upsert_trader` `10^21` then refresh | no overflow on either path. `10^21` does **not** hit `10^38-1`; cap is **A1** |
| I7 | Unscoped leaderboard `sort=volume_24h` | 18-dec sender ranks by raw integer; JSON plain string (`bd_plain_string`, not `1e+19`) |
| I7b | Pair-scoped leaderboard `total_volume` | same `bd_plain_string` (both `TraderResponse` constructors in `api/traders.rs`) |
| I8 | `refresh_all_volume_windows(..., false)` | no `Failed to refresh rolling trader volumes` for I1 |
| I9 | Priced 6-dec USD | `total_volume_usd` still `(38, 18)`-legal; I1 raw does not smash USD; unpriced stays NULL (**R5**) |
| I10 | Canonical skip leftover (two parts; both required) | **(1)** Overflow assert on a **temp** `(38, 18)` destination (or a dedicated overflow test): assignment of `LEAST(10^38-1)` and of `10^21` must fail the old type. **(2)** Leftover-state heal: `swap_events` with no `traders` row **and** a second sender with stale `total_trades` (register-then-overflow), then heal → `refresh_trader_total_volume_usd` → refresh `Ok` → unscoped board + capped `total_volume`. Priced orphan gets USD; unpriced stays NULL. Parser skip (`insert_swap` → overflow `upsert_trader` → `trade_exists`) is the production cause of this row shape; I10 does not replay it on the shared schema. |
| A1 | `offer_amount` / running sum ≥ `10^38` | `LEAST` = `10^38-1`; no wrap to negative. I6 never hits this cap. |

Issue I1–I9 remain. Attack expected behavior (not a “review checklist” dump):

- **A2:** Negative / non-finite offer is rejected at ingest (`swap_events` NUMERIC / CHECKs). Rolling `SUM` never sees those rows. Do not `COALESCE` NULL into USD.
- **A3 / A4:** Forbidden product choices (USD `10^20` clamp on raw; zero USD to “heal” overflow).
- **A5:** Rolling SQL stays bound timestamps only; no user `window` string concat.
- **A6:** One transaction for window UPDATE + idle zero-out; failed refresh logs and retries next loop (no torn 24h vs 30d).
- **A7:** After I7, huge raw volume sorts as integer, not NULL-first.
- **A8:** Future `block_timestamp` inflates the window (same as #577 A1). Do not clamp ingest with `Utc::now()`; document.
- **A9:** Pair-scoped GET `SUM(offer_amount)` is out of scope for stored columns; must not 500 the unscoped rollup refresh.

`make verify-issue-1277` (implement MR may add the Makefile target):

```bash
make setup-indexer-postgres
cd indexer && cargo test --test indexer_volume_window_decay --test volume_usd_catalog --test api_traders --test indexer_trader_rolling_volume_numeric -- --test-threads=1
```

The new crate name may shift; it must contain I10 + **A1**. Still run `make verify-issue-577`.

## Rollout

1. Merge indexer change.
2. Coolify Postgres runs sqlx migrate (widen + heal + #553 USD stamp).
3. Indexer restart loads new binary (`LEAST` upsert, `bd_plain_string`, mismatch-gated startup heal in the #1269 slot).
4. Startup `refresh_all_volume_windows(..., true)` stamps rolling windows (**D5**) after heal.

Do not treat env-only restart as enough: old binary + new columns is OK; new binary + old columns still overflows. Migrate first. Coolify = sqlx migrate + indexer restart (**V1277-8**).

## Rollback

- Revert the indexer image **without** narrowing columns.
- Leave `(38, 0)` in place. A second forward migration may drop the startup heal if needed.
- **Do not** `ALTER … TYPE NUMERIC(38, 18)` after any `10^21` value exists (**V1277-8**). No narrowing down-migration.

## Integration completion criteria

- `\d traders`: four raw columns `numeric(38,0)`; `total_volume_usd` `numeric(38,18)`.
- I1 + I10 + **A1** green; `make verify-issue-577` still green.
- Unscoped `GET /api/v1/traders/leaderboard?sort=volume_24h` for the I1 wallet: `volume_24h` is `"1000000000000000000000"` (plain digits). Pair-scoped `total_volume` is also plain digits (**I7b**).
- Coolify: migrate applied, indexer up, no rolling-trader error in the 5-minute log for that dataset.
- ADR 0005 + this file + skill **V1277-1–V1277-8** + invariants/runbook pointers land with the implement MR (design branch may already contain the docs).
