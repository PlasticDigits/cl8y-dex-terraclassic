# Agent playbook: responsive header & navigation density

Use when changing **`Layout.tsx`**, **primary / More nav items**, **`HEADER_FULL_NAV_MIN_WIDTH_PX`**, or Playwright tests under **`frontend-dapp/e2e/navigation.spec.ts`** that assert header overlap.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Responsive shell & header navigation](../docs/frontend.md#responsive-header-navigation) | Breakpoint invariants, mobile vs tablet vs desktop behavior ([GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136)) |
| `frontend-dapp/src/components/common/navItems.ts` | `PRIMARY_NAV_ITEMS`, `MORE_NAV_ITEMS`, `getHeaderMoreMenuItems`, `HEADER_FULL_NAV_MIN_WIDTH_PX` |
| `frontend-dapp/src/hooks/useMediaQuery.ts` | Subscribes to `matchMedia` for the full-desktop header breakpoint |
| `frontend-dapp/src/index.css` | `.app-desktop-nav` vs `.app-mobile-nav-shell` visibility (`max-width: 767px`) |

## Rules of thumb

1. **Do not** duplicate route lists in `Layout.tsx`; extend `navItems.ts` and keep **one** source of truth for what appears under header More at each tier.
2. **Tablet compact** (`768px`–`1023px`): folding Pool/Limits/Trade/Charts into header More is intentional — preserve reachable CTAs and overlap coverage in E2E.
3. **Mobile More sheet** must stay **`MORE_NAV_ITEMS` only**; primary tabs remain on the bottom bar — do not merge primary links into the mobile sheet without revisiting UX and tests.

## Related

- Production build / source maps: [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md)
