# Agent playbook: shell tab navigation (client-side routing)

Use when changing **header / mobile nav links**, **`Layout.tsx` routing UX**, or regressions where **URL or page content does not update** after clicking a tab ([GitLab **#182**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/182)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Responsive shell & header navigation](../docs/frontend.md#responsive-header-navigation) | Breakpoints + **client-side tab navigation** invariant |
| [`AppShellNavLink.tsx`](../frontend-dapp/src/components/common/AppShellNavLink.tsx) | Shared `Link` + `useMatch` + explicit `navigate()` on plain left-click |
| [`Layout.tsx`](../frontend-dapp/src/components/common/Layout.tsx) | **`Outlet key={location.pathname}`** — lazy route content must remount on tab change ([#138](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)) |
| [`navItems.ts`](../frontend-dapp/src/components/common/navItems.ts) | `PRIMARY_NAV_ITEMS` (includes **`/portfolio`** — [GitLab **#212**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/212)), `MORE_NAV_ITEMS`, optional **UST1** via `includeUst1` when window env is set ([GitLab **#506**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/506)), optional **Mint** via `includeMint` when `VITE_FAUCET_ADDRESS` is set ([GitLab **#473**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/473)) |
| [`RouteContentReadyContext.tsx`](../frontend-dapp/src/contexts/RouteContentReadyContext.tsx) | Deferred NFA footer — **no render-phase `setState`** (can break router; #182) |
| [`navigation.spec.ts`](../frontend-dapp/e2e/navigation.spec.ts) | E2E: Pool/Trade/Swap tab transitions at 1440px desktop |

## Rules of thumb

1. **Do not** put raw `<NavLink>` in `Layout` for shell tabs — use **`AppShellNavLink`** so Keplr and similar extensions cannot leave history stale while `:active` styles still flash.
2. **Do not** remove **`key={location.pathname}`** from `<Outlet />` — without it, URL/active nav can update while the prior lazy page (e.g. Swap) stays mounted ([#138](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)).
3. **Do not** add render-phase `setState` in `RouteContentReadyProvider` when `pathname` changes; `readyForPath === pathname` already prevents stale footer ready state ([GitLab **#138**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)).
4. Keep route lists in **`navItems.ts`** only; extend Playwright when adding primary tabs at full-desktop width.
5. **Swap route is `/`** (label “Swap”). **`/swap` and `/swap/` redirect to `/` while preserving `search` and `hash`** ([GitLab **#711**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/711)). Catch-all `*` still sends other unknown paths to `/` (query dropped). Do not mount `SwapPage` at `/swap` — tab `end: true` would go inactive.

## Related

- My Portfolio route: [`AGENTS_FRONTEND_PORTFOLIO.md`](./AGENTS_FRONTEND_PORTFOLIO.md) ([GitLab **#212**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/212))
- Responsive header density: [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md) ([GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136))
- NFA footer / route ready gate: [`AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](./AGENTS_FRONTEND_RISK_DISCLAIMERS.md) ([GitLab **#138**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138))
- Swap query params (`/swap?from=`): [`AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md`](./AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md) ([GitLab **#711**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/711))
- Lazy route chunks: [`AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md`](./AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md) ([GitLab **#172**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/172) offline Try Again; [**#706**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/706) stale Coolify hash)
- Soft-launch Mint / faucet: [`AGENTS_SOFT_LAUNCH_FAUCET.md`](./AGENTS_SOFT_LAUNCH_FAUCET.md) ([GitLab **#473**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/473)) — Mint is a **conditional** More item (F11), not a primary tab
- UST1 oracle window: [`AGENTS_UST1_WINDOW_UI.md`](./AGENTS_UST1_WINDOW_UI.md) ([GitLab **#506**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/506)) — label **UST1**, never **Mint**; conditional More item (`includeUst1`, **U1**/**U2**)
