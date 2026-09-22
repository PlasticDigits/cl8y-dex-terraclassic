# ADR 0012: CMM-held v2 LP — 30d fee bps and volume rollover

## Status

Proposed ([#1317](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1317)). This ADR is the versioned contract. Implement keeps **Proposed** until a reviewer accepts the design. Do not self-mark **Accepted**.

ADR **0008** is reserved on `cac-design-issue-1300`. ADR **0010** is reserved on `cac-design-issue-1305`. ADR **0011** is reserved on `cac-design-issue-1311`. This ticket is **0012**. Do not merge this design branch (`cac-design-issue-1317`) as a design-only PR.

Playbook: [`skills/AGENTS_INDEXER_CMM_LP_CENSUS.md`](../../skills/AGENTS_INDEXER_CMM_LP_CENSUS.md) (**I1317** / **P1317**). Overview: [`architecture.md`](../architecture.md#cmm-lp-census). Ranking that this does not reopen: [#1263](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1263) (**I1263**). Fee uniqueness this amends only for book attribution: [#1269](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1269) (**F1269-3**).

**Slice-0 apply — insert; do not whole-file checkout.** Overlap files also move on `cac-design-issue-1300`, `cac-design-issue-1305`, and `cac-design-issue-1311`: `docs/architecture.md`, `docs/indexer-invariants.md`, `docs/frontend.md`, `docs/README.md`, `docs/testing.md`, `AGENTS.md`, `skills/AGENTS_FRONTEND_PROTOCOL_STATS.md`, `skills/AGENTS_INDEXER_PROTOCOL_FEE_HOPS.md`. Land those as inserts of the #1317 sections. The new ADR and `skills/AGENTS_INDEXER_CMM_LP_CENSUS.md` are additive files.

## Outcome

`/protocol` **Top pairs (30d)** keeps today’s top-5 membership, order, gem filter, and full-pool **Vol/LP**. Three columns follow **Vol/LP**:

| Column | Meaning |
|--------|---------|
| **CMM v2 LP** | Trailing-30-day time-weighted USD of factory LP shares held by the CMM custodian |
| **30d fee bps** | Pair trading fees over that same window, in basis points of that CMM LP |
| **Vol/CMM LP** | Trailing 30d volume ÷ that CMM LP, rendered with the existing `×` formatter |

Worked row (API plain strings): CMM LP `500`, trading fees `10`, volume `1234.5` → `fees_bps_per_cmm_lp = 200`, `volume_per_cmm_lp = 2.469`. The volume cell uses `formatVolumePerTvl`, which already renders `2.469` as `2.47×` (three significant figures, never `%`). The fee cell shows `200` with no `%` and no `×`. Full-pool Vol/LP is unchanged.

`GET /api/v1/protocol/top-pairs` reads stamps only. Missing, non-positive, or overflow (`≥ 10^20`) CMM LP or ratio is omitted (same as `liquidity_usd` / `volume_per_tvl`), never `Infinity`. The dApp shows an em dash. Items that lack the new keys (indexer not migrated) still render the original four columns and em dashes in the three new cells.

## Context

[#1263](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1263) ranks five factory pairs by `pair_volume_30d` joined to current `pair_liquidity_usd`. **Vol/LP** is trailing volume ÷ **everyone’s** spot pool USD. [#1207](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1207) stays a farming memo. This census is not an APR.

`liquidity_events.provider` is the provide **sender**. Pair `provide_liquidity` also emits **`receiver`**, and the mint runbooks provide with `receiver` = the CMM treasury while the admin wallet is the sender, then CW20-transfer leftover LP to that treasury. A sum of `provider = CMM` misses that inventory. There is no LP-share index and no time-weighted custodian balance.

`protocol_fee_events.pair_id` is set for `swap_amm` only. `book_take` and `limit_place` are ingested with `pair_id: None` even though the parser already resolved `pair.id` ([ADR 0005](./0005-protocol-fee-multihop-hops.md) **F1269-3**). Wrap, unwrap, and UST1 window fees are not pair trading fees.

Venue-wide fee StatBoxes stay a protocol total. They are not a CMM balance.

## Non-goals

- Re-ranking top pairs by fee bps or by CMM volume rollover. Changing **Vol/LP**, `GET /pairs`, `GET /overview`, or DeFiLlama adapters.
- Farm, APR, points, fee-split, `FEE_CONFIG` change, LP custody moves, or `/pool` incentive copy. [#1207](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1207) and [#558](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/558) stay as they are.
- Per-address portfolio LP. A new custodian env var or a second hardcoded bech32 in Rust.
- Wasm / factory migrate. Counting wrap, unwrap, `ust1_mint`, `ust1_redeem`, spread, burn tax, gas, or hook fees in the bps numerator.
- LCD or raw-event scans on the GET path. Deploy, spend, custody, or policy expansion under [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297).

## Decision

### Custodian

Reuse `CMM_GOVERNANCE_ADDR` (`Config.cmm_governance_addr`). That pin is already the documented CMM treasury (columbus-5 value in `indexer/.env.example`; mint runbooks use the same bech32 as LP receiver). Empty or invalid → every new field omitted. Do not fall back to a literal address in code. Compare holders case-insensitively; store lowercase.

### CMM LP denominator

Time-weighted **raw LP shares** held by that custodian over `[now − 30d, now]`, marked at the **current** pool USD per share:

```
cmm_lp_usd = tw_shares / total_supply × pair_liquidity_usd
```

`pair_liquidity_usd` is the existing #655 / #569 catalog stamp (full-pool v2 TVL). `total_supply` is the LP CW20 `token_info.total_supply` from the latest off-request checkpoint. Both are raw-share and human-USD consistent: the ratio is USD per raw share. This is not full-pool TVL and not “pool minus CMM”. It is not an integral of historical pool USD. A price move marks the average share balance at today’s catalog. The tooltip says so. It is not an APR.

**Ledger** `lp_holder_deltas`. Chain inserts name their conflict target:

```sql
INSERT INTO lp_holder_deltas (...)
ON CONFLICT (tx_hash, pair_id, kind, holder, event_index)
WHERE kind IN ('provide', 'withdraw', 'transfer_out', 'transfer_in')
DO NOTHING
```

Unique index `lp_holder_deltas_event_uidx` matches that column list and predicate. `event_index` is the 0-based wasm-event index in the transaction walk, so two transfers of the same holder in one transaction both insert. Chain rows require non-null `tx_hash` and `event_index` (a null `event_index` would not dedupe). `reconcile` rows leave both null. Do not call `liquidity_event_exists`. That check is `(tx_hash, pair_id, event_type)` and collapses a pair to one add or one remove per transaction.

| Kind | Holder | Signed shares | Wasm keys |
|------|--------|---------------|-----------|
| `provide` | `receiver`, or `sender` when `receiver` is absent | `+share` | pair `action=provide_liquidity` |
| `withdraw` | `sender` | `−withdrawn_share` | pair `action=withdraw_liquidity` |
| `transfer_out` | `from` | `−amount` | LP-token `transfer`, `transfer_from`, or `send` |
| `transfer_in` | `to` | `+amount` | same event as `transfer_out` |
| `reconcile` | custodian | LCD balance minus expected | not a wasm event |

Match `transfer` / `transfer_from` / `send` only when the emitter is `pairs.lp_token`. Ignore `mint` and `burn`. Ignore a transfer or send when `from` or `to` is the pair contract, so a withdraw `send` is not a second debit. The poller already walks every transaction in the block. Parsing these attributes does not add a subscription.

Reconciliation rows use a different target: unique index `lp_holder_deltas_reconcile_uidx` on `(pair_id, holder, block_height) WHERE kind = 'reconcile'`, inserted with `ON CONFLICT (pair_id, holder, block_height) WHERE kind = 'reconcile' DO NOTHING`.

**Checkpoint** `lp_custody_checkpoints` (one row per pair + holder), written off the GET path. Production LCD queries run only for `CMM_GOVERNANCE_ADDR`. Do not LCD every LP holder.

Columns: `shares`, `observed_at`, `observed_height`, `total_supply`, `last_lcd_at`. Primary key `(pair_id, holder)`. Tests inject a balance reader. Production uses the LCD client. No checkpoint query on GET.

**The first successful read is a fixed anchor.** Insert only when the row is missing.

- `shares` is that read’s CW20 `balance`.
- `observed_height` is the LCD query height. It is greater than or equal to the height of the last delta included in that balance. Deltas with `block_height <= observed_height` are already inside `shares` and are not applied again.
- `observed_at` is the block time of `observed_height`, not wall-clock `Utc::now()`.
- `total_supply` is `token_info.total_supply` from the same read. `last_lcd_at` starts equal to `observed_at`.

Later reads must not `UPDATE` `shares`, `observed_at`, or `observed_height`.

**Later LCD reads** happen at most once per 24 hours, gated by `last_lcd_at`, and only for the custodian. A newer delta does not call LCD and does not move the anchor. On success:

- `expected = anchor.shares + Σ deltas` whose `block_height` is greater than `observed_height` and less than or equal to this query height (chain deltas and earlier `reconcile` rows).
- Store `lcd_balance − expected` as one `reconcile` delta at this query’s block time and height when the difference is not zero.
- `UPDATE` only `total_supply` and `last_lcd_at`.

On LCD failure, keep the anchor and still fold ledger deltas. If no anchor exists, the stamp is NULL. Do not replace `[window_start, latest_observed_at)` with the latest spot balance.

**Integral** (pure function, unit-tested):

- No anchor → `cmm_lp_usd` NULL (unknown). Do not treat “no rows” as zero. That is the idle/unknown em dash.
- Anchor `observed_at` **inside** the window: `[window_start, observed_at)` holds `anchor.shares` constant. That prefix belongs to the first anchor only. It ages out once `observed_at` is before `window_start`. A later LCD read does not slide the prefix forward.
- Deltas apply only when `block_height > observed_height`, as steps at `block_timestamp`. If that timestamp is earlier than `observed_at`, clamp the step to `observed_at`.
- Anchor **before** the window: opening balance = `anchor.shares` + deltas after `observed_height` up through `window_start`, then a normal step integral. No flat prefix.
- `tw_shares = share_seconds / window_seconds` with `window = 30` UTC days (`chrono` duration, not a calendar-month boundary).
- Running balance that would go negative clamps to 0. Log a warning. Do not fail the other refreshes.
- Stamp NULL when `tw_shares ≤ 0`, `total_supply ≤ 0`, `pair_liquidity_usd` is missing, or the USD result is `≥ 10^20`.

**Regression:** anchor `shares = 100` at day 0 of a 30-day window, a `+400` custodian delta at day 20, and an LCD refresh on day 29 that returns `500`. Evaluate at day 30. The anchor stays `shares = 100` and `observed_at = day 0`. The day-29 read finds `expected = 500` and writes no nonzero reconcile row. `tw_shares = 100 × 20/30 + 500 × 10/30`. A result of `500` fails.

Known-zero custody (anchor shares 0 and no later positive balance) stores NULL on the stamp as well, because non-positive CMM LP is not a denominator. The cell is an em dash. 30d vol / v2 LP / Vol/LP still render.

### Trading-fee numerator

`pair_trading_fees_30d.fees_usd` is the trailing-30d sum of **priced** `protocol_fee_events.fee_usd` for `swap_amm`, `book_take`, and `limit_place` attributed to that pair.

- `swap_amm` uses the existing `pair_id`.
- New `book_take` / `limit_place` rows persist `pair.id` (already known at ingest). They move onto the existing pair-scoped partial unique `(tx_hash, source, pair_id, ordinal)`. `ordinal` stays `order_id`. Wrap, unwrap, and UST1 window stay `pair_id` NULL on the nopair partial. Never `ON CONFLICT DO UPDATE`.
- Backfill: set `pair_id` only when **exactly one** `limit_order_fills` row matches `(tx_hash, order_id = ordinal)` for `book_take`, or one `limit_order_placements` row for `limit_place`. Prefer an amount match when several candidates appear; if still not unique, leave `pair_id` NULL. Unattributed fees are excluded from every pair stamp. Do not invent a second historical row the nopair unique already dropped.
- Same NULL / overflow / idle rules as `pair_volume_30d`: all trading-fee rows unpriced → NULL (do not coerce to 0). No trading-fee rows → `0`. Mixed priced and unpriced → sum of priced `fee_usd > 0` only (same `FILTER` as volume). `≥ 10^20` → NULL.
- `fees_bps = fees_usd / cmm_lp_usd × 10000`, then the same `≤ 0` / `≥ 10^20` cut as `compute_volume_per_tvl`. Idle `0` fees therefore omit `fees_bps_per_cmm_lp` (em dash). Wrap-only or window-only activity does not enter the sum, so the numerator is `0` and the bps cell is an em dash.
- `volume_per_cmm_lp` reuses `compute_volume_per_tvl(volume_usd_30d, cmm_lp_usd)`.

### GET and cache

`LIST_TOP_PAIRS_SQL` keeps the #1263 `WHERE` / `ORDER BY` / `LIMIT 5`. It `LEFT JOIN`s `pair_cmm_lp_usd_30d` and `pair_trading_fees_30d` only. **EXPLAIN** must not mention `swap_events`, `pair_reserves`, `protocol_fee_events`, `liquidity_events`, `lp_holder_deltas`, or `lp_custody_checkpoints`.

The 60s cache stores the whole `ProtocolTopPairsResponse`, including the four new fields. `reset_protocol_top_pairs_cache` stays the test reset. There is no second cache map. A body cached inside this binary cannot omit the fields by type. Query allowlist is unchanged (`limit` / `window` only; `from` / `to` / `sort` / `ticker` → **400**).

Handler comment (required, next to the SQL): CMM v2 LP is time-weighted custodian shares marked at current `pair_liquidity_usd / total_supply`; provide credits `receiver` by `share`; the first LCD balance is a fixed anchor; later reads add a reconciliation delta and do not install today’s balance; GET does not scan events or the chain.

### UI

Columns after Vol/LP, in order: **CMM v2 LP**, **30d fee bps**, **Vol/CMM LP**. Test ids `protocol-top-pair-cmm-lp-{i}`, `protocol-top-pair-fee-bps-{i}`, `protocol-top-pair-cmm-ratio-{i}`.

| Cell | Formatter |
|------|-----------|
| CMM v2 LP | `formatProtocolUsd` (same `$` / em dash as v2 LP) |
| 30d fee bps | new `formatFeeBps`: finite `> 0` → digits, no `%`, no `×`; else em dash |
| Vol/CMM LP | existing `formatVolumePerTvl` |

Tooltips (no farm, APR, or yield):

- CMM v2 LP: time-weighted CMM LP shares over 30 days, valued at the current pool USD per share. The first balance check is a fixed anchor. Later checks do not replace that history with today’s balance. Not full-pool v2 LP.
- 30d fee bps: trailing 30-day pair trading fees (AMM, book take, limit place) in basis points of that CMM LP. Wrap and UST1 window fees are not included.
- Vol/CMM LP: trailing 30-day volume divided by that CMM LP. A multiple, not a percent, and not full-pool Vol/LP.

Keep the table inside the existing `overflow-x-auto` wrapper. Numeric headers and cells use `whitespace-nowrap`. Do not drop columns on a narrow viewport and do not overlap text. Flat table; no per-row `card-glass`.

## Component / state / interface changes

| Layer | Change |
|-------|--------|
| Schema | Additive `lp_holder_deltas` (event unique `(tx_hash, pair_id, kind, holder, event_index)`; reconcile unique `(pair_id, holder, block_height)`), `lp_custody_checkpoints` (immutable `shares` / `observed_at` / `observed_height`), `pair_trading_fees_30d`, `pair_cmm_lp_usd_30d`. No edit of `20260821120000_protocol_fees.sql` or `20260916120001_protocol_fee_events_pair_id.sql` in place. |
| Parser | Provide credits `receiver` (else `sender`) by `share`. Withdraw debits `sender` by `withdrawn_share`. LP-token `transfer` / `transfer_from` / `send` write `transfer_out` and `transfer_in` from `from`, `to`, and `amount`. Book/place fee drafts set `pair_id`. |
| Refresh | `refresh_pair_trading_fees_30d` and `refresh_pair_cmm_lp_usd_30d` from `refresh_all_volume_windows_with_pins` (~5 min + startup), after `refresh_pair_volumes_30d`. Failure logs and continues. |
| GET | Four optional strings on `ProtocolTopPairItem`: `cmm_lp_usd_30d`, `trading_fees_usd_30d`, `fees_bps_per_cmm_lp`, `volume_per_cmm_lp`. `skip_serializing_if` none, same as `liquidity_usd`. `utoipa::ToSchema` is the OpenAPI contract. |
| dApp | `ProtocolTopPairItem`, three columns, copy, `formatFeeBps`. Missing keys → em dash. |

`trading_fees_usd_30d` is on the API so a reviewer can recompute bps. It is not a table column. Idle `0` serializes as `"0"`. Unpriced fees omit the key. Bps still omit when the ratio is `≤ 0`.

## Affected invariants

| ID | Effect |
|----|--------|
| **I1317-1–I1317-8** / **P1317-1–P1317-8** | New. See the playbook. |
| **I1263-1–I1263-8** / **P1263** | Unchanged ranking, gem list, Vol/LP, query allowlist. Extra LEFT JOINs are stamps only. |
| **F1269-3** / **PFee-14** | Wrap / unwrap / ust1_* stay NULL `pair_id`. `book_take` / `limit_place` store `pair_id` when the fill or placement is unique. Unattributed rows stay on the nopair partial. Still never `DO UPDATE`. `swap_amm` hop uniqueness unchanged. |
| **L7** / **PFee-5** | Hybrid fee is still pool commission plus fill commission once. This census only **groups** those rows by pair. |
| **P655** / **I1263-8** | USD mark uses `pair_liquidity_usd` / `protocol_pair_tvl`. No `$1` UST1, no `2.5×` USTR, no vFDUSD, no CG `liquidity_in_usd`. |
| **P562** | Gems stay out of the top-5. |
| **#568** | Do not rewrite non-null `fee_usd`. The 30d stamp reads the stored stamp. |
| **C653** | No `card-glass` inside the top-pairs panel. |

## Alternatives

| Option | Why not |
|--------|---------|
| Spot CMM LCD balance as the denominator | Repeats the mismatch the v2 LP tooltip already states (current USD vs a 30-day flow). LCD on GET is forbidden. |
| Move `observed_at` forward on each 24h LCD refresh | The anchor stays inside the 30-day window, the flat prefix never ages out, and `cmm_lp_usd` stays approximately today’s custodian balance. Fee bps and Vol/CMM LP then use a spot denominator. |
| Sum `liquidity_events.provider = CMM` | Provider is the provide sender. Production provides credit `receiver`. Leftover LP moves by CW20 transfer. |
| Integrate historical pool USD | No per-pair USD series at liquidity-event resolution. The issue asks for share-seconds valued with the existing pool-USD / total-supply ratio. |
| Wait 30 days before showing a number | Hides current CMM inventory. The first-anchor prefix is disclosed and ages out. Later reads must not restart it. |
| Rank by the new ratios | Reopens #1263. Forbidden. |
| Put book fees in the numerator without `pair_id` | Those rows are NULL today and cannot be summed per pair. Joining fills on every refresh is a second attribution path; persisting `pair_id` at ingest matches `swap_amm`. |
| `ON CONFLICT DO UPDATE` for fees or deltas | Replay could replace treasury USD or share deltas. Forbidden. |
| New `CMM_LP_CUSTODIAN` env | The treasury pin already exists. A second variable can drift. |
| Show bps as a percent, or Vol/CMM LP with extra digits | Fee figure is basis points. Volume uses the existing `×` formatter (`2.469` → `2.47×`). |

## Complexity added / removed

**Added:** one custody ledger with a named event conflict target, one insert-once LCD anchor, reconciliation deltas for later custodian reads, two stamp tables, provide-receiver and LP-transfer parsing, `pair_id` on book/place fees, two refresh functions, four optional JSON fields, three columns and one formatter.

**Removed:** nothing. GET does not grow a scan. No new route, fee source, wasm message, or chain subscription. A 24h refresh does not gain a second balance series; it writes the drift from the fixed anchor.

Net: off-request stamps so the top-pairs table can show CMM capital and pair trading fees without redefining Vol/LP.

## Migration

New file only: `indexer/migrations/20260922180000_pair_cmm_lp_and_trading_fees_30d.sql`. Prefix must stay unique (`sqlx` versions on the numeric stem). Do not reuse `20260921120000` / `20260921120001`.

Order: create the four tables and the two delta unique indexes → backfill `book_take` / `limit_place` `pair_id` where the match is unique → do not delete fee rows. Idempotent `IF NOT EXISTS`. Deltas and checkpoints start empty; the first successful LCD read inserts the anchor. Historical provides are not rewritten from `liquidity_events.provider`.

Paired revert: `indexer/migrations/revert/20260922180000_pair_cmm_lp_and_trading_fees_30d.down.sql`. After this change, two `book_take` or `limit_place` rows can share `(tx_hash, source, ordinal)` on different pairs. `protocol_fee_events_nopair_tx_source_ordinal_uidx` is `UNIQUE (tx_hash, source, ordinal) WHERE pair_id IS NULL`, so a blanket `SET pair_id = NULL` fails. Before nulling, keep the lowest `id` per `(tx_hash, source, ordinal)` and delete the other `book_take` / `limit_place` rows:

```sql
DELETE FROM protocol_fee_events e
USING (
    SELECT tx_hash, source, ordinal, MIN(id) AS keep_id
    FROM protocol_fee_events
    WHERE source IN ('book_take', 'limit_place')
    GROUP BY tx_hash, source, ordinal
) d
WHERE e.tx_hash = d.tx_hash
  AND e.source = d.source
  AND e.ordinal = d.ordinal
  AND e.id <> d.keep_id;

UPDATE protocol_fee_events
SET pair_id = NULL
WHERE source IN ('book_take', 'limit_place')
  AND pair_id IS NOT NULL;
```

Then drop the four new tables. Leave `swap_amm` pair ids and the #1269 indexes unchanged.

No wasm migrate. No dApp env key. No new indexer secret. `CMM_GOVERNANCE_ADDR` is the existing community-tax pin.

## Observability

- Refresh failure: `tracing::error` (startup: `warn`) with the label `pair 30d trading fees` or `pair cmm lp usd`, then continue the rest of `refresh_all_volume_windows_with_pins`.
- Custodian unset: one `info` per refresh, `custodian_configured=false`. Do not log hostnames, DSNs, or credentials. Do not log the bech32 on every tick.
- Negative-share clamp: `warn` with `pair_id` only.
- LCD anchor failure with no row: `warn`; stamp stays NULL.
- Nonzero reconciliation delta: `info` with `pair_id` and the signed difference. Do not log the custodian bech32.
- Do not add `/metrics`. Do not `SUM` the new tables on GET.

## Failure modes

| Failure | Behavior |
|---------|----------|
| `CMM_GOVERNANCE_ADDR` unset or invalid | New fields omitted. Original columns unchanged. |
| No anchor yet (LCD down on first pass) | CMM LP and both ratios omitted. Fees key may still show `"0"` or a sum. |
| Anchor inside the window | Flat share prefix only until the first `observed_at`. A day-29 refresh does not extend it. |
| LCD refresh while the anchor is inside the 30d window | Compare LCD with `anchor.shares + later deltas`. Insert one reconcile delta. Leave `shares` and `observed_at` unchanged. |
| Provide sender ≠ receiver | Ledger credits `receiver` by `share`. Sender-only `provider` is not the balance. |
| LP `mint`/`burn` plus provide/withdraw | `mint` and `burn` ignored. One economic delta from provide or withdraw. |
| Withdraw `send` to the pair | `to` is the pair contract, so the transfer is ignored. Withdraw debit stands. |
| Two LP transfers of one holder in one tx | Both rows insert. `event_index` is part of the conflict target. |
| User transfer of LP to CMM | `transfer_out` and `transfer_in`. CMM balance moves without a liquidity event. |
| Running share balance would go negative | Clamp to 0, warn, stamp from the clamped integral. |
| Unpriced trading fees only | `fees_usd` NULL, bps omitted. |
| Wrap / UST1 window fees only | Numerator `0`, bps omitted. |
| `book_take` backfill matches two pairs | Leave `pair_id` NULL. Exclude from every pair sum. |
| CMM LP `≤ 0` or `≥ 10^20` | Omit `cmm_lp_usd_30d` and both ratios. Other cells render. |
| Indexer binary without the keys | dApp em dashes in the three new cells. Panel still lists the original columns. |
| GET cache hit inside 60s | Serves the previous full body (existing #1263 behavior). Tests call `reset_protocol_top_pairs_cache`. |
| Stamp refresh fails | Log and continue. Last good stamp remains. GET does not fall through to a live scan. |

## Ordered implementation slices

| Slice | Who | Deliverable | Blocks |
|-------|-----|-------------|--------|
| **0 — this design** | design | This ADR, architecture `#cmm-lp-census`, invariant rows, playbook, README / testing pointers | Slice 1 |
| **1 — ledger + stamps** | implement | Migration `20260922180000`, unique-safe down.sql, parser deltas, book/place `pair_id`, both refreshes, pure integral + bps tests including the day-29 anchor regression, Postgres fixtures P2–P4 / P6 / P8. Hook refreshes into `refresh_all_volume_windows_with_pins`. | Slice 2 |
| **2 — GET** | implement | LEFT JOINs, four fields, `ToSchema`, EXPLAIN has no raw tables, cache reset test, disallowed query keys still **400**. `make verify-issue-1263` stays green. | Slice 3 |
| **3 — UI** | implement | Three columns, tooltips, `formatFeeBps`, missing-key dashes, nowrap + existing scroll wrapper. No farm/APR copy. | Slice 4 |
| **4 — verify** | implement | `make verify-issue-1317` runs the indexer tests and the Protocol page / formatter tests (Postgres, no chain). Makefile + `docs/testing.md` target. Keep `verify-issue-1263` and `verify-issue-1269` green. | none |

Slices 1–4 land in one implement change. No operator leftover and no founder card. Setting `CMM_GOVERNANCE_ADDR` is the existing #594 pin; if it is unset, the new cells stay em dashes (fail closed), which is not a deploy.

[#1263](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1263) and [#1269](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1269) are already on `main`. They are not open blockers.

## Tests

| ID | Expected |
|----|----------|
| P1 | `10 / 500 × 10000 = 200`. `1234.5 / 500` plain string `2.469`. Zero, negative, missing, `≥ 10^20` denominator or ratio → `None`. |
| P2 | Fixture with `swap_amm` + `wrap` on one pair: stamp includes `swap_amm` only. `book_take` / `limit_place` included when `pair_id` is set. Wrap and `ust1_*` excluded. |
| P3 | Two custodian deltas inside 30d match a hand integral. A non-custodian provider does not move it. Provide credits `receiver` by `share`, not sender. Withdraw debits `withdrawn_share`. A user LP transfer writes `transfer_out` and `transfer_in`. Replaying the same `(tx_hash, pair_id, kind, holder, event_index)` inserts nothing. Two transfers of one holder in one tx both remain. `liquidity_event_exists` is not the guard. |
| P4 | GET SQL is the stamp join. EXPLAIN lists none of the raw tables in Decision. Cache after `reset_protocol_top_pairs_cache` returns the new fields. `from` / `sort` still **400**. |
| P5 | Headers **CMM v2 LP**, **30d fee bps**, **Vol/CMM LP**. Fee cell `200`. Volume cell matches `formatVolumePerTvl('2.469')` (`2.47×`). Nulls are em dashes. Vol/LP snapshot unchanged. Wrapper keeps `overflow-x-auto`; new cells are `whitespace-nowrap`. |
| P6 | No checkpoint and no trading fees → new cells em dash. The pair can still appear from `pair_volume_30d`. |
| P7 | Tooltips name time-weighted CMM LP, the fixed first anchor, exclude wrap/window from bps, and say the multiple is not full-pool Vol/LP. Copy has no farm, APR, or yield. |
| P8 | Evaluate at day 30. Shares 100 for 20 days, then 500 for 10 days. LCD refresh on day 29 returns 500 and does not change `shares` or `observed_at`. `tw_shares` stays `100 × 20/30 + 500 × 10/30`. A result of 500 fails. |
| P9 | Down.sql deletes extra `book_take` / `limit_place` rows, keeping `MIN(id)` per `(tx_hash, source, ordinal)`, before `SET pair_id = NULL`. `swap_amm` pair ids and both #1269 partial indexes remain. |

UI acceptance `2.469×` in the issue is the API plain string. The cell is the existing formatter.

## Rollout

Expand-only schema. Old dApp ignores unknown JSON keys. New dApp treats missing keys as em dashes. Ship indexer migrate (boot `sqlx::migrate!()`) and the dApp in the same implement change. No feature flag. Columns appear after the first successful stamp refresh (~5 min) and a checkpoint. Until then the three cells are em dashes and the original columns stay.

Auto-deploy of an additive migration follows [ADR 0006](./0006-indexer-health-git-sha.md): default **2(b)** keep schema and ship a hotfix that still contains `20260922180000`. **2(c)** (down.sql + ledger `DELETE`, prior image) only when the new tables themselves must go away. Do not `UPDATE _sqlx_migrations.success`.

## Rollback

Code rollback that still contains the migration is **2(b)** (stamps may be wrong; GET stays stamp-only). Restoring a binary that does not contain `20260922180000` requires **2(c)** using the paired down.sql: keep the lowest `book_take` / `limit_place` `id` per `(tx_hash, source, ordinal)`, delete the other rows of those sources, then NULL `pair_id` on the survivor, drop the four new tables, and `DELETE` that `_sqlx_migrations` version while the process is stopped. `swap_amm` pair ids and the #1269 indexes stay. Frontend rollback is safe against either API shape.

## Integration completion criteria

- `make verify-issue-1317` passes (P1–P9). `make verify-issue-1263` and `make verify-issue-1269` still pass.
- The day-29 refresh regression stays `100 × 20/30 + 500 × 10/30` and does not return 500.
- OpenAPI schema from `ProtocolTopPairItem` lists `cmm_lp_usd_30d`, `trading_fees_usd_30d`, `fees_bps_per_cmm_lp`, `volume_per_cmm_lp`.
- A reviewer can recompute the `500` / `10` / `1234.5` row from the stamp inputs without reading `swap_events` on the request path.
- EXPLAIN for `LIST_TOP_PAIRS_SQL` does not name the raw tables in Decision.
- No farm/APR copy, no Vol/LP redefinition, no GET LCD, no new custodian env, no wasm migrate.
- This design branch is not merged as a PR. Status stays **Proposed** until review accepts it.
