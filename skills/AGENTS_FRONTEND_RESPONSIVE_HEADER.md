# Agent playbook: responsive header & navigation density

Use when changing **`Layout.tsx`**, **primary / More nav items**, **`HEADER_FULL_NAV_MIN_WIDTH_PX`**, or Playwright tests under **`frontend-dapp/e2e/navigation.spec.ts`** that assert header overlap.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Responsive shell & header navigation](../docs/frontend.md#responsive-header-navigation) | Breakpoint invariants, mobile vs tablet vs desktop behavior ([GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136)) |
| [docs/frontend.md § Trade page — responsive layout (sub-desktop)](../docs/frontend.md#trade-page-responsive-layout) | Trade workspace grid below `lg:` — chart + ticket row on tablet ([GitLab **#146**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/146)); playbook [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md) |
| [docs/frontend.md § Keyboard focus visibility (WCAG 2.4.7)](../docs/frontend.md#keyboard-focus-visible-wcag-247) | `:focus-visible` rings on `.app-nav-link`, `.wallet-trigger`, More/menu triggers ([GitLab **#144**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/144)) |
| [docs/frontend.md § Responsive shell — theme toggle](#responsive-header-navigation) | Header vs mobile More sheet placement ([GitLab **#170**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/170)); playbook [`AGENTS_FRONTEND_THEME_TOGGLE.md`](./AGENTS_FRONTEND_THEME_TOGGLE.md) |
| `frontend-dapp/src/components/common/navItems.ts` | `PRIMARY_NAV_ITEMS`, `MORE_NAV_ITEMS`, `getHeaderMoreMenuItems`, `HEADER_FULL_NAV_MIN_WIDTH_PX` |
| `frontend-dapp/src/hooks/useMediaQuery.ts` | Subscribes to `matchMedia` for the full-desktop header breakpoint |
| `frontend-dapp/src/index.css` | `.app-desktop-nav` vs `.app-mobile-nav-shell` visibility (`max-width: 767px`) |

## Rules of thumb

1. **Do not** duplicate route lists in `Layout.tsx`; extend `navItems.ts` and keep **one** source of truth for what appears under header More at each tier.
2. **Tablet compact** (`768px`–`1023px`): folding Pool/Limits/Trade/Charts into header More is intentional — preserve reachable CTAs and overlap coverage in E2E.
3. **Mobile More sheet** must stay **`MORE_NAV_ITEMS` only**; primary tabs remain on the bottom bar — do not merge primary links into the mobile sheet without revisiting UX and tests.
4. **Header brand** — logo + **`CL8Y DEX`** title only; do **not** reintroduce a “Terra Classic ecosystem” (or similar) kicker in the sticky header — it crowded the nav row and regressed [#136](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136). Network branding belongs in footer / `NetworkBadge`, not a second header line.

## Related

- Theme toggle (header vs mobile More): [`AGENTS_FRONTEND_THEME_TOGGLE.md`](./AGENTS_FRONTEND_THEME_TOGGLE.md)
- Trade page tablet grid (`/trade`): [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- Keyboard focus rings / WCAG 2.4.7: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md)
- Production build / source maps: [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md)
