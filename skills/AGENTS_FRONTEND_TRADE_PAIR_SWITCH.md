# Agent playbook: Trade page pair switch latency

Use when changing **`TradePage.tsx`**, **`TradePairSwitchStatus.tsx`**, **`tradePairPrefetch.ts`**, **`tradePairWorkspaceFetching.ts`**, **`MenuSelect`** `onOptionIntent`, or tests/docs tied to GitLab [**#180**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/180) (W13-C2 — pair switch feels frozen / 8–10s load).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Trade page — pair switch latency](../docs/frontend.md#trade-page-pair-switch-latency) | Invariants: parallel fetch, loading status, prefetch, no `getPair` gate on chart mount |
| `frontend-dapp/src/pages/TradePage.tsx` | `TradeChartSlot`, `useIsFetching` + `isTradePairWorkspaceQuery`, `prefetchTradePairWorkspace` on route/change/hover |
| `frontend-dapp/src/utils/tradePairPrefetch.ts` | Prefetch `getPair`, `getCandles`, `getTrades`, both `limitBookPage` sides via **`prefetchInfiniteQuery`** ([#354](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/354)) |
| `frontend-dapp/src/components/trade/TradePairSwitchStatus.tsx` | `data-testid="trade-pair-switch-loading"` |
| `frontend-dapp/src/pages/TradePage.test.tsx` | Regression: chart before `getPair`, loading banner, prefetch on select |

## Invariants (summary)

1. **No serial waterfall:** `PriceChart` must mount when `pairAddr` is valid — **not** after `getPair` returns. Candle and book queries run **in parallel** with indexer pair metadata.
2. **Immediate feedback:** While workspace queries for the active pair are fetching, show **`TradePairSwitchStatus`** (`role="status"`, `aria-live="polite"`). `PriceChart` keeps its own **Loading chart…** spinner.
3. **Prefetch:** Call `prefetchTradePairWorkspace` on pair route change, on `MenuSelect` `onChange`, and on `onOptionIntent` (pointer/focus on another option) so hover reduces click latency.
4. **404 / retry unchanged:** When `getPair` fails with a logical miss (404), `TradeChartSlot` still shows **`RetryError`** — do not mount candles over a bad deep link ([#177](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/177)).
5. **Do not use `keepPreviousData` for pair-scoped keys** on switch — stale symbols/tape from the prior pair confuse traders. **`PriceChart`** keeps prior candle rows only when **`pairAddress` is unchanged** (interval refetch); canvas remounts with **`key={pairAddress}`** ([#148](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/148)).
6. **Stale `getCandles` race ([#226](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/226)):** A slow candle response for pair A must not repaint the chart after the user selects pair B. Covered in `PriceChart.test.tsx` (mocked delays); see [price chart invariants § stale getCandles](../docs/frontend.md#trade-page-price-chart-invariants).
7. **Regression tests:** Extend `TradePage.test.tsx` when changing fetch order or loading UI.

## Related

- Price chart mount / layout: [`AGENTS_FRONTEND_PRICE_CHART.md`](./AGENTS_FRONTEND_PRICE_CHART.md)
- Trade workspace layout: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- Query manual retry: [`AGENTS_FRONTEND_QUERY_RETRY.md`](./AGENTS_FRONTEND_QUERY_RETRY.md)
