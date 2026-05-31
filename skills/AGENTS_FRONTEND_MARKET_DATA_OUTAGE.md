# Agent playbook: market data loading & outage (frontend)

Use when adding or changing **indexer-backed** pages, global outage banners, pair-switch loading affordances, or Vitest/E2E coverage for market-data-down UX ([GitLab **#215**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/215), `/limits` alignment [#218](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/218)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Market data loading & outage (global)](../docs/frontend.md#market-data-loading-outage) | Cross-route invariants, shared components, 404 vs outage |
| [docs/frontend.md § Trade page — indexer outage banner](../docs/frontend.md#trade-page-indexer-outage-banner) | Trade-specific tail (#166), panel testids (#165), no false chain fallback (#164) |
| [docs/frontend.md § Limit orders page — market data outage](../docs/frontend.md#limits-page-market-data-outage) | `/limits` banner (`limits-market-data-outage-banner`), pair-switch loading, shared book testids |
| [`marketDataServiceCopy.ts`](../frontend-dapp/src/utils/marketDataServiceCopy.ts) | Shared title + per-route banner leads (Charts, Trader, Pool, Protocol, Swap, Limits) |
| [`indexerTradeOutageCopy.ts`](../frontend-dapp/src/utils/indexerTradeOutageCopy.ts) | Trade lead, tail, book/tape/chart panel strings |
| [`marketDataOutage.ts`](../frontend-dapp/src/utils/marketDataOutage.ts) | `detectMarketDataOutage` (trade: `detectTradeIndexerOutage`; swap: `detectSwapIndexerOutage`) |
| [`indexerErrors.ts`](../frontend-dapp/src/utils/indexerErrors.ts) | `isIndexerUnavailableError`, `isIndexerPairNotFoundError` |
| [`MarketDataServiceOutageBanner.tsx`](../frontend-dapp/src/components/common/MarketDataServiceOutageBanner.tsx) | Retail banner; trade uses `layout="inline"` + `trade-indexer-outage-banner` |
| [`MarketDataLoadingStatus.tsx`](../frontend-dapp/src/components/common/MarketDataLoadingStatus.tsx) | `role="status"` / `aria-live="polite"` loading row |
| [docs/testing.md § Frontend E2E — indexer outage](../docs/testing.md#frontend-e2e-indexer-outage) | `make test-e2e-indexer-outage` (workflow job name `frontend-e2e-indexer-outage`); project `e2e-indexer-outage` ([#219](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/219)) |

## Rules of thumb

1. **Never** put `VITE_INDEXER_URL`, `INDEXER_URL`, or raw hostnames in retail DOM ([#174](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/174)).
2. **Do not** treat indexer **404** as a global outage — use not-found / `RetryError` paths ([#177](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/177)).
3. **Keep LCD separate** — chain probe failures use [`LcdConnectivityBanner`](./AGENTS_FRONTEND_LCD_CONNECTIVITY.md), not market-data copy ([#171](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/171)).
4. **Trade regressions** — preserve `trade-indexer-outage-banner`, `trade-pair-switch-loading`, and panel `trade-*-unavailable` testids; run [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx).
5. **`/limits`** — `detectMarketDataOutage(indexerPairQuery, tradesQuery)` only; `limits-market-data-outage-banner` + optional `limits-pair-switch-loading`; run [`LimitOrdersPage.test.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.test.tsx).
6. **`/` / `/swap`** — `detectSwapIndexerOutage(simQuery, simData)`; track `indexerTransportFailed` when indexer HTTP fails but LCD pool sim succeeds; hide stale quotes on `simQuery.isError`; `swap-market-data-outage-banner`; wrap/unwrap paths skip indexer — run [`SwapPage.test.tsx`](../frontend-dapp/src/pages/SwapPage.test.tsx) ([#241](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/241)).
7. **New retail routes** — reuse `MarketDataServiceOutageBanner` + `detectMarketDataOutage`; add a page-specific lead in `marketDataServiceCopy.ts` and a Vitest outage case.

## Related

- Indexer-outage Playwright automation (`make test-e2e-indexer-outage`): [`AGENTS_E2E_INDEXER_OUTAGE.md`](./AGENTS_E2E_INDEXER_OUTAGE.md) ([#219](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/219))
- Trade layout / pair switch: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md), [`AGENTS_FRONTEND_TRADE_PAIR_SWITCH.md`](./AGENTS_FRONTEND_TRADE_PAIR_SWITCH.md)
- Limit price / place gate under outage: [`AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md)
- `/limits` book row actions (shared `trade-book-*` testids): [`AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md`](./AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md)
- Retail error funnel: [`AGENTS_FRONTEND_USER_ERRORS.md`](./AGENTS_FRONTEND_USER_ERRORS.md)
- Query retry (404 chart): [`AGENTS_FRONTEND_QUERY_RETRY.md`](./AGENTS_FRONTEND_QUERY_RETRY.md)
- Gap analysis §4: [`gaps/GAP_1780023683.md`](../gaps/GAP_1780023683.md)
