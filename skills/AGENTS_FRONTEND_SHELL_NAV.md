# Agent playbook: shell tab navigation (client-side routing)

Use when changing **header / mobile nav links**, **`Layout.tsx` routing UX**, or regressions where **URL or page content does not update** after clicking a tab ([GitLab **#182**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/182)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Responsive shell & header navigation](../docs/frontend.md#responsive-header-navigation) | Breakpoints + **client-side tab navigation** invariant |
| [`AppShellNavLink.tsx`](../frontend-dapp/src/components/common/AppShellNavLink.tsx) | Shared `NavLink` + explicit `navigate()` on plain left-click |
| [`navItems.ts`](../frontend-dapp/src/components/common/navItems.ts) | `PRIMARY_NAV_ITEMS`, `MORE_NAV_ITEMS`, `getHeaderMoreMenuItems` |
| [`RouteContentReadyContext.tsx`](../frontend-dapp/src/contexts/RouteContentReadyContext.tsx) | Deferred NFA footer — **no render-phase `setState`** (can break router; #182) |
| [`navigation.spec.ts`](../frontend-dapp/e2e/navigation.spec.ts) | E2E: Pool/Trade/Swap tab transitions at 1440px desktop |

## Rules of thumb

1. **Do not** put raw `<NavLink>` in `Layout` for shell tabs — use **`AppShellNavLink`** so Keplr and similar extensions cannot leave history stale while `:active` styles still flash.
2. **Do not** add render-phase `setState` in `RouteContentReadyProvider` when `pathname` changes; `readyForPath === pathname` already prevents stale footer ready state ([GitLab **#138**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)).
3. Keep route lists in **`navItems.ts`** only; extend Playwright when adding primary tabs at full-desktop width.
4. **Swap route is `/`** (label “Swap”), not `/swap` — catch-all `*` redirects unknown paths to `/`.

## Related

- Responsive header density: [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md) ([GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136))
- NFA footer / route ready gate: [`AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](./AGENTS_FRONTEND_RISK_DISCLAIMERS.md) ([GitLab **#138**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138))
- Lazy route chunks: [`AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md`](./AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md) ([GitLab **#172**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/172))
