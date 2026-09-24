# ADR 0012: Price candles when a factory leg is not a catalog quote

## Status

Proposed ([#1315](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1315)). Revision 3 (wire omits `asset_1` OHLC from `open/high/low/close`, rollback deletes those rows, gap-fill done marker only on zero failures, Charts pill matches the pane).

Design only. This ADR does not deploy, spend, expand custody or policy, or write an approval marker. Issue keywords are not approval. Deploy, spend, custody, and policy expansion stay under [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297).

Overview: [`architecture.md`](../architecture.md#alpha-pair-price-candles). Invariant row: [`indexer-invariants.md`](../indexer-invariants.md) **Unpriced-quote candles (#1315)**. Chart heading rule: [`frontend.md`](../frontend.md) USD invert paragraph.

Numbering: ADR **0008** stays on `origin/cac-design-issue-1300`. ADR **0010** / **Q23** stay on `origin/cac-design-issue-1305`. ADR **0011** / **Q24** stay on `origin/cac-design-issue-1311`. This ticket is **0012**. It does not take a **Q** row (those are leftover-ops). Do not merge this design branch (`cac-design-issue-1315`) as a design-only PR. Implement keeps Status **Proposed ([#1315])** until a reviewer accepts the design.

## Outcome

1. A factory pair that includes ALPHA and has at least one indexed swap with a positive human quote-per-base price shows candlesticks on `/charts` at the default interval, not `PriceChartEmptyState`.
2. When the other leg is a P522-Q catalog quote (UST1, USTC/cUSTC/`uusd`, LUNC/cLUNC/`uluna`, USTR, or the pinned USDT CW20), candle `open/high/low/close` are **USD of 1 human unit of the non-catalog leg** (ALPHA), whichever `asset_infos` slot that leg occupies. `usd_leg` is the leg used **at that write**, stored on the row. A later catalog change does not reclassify it.
3. When neither leg is catalog (CL8Y/ALPHA), `*_human` OHLC is stored, USD columns stay null, and the chart draws that human series under **Price ({asset_1} per {asset_0})**. Human numbers are not copied into USD columns and the heading is not **Price (USD)**.
4. `swap_events.price` and `swap_events.price_usd` keep **P522-1** / **P522-2**. No `quote_usd_kind("ALPHA")` or `quote_usd_kind("CL8Y")` arm. No peg. A spoofed symbol does not become a catalog quote.
5. Pairs with no positive-price swaps still use the existing empty-state copy.

## Context

`/charts` mounts `PriceChartEmptyState` when `indexerCandlesToFactoryPoints` returns no finite positive USD bars. Ingest only calls `update_candles_for_swap` when `price_usd` is a positive USD of 1 human `asset_0`, and that figure is `human_quote_per_base × usd(asset_1)`. `quote_usd_kind` does not classify ALPHA or CL8Y, so a pair whose `asset_1` is ALPHA (UST1/ALPHA with ALPHA in the quote slot, or CL8Y/ALPHA) stores no candle at all — including `*_human`.

Factory order follows chain `asset_infos`. `asset_0` is base and `asset_1` is quote ([`swap_orientation.rs`](../../indexer/src/indexer/swap_orientation.rs)). The stable is not forced into the quote slot. Reported factory markets from #558 are UST1/ALPHA and CL8Y/ALPHA. Registry ALPHA is `terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz` (6 decimals). That address is the reported token, not a new price pin. Terraport ALPHA/LUNC and ALPHA/USTC rows in `communityTaxMigratePairs.ts` are other-venue inventory.

`candles.open/high/low/close` are already nullable `NUMERIC` (initial schema; renamed in `20260310000002`). Rust `CandleRow` and `CandleResponse` still require those fields, so a NULL USD row cannot round-trip until those types become optional. `*_human` is already `NUMERIC(78, 18)` NULL-able.

Charts default invert is off (**C680**: UST1/cUSTC hero shows USD of `asset_0`). Trade inverts only when `asset_0` is UST1 (**T524-3**). Both-catalog pairs must keep that behavior. Idle marks (**C568**) price `asset_1` only and must not start writing a catalog base’s USD into a series that means USD of ALPHA.

## Non-goals

- Pegging ALPHA or CL8Y to $1, or adding either symbol to `quote_usd_kind`.
- An allowlist that charts only the registry ALPHA contract. The cross is generic for any non-catalog leg against a catalog leg. The registry address is not a mark.
- Reopening closed [#1258](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1258) (USDT pin), [#543](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/543) (human numbers on a **Price (USD)** control), [#522](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522) (catalog membership), [#113](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/113) (empty-state copy), [#568](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/568) (live hub rewrite of existing USD history), or [#1250](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1250) (ALPHA code-id migrate).
- Changing `swap_events.price`, `swap_events.price_usd`, `volume_usd` (**L10** / P522-Q), CG/CMC `last_price`, fee `economic_token_marks` ([#683](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/683)), or idle-mark coverage.
- Changing the UST1/cUSTC Charts hero (**C680**) or Trade’s both-catalog default (**T524-3** / **C543-3**).
- Invert on a human-only chart (no reciprocal `1/x` series in this ticket).
- Terraport / GDEX migrate inventory, a live chain, or a deployed indexer as the regression gate.
- Coolify, HMAC / `autonomy.rs`, a founder card, or a design-only PR.
- Self-marking this ADR **Accepted**.

## Decision

Classify each factory pair from the same identity helper as P522-Q (`quote_usd_kind_for_identity` / pinned USDT). Symbol `ALPHA` or `CL8Y` alone is `None`.

Let `H` be the existing human quote-per-base (`asset_1` per `asset_0`, already decimal-scaled).

| Legs in the catalog | Candle USD `open/high/low/close` | `usd_leg` | `*_human` |
|---------------------|----------------------------------|-----------|-----------|
| Both | `H × usd(asset_1)` = USD of 1 human `asset_0` (unchanged) | `asset_0` | `H` |
| Only `asset_1` | same formula (non-catalog leg is `asset_0`) | `asset_0` | `H` |
| Only `asset_0` | `usd(asset_0) / H` = USD of 1 human `asset_1` | `asset_1` | `H` |
| Neither | NULL | omit | `H` |

`H × ust1_usd` when ALPHA is `asset_0`, and `ust1_usd / H` when ALPHA is `asset_1`, are the same quantity: USD of 1 human ALPHA. That is the issue check `human_alpha_price × ust1_usd` where `human_alpha_price` means UST1 per 1 ALPHA.

`swap_events.price_usd` stays `H × usd(asset_1)` or NULL. Do not persist `usd(asset_0) / H` on the swap row. Candle USD for the flipped slot is computed at candle write, not by overloading tape USD.

Non-positive `H`, non-finite values, and overflow (`NUMERIC(38, 18)` for USD, `NUMERIC(78, 18)` for human) skip the whole candle write. A missing catalog print (oracle or hub down) on a pair that has **any** catalog leg also skips the **whole** candle: no USD row and no human-only row. `HumanOnly` is only the neither-catalog decision. Do not substitute human `H` into the USD columns. Do not read `economic_token_marks`.

A one-catalog pair that stored `*_human` with NULL USD during a hub outage would make the client fall through to **Price ({quote} per {base})**, then flip back to **Price (USD)** on the next good print. That row is not written.

Idle marks stay as they are: `mark_quote_kind` requires a catalog `asset_1`. They must not gain an `asset_0` arm that would merge ~$1 UST1 into a bucket whose USD means ALPHA.

### Interface

GET `/api/v1/pairs/{addr}/candles` (`CandleResponse`):

- `open`, `high`, `low`, `close` stay **USD of 1 human `asset_0`**. Omit them when NULL, and omit them when `usd_leg` is `asset_1` (`skip_serializing_if`). A cached client drops a bar that has no finite positive `open`.
- When `usd_leg=asset_1`, send that subject OHLC on optional `subject_open`, `subject_high`, `subject_low`, `subject_close`. The database may keep the same numbers in `open/high/low/close` plus `usd_leg`. The new client plots `subject_*`. It does not plot omitted `open` as USD of `asset_0`.
- Add optional `usd_leg`: `"asset_0"` or `"asset_1"`, present when the row records a leg. Persist it on the candle row at insert. GET returns that stored value. Do not recompute it from the pair’s catalog membership at read time. Rows written before the column (both-catalog and quote-catalog history) omit it; clients treat a missing `usd_leg` as `asset_0` and read `open/high/low/close`.
- `*_human` unchanged in meaning. A neither-catalog row omits USD, omits `usd_leg`, and omits `subject_*`.

`IndexerCandle.open/high/low/close` are optional strings, plus optional `usd_leg` and `subject_*`.

`CandleRow` USD fields become `Option<BigDecimal>`. `upsert_candle` binds NULL USD for the neither-catalog path.

### Frontend state

No new store. The pair’s catalog class (same identity helper as ingest) chooses the pane. `PriceChart` does not treat “no positive USD” as a human chart when a catalog leg exists.

- **Catalog pair** (one or both legs): **USD series** under **Price (USD)** when at least one bar has finite positive USD OHLC. Bars without positive USD are dropped. If none remain, the empty state — even if `*_human` is present on the payload. No quote-per-base heading.
- **Neither-catalog pair:** **human series** when at least one bar has finite positive `*_human`. Heading is `Price ({asset_1 symbol} per {asset_0 symbol})`. Hide the invert pill. Do not call `invertUsd`. The pane last-close is that series’ last close, not a USD figure and not raw `trades[].price`.
- **Empty** when a neither-catalog payload also has no positive `*_human`, and for a no-trade pair. Copy in `PriceChartEmptyState` stays as shipped. **C543-9** is this rule.

`inverted` still means “display `asset_1`” for the Trade ticket, pill, and **Buy {displayBase}** label. **T524-3** still defaults it on when `asset_0` is UST1. Do not clear it to avoid a second divide. When `usd_leg=asset_1`, that flag selects the displayed token and does not select `invertUsd`. When `usd_leg` is missing or `asset_0`, `inverted` still selects `applyChartDisplayInvert`.

Price-pane displayed token:

- Missing `usd_leg` or `usd_leg=asset_0`: unchanged. Charts hero and Trade invert unchanged. The other leg uses `applyChartDisplayInvert` (`invertUsd` = stored / `H`, high with high, then order so high ≥ low). Never `1/x` of the USD series.
- `usd_leg=asset_1`: stored OHLC is already USD of `asset_1`. On `/charts`, when there is no `?price=` and no Charts session flag, this default selects `asset_1` for both the price pane and the Charts pill (ALPHA USD, not a flat catalog line). `defaultChartsDisplayInverted()` stays `false` for every other pair. That default does not write `cl8y-dex-trade-pair-invert:`. Do not write `?price=` until candle `usd_leg` is known. Trade keeps its own default `inverted`. Explicit `?price=` or a Charts session flag still selects either leg. When the displayed token is `asset_1`, plot `subject_*`. When it is `asset_0`, plot `subject × H` and do **not** call `applyChartDisplayInvert`. Pair `open` with `open_human` and `close` with `close_human`. Pair stored `high` with human `low` and stored `low` with human `high`, then set high = max and low = min so high ≥ low. Drop the bar when any factor is missing, non-positive, or non-finite. `applyChartDisplayInvert` divides and pairs high with high; that function stays on the `usd_leg=asset_0` branch.

Both-catalog pairs stay on the first branch (**C680**, `usd_leg` omitted or `asset_0`).

The price-pane default does not retarget 24h OHLC, TWAP, or tape Price. Those stay on `swap_events.price_usd` (and human TWAP stays quote-per-base). When ALPHA is `asset_1`, `price_usd` is NULL, so those read **—**. Do not fill them from candle subject USD.

Volume histogram uses the same bars as the plotted series (neither-catalog rows included). Color follows plotted close vs open. Volume is still not inverted. SMA/RSI run on the plotted series.

`/trade` uses the same `PriceChart` plot rules. Its ticket and labels still follow `inverted`.

### Complexity

Added: one pure subject function, optional USD on the candle row/JSON, a stored `usd_leg`, a zero-row gap fill, a neither-catalog heading, and one multiply mapper for `usd_leg=asset_1`. Removed: the early return that discarded `*_human` on a neither-catalog pair, and any reading that backfills holes on pairs that already have candles. `applyChartDisplayInvert` stays for `usd_leg=asset_0`. No new service, catalog arm, or peg.

## Alternatives

| Alternative | Why not |
|-------------|---------|
| Peg ALPHA or CL8Y at $1, or `quote_usd_kind("ALPHA")` | Forbidden by the issue. Spoof symbol would inherit it. |
| Chart only the registry ALPHA contract | That pin is a special mark. The bug is any unpriced `asset_1`. |
| Plot `*_human` on **Price (USD)** | Breaks **#543** / **P522-5**. |
| Store only USD of `asset_0` when `asset_0` is catalog (`usd(asset_0)`, ignore `H`) | Charts default would draw a flat UST1 line. Stored candle USD would not be USD of ALPHA. |
| Put `usd(asset_0)/H` into `swap_events.price_usd` | Breaks **P522-2** (tape USD of `asset_0`) for every consumer. |
| Price CL8Y/ALPHA from `economic_token_marks` | Fee marks stay #683. This pair has no catalog leg; the chart is human quote-per-base. |
| `TRUNCATE candles` and rebuild | Wipes good USD history. The #543 migration already did that once; do not repeat it. |
| Extend **C568** idle marks onto unpriced quotes | Would merge live hub USD into the current bucket and fight the ALPHA series. |
| Insert missing buckets on pairs that already have candles, using a live UST1/USTR hub snapshot | Rewrites history. Idle marks must not move an old print. Healthy pairs, including ALPHA-as-`asset_0`, stay untouched. |
| Clear Trade `inverted` when `usd_leg=asset_1` | `inverted` is the ticket, pill, and Buy label. Clearing it to skip a divide breaks that ticket. The chart uses displayed token vs `usd_leg` instead. |
| Write `*_human` and NULL USD when a catalog print is missing | The pane would switch to quote-per-base, then back to **Price (USD)** on the next print. `HumanOnly` is neither-catalog only. |
| Fill 24h OHLC, TWAP, or tape Price from candle subject USD | Those stay on swap `price_usd` (**—** when ALPHA is `asset_1`). The new default is the price pane only. |

## Affected invariants

- **P522-1 / P522-2 / P522-3** unchanged. Unknown quote → `swap_events.price_usd` NULL. USDT stays the pinned CW20, not a symbol arm.
- **P522-4** still holds for both-catalog pairs (USD from `price_usd` of `asset_0` only). **#1315** adds the mixed and neither-catalog candle rows above. Human `H` still never lands in USD columns.
- **P522-5** still holds for 24h OHLC, TWAP, tape Price, and limit USD notional. The price pane is the #1315 series. Do not fill the stats from candle subject USD.
- **C543** still holds on any control labeled **Price (USD)**. **C543-9** adds the neither-catalog heading and keeps the empty state when that payload also has no positive `*_human`. The human series is not a **Price (USD)** control.
- **C568** idle marks unchanged. Gap fill must not `UPDATE` any existing candle or any `swap_events.price_usd`.
- **C680 / T524-3** unchanged for both-catalog pairs. `usd_leg=asset_1` defaults only the price pane. Trade `inverted` stays the ticket flag. **C680-3** page-wide retarget does not pull 24h OHLC, TWAP, or tape Price off swap `price_usd` for this pair.
- **C705** newest-N read unchanged. Neither-catalog NULL USD rows are still returned. The USD series drops them. The human heading uses them only when the pair has no catalog leg.
- **#113** empty-state copy unchanged.
- **H10** CG/CMC `last_price` stays human.

## Migration

No `TRUNCATE`. No rewrite of non-null candle USD and no rewrite of `swap_events.price_usd`.

New unique migration prefix (do not reuse `20260916120000`):

- `candles.usd_leg` text, NULL. NULL on a row that has USD means that write was USD of `asset_0`. New USD writes set `asset_0` or `asset_1`. Neither-catalog rows leave `usd_leg` NULL and leave USD NULL.
- `candle_gap_fill_1315 (pair_id, interval, open_time)` primary key, plus a single done marker row so a **successful** boot does not scan forever. Set the marker only when the failure count is 0. On failure, the next boot retries pairs that still have zero `candles` rows. Stamped buckets stay as written.

After `sqlx::migrate`, one indexer pass (info log, does not crash boot on failure). Select only pairs that have `se.price > 0` **and zero `candles` rows**. Leave every pair that already has a candle row, including ALPHA-as-`asset_0`. Do not scan missing buckets on pairs that already have candles.

For each selected pair, insert buckets from those swaps:

- `*_human` from `se.price`. USD from `candle_usd_subject` at pass time. A missing catalog print skips that swap (no human-only insert).
- USTC/LUNC from as-of `oracle_prices` at the swap time (same helper family as #568). USDT = advisory $1. UST1/USTR = hub snapshot **at this pass only**, and only because the pair has no candle clock yet. Stamp `(pair_id, interval, open_time)` and the `usd_leg` used for that insert. Later hub refresh must not `UPDATE` stamped rows.

Forward swaps after deploy use the catalog print at ingest time (same clock as other candles) and store that write’s `usd_leg`.

`down.sql`, before an old binary serves GET, deletes stamped gap-fill keys, every `usd_leg = 'asset_1'` row, and every row with NULL `open`, then drops `candles.usd_leg` and the stamp objects. See Rollback.

## Observability

Tracing only (no `/metrics`, [#200](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/200)).

- Debug `candle_write` with `pair_id` and `mode` = `usd_asset_0` | `usd_asset_1` | `human_only` | `skip`.
- One info line from the gap fill: zero-candle pairs considered, rows inserted, swaps skipped, failure count. Do not log a per-bucket scan of pairs that already have candles.

## Failure modes

| Case | Behavior |
|------|----------|
| Oracle or hub missing for a pair that has a catalog leg | Skip the whole candle. Do not insert `*_human` with NULL USD. The USD chart stays empty until a later swap has a catalog print. Neither-catalog pairs do not need the oracle. |
| `H <= 0`, overflow, NaN | Skip the candle write. Client drops non-finite / non-positive / empty OHLC before `setData`. |
| Spoof CW20, symbol `ALPHA`, other leg unpriced | `quote_usd_kind` is `None`. Neither-catalog path: human OHLC, no USD. |
| Spoof CW20, symbol `ALPHA`, other leg UST1 | Generic mixed rule. USD is the UST1 cross of that CW20, not $1 and not a registry pin. |
| Idle mark tick on ALPHA-as-`asset_1` | Still skipped. Must not merge catalog USD into the ALPHA series. |
| Old indexer binary reads a NULL `open` | `CandleRow` decode error on that pair’s GET. Rollback runs `down.sql` first. |
| Indexer live before the understanding client | Both-catalog charts stay correct (missing `usd_leg` means `asset_0`). `usd_leg=asset_1` omits `open/high/low/close`, so today’s client drops the bar and shows the empty pane instead of plotting USD of `asset_1` as USD of `asset_0`. The new client plots `subject_*`. |

## Slices

| Slice | Owner | Work | Depends on |
|-------|--------|------|------------|
| **0 — this design** | design | This ADR, architecture `#alpha-pair-price-candles`, invariant row, `frontend.md` sentence, `docs/README.md` index, `docs/testing.md` row, skill pointers. Branch `cac-design-issue-1315` only. | — |
| **1** | implement | Pure `candle_usd_subject(base, quote, H, prints) -> UsdAsset0 \| UsdAsset1 \| HumanOnly \| Skip` in `pair_price_usd.rs` plus unit tests. No Postgres. | Slice 0 on the MR |
| **2** | implement | `candle_builder` / `parser.rs` call it with both legs. `swap_events.price_usd` path unchanged. NULL USD upsert. Skip rules. | 1 |
| **3** | implement | `CandleRow` / `CandleResponse` / `usd_leg`. | 2 |
| **4** | implement | Gap fill only for pairs with zero candle rows. Stamp table + stored `usd_leg` + `down.sql`. | 1 |
| **5** | implement | `priceChartCandles.ts` (`usd_leg=asset_1` multiply mapper; do not route that series through `applyChartDisplayInvert`), `PriceChart.tsx`, `IndexerCandle`, vitest. Trade `inverted` unchanged. | 3 |
| **6** | implement | `make verify-issue-1315` (lib tests + those vitest files). No chain, no Playwright. | 1, 5 |

Implement lands slices 1–6 on one MR from current `main`. **Insert** slice 0 into `docs/architecture.md`, `docs/indexer-invariants.md`, `docs/frontend.md`, `docs/README.md`, `docs/testing.md`, and the skill files. Do not `git checkout <design-sha> --` those shared files (overlap with #1300 / #1305 / #1311). Copy this ADR in as a new file. Keep Status **Proposed**.

## Tests

Gate: `make verify-issue-1315` runs indexer `--lib` tests for the subject function and candle skip, plus `priceChartCandles.test.ts` and the `PriceChart` human-only fixture. No LocalTerra and no running indexer.

Indexer:

- ALPHA as `asset_1`, UST1 as `asset_0`: USD = `ust1_usd / H`, `usd_leg=asset_1`, human columns = `H`, `swap_events` formula still returns NULL `price_usd`.
- ALPHA as `asset_0`, UST1 as `asset_1`: USD = `H × ust1_usd`, `usd_leg=asset_0` (same number as the other slot for the same ALPHA price).
- CL8Y/ALPHA: human OHLC set, USD none, decision is `HumanOnly`.
- One catalog leg and a missing print → `Skip`. No human-only row.
- `quote_usd_kind("ALPHA")` and `quote_usd_kind("CL8Y")` are `None`. Spoof contract is not a catalog identity.
- `H <= 0` and overflow → `Skip`.
- `usd_leg` on a written row stays the leg from that write after a fixture catalog change.
- `mark_quote_kind` is still `None` for an ALPHA `asset_1` (idle path does not write).
- Gap-fill selector is `se.price > 0` and zero `candles` rows. A pair that already has a row is not a candidate.

Frontend:

- Neither-catalog rows produce points and `PriceChart` does not mount `PriceChartEmptyState`; the heading is **Price ({quote} per {base})**.
- A catalog pair with no positive USD mounts the empty state even if `*_human` is on the fixture.
- All-invalid USD and missing human still mount the empty state.
- `NaN`, `Infinity`, empty strings, non-numeric fields never reach `setData`.
- `usd_leg=asset_1` with displayed token `asset_1` plots stored OHLC and does not call `applyChartDisplayInvert`. Trade `inverted` stays true when `asset_0` is UST1 (ticket label still **Buy ALPHA**).
- Displayed token `asset_0` on that series is `stored × H`: open with `open_human`, close with `close_human`, stored high with human low, stored low with human high, then high ≥ low.
- 24h OHLC, TWAP, and tape Price fixtures stay on swap `price_usd` (**—** when ALPHA is `asset_1`). They are not candle subject USD.
- Existing UST1/cUSTC invert tests stay green (`verify-issue-543` / `524` / `680` are regression, not this gate).

## Rollout

One implement MR. Boot migrates, then runs the zero-row gap fill. The indexer app and the dApp are two Coolify apps and do not cut over together. Until the new client is running, `usd_leg=asset_1` rows omit `open/high/low/close`, so a cached dApp shows the empty pane for that pair. This ADR does not deploy the change; production acceptance is tracked separately by [#1324](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1324) and its [Columbus-5 runbook](../runbooks/pair-twap-uint256-columbus5.md).

## Rollback

1. Run `down.sql` before an old indexer binary serves GET `/candles` (NULL `open`, or `open` that is USD of `asset_1`, fails the old read contract). Delete stamped gap-fill keys, every `usd_leg = 'asset_1'` row, and every row with NULL `open`. Then drop `candles.usd_leg` and the stamp objects.
2. Restore the previous indexer image, and the previous dApp image when it is what is running.
3. Forward ingest returns to skipping unpriced quotes. Both-catalog candles that were never stamped stay.

Do not `TRUNCATE candles` as a rollback.

## Integration completion

- ADR status stays **Proposed** until a reviewer accepts the design. Implementation and production rollout are separate; [#1324](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1324) records the live indexer and dApp verification without self-accepting this ADR.
- Slice-0 text is inserted on `main` via that MR, not left only on `cac-design-issue-1315`.
- `make verify-issue-1315` is green without a chain.
- Fixture UST1/ALPHA candle USD equals USD of 1 human ALPHA in both slots, and `swap_events.price_usd` is NULL when ALPHA is `asset_1`.
- Fixture CL8Y/ALPHA renders bars under a quote-per-base heading.
- A pair with no positive-price swaps still shows the existing empty state.
- No new `quote_usd_kind` arm and no ALPHA/CL8Y peg.
- Idle marks still skip a non-catalog `asset_1`.
- `down.sql` deletes stamped gap keys, every `usd_leg = 'asset_1'` row, and every NULL `open` row, then drops the column and stamp objects.
- A pair that already had candle rows is unchanged by the gap fill.
- A missing catalog print on a one-catalog pair inserts nothing.
- Trade `inverted` still drives the ticket when `usd_leg=asset_1`. The price pane does not use `applyChartDisplayInvert` on that series.
- 24h OHLC, TWAP, and tape Price are still swap `price_usd` (**—** when ALPHA is `asset_1`).
