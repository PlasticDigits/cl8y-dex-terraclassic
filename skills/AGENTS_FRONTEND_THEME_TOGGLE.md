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
2. **Desktop/tablet (`≥768px`)**: render `ThemeSegmentedControl` inside `.app-header-controls` with `groupClassName="app-header-theme-group"` (compact moon/sun icons via [`shellPrefIcons.tsx`](../frontend-dapp/src/components/common/shellPrefIcons.tsx)).
3. **Mobile (`≤767px`)**: keep the control in the **More** sheet (`app-mobile-theme-group`); footer shell is hidden — do not rely on footer for theme on narrow viewports. Accessible names stay **Dark theme** / **Light theme**.
4. Persist theme via `localStorage` key `cl8y-dex-theme` and `document.documentElement[data-theme]` (unchanged).
5. **Density ([#483](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/483)):** theme group must keep **≥ ~8px** horizontal gap from the last desktop nav control (More). Do not re-add desktop `NetworkBadge` next to theme — ribbon owns network context.

## Palette notes (#488)

- Themes live in [`theme-dark.css`](../frontend-dapp/src/theme-dark.css) / [`theme-light.css`](../frontend-dapp/src/theme-light.css): cool navy `#0d111c` / light `#f4f6fb`, blue CTAs, gold brand.
- **Gold usage:** border/text emphasis only — active theme segment / nav uses cool `--accent-surface` + gold hairline, not dirty brown/gold fills ([#488] reopen).
- [`trade-bootstrap.css`](../frontend-dapp/public/bootstrap/trade-bootstrap.css) + [`theme.js`](../frontend-dapp/public/bootstrap/theme.js) must apply the same `data-theme` before Vite hydrates — avoid warm-brown flash on `/trade`.
- Design-system playbook: [`AGENTS_FRONTEND_DESIGN_SYSTEM.md`](./AGENTS_FRONTEND_DESIGN_SYSTEM.md).

## Related

- UI sound mute (adjacent shell control): [`AGENTS_FRONTEND_SOUND_MUTE.md`](./AGENTS_FRONTEND_SOUND_MUTE.md) ([#487](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/487))
- Responsive header / nav density: [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md)
- Keyboard focus on theme buttons: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md)
- Visual spec: [`docs/design-system.md`](../docs/design-system.md)
