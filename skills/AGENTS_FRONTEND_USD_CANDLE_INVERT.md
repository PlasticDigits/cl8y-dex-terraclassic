# Agent playbook: Price (USD) candles = Last (`invertUsd`, not `1/x`) (GitLab #543)

Audience: third-party agents touching `/trade` or `/charts` **Price (USD)** candles, Last headline, Y-axis `priceFormat`, indexer `GET /candles`, or #524 invert math.

**Issue:** [GitLab **#543**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/543)  
**Invariants:** [`docs/frontend.md` § Trade pair display invert](../docs/frontend.md#trade-pair-display-invert) (**C543-1–C543-9**, **T524-4**, **T524-6**)  
**Related:** [#524](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/524) UI invert, [#522](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522) factory USD of `asset_0`, [#151](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151) non-negative scale, [#226](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/226) finite candles.

## Problem class

After #524, **Last** used `invertUsd(price_usd, human)` (USD of the displayed base) but candles used `invertOhlc` = naive `1/x` of factory USD. USTR printed as `~$1` on the pane while Last showed `$0.012`; inverted cLUNC printed `~21260` instead of `~$1`. Lightweight-charts default 2-dp `priceFormat` also rendered `$0.000047` as `0.00`.

## Do / don’t

- **Do** invert USD candles with `invertUsd` / `invertUsdNumber` per bar: `display_usd = factory_usd / human_quote_per_base`.
- **Do** give the client both series: factory USD in `open/high/low/close`, human quote-per-base in `*_human`.
- **Do** drop bars with missing / `≤ 0` / non-finite USD (no human-on-USD-axis fallback).
- **Do** set adaptive `priceFormat` via series `applyOptions` (do not recreate the chart — #148).
- **Do** run SMA/RSI on the **display USD** series after invert. Volume stays quote/base volume, **human-scaled by pair-leg decimals** ([#564](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/564)).
- **Don’t** call `invertOhlc` / `1/x` on a USD-of-`asset_0` series. `invertOhlc` is **human** book/limit only.
- **Don’t** scale every historical bar by the *latest* tape human price.
- **Don’t** change indexer `swap_events.price` / `price_usd`, CG/CMC `last_price`, or on-chain convert-on-submit (**T524-1**, **T524-2**).
- **Don’t** change default invert rules (**T524-3**). Do not substring-match `cUSTC`.
- **Don’t** describe invert as mint/redeem (**U1**).

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/utils/tradePairDisplayOrientation.ts` | `invertUsd` / `invertUsdNumber` (USD); `invertOhlc` (human only) |
| `frontend-dapp/src/components/charts/priceChartCandles.ts` | `indexerCandlesToFactoryPoints` + `applyChartDisplayInvert` |
| `frontend-dapp/src/components/charts/priceChartPriceScale.ts` | `usdCandlePriceFormat` adaptive precision / `minMove` |
| `frontend-dapp/src/components/charts/PriceChart.tsx` | Wires display USD series; Last already uses `resolveDisplayTapeLastPriceUsd` |
| `frontend-dapp/src/components/charts/PriceChartLightweightCanvas.tsx` | `applyOptions({ priceFormat })` on invert / pair / interval |
| `indexer/src/db/queries/candles.rs` | USD from `price_usd` only; additive `*_human` from `se.price` |
| `indexer/migrations/20260817000000_candle_human_ohlc.sql` | Additive columns + rebuild |

## Regression

```bash
make verify-issue-543
make verify-issue-524
make verify-issue-522
```

Vitest: `priceChartCandles.test.ts`, `tradePairDisplayOrientation.test.ts`, `pairPriceUsd.test.ts`, `priceChartPriceScale.test.ts`, `PriceChart.test.tsx`, `PriceChartLightweightCanvas.test.tsx`, `priceChartLightweightRealLibrary.charts.test.ts` (invert + format).

Indexer: `candle_human_usd.rs`, `candle_skip_zero_price.rs`.

## Related

- [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) — pill / ticket / convert-on-submit (#524)
- [`AGENTS_FRONTEND_CHARTS_PAIR_STATS.md`](./AGENTS_FRONTEND_CHARTS_PAIR_STATS.md) — pair 24h Stats + TWAP human scale; histogram decimals (#564)
- [`AGENTS_FRONTEND_PRICE_CHART.md`](./AGENTS_FRONTEND_PRICE_CHART.md) — canvas / Y-axis / setData
- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — factory `price_usd` meaning unchanged
