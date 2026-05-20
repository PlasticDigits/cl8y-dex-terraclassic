# Agent playbook: theme toggle placement

Use when moving or restyling the **dark/light** segmented control ([`ThemeSegmentedControl.tsx`](../frontend-dapp/src/components/common/ThemeSegmentedControl.tsx)) or changing where it appears in [`Layout.tsx`](../frontend-dapp/src/components/common/Layout.tsx).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Responsive shell & header navigation](../docs/frontend.md#responsive-header-navigation) | Theme toggle invariants ([GitLab **#170**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/170)) |
| [`Layout.tsx`](../frontend-dapp/src/components/common/Layout.tsx) | `showMobileLegalStrip` (`max-width: 767px`) gates header vs mobile More placement |
| [`index.css`](../frontend-dapp/src/index.css) | `.app-header-theme-group`, `.app-mobile-theme-group`, shared `.app-footer-theme-button` styles |
| [`navigation.spec.ts`](../frontend-dapp/e2e/navigation.spec.ts) | Desktop header visibility on `/pool`; mobile More sheet clearance above bottom nav |

## Rules of thumb

1. **Do not** put the only desktop theme control in the page footer — long routes (`/pool`, `/trade`) hide it below the fold ([**#170**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/170)).
2. **Desktop/tablet (`≥768px`)**: render `ThemeSegmentedControl` inside `.app-header-controls` with `groupClassName="app-header-theme-group"` and `labelStyle="short"`.
3. **Mobile (`≤767px`)**: keep the control in the **More** sheet (`app-mobile-theme-group`, `labelStyle="long"`); footer shell is hidden — do not rely on footer for theme on narrow viewports.
4. Persist theme via `localStorage` key `cl8y-dex-theme` and `document.documentElement[data-theme]` (unchanged).

## Related

- Responsive header / nav density: [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md)
- Keyboard focus on theme buttons: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md)
