# Agent playbook: responsive header & navigation density

Use when changing **`Layout.tsx`**, **primary / More nav items**, **`HEADER_FULL_NAV_MIN_WIDTH_PX`**, sticky header spacing, footer **environment ribbon**, header **NetworkBadge** visibility, or Playwright tests under **`frontend-dapp/e2e/navigation.spec.ts`** that assert header overlap / sticky clearance.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Responsive shell & header navigation](../docs/frontend.md#responsive-header-navigation) | Breakpoint invariants, mobile vs tablet vs desktop behavior ([GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136), nav→controls **#483**, footer ribbon) |
| [docs/frontend.md § Trade page — responsive layout (sub-desktop)](../docs/frontend.md#trade-page-responsive-layout) | Trade workspace grid below `lg:` — chart + ticket row on tablet ([GitLab **#146**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/146)); playbook [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md) |
| [docs/frontend.md § Keyboard focus visibility (WCAG 2.4.7)](../docs/frontend.md#keyboard-focus-visible-wcag-247) | `:focus-visible` rings on `.app-nav-link`, `.wallet-trigger`, More/menu triggers ([GitLab **#144**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/144)) |
| [docs/frontend.md § Responsive shell — theme toggle](#responsive-header-navigation) | Header vs mobile More sheet placement ([GitLab **#170**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/170)); playbook [`AGENTS_FRONTEND_THEME_TOGGLE.md`](./AGENTS_FRONTEND_THEME_TOGGLE.md) |
| `frontend-dapp/src/components/common/navItems.ts` | `PRIMARY_NAV_ITEMS`, `MOBILE_BOTTOM_NAV_ITEMS`, `getMobileMoreMenuItems`, `MORE_NAV_ITEMS`, `getHeaderMoreMenuItems`, `HEADER_FULL_NAV_MIN_WIDTH_PX` (`1200`), `TABLET_COMPACT_HEADER_MAX_WIDTH_PX` |
| `frontend-dapp/src/hooks/useMediaQuery.ts` | Subscribes to `matchMedia` for the full-desktop header breakpoint |
| `frontend-dapp/src/index.css` | `.app-top-sticky`, footer `.app-env-ribbon*`, `.app-desktop-nav` vs `.app-mobile-nav-shell` visibility (`max-width: 767px`) |

## Rules of thumb

1. **Do not** duplicate route lists in `Layout.tsx`; extend `navItems.ts` and keep **one** source of truth for what appears under header More at each tier.
2. **Tablet compact** (`768px`–`1199px`): folding Pool/Limits/Trade/Charts into header More is intentional — preserve reachable CTAs and overlap coverage in E2E (including **1024px–1199px** where full row still cramped).
3. **Mobile bottom bar ([#347](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/347)):** **`MOBILE_BOTTOM_NAV_ITEMS`** (Swap, Trade, Pool, Limits) + **More** on a **5-column** grid; Portfolio/Charts + **`MORE_NAV_ITEMS`** live in **`getMobileMoreMenuItems()`** sheet. Hide **`.app-brand-copy`** on ≤767px so the wallet chip does not crowd the header.
4. **Header brand** — logo + **`CL8Y DEX`** title only; do **not** reintroduce a “Terra Classic ecosystem” (or similar) kicker in the sticky header — it crowded the nav row and regressed [#136](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136). Network branding belongs in the **footer environment ribbon** (and wallet chip), not a second header line.
5. **Desktop/tablet NetworkBadge ([#483](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/483)):** do **not** re-add header `NetworkBadge` beside theme/wallet on ≥768px — wallet chip + footer `EnvironmentRibbon` carry network context. Mobile may keep the badge next to the wallet chip. Assert **≥ ~8px** gap between last nav control (More) and `.app-header-theme-group` at **1200 / 1280 / 1440**. Full inline nav starts at **`HEADER_FULL_NAV_MIN_WIDTH_PX` = 1200**.
6. **Footer environment ribbon:** keep `.app-env-ribbon` inside `footer.app-footer-shell` on **all** breakpoints (including mobile above the bottom tab bar). Do **not** move it back under `.app-top-sticky` without updating docs, skills, and `navigation.spec.ts`.
7. **Sticky header density:** prefer compact header margin/padding/brand mark; Trade H1 should clear `.app-top-sticky` by **≥ ~16px** at `scrollY=0`. Keep sticky stack opaque (`var(--bg-0)`).
8. **Header menus:** with the ribbon in the footer, open More / wallet menus no longer compete with a under-header ribbon band (historical [#486](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/486)). Still keep menus clickable and below modal portals (`z-index: 200`).
9. **Trade ticket footer CTA** is a separate surface (`.trade-ticket-submit-footer`) with the same opacity intent — see [`AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md`](./AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md) ([#527](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/527)) and [`AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md`](./AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md) ([#500](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/500)).

## Related

- **Shell tab routing (URL + Outlet must update on click):** [`AGENTS_FRONTEND_SHELL_NAV.md`](./AGENTS_FRONTEND_SHELL_NAV.md) ([GitLab **#182**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/182))
- Connected wallet chip (LUNC balance, network label, dropdown): [`AGENTS_FRONTEND_WALLET_CHIP.md`](./AGENTS_FRONTEND_WALLET_CHIP.md) ([GitLab **#140**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/140), [#186](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/186))
- Theme toggle (header vs mobile More): [`AGENTS_FRONTEND_THEME_TOGGLE.md`](./AGENTS_FRONTEND_THEME_TOGGLE.md)
- Environment ribbon / risk strip: [`AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](./AGENTS_FRONTEND_RISK_DISCLAIMERS.md) ([GitLab **#138**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138))
- Trade page tablet grid (`/trade`): [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- Trade ticket footer CTA opacity / dock: [`AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md`](./AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md) ([#527](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/527)), [`AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md`](./AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md) ([#500](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/500))
- Keyboard focus rings / WCAG 2.4.7: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md)
- Production build / source maps: [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md)
