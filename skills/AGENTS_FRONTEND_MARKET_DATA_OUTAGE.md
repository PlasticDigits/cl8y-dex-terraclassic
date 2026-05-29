# Agent playbook: market data loading & outage (frontend)

Use when adding or changing **indexer-backed** pages, global outage banners, pair-switch loading affordances, or Vitest/E2E coverage for market-data-down UX ([GitLab **#215**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/215)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Market data loading & outage (global)](../docs/frontend.md#market-data-loading-outage) | Cross-route invariants, shared components, 404 vs outage |
| [docs/frontend.md § Trade page — indexer outage banner](../docs/frontend.md#trade-page-indexer-outage-banner) | Trade-specific tail (#166), panel testids (#165), no false chain fallback (#164) |
| [`marketDataServiceCopy.ts`](../frontend-dapp/src/utils/marketDataServiceCopy.ts) | Shared title + per-route banner leads (Charts, Trader, Pool, Protocol) |
| [`indexerTradeOutageCopy.ts`](../frontend-dapp/src/utils/indexerTradeOutageCopy.ts) | Trade lead, tail, book/tape/chart panel strings |
| [`marketDataOutage.ts`](../frontend-dapp/src/utils/marketDataOutage.ts) | `detectMarketDataOutage` (trade: `detectTradeIndexerOutage`) |
| [`indexerErrors.ts`](../frontend-dapp/src/utils/indexerErrors.ts) | `isIndexerUnavailableError`, `isIndexerPairNotFoundError` |
| [`MarketDataServiceOutageBanner.tsx`](../frontend-dapp/src/components/common/MarketDataServiceOutageBanner.tsx) | Retail banner; trade uses `layout="inline"` + `trade-indexer-outage-banner` |
| [`MarketDataLoadingStatus.tsx`](../frontend-dapp/src/components/common/MarketDataLoadingStatus.tsx) | `role="status"` / `aria-live="polite"` loading row |
| [docs/testing.md § Frontend E2E — indexer outage](../docs/testing.md#frontend-e2e-indexer-outage) | Opt-in `E2E_INDEXER_OUTAGE=1` Playwright |

## Rules of thumb

1. **Never** put `VITE_INDEXER_URL`, `INDEXER_URL`, or raw hostnames in retail DOM ([#174](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/174)).
2. **Do not** treat indexer **404** as a global outage — use not-found / `RetryError` paths ([#177](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/177)).
3. **Keep LCD separate** — chain probe failures use [`LcdConnectivityBanner`](./AGENTS_FRONTEND_LCD_CONNECTIVITY.md), not market-data copy ([#171](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/171)).
4. **Trade regressions** — preserve `trade-indexer-outage-banner`, `trade-pair-switch-loading`, and panel `trade-*-unavailable` testids; run [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx).
5. **New retail routes** — reuse `MarketDataServiceOutageBanner` + `detectMarketDataOutage`; add a page-specific lead in `marketDataServiceCopy.ts` and a Vitest outage case.

## Related

- Trade layout / pair switch: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md), [`AGENTS_FRONTEND_TRADE_PAIR_SWITCH.md`](./AGENTS_FRONTEND_TRADE_PAIR_SWITCH.md)
- Retail error funnel: [`AGENTS_FRONTEND_USER_ERRORS.md`](./AGENTS_FRONTEND_USER_ERRORS.md)
- Query retry (404 chart): [`AGENTS_FRONTEND_QUERY_RETRY.md`](./AGENTS_FRONTEND_QUERY_RETRY.md)
- Gap analysis §4: [`gaps/GAP_1780023683.md`](../gaps/GAP_1780023683.md)
