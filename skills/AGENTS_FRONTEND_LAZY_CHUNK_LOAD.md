# Agent playbook: lazy route chunk failures (offline navigation)

Use when reproducing or fixing **offline lazy-load** crashes, broken **Try Again** on route errors, or dev-server URLs in error UI ([GitLab **#172**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/172)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Lazy route chunks](../docs/frontend.md#lazy-route-chunks) | Invariant table, regression paths |
| [`LazyRoute.tsx`](../frontend-dapp/src/components/common/LazyRoute.tsx) | `loadAttempt` + fresh `lazy(loader)` on retry |
| [`ErrorBoundary.tsx`](../frontend-dapp/src/components/common/ErrorBoundary.tsx) | Route vs app shell, `onRetry`, chunk headline |
| [`chunkLoadError.ts`](../frontend-dapp/src/utils/chunkLoadError.ts) | `isChunkLoadError`, technical-detail scrub |
| [`humanizeOffChainError.ts`](../frontend-dapp/src/utils/humanizeOffChainError.ts) | Retail copy for dynamic import strings |

## Rules of thumb

1. **New lazy routes** — wrap with `<LazyRoute loader={() => import('./pages/…')} />` in [`App.tsx`](../frontend-dapp/src/App.tsx); do not add bare `React.lazy` + route `ErrorBoundary` without `onRetry`.
2. **Try Again must re-import** — never only `setState({ hasError: false })` on chunk failures; bump `loadAttempt` via `LazyRoute`'s `onRetry`.
3. **Precondition for QA** — app must already be loaded in-tab before DevTools **Offline**; a cold load while offline is Chrome's native page, not this UI.
4. **Copy funnel** — chunk strings humanize through [`humanizeUserFacingError`](../frontend-dapp/src/utils/humanizeUserFacingError.ts); see also [`AGENTS_FRONTEND_USER_ERRORS.md`](./AGENTS_FRONTEND_USER_ERRORS.md).

## Related

- **Trade hard-reload skeleton / LCP:** [GitLab **#179**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/179), [docs/frontend.md § Trade page — initial load](../docs/frontend.md#trade-page-initial-load), [`AGENTS_FRONTEND_TRADE_INITIAL_LOAD.md`](./AGENTS_FRONTEND_TRADE_INITIAL_LOAD.md).
- **LCD outage (queries, not chunks):** [GitLab **#171**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/171), [`AGENTS_FRONTEND_LCD_CONNECTIVITY.md`](./AGENTS_FRONTEND_LCD_CONNECTIVITY.md).
- **Trader route error reset on address change:** [GitLab **#126**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/126), [docs/frontend.md § Trader profile](../docs/frontend.md#trader-profile-indexer).
