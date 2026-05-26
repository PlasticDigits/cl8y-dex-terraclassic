# Agent playbook: Price (USD) chart (lightweight-charts)

Use when changing **`PriceChart.tsx`**, **`PriceChartLightweightCanvas.tsx`**, **`priceChartLightweightIndicatorSync.ts`**, **`PriceChartOverlayMenu.tsx`**, **`chartHeadlinePrice.ts`**, **`priceChartCandles.ts`**, **`priceChartIndicators.ts`**, **`priceChartPriceScale.ts`**, candle mapping or **USD Y-axis** behavior, Vitest stubs in **`lightweightChartsJsdomMock.ts`**, or tests/docs tied to GitLab [**#113**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/113), [**#148**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/148) (timeframe selector freeze), [**#149**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/149), [**#150**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/150), [**#151**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151), [**#180**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/180) (pair switch must not wait on parent `getPair`).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Trade page — price chart invariants](../docs/frontend.md#trade-page-price-chart-invariants) | Empty-state rules, volume quote/base fallback, indicators, fullscreen, **USD Y-axis**, **viewport / flex** ([#151](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151)) |
| `frontend-dapp/src/components/charts/chartHeadlinePrice.ts` | **Last** headline: tape USD vs last candle close ([#149](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/149)) |
| `frontend-dapp/src/components/charts/priceChartPriceScale.ts` | Pure helpers: visible-range min `low`, clamp autoscale so the price pane never scales below **0** or below the **lowest visible candle** ([#151](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151)) |
| `frontend-dapp/src/components/charts/PriceChartLightweightCanvas.tsx` | Multi-pane lightweight-charts: `autoscaleInfoProvider` on candles; volume histogram (pane 1); optional MA / RSI applied via [`priceChartLightweightIndicatorSync.ts`](../frontend-dapp/src/components/charts/priceChartLightweightIndicatorSync.ts) (no full chart recreate on toggle) |
| `frontend-dapp/src/components/charts/priceChartIndicators.ts` | Pure SMA / RSI math |
| `frontend-dapp/src/components/charts/__tests__/priceChartPriceScale.test.ts` | Regression tests for scale clamp |
| `frontend-dapp/src/test/lightweightChartsJsdomMock.ts` | Vitest double for `createChart` / series APIs |

## Invariants (summary)

1. **Naming:** Use **TradingView lightweight-charts** or **lightweight-charts** — not the hosted **TradingView Widget** product ([docs/frontend.md](../docs/frontend.md)).
2. **Non-negative USD scale:** The candlestick pane’s right price scale must not show autoscale **below zero** or invent dead space **below the lowest visible candle low** ([#151](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151)).
3. **Volume bars:** Histogram uses **quote** volume first; if `volume_quote` is zero, **base** volume is used so local indexers still draw bars ([#150](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/150)).
4. **Indicators:** MA / RSI toggles **do not recreate** the chart. After the first `createChart`, `syncPriceChartIndicatorOverlays` adds/removes line series on pane 0 and adds/removes the RSI pane (`addPane` / `removePane(2)`) so QA toggles always match UI state ([GitLab **#150**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/150)). Pure math stays in `priceChartIndicators.ts`; sync logic: `priceChartLightweightIndicatorSync.ts`.
5. **Fullscreen:** Uses **`requestFullscreen`** on the chart card root; listen for **`fullscreenchange`** for aria / button label.
6. **Headline price:** **Last** beside the title prefers indexer tape USD (`tapeLastPriceUsd` prop), else last candle close ([#149](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/149)).
7. **Tests:** Extend **`lightweightChartsJsdomMock`** when adding series APIs (`createPriceLine`, `priceScale`, etc.). Prefer **`data-testid`** over canvas assertions in Vitest.
8. **Plot height in `/trade`:** `PriceChart` is **`h-full flex flex-col min-h-0`**; the plot uses **`flex-1 min-h-0`** with **`min-h-[min(52vh,280px)]`** (not a fixed tall block) so parents with **`overflow-hidden`** do not clip the canvas ([#151](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151)).
9. **Post-layout sizing:** After `createChart`, **`PriceChartLightweightCanvas`** calls **`applySize`** on a **double `requestAnimationFrame`** so width/height match the container after flex/grid layout ([#151](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151)).
10. **Timeframe switches ([#148](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/148)):** Candles query uses **`keepPreviousCandlesForIntervalSwitch`** (`priceChartCandlesPlaceholder.ts`) so **interval** changes do **not** unmount the canvas, but **pair** changes drop placeholder data and remount via **`key={pairAddress}`** (aligns with [pair switch](./AGENTS_FRONTEND_TRADE_PAIR_SWITCH.md)). Only the **first** load shows **Loading chart…**; interval refetches show **`price-chart-interval-loading`**. **`createChart` runs once** per canvas mount; interval updates use **`setData`**. Guard stale async inits with **`chartInitIdRef`** in `PriceChartLightweightCanvas`. Regression: `PriceChart.test.tsx` — *reuses one chart instance across many interval switches*; *remounts chart on pair switch and keeps interval switches responsive*.

## Related

- Trade page grid / breakpoints: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- Pair switch latency (parent mounts chart on `pairAddr`, not `getPair`): [`AGENTS_FRONTEND_TRADE_PAIR_SWITCH.md`](./AGENTS_FRONTEND_TRADE_PAIR_SWITCH.md) ([GitLab **#180**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/180))
- Local indexer + wallet QA: [`AGENTS_LOCALNET_TRADING_SWARM.md`](./AGENTS_LOCALNET_TRADING_SWARM.md), [`AGENTS_BUNDLE_DEV_WALLET.md`](./AGENTS_BUNDLE_DEV_WALLET.md)
- QA / Playwright workers: [`.cursor/rules/playwright-workers.mdc`](../.cursor/rules/playwright-workers.mdc)
