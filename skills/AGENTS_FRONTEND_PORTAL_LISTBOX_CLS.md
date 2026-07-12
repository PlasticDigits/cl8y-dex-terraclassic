# Agent playbook: Portal listbox layout stability (CLS)

Use when changing **`MenuSelect`**, **`TokenSelect`**, **`TokenSearchSelect`**, **`PairSearchSelect`**, **`usePortalListbox`**, **`portalListboxPosition.ts`**, `.token-select-*` CSS, or trade/charts pair pickers ([GitLab **#181**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/181)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Portal listboxes — layout stability](../docs/frontend.md#portal-listbox-layout-stability) | Invariants: fixed portal, stable trigger footprint, scrollbar gutter, CLS &lt; 0.1 |
| [`frontend-dapp/src/components/ui/portalListboxPosition.ts`](../frontend-dapp/src/components/ui/portalListboxPosition.ts) | Pure `computePortalListboxStyle` (unit-tested flip/clamp) |
| [`frontend-dapp/src/components/ui/PortalListbox.tsx`](../frontend-dapp/src/components/ui/PortalListbox.tsx) | Sync position on open + scroll/resize listeners |
| [`frontend-dapp/src/components/ui/usePortalListboxKeyboard.ts`](../frontend-dapp/src/components/ui/usePortalListboxKeyboard.ts) | WAI-ARIA listbox keyboard APG ([GitLab **#244**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/244)) |
| [`frontend-dapp/e2e/trade-pair-select-cls.spec.ts`](../frontend-dapp/e2e/trade-pair-select-cls.spec.ts) | Playwright: `#trade-pair-select` and desktop workspace Y/X stable on open |

## Rules of thumb

1. **Never** render the listbox in normal document flow — always portal to `document.body` with **`position: fixed`** coords from `computePortalListboxStyle`.
2. **Do not** add a second `setState` pass for initial open position; read the anchor rect during render when `open && canShow` (see `usePortalListbox`).
3. **Preserve** the reserved focus ring on `.token-select-trigger` (transparent `0 0 0 2px` in base `box-shadow`); only swap ring color on `:focus-visible`.
4. New pickers should wrap triggers in **`.token-select-root`** and reuse **`.token-select-trigger`** / **`.token-select-dropdown`** classes.
5. After CSS or positioning changes, run **`portalListboxPosition.test.ts`** and, with LocalTerra up, **`trade-pair-select-cls.spec.ts`**; manual Lighthouse CLS on `/trade` at ~1440px desktop.

## Related

- Keyboard listbox APG: [`AGENTS_FRONTEND_PORTAL_LISTBOX_KEYBOARD.md`](./AGENTS_FRONTEND_PORTAL_LISTBOX_KEYBOARD.md) ([GitLab **#244**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/244))
- Swap token search combobox: [`AGENTS_FRONTEND_TOKEN_SEARCH.md`](./AGENTS_FRONTEND_TOKEN_SEARCH.md) ([GitLab **#481**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/481))
- Keyboard focus rings: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md)
- Trade page layout: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- Mobile tab bar inset for flip-above: `getMobileBottomNavInsetPx` ([`mobileBottomNav.ts`](../frontend-dapp/src/lib/mobileBottomNav.ts))
