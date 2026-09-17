# ADR 0005: Persist every multihop AMM hop in `protocol_fee_events`

## Status

Proposed ([#1269](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1269), implement [PR #1274](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1274))

Indexer uniqueness + backfill. This ADR does **not** retune on-chain bps, add a `FeeSource`, change GET JSON shape, or expand deploy/spend/custody policy. Coolify indexer migrate + restart after this lands is ordinary leftover ops — not a founder card and not [#297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297) authority.

Playbook (do not duplicate here): [`skills/AGENTS_INDEXER_PROTOCOL_FEE_HOPS.md`](../../skills/AGENTS_INDEXER_PROTOCOL_FEE_HOPS.md) (**F1269-1–F1269-8**). Overview: [`architecture.md`](../architecture.md#indexer-protocol-fee-ledger). Invariants row: [`indexer-invariants.md`](../indexer-invariants.md) **Protocol fees (#586 / #1269)**.

## Outcome

Every factory-listed AMM hop with `commission_amount > 0` becomes one `protocol_fee_events` row (`source=swap_amm`). A router tx with two distinct pairs and per-pair `swap_index == 0` stores **two** hops. Same-pair `swap_index` 0 then 1 still stores two. Replay of the same hop inserts **zero** extra rows and never overwrites a stored amount.

After migrate + one-shot/poller backfill + one aggregator tick, trailing 7d `swap_amm` USD on `/protocol` equals the hop-complete priced `SUM` (existing clamp / unpriced rules), not the collision-truncated census. GET `/overview` and `/protocol/fees` stay O(1) rollup / 60s cache.

## Context

`parse_swaps` assigns `swap_index` **per pair** (pair B restarts at 0 — [#287](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/287)). That ordinal is correct for `swap_events` unique `(tx_hash, pair_id, swap_index)` and for fill linkage ([#316](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/316) / [#331](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/331)).

[#586](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/586) reused that per-pair index as a **per-tx** fee `ordinal` under `UNIQUE (tx_hash, source, ordinal)` with **no pair**. Hop 2+ in the same router tx collided; `ON CONFLICT DO NOTHING` kept the first hop. Volume was complete; treasury was not. `trade_exists` returned before fee ingest, so poller replay could not heal missing rows.

Operator reconstruction of one trailing 7-day AMM window: on-chain commissions ≈ **$176** vs dashboard ≈ **$110**. Dropping later hops that share `(tx_hash, swap_amm, ordinal=0)` closely reproduced the dashboard. Multihop is the default Swap path ([#101](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/101)); the gap is structural, not a display rounding bug.

Open children [#1209](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1209) / [#1210](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1210) / [#1211](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1211) must inherit the **widened** key ([#1213](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1213) **L1213**). They do not land in this slice.

## Non-goals

- Changing pair `fee_bps`, router hop count, or on-chain events.
- Adding `FeeSource` values (`pair_creation`, SKU invoices, cohort) — those stay #1209 / #1210 / #1211.
- Counting `spread_amount`, burn tax, gas, `hook_fee_amount` / AfterSwap, LP, book escrow, or `book_commission_amount` (L7 / **PFee-5** / **F1269-6**).
- `ON CONFLICT DO UPDATE` (replay/spoof must not replace treasury USD).
- Live `SUM(protocol_fee_events)` on GET (**PFee-8**).
- Rewriting non-null `fee_usd` from the live hub ([#568](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/568)).
- Frontend chrome of `ProtocolFeeStats` (no copy change unless labels lie).
- Reopening #287 or #586 ACs; DeFiLlama adapter nulls ([#687](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/687)); top-pairs volume ([#1263](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1263)).
- Deploy/spend/custody/policy expansion under agent-control #297.

## Decision

**Mirror #287.** Keep `swap_amm.ordinal = swap.swap_index`. Scope uniqueness with `pair_id`.

| Source | `pair_id` | Unique index |
|--------|-----------|--------------|
| `swap_amm` | factory `pairs.id` (NOT NULL) | `protocol_fee_events_pair_tx_source_ordinal_uidx` on `(tx_hash, source, pair_id, ordinal)` WHERE `pair_id IS NOT NULL` |
| wrap / unwrap / ust1_mint / ust1_redeem / book_take / limit_place | NULL | `protocol_fee_events_nopair_tx_source_ordinal_uidx` on `(tx_hash, source, ordinal)` WHERE `pair_id IS NULL` |

PostgreSQL `UNIQUE` treats NULL as distinct. A nullable `pair_id` **without** these partials would break wrap/window replay (**A6** / **F1269-3**). Sentinel `pair_id = 0` is rejected: `0` is not a `pairs.id`, and it would mix non-pair sources onto the pair-scoped index.

`insert_fee_event` uses inference `ON CONFLICT DO NOTHING` (no single-constraint target) so either partial can fire. Never `DO UPDATE`.

**Heal paths (both required):**

1. **Ingest:** `trade_exists` still skips swap insert, then calls `ingest_swap_amm_fee` (idempotent).
2. **Backfill:** one-shot SQL in migration `20260916120000_protocol_fee_events_pair_id.sql` plus poller startup `backfill_missing_swap_amm_fees`. Attach colliding NULL-`pair_id` `swap_amm` rows to the first matching hop (amount match preferred, then earliest `swap_events.id`). Insert remaining hops from `swap_events.commission_amount > 0`. Delete leftover NULL-`pair_id` `swap_amm` copies so the first hop is not double-counted. `fee_usd` NULL until the existing NULL-only stamp helper.

Do **not** change `parse_swaps` to a global-in-tx fee ordinal. That would fork fee rows from `swap_events` and from fill `swap_index`.

## Component / state / interface changes

| Layer | Change |
|-------|--------|
| Schema | Additive `protocol_fee_events.pair_id INT NULL REFERENCES pairs(id)`. Drop `UNIQUE (tx_hash, source, ordinal)`. Two partial unique indexes named above. Do not edit `20260821120000_protocol_fees.sql` in place. |
| Draft / insert | `FeeEventDraft.pair_id: Option<i32>`. `process_swap` / `ingest_swap_amm_fee` bind `pair.id` for `swap_amm`. Other sources keep `None`. |
| Parser | `trade_exists` early-return still skips swap insert; fee ingest retries. Zero-commission hops: no fee row. #285 undiscovered pair: no fee row. |
| Poller | Startup runs `backfill_missing_swap_amm_fees`; log + continue on error (next restart retries). Then existing volume aggregator refresh. |
| Rollup / GET | Unchanged contract. Refresh after backfill. No new JSON fields. `FeeSource::ALL` length stays 7 until #1209. |
| dApp | No required chrome change. |

## Affected invariants

| ID | Effect |
|----|--------|
| **F1269-1–F1269-8** | New hop uniqueness + backfill + wrap NULL semantics. |
| **PFee-8 / PFee-14** | GET still rollup-only; uniqueness named in census. |
| **L7 / PFee-5** | Hybrid still pool `commission_amount` + fill `commission_amount` once. |
| **#285** | Unreserved / undiscovered pair still produces no fee row. |
| **#568 / EFee** | Stamp at ingest / NULL-only backfill; never rewrite non-null `fee_usd`. |
| **L1213-6** | Children copy the **widened** key, not the colliding 3-column unique. |
| **#287 / L21 fill linkage** | `swap_index` remaining per-pair is intentional. |
| **C1 cursor** | Unchanged. Fee heal on replay does not require cursor rewind. |

## Alternatives

| Option | Why not |
|--------|---------|
| Per-tx global `swap_amm` ordinal; keep 3-column unique | Simpler schema, but fee rows no longer join `swap_events` on `(tx, pair, swap_index)`. Fill / hop debug forks. Still needs backfill. Do **not** mix global ordinals with per-pair `swap_index` on the same unique key. |
| `pair_id NOT NULL DEFAULT 0` sentinel | Avoids partial indexes, but `0` is not a pair and puts wrap/window on the pair-scoped key. Easy to leak wrap rows into AMM joins. |
| Nullable unique without partials | PG NULLS DISTINCT → two wrap rows with identical `(tx, source, ordinal)` both persist (**A6**). |
| `ON CONFLICT DO UPDATE` | Lets a later payload replace treasury USD. Forbidden. |
| Rely on indexer replay alone | `trade_exists` skipped fee ingest historically; even with retry, historical hops need SQL backfill from `swap_events`. |
| Close the gap by counting `book_commission_amount` / spread / hook | Violates L7 / PFee. |

## Complexity added / removed

**Added:** nullable `pair_id`; two partial unique indexes; attach + insert + delete backfill; ingest retry on `trade_exists`; inference `ON CONFLICT`.

**Removed:** lossy 3-column uniqueness that treated replay as success while dropping hops. No new HTTP route, source enum, or GET scan.

Net: one extra column and two indexes in exchange for fee census matching hop-complete volume.

## Migration

New file only: [`indexer/migrations/20260916120000_protocol_fee_events_pair_id.sql`](../../indexer/migrations/20260916120000_protocol_fee_events_pair_id.sql).

Order inside the migration: add column → drop old unique → create partials → attach colliding rows → insert missing hops → delete NULL-`pair_id` duplicates that now have a pair-scoped copy.

Idempotent: `IF NOT EXISTS` / `DROP IF EXISTS` / `ON CONFLICT DO NOTHING`. Poller SQL must stay in sync with the migration (`backfill_missing_swap_amm_fees`).

`fee_usd` on inserted hops is NULL until `backfill_null_fee_usd` (existing as-of helper). Unpriced hops stay NULL; activity + all unpriced → API `null`, not `$0`.

No wasm migrate. No factory `UpdateConfig`. No dApp env keys.

## Observability

- `tracing::info!(inserted, "backfilled missing swap_amm protocol fee hops")` when insert count > 0.
- `tracing::error!("swap_amm hop fee backfill failed: …")` on poller startup failure; process continues.
- After aggregator tick: `protocol_fee_stats_by_source` `swap_amm` `event_count` and `global_stats_24h.fee_event_count_*` rise by recovered hops (priced USD only in totals).
- GET still must not scan events. Operator check is SQL off the request path, then `/protocol` 7d AMM vs hop-complete priced SUM.

## Failure modes

| Mode | Behavior |
|------|----------|
| Duplicate delivery / reorg re-delivery | Partial unique + `DO NOTHING` → 0 new rows; amounts unchanged. |
| Backfill double-run | Unique key stops a second insert; delete step is existence-guarded. |
| Attach picks the wrong hop | Amount-match preferred, then earliest `swap_events.id`. Residual NULL-`pair_id` rows are deleted only when a pair-scoped copy exists for the same `(tx, ordinal)`. |
| Poller backfill SQL error | Log + continue; hops remain missing until next successful startup/migrate. Do not halt the cursor. |
| Undiscovered / spoof pair (#285) | No `pairs.id` → no `swap_amm` row. |
| Zero / non-positive commission | No fee row (`amount_raw > 0` CHECK). |
| Hostile huge `swap_index` | Existing parse fail-closed / i32; no overwrite of another hop. |
| Operator `SUM` on GET to “hide” missing ingest | Forbidden (DoS / V5). |
| Gem / vFDUSD identity in backfill USD | Same omit rules as #683; stamp NULL. |

## Ordered implementation slices

1. **Schema** — migration: `pair_id` + two partial uniques; drop 3-column unique. No ingest yet would still collide at runtime until slice 2.
2. **Ingest** — `FeeEventDraft.pair_id`; `insert_fee_event` inference conflict; `ingest_swap_amm_fee`; `trade_exists` retry.
3. **Backfill** — migration SQL + `backfill_missing_swap_amm_fees` + poller startup.
4. **Docs / verify** — invariants, runbooks, **PFee-14**, skill, `make verify-issue-1269`.
5. **Ops leftover** — Coolify indexer migrate + restart + aggregator tick; confirm 7d AMM vs hop-complete SUM; confirm replay does not inflate `fee_event_count`. Not this design PR.

**Dependencies (already on `main`, not open blockers):** #287 swap uniqueness, #586 fee ledger, #613/#614 wrap/window NULL `pair_id` sources, #683 stamp helper.

**Downstream (must wait on this key):** #1209 / #1210 / #1211.

## Tests

| ID | Path | Expect |
|----|------|--------|
| T1 | Parse pairA, pairA, pairB | `swap_index` 0, 1, 0. Fees: **3** `swap_amm` rows if all commissions > 0 |
| T2 | Insert same `(tx, swap_amm, pair, ordinal)` twice | Second `rows_affected == 0` |
| T3 | Same tx/source/ordinal, **different** pair | Both persist |
| T4 | 2-hop router wasm with commissions | `protocol_fee_events` 2; `swap_events` 2 |
| T5 | Replay T4 | Counts unchanged |
| T6 | Rollup 7d after T4 | `swap_amm` `event_count` + USD include both hops |
| T7 | `commission_amount = 0` | No fee row |
| T8 | Wrap ordinals 0 then 1, same tx | Both persist; replay deduped |
| T9 | Backfill already-complete txs | No duplicate `swap_amm` |
| T10 | `hybrid_counts_amm_and_book_once` | Still green |
| A1–A11 | Replay overwrite, spoof, NULL unique, GET SUM, L7 padding | Fail closed as in #1269 |

Harness: `indexer_protocol_fees.rs` + parser unit `parse_swaps_assigns_per_pair_swap_index` + `make verify-issue-1269`. Keep `verify-issue-586` / `613` / `614` / `683`. Postgres-only (`make setup-indexer-postgres`). No LocalTerra, no Vitest.

## Rollout

1. Merge schema + ingest + backfill (already in #1274 on `main`).
2. Deploy indexer binary + run migrations (sqlx on startup).
3. Confirm poller log for backfill insert count (may be 0 if migration already inserted).
4. Wait one aggregator tick (~5 min) or restart so `refresh_protocol_fee_stats` runs after `backfill_null_fee_usd`.
5. Compare `/protocol` 7d AMM to hop-complete priced SUM from `swap_events.commission_amount`. Collision-only reconstruction must no longer match the dashboard.
6. Restart / poller replay: `fee_event_count` must not inflate.

No frontend deploy required. No wasm store.

## Rollback

- **Binary-only rollback** (new indexes remain): ingest without `pair_id` would insert NULL-`pair_id` `swap_amm` and collide again on the non-pair partial; **do not** roll back the binary without rolling back the schema.
- **Schema rollback** (drop partials, restore 3-column unique): **re-introduces silent hop loss**. Only as a last resort after deleting pair-scoped extras; not a safe “undo census”.
- **Preferred:** fix-forward. Leave partial indexes in place.

This is not a chain halt, pause, or treasury rotate.

## Integration completion criteria

- AC1–AC9 from #1269 hold (two-pair `swap_index==0` → 2 rows; same-pair 0,1 → 2; replay 0 extras; wrap/window/book/place uniqueness unchanged; 7d AMM matches hop-complete priced SUM; GET does not scan events; L7; docs/verify; #285).
- `make verify-issue-1269` green (docs grep includes this ADR).
- After Coolify leftover: 7d AMM ≠ collision-truncated reconstruction; replay does not inflate `fee_event_count`.
- #1209 / #1210 / #1211 specs name the widened key (out of this slice to implement).

## Links

- Bug: [#1269](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1269)
- Implement: [PR #1274](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1274)
- Skill: [`AGENTS_INDEXER_PROTOCOL_FEE_HOPS.md`](../../skills/AGENTS_INDEXER_PROTOCOL_FEE_HOPS.md)
- Census: [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](../../skills/AGENTS_FRONTEND_PROTOCOL_STATS.md) **PFee-14**
- Fee-ledger home: [`AGENTS_INDEXER_FEE_LEDGER_HOME.md`](../../skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md)
- Runbooks: [`overview-global-stats-brin.md`](../runbooks/overview-global-stats-brin.md), [`indexer-reorg-replay-dedup.md`](../runbooks/indexer-reorg-replay-dedup.md)
- Verify: `make verify-issue-1269`
