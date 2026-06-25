# Agent playbook: Trade page initial load / LCP (W13-C1)

Use when fixing **blank white flash** on hard reload of `/trade`, **missing loading skeleton**, or Lighthouse **LCP** dominated by the legal footer ([GitLab **#179**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/179)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Trade page — initial load / LCP](../docs/frontend.md#trade-page-initial-load) | Invariant table, manual Lighthouse checklist |
| [`index.html`](../frontend-dapp/index.html) | Pre-React `#trade-bootstrap-shell` for `/trade` paths |
| [`trade-bootstrap.css`](../frontend-dapp/public/bootstrap/trade-bootstrap.css) | Critical-path skeleton tokens — must match `theme-dark.css` / `theme-light.css` `--bg-0` ([#416](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/416)) |
| [`TradePageWorkspaceSkeleton.tsx`](../frontend-dapp/src/components/trade/TradePageWorkspaceSkeleton.tsx) | Shared skeleton (`data-testid="trade-workspace-skeleton"`) |
| [`TradePageRouteFallback.tsx`](../frontend-dapp/src/components/trade/TradePageRouteFallback.tsx) | `/trade` Suspense fallback wired in [`App.tsx`](../frontend-dapp/src/App.tsx) |
| [`RouteContentReadyContext.tsx`](../frontend-dapp/src/contexts/RouteContentReadyContext.tsx) | Pathname-scoped ready gate — defers legal footer in [`Layout.tsx`](../frontend-dapp/src/components/common/Layout.tsx) ([GitLab #138](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138) nav race) |
| [`LazyRoute.tsx`](../frontend-dapp/src/components/common/LazyRoute.tsx) | Optional `fallback` prop + `RouteContentReadyMarker` |

## Rules of thumb

1. **Do not shrink the skeleton** — LCP must beat `.app-legal-footer-notice`; keep `min-h-[min(72vh,640px)]` (or equivalent) on the workspace placeholder.
2. **Three layers** — HTML bootstrap (pre-JS) → `TradePageRouteFallback` (chunk) → `TradePage` skeleton while `allPairs` loads; all should reuse `TradePageWorkspaceSkeleton` where possible.
3. **Footer stays deferred** until `RouteContentReadyMarker` mounts; do not render `LegalFooterNotice` early for “compliance” on trade load.
4. **Lazy-route offline retries** — trade fallback is separate from chunk **Try Again** ([`AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md`](./AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md)); keep both behaviors when touching `LazyRoute`.
5. **E2E** — [`trade-page-initial-load.spec.ts`](../frontend-dapp/e2e/trade-page-initial-load.spec.ts) needs LocalTerra + indexer like other trade suites.

## Related

- Trade responsive layout: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- Lazy chunk failures: [`AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md`](./AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md)
- LCD outage (infinite skeleton guard): [`AGENTS_FRONTEND_LCD_CONNECTIVITY.md`](./AGENTS_FRONTEND_LCD_CONNECTIVITY.md)
