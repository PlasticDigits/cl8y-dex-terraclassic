# Agent playbook: time-stamped candle USD + idle mark-to-market (GitLab #568)

Audience: third-party agents touching indexer `hub_prices` refresh, `swap_events.price_usd`, `GET /candles`, or `/charts` `/trade` **Price (USD)** for UST1/cUSTC, UST1/USTR, or LUNC-quoted pairs.

**Issue:** [GitLab **#568**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/568)  
**Invariants:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **Candle USD clock #568**)  
**Verify:** `make verify-issue-568`

## Problem class

Two coupled indexer bugs flattened Charts **Price (USD)** to the *live* hub tick:

1. Every hub refresh rewrote **all** historical `swap_events.price_usd` and candle USD as `human × current hub_prices`.
2. Candles were swap-only, so idle USTC/LUNC/hub moves never extended bars.

Inverted UST1/cUSTC (`invertUsd` = quote USD) became a horizontal line at today’s CoinGecko-ish USTC even when CEX USTC moved and the pair had no new swaps.

## Invariants (C568-1–C568-8)

| ID | Rule |
|----|------|
| **C568-1** | Hub refresh **replaces** the live `hub_prices` snapshot only. It must **not** `UPDATE` historical `swap_events.price_usd` or past candle USD. |
| **C568-2** | Ingest still stamps `price_usd` as-of that block (P522-2 × then-current oracle/hub). Two prints at USTC `$0.005` then `$0.004` keep distinct `price_usd` after a later refresh. |
| **C568-3** | After hub/oracle materialize, a mark writer upserts the **current** bucket for each interval (`1m`…`1w`) on catalog-quoted **active** factory pairs: `usd = last_human × as-of quote USD`. Clock is indexer wall time (`truncate_to_interval`), not an absent swap’s block time. |
| **C568-4** | `last_human` prefers the latest swap `price`; if none, reserve-spot human quote-per-base (H2/H3) so a seeded idle UST1/cUSTC pool still charts. Human OHLC on swap bars stays DEX prints. |
| **C568-5** | Mark-only bars have `trade_count = 0` and zero `volume_*`. Swap bars still increment. Histogram must not invent volume. Overview 24h USD stays `swap_events.volume_usd` (**C5** / **C8**). |
| **C568-6** | Quote catalog only (P522-Q / A1): USTC/cUSTC/`uusd`, LUNC/cLUNC/`uluna`, UST1, USTR by **contract/denom**, not symbol. Native `symbol=USTR` gems get no USD marks. Oracle down → no new marks, no `$1` freeze (**H1**). Overflow / non-positive → skip. |
| **C568-7** | UI invert stays per-bar `invertUsd` (**C543** / **T524**). Do **not** client-stitch CoinGecko or `GET /oracle/history` onto pair charts. CG/CMC `last_price` stays human (**H10**). `/oracle/price/ust1` stays **400**. |
| **C568-8** | One-shot SQL repair rebuilds USTC/LUNC-quoted history from `oracle_prices` average as-of timestamp. **UST1/USTR-quoted history cannot be replayed** (no `hub_prices` time series). Not the 10s loop. Marks are advisory (**X5**), not NAV / settlement / limit validation / `/ust1` window. |

## Do / don’t

- **Do** call `apply_idle_usd_marks` after `hub_prices` snapshot replace (book-snapshot cadence + current USTC/LUNC handles).
- **Do** keep GET `/candles` a materialized read (limit 1–1000, 90d default). Do not join `oracle_prices ⋈ swap_events` on GET.
- **Do** merge mark USD with `merge_candle_ohlc` (`open` unchanged in an in-progress bucket).
- **Don’t** restore `backfill_usd_from_hub` (full-table rewrite every ~10s).
- **Don’t** add `ustr` / `ust1` / `custc` to CEX `OracleTicker`.
- **Don’t** fabricate `trade_count` for oracle ticks.
- **Don’t** back-fill arbitrary past gaps on every tick (that is the one-shot migration).

## Canonical code

| File | Role |
|------|------|
| `indexer/src/db/queries/hub_prices.rs` | Snapshot replace only; then idle marks |
| `indexer/src/indexer/candle_mark.rs` | Catalog pairs × current buckets |
| `indexer/src/indexer/candle_builder.rs` | `update_candles_for_mark` (no volume increment) |
| `indexer/src/indexer/pair_price_usd.rs` | `mark_price_usd` / reserve human |
| `indexer/src/db/queries/usd_as_of.rs` | One-shot USTC/LUNC repair |
| `indexer/migrations/20260820156800_repair_candle_usd_as_of_oracle.sql` | Prod/QA repair |
| `frontend-dapp/src/components/charts/priceChartCandles.ts` | Per-bar invertUsd; mark volume 0 |

## Reconstruction (ops)

USTC/LUNC quotes: `repair_ustc_lunc_usd_as_of_oracle` / the #568 migration.  
UST1/USTR quotes: live-only going forward. Optional later `hub_prices_history` is **out of scope** unless product wants a Protocol sparkline — pair charts must not require a new HTTP ticker.

## Regression

```bash
make verify-issue-568
make verify-issue-556
make verify-issue-543
make verify-issue-522
make verify-issue-515
```

Indexer: `cargo test --lib pair_price_usd` and `cargo test --test candle_usd_mark --test api_hub_prices --test candle_human_usd -- --test-threads=1`.  
Frontend: `priceChartCandles.test.ts` (varying as-of quote USD; latest human ≠ historical human).

## Related

- [`AGENTS_INDEXER_HUB_USD.md`](./AGENTS_INDEXER_HUB_USD.md) — live hub marks (**H1–H10**); do not rewrite tape
- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — P522 factory USD meaning
- [`AGENTS_FRONTEND_USD_CANDLE_INVERT.md`](./AGENTS_FRONTEND_USD_CANDLE_INVERT.md) — `invertUsd` per bar
- [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) — default invert UST1-as-base
- [`AGENTS_INDEXER_EXTERNAL_ORACLE.md`](./AGENTS_INDEXER_EXTERNAL_ORACLE.md) — CEX catalog stays 3 tickers
