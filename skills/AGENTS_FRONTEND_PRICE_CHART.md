# Agent playbook: Price (USD) chart (lightweight-charts)

Use when changing **`PriceChart`**, **`PriceChartLightweightCanvas`**, candle mapping in **`priceChartCandles.ts`**, **price-scale / Y-axis** behavior, or Vitest coverage under **`frontend-dapp/src/components/charts/`**.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Trade page — price chart invariants](../docs/frontend.md#trade-page-price-chart-invariants) | Empty-state rules, lightweight-charts vs hosted TradingView, **USD Y-axis invariants** ([GitLab **#113**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/113), [**#151**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151)) |
| `frontend-dapp/src/components/charts/priceChartPriceScale.ts` | Pure helpers: visible-range min `low`, clamp autoscale so the price pane never scales below **0** or below the **lowest visible candle** |
| `frontend-dapp/src/components/charts/PriceChartLightweightCanvas.tsx` | `autoscaleInfoProvider` on the candlestick series |
| `frontend-dapp/src/components/charts/__tests__/priceChartPriceScale.test.ts` | Regression tests for scale clamp |

## Invariants (summary)

1. **Non-negative USD scale:** The right price scale for the candlestick pane must not show labels **below zero** when autoscaling ([GitLab **#151**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151)).
2. **Respect visible lows:** After default autoscale runs, raise `priceRange.minValue` to at least `max(0, min(low) over logically visible bars)` so padding does not invent “negative price” dead space below the candles.
3. **Volume pane** uses a separate pane/series; this playbook applies to the **USD OHLC** series only.

## Related

- Trade grid / responsive layout: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- Local indexer + wallet QA: [`AGENTS_LOCALNET_TRADING_SWARM.md`](./AGENTS_LOCALNET_TRADING_SWARM.md), [`AGENTS_BUNDLE_DEV_WALLET.md`](./AGENTS_BUNDLE_DEV_WALLET.md)
