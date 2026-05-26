# Agent playbook: frontend risk disclaimers and first-visit acknowledgement

Use when changing **NFA / risk copy**, **`RiskAcknowledgementModal`**, **`EnvironmentRibbon`**, **`riskAcknowledgement` storage**, or **Playwright env** around the first-visit gate ([GitLab #138](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Risk surfacing, NFA copy, and first-visit acknowledgement](../docs/frontend.md#legal-risk-surfacing) | Invariants, `VITE_PLAYWRIGHT_E2E`, storage key / version |
| `frontend-dapp/src/utils/riskAcknowledgement.ts` | `localStorage` persistence and `RISK_ACK_VERSION` |
| `frontend-dapp/src/components/legal/legalCopy.ts` | Single source for bullets and NFA strings shared by modal + footer |
| `frontend-dapp/src/components/ui/Modal.tsx` | `dismissible` prop for blocking dialogs |
| `frontend-dapp/playwright.config.ts` | Sets `VITE_PLAYWRIGHT_E2E=true` on `webServer` so E2E is not blocked |
| `frontend-dapp/src/contexts/RouteContentReadyContext.tsx` | Pathname-scoped ready gate for deferred NFA footer (fixes missed `window` events on nav — [GitLab #138](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)) |
| `frontend-dapp/src/components/common/RouteContentReadyMarker.tsx` | Calls `useMarkRouteContentReady()` when lazy route content mounts |
| `frontend-dapp/src/services/terraclassic/__tests__/cosmesPatch127.test.ts` | Patch-package integrity for **`npm run test:unit`** checklist ([#138](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138) verification) |
| `scripts/e2e-seed-hybrid-book.sh` | Playwright global-setup hybrid seed; must accept bare **`u64`** `order_book_head` on re-run ([#138](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)) |

## Rules of thumb

1. **Do not** make the first-visit risk modal closable via backdrop or Escape without product/legal sign-off; keep **`dismissible={false}`** on that use case.
2. **Bump `RISK_ACK_VERSION`** when changing disclaimer meaningfully so returning browsers re-run the gate.
3. **Keep `VITE_PLAYWRIGHT_E2E` off** for real builds and manual QA; it exists only to unblock Playwright against the same dev server command.
4. **Edit copy in one place** (`legalCopy.ts`) unless you intentionally split mobile vs desktop tone. **Mobile footer:** when the shell hides the desktop footer (`max-width: 767px`), `Layout` mounts a second `LegalFooterNotice` in `.app-mobile-legal-strip` so NFA copy still appears on phones.
5. **Do not** coordinate legal footer visibility with `window` custom events from `RouteContentReadyMarker`; use **`RouteContentReadyProvider`** with `readyForPath === pathname` (regression: `RouteContentReadyContext.test.tsx`, E2E “NFA footer copy promptly after route changes”). **Do not** add render-phase `setState` on pathname change in that provider — it can break shell tab navigation ([GitLab **#182**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/182); see [`AGENTS_FRONTEND_SHELL_NAV.md`](./AGENTS_FRONTEND_SHELL_NAV.md)). Keep **`Outlet key={pathname}`** in `Layout` so lazy pages remount when tabs change ([#138](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)).

## Related

- **Responsive shell / header:** [`docs/frontend.md` § Responsive shell & header navigation](../docs/frontend.md#responsive-header-navigation), [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md) ([GitLab #136](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136)).
- **Production build / maps:** [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md).
- **E2E verification blockers ([#138](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)):** [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md) (`cosmesPatch127.test.ts`), [`AGENTS_E2E_HYBRID_SWAP.md`](./AGENTS_E2E_HYBRID_SWAP.md) (`e2e-seed-hybrid-book.sh` idempotency), [`docs/testing.md`](../docs/testing.md) § E2E Tests — **#138 verification** checklist.

## Verification checklist (#138)

After LocalTerra + `deploy-dex-local.sh` + indexer (`VITE_INDEXER_URL`):

1. `cd frontend-dapp && npm ci && npm run test:unit` — includes `cosmesPatch127.test.ts`
2. `bash scripts/e2e-seed-hybrid-book.sh` twice — second run skips with existing head order
3. `npx playwright test e2e/navigation.spec.ts -g "NFA footer"` — both specs pass
4. `npx playwright test e2e/navigation.spec.ts -g "navigates to Pool"` — Pool heading visible after tab click

Shell tab nav / Outlet remount: [`AGENTS_FRONTEND_SHELL_NAV.md`](./AGENTS_FRONTEND_SHELL_NAV.md) (`f58cce5`).
