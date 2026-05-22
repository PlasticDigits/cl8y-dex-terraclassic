# Agent playbook: React Query manual retry (frontend)

Use when wiring **`RetryError`** or any **Retry** control on reads that use a **high `staleTime`** and can fail with indexer/LCD errors ([GitLab **#177**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/177)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Trade page — chart pair fetch retry](../docs/frontend.md#trade-page-chart-retry) | Invariants for `/trade` chart **`getPair`** recovery |
| [`frontend-dapp/src/hooks/useQueryManualRetry.ts`](../frontend-dapp/src/hooks/useQueryManualRetry.ts) | **`invalidateQueries` + `refetch({ cancelRefetch: false })`** |
| [`frontend-dapp/src/pages/TradePage.tsx`](../frontend-dapp/src/pages/TradePage.tsx) | Chart panel: skeleton while **`isFetching`**, **`data-testid="trade-chart-retry-error"`** |
| [`frontend-dapp/src/components/ui/RetryError.tsx`](../frontend-dapp/src/components/ui/RetryError.tsx) | Optional **`isRetrying`**, **`data-testid="retry-error-button"`** |

## Rules of thumb

1. **Prefer the hook** — `const { retry, isRetrying } = useQueryManualRetry(queryKey, query)`; pass **`retry`** to **`RetryError`** / **`onRetry`**.
2. **Show in-flight UI** — hide error while **`isFetching`** after failure, or pass **`isRetrying`** so the button disables and reads **Retrying…**.
3. **404 vs outage on Trade** — chart retry is for logical indexer misses; **`trade-indexer-outage-banner`** is only when **`isIndexerUnavailableError`** is true ([`indexerErrors.ts`](../frontend-dapp/src/utils/indexerErrors.ts)).
4. **Tests** — assert the service/mock **`queryFn`** runs again after click; scope **`trade-chart-retry-error`** when both mobile and desktop layouts mount.

## Related

- **User-facing error copy:** [`AGENTS_FRONTEND_USER_ERRORS.md`](./AGENTS_FRONTEND_USER_ERRORS.md) ([GitLab **#145**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/145)).
- **LCD auto-recovery (invalidate all):** [`AGENTS_FRONTEND_LCD_CONNECTIVITY.md`](./AGENTS_FRONTEND_LCD_CONNECTIVITY.md) ([GitLab **#171**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/171)).
- **Trade layout / breakpoints:** [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md) ([GitLab **#146**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/146)).
