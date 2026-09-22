# ADR 0012: Price candles when a factory leg is not a catalog quote

## Status

Proposed ([#1315](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1315))

Design only. This ADR does not deploy, spend, expand custody or policy, or write an approval marker. Issue keywords are not approval. Deploy, spend, custody, and policy expansion stay under [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297).

Overview: [`architecture.md`](../architecture.md#alpha-pair-price-candles). Invariant row: [`indexer-invariants.md`](../indexer-invariants.md) **Unpriced-quote candles (#1315)**. Chart heading rule: [`frontend.md`](../frontend.md) USD invert paragraph.

Numbering: ADR **0008** stays on `origin/cac-design-issue-1300`. ADR **0010** / **Q23** stay on `origin/cac-design-issue-1305`. ADR **0011** / **Q24** stay on `origin/cac-design-issue-1311`. This ticket is **0012**. It does not take a **Q** row (those are leftover-ops). Do not merge this design branch (`cac-design-issue-1315`) as a design-only PR. Implement keeps Status **Proposed ([#1315])** until a reviewer accepts the design.

## Outcome

1. A factory pair that includes ALPHA and has at least one indexed swap with a positive human quote-per-base price shows candlesticks on `/charts` at the default interval, not `PriceChartEmptyState`.
2. When the other leg is a P522-Q catalog quote (UST1, USTC/cUSTC/`uusd`, LUNC/cLUNC/`uluna`, USTR, or the pinned USDT CW20), candle `open/high/low/close` are **USD of 1 human unit of the non-catalog leg** (ALPHA), whichever `asset_infos` slot that leg occupies. `usd_leg` on GET says which slot that is.
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

Non-positive `H`, non-finite values, and overflow (`NUMERIC(38, 18)` for USD, `NUMERIC(78, 18)` for human) skip the whole candle write. A missing catalog print (oracle or hub down) skips the USD write for that swap. Do not substitute human `H` into the USD columns. Do not read `economic_token_marks`.

Idle marks stay as they are: `mark_quote_kind` requires a catalog `asset_1`. They must not gain an `asset_0` arm that would merge ~$1 UST1 into a bucket whose USD means ALPHA.

### Interface

GET `/api/v1/pairs/{addr}/candles` (`CandleResponse`):

- `open`, `high`, `low`, `close` become optional strings. Omit them when NULL (`skip_serializing_if`).
- Add optional `usd_leg`: `"asset_0"` or `"asset_1"`, present only when USD OHLC is present. Read-time value from the pair’s current catalog classification (one leg for the whole pair). Old both-catalog rows already match `asset_0`; clients that ignore the field keep today’s behavior on those pairs.
- `*_human` unchanged in meaning.

`IndexerCandle.open/high/low/close` become `string | null | undefined` plus optional `usd_leg`.

`CandleRow` USD fields become `Option<BigDecimal>`. `upsert_candle` binds NULL USD for the neither-catalog path.

### Frontend state

No new store. `PriceChart` derives one series from the GET payload:

- **USD series** when at least one bar has finite positive USD OHLC. Heading stays **Price (USD)**. Bars without positive USD are dropped from this series (still no human-on-USD fallback).
- **Human series** when no bar has positive USD and at least one bar has finite positive `*_human`. Heading is `Price ({asset_1 symbol} per {asset_0 symbol})`. Hide the invert pill. Do not call `invertUsd`. Last is that series’ last close, not a USD figure and not raw `trades[].price` stuffed into a USD prop.
- **Empty** otherwise, including a no-trade pair. Copy in `PriceChartEmptyState` stays as shipped.

Display token for a USD series:

- No `usd_leg`, or `usd_leg=asset_0`: existing path. Charts hero and Trade invert unchanged. Inverted display uses per-bar `invertUsd(usd, H)` and swaps high/low. Never `1/x` of the USD series.
- `usd_leg=asset_1`: the stored series is already USD of `asset_1`. Default the displayed token to `asset_1` when the user has no `?price=` and no stored invert flag, so `/charts` on UST1/ALPHA shows ALPHA USD instead of a flat catalog line. Explicit `?price=` / pill still selects either leg. Plot stored OHLC when the displayed token is `asset_1`. When the displayed token is `asset_0`, plot `stored × H` (USD of `asset_0`) per bar and swap high/low so high ≥ low. Drop the bar when `H` is missing or non-positive. This is not a change to **C680** (both legs catalog, `usd_leg` stays `asset_0`) or to **T524-3** (Trade already displays `asset_1` when `asset_0` is UST1; do not run `invertUsd` on top of an `asset_1` series).

Volume histogram uses the same bars as the plotted series (human-only rows included). Color follows plotted close vs open. Volume is still not inverted. SMA/RSI run on the plotted series.

`/trade` uses the same `PriceChart`, so it follows the same series rules.

### Complexity

Added: one pure subject function, optional USD on the candle row/JSON, `usd_leg`, a one-shot gap fill, and a human heading branch. Removed: the early return that discarded `*_human` whenever quote USD was missing. No new service, catalog arm, or peg.

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

## Affected invariants

- **P522-1 / P522-2 / P522-3** unchanged. Unknown quote → `swap_events.price_usd` NULL. USDT stays the pinned CW20, not a symbol arm.
- **P522-4** still holds for both-catalog pairs (USD from `price_usd` of `asset_0` only). **#1315** adds the mixed and neither-catalog candle rows above. Human `H` still never lands in USD columns.
- **P522-5 / C543** still hold on any control labeled **Price (USD)**. The human series uses a different heading.
- **C568** idle marks unchanged. Gap fill must not `UPDATE` a non-null USD OHLC or any `swap_events.price_usd`.
- **C680 / T524-3** unchanged for both-catalog pairs. Mixed `usd_leg=asset_1` defaults the displayed token to that leg only when no `?price=` and no stored invert.
- **C705** newest-N read unchanged. NULL USD rows are still returned (they are real buckets) and the client drops them from the USD series.
- **#113** empty-state copy unchanged.
- **H10** CG/CMC `last_price` stays human.

## Migration

No `TRUNCATE`. No rewrite of non-null candle USD and no rewrite of `swap_events.price_usd`.

New unique migration prefix (do not reuse `20260916120000`):

- `candle_gap_fill_1315 (pair_id, interval, open_time)` primary key, plus a single done marker row so boot does not scan forever.

After `sqlx::migrate`, one indexer pass (info log, does not crash boot on failure):

- Pairs that already have candle rows: leave them. ALPHA-as-`asset_0` with a catalog `asset_1` is already correct.
- Pairs with `se.price > 0` and no candle row for that bucket: insert `*_human` from `se.price`.
- USD on those new rows only: USTC/LUNC from as-of `oracle_prices` at the swap time (same helper family as #568); USDT = advisory $1; UST1/USTR = hub snapshot **at this pass only**. Stamp the inserted keys. Later hub refresh must not `UPDATE` them.
- Forward swaps after deploy use the catalog print at ingest time (same clock as other candles).

`down.sql` deletes only stamped gap-fill keys, then drops the stamp table. See Rollback.

## Observability

Tracing only (no `/metrics`, [#200](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/200)).

- Debug `candle_write` with `pair_id` and `mode` = `usd_asset_0` | `usd_asset_1` | `human_only` | `skip`.
- One info line from the gap fill: pairs considered, rows inserted, rows skipped, failure count.

## Failure modes

| Case | Behavior |
|------|----------|
| Oracle or hub missing for the catalog leg | Skip USD for that swap. Do not write `H` into USD columns. If the bucket has no USD yet, GET omits USD and the USD chart stays empty until a later swap has a catalog print. Human-only pairs do not need the oracle. |
| `H <= 0`, overflow, NaN | Skip the candle write. Client drops non-finite / non-positive / empty OHLC before `setData`. |
| Spoof CW20, symbol `ALPHA`, other leg unpriced | `quote_usd_kind` is `None`. Neither-catalog path: human OHLC, no USD. |
| Spoof CW20, symbol `ALPHA`, other leg UST1 | Generic mixed rule. USD is the UST1 cross of that CW20, not $1 and not a registry pin. |
| Idle mark tick on ALPHA-as-`asset_1` | Still skipped. Must not merge catalog USD into the ALPHA series. |
| Old indexer binary reads a NULL `open` | `CandleRow` decode error on that pair’s GET. Rollback runs `down.sql` first. |
| Indexer ships before the dApp | Both-catalog charts unchanged. Mixed pairs show USD numbers under **Price (USD)** but the dApp still treats them as USD of `asset_0`, and Trade `invertUsd` double-applies when `usd_leg=asset_1`. Ship indexer and dApp together. |

## Slices

| Slice | Owner | Work | Depends on |
|-------|--------|------|------------|
| **0 — this design** | design | This ADR, architecture `#alpha-pair-price-candles`, invariant row, `frontend.md` sentence, `docs/README.md` index, `docs/testing.md` row, skill pointers. Branch `cac-design-issue-1315` only. | — |
| **1** | implement | Pure `candle_usd_subject(base, quote, H, prints) -> UsdAsset0 \| UsdAsset1 \| HumanOnly \| Skip` in `pair_price_usd.rs` plus unit tests. No Postgres. | Slice 0 on the MR |
| **2** | implement | `candle_builder` / `parser.rs` call it with both legs. `swap_events.price_usd` path unchanged. NULL USD upsert. Skip rules. | 1 |
| **3** | implement | `CandleRow` / `CandleResponse` / `usd_leg`. | 2 |
| **4** | implement | Gap-fill pass + stamp table + `down.sql`. | 1 |
| **5** | implement | `priceChartCandles.ts`, `PriceChart.tsx`, `IndexerCandle`, vitest. | 3 |
| **6** | implement | `make verify-issue-1315` (lib tests + those vitest files). No chain, no Playwright. | 1, 5 |

Implement lands slices 1–6 on one MR from current `main`. **Insert** slice 0 into `docs/architecture.md`, `docs/indexer-invariants.md`, `docs/frontend.md`, `docs/README.md`, `docs/testing.md`, and the skill files. Do not `git checkout <design-sha> --` those shared files (overlap with #1300 / #1305 / #1311). Copy this ADR in as a new file. Keep Status **Proposed**.

## Tests

Gate: `make verify-issue-1315` runs indexer `--lib` tests for the subject function and candle skip, plus `priceChartCandles.test.ts` and the `PriceChart` human-only fixture. No LocalTerra and no running indexer.

Indexer:

- ALPHA as `asset_1`, UST1 as `asset_0`: USD = `ust1_usd / H`, `usd_leg=asset_1`, human columns = `H`, `swap_events` formula still returns NULL `price_usd`.
- ALPHA as `asset_0`, UST1 as `asset_1`: USD = `H × ust1_usd`, `usd_leg=asset_0` (same number as the other slot for the same ALPHA price).
- CL8Y/ALPHA: human OHLC set, USD none, decision is `HumanOnly`.
- `quote_usd_kind("ALPHA")` and `quote_usd_kind("CL8Y")` are `None`. Spoof contract is not a catalog identity.
- `H <= 0` and overflow → `Skip`.
- `mark_quote_kind` is still `None` for an ALPHA `asset_1` (idle path does not write).

Frontend:

- Human-only rows produce points and `PriceChart` does not mount `PriceChartEmptyState`; the heading is not `Price (USD)`.
- All-invalid USD and missing human still mount the empty state.
- `NaN`, `Infinity`, empty strings, non-numeric fields never reach `setData`.
- `usd_leg=asset_1` plots stored USD when displaying `asset_1`, and `stored × H` when displaying `asset_0`. No `1/x` of USD. Human fixtures are not written into USD fields.
- Existing UST1/cUSTC invert tests stay green (`verify-issue-543` / `524` / `680` are regression, not this gate).

## Rollout

One implement MR. Indexer image (migration + gap fill + API) and dApp ship together so `usd_leg=asset_1` is not inverted twice. Boot migrates then runs the gap fill. This design does not deploy.

## Rollback

1. Run `down.sql`: delete `candles` rows whose `(pair_id, interval, open_time)` is in `candle_gap_fill_1315`, then drop the stamp objects. Do this before an old indexer binary serves GET `/candles` (NULL `open` fails `CandleRow`).
2. Restore the previous indexer and dApp images together.
3. Forward ingest returns to skipping unpriced quotes. Both-catalog candles that were never stamped stay.

Do not `TRUNCATE candles` as a rollback.

## Integration completion

- Status on the implement MR is still **Proposed ([#1315])**.
- Slice-0 text is inserted on `main` via that MR, not left only on `cac-design-issue-1315`.
- `make verify-issue-1315` is green without a chain.
- Fixture UST1/ALPHA candle USD equals USD of 1 human ALPHA in both slots, and `swap_events.price_usd` is NULL when ALPHA is `asset_1`.
- Fixture CL8Y/ALPHA renders bars under a quote-per-base heading.
- A pair with no positive-price swaps still shows the existing empty state.
- No new `quote_usd_kind` arm and no ALPHA/CL8Y peg.
- Idle marks still skip a non-catalog `asset_1`.
- `down.sql` deletes only stamped gap keys.
