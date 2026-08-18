# Agent playbook: Pair Price (USD) scale + oracle (GitLab #522)

Audience: third-party agents touching indexer swap prices, candles, Trade/Charts **Price (USD)**, or limit-order USD notional.

**Issue:** [GitLab **#522**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522)  
**Invariants table:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **Pair price human + USD #522**)  
**Related:** [#466](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/466) orientation, [#515](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/515) oracles, [#508](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/508) UST1 secondary AMM

## Problem class

`swap_events.price` was a **raw** quote-per-base integer ratio. The dApp passed `trades[0].price` into **Price (USD)** (headline, candles, pair Open/Close, limit escrow USD). Mixed-decimal pairs (UST1 6 / USTR 18) showed `79.7181T`; same-decimal UST1/cUSTC showed `~206` instead of ~$1.

## Invariants (P522-1–P522-5)

| ID | Rule |
|----|------|
| **P522-1** | `swap_events.price` is **human** quote-per-base: `raw × 10^(decimals_base − decimals_quote)`. Orientation stays #466 (quote per base). |
| **P522-2** | `swap_events.price_usd` is **USD of 1 human unit of pair base (`asset_0`)**: `price × USD(1 human quote)`. |
| **P522-3** | Quote USD catalog: `uusd`/USTC/cUSTC → #515 `ustc`; `uluna`/LUNC/cLUNC → #515 `lunc`; **UST1 = `hub_prices.ust1`**; **USTR = `hub_prices.ustr`** ([#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556)). Unknown quote → `price_usd` NULL. Client `pairPriceUsd.ts` may keep a **legacy** 2.5× fallback for pre-upgrade indexers only. |
| **P522-4** | Candle `open/high/low/close` are factory **USD from `price_usd` only** (no human fallback — [#543](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/543)). Additive `*_human` columns are human quote-per-base for per-bar `invertUsd`. Bars with NULL `price_usd` are omitted. |
| **P522-5** | UI **Price (USD)** (headline, chart, Charts Open/Close, limit USD notional) must use `price_usd` / `*_usd` stats — **never** raw or human `trades[].price`. Trades table **Price** stays human quote-per-base and must not compact-format as `T`. |

## Do / don’t

- **Do** pass `tapeLastPriceUsd={resolveTapeLastPriceUsd({ priceUsd: trade.price_usd, … })}`.
- **Do** keep CG/CMC `last_price` on human `price` (quote per base) so aggregators are not silently switched to USD.
- **Don’t** wire `trades[0].price` into any control labeled USD.
- **Don’t** treat `formatNum` compact `T` as a price formatter — use `formatPairPrice`.
- **Don’t** double-scale: after this migration, `price` is already human. Client fallback scales `price` as **raw** only when `price_usd` is missing (pre-upgrade indexer).

## Regression checklist

1. `cd indexer && cargo test --lib pair_price_usd swap_orientation -- --quiet`
2. `cd indexer && cargo test --test swap_price_human_usd --test swap_price_orientation -- --test-threads=1`
3. Frontend: `pairPriceUsd.test.ts`, `chartHeadlinePrice.test.ts`, `formatAmount.test.ts`
4. `make verify-issue-522`

## Related

- [`AGENTS_INDEXER_EXTERNAL_ORACLE.md`](./AGENTS_INDEXER_EXTERNAL_ORACLE.md) — USTC/LUNC feeds
- [`AGENTS_FRONTEND_PRICE_CHART.md`](./AGENTS_FRONTEND_PRICE_CHART.md) — headline / Y-axis
- [`AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md) — escrow USD anchor
- [`AGENTS_UST1_SECONDARY_AMM.md`](./AGENTS_UST1_SECONDARY_AMM.md) — UST1/cUSTC + UST1/USTR pairs
- [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) — **UI-only** invert of Price (USD) for UST1-as-base pairs ([#524](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/524)). **P522-1–P522-3 / P522-5 indexer math is unchanged.** Additive candle `*_human` is [#543](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/543).
- [`AGENTS_FRONTEND_USD_CANDLE_INVERT.md`](./AGENTS_FRONTEND_USD_CANDLE_INVERT.md) — dApp USD candles use `invertUsd`, not `1/x` ([#543](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/543))
- [`AGENTS_FRONTEND_CHARTS_OVERVIEW.md`](./AGENTS_FRONTEND_CHARTS_OVERVIEW.md) — Charts 24h volume USD uses [`volume_usd_for_swap`](../indexer/src/indexer/pair_price_usd.rs) (P522-Q / hub USD, [#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548) / [#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556)). Pair-list USD badges stay [#544](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/544).
- [`AGENTS_INDEXER_HUB_USD.md`](./AGENTS_INDEXER_HUB_USD.md) — DEX hub marks replace `$1` / `2.5×` ingest ([#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556)); `make verify-issue-556`
- [`AGENTS_FRONTEND_PAIR_CATALOG_RANK.md`](./AGENTS_FRONTEND_PAIR_CATALOG_RANK.md) — pair picker order + **human** `volume_quote_24h` display ([#534](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534)). Indexer `volume_quote_24h` stays **raw**; do not treat it like human `price`.
