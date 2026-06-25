# Frontend — Cyberminimalist Glass System

Use when adding or restyling dApp UI so new work matches existing glass primitives instead of ad-hoc Tailwind colors.

## Source of truth

- Spec: [`docs/design-system.md`](../docs/design-system.md)
- CSS: [`frontend-dapp/src/index.css`](../frontend-dapp/src/index.css)
- Themes: [`theme-dark.css`](../frontend-dapp/src/theme-dark.css), [`theme-light.css`](../frontend-dapp/src/theme-light.css)
- Focus/a11y: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md)

## Rules

1. **No `*-neo` classes** — renamed to `*-glass` in [#415](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/415). Grep `frontend-dapp/src` for `-neo` before merging.
2. **Buttons:** `btn-primary` (CTA), `btn-muted` (secondary/toggle-off). There is no `btn-neo`.
3. **Panels:** `shell-panel` / `shell-panel-strong` for page sections; `card-glass` for nested blocks. Use `Card` when appropriate.
4. **Tabs/segments:** `tab-glass`, `tab-glass-active`, `tab-glass-inactive` — not raw Tailwind border utilities.
5. **Tokens:** Style with `var(--ink)`, `var(--line)`, etc. Legacy names like `--mint` remain; do not introduce a parallel blue Tailwind primary for product chrome.
6. **Tailwind colors:** [`tailwind.config.js`](../frontend-dapp/tailwind.config.js) aliases `bg-*`, `ink`, `line`, `mint`, `accent` to CSS variables ([#416](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/416)). The old `primary` / `dex` blue palettes are removed.
7. **Trade bootstrap:** [`trade-bootstrap.css`](../frontend-dapp/public/bootstrap/trade-bootstrap.css) must stay tiny and use the same token names as theme files — no hard-coded blues on the `/trade` critical path.
8. **`.glass` is deprecated** — do not add usages; prefer `shell-panel` or `card-glass`.

## Quick matrix

| Need | Class |
|------|-------|
| Page section | `shell-panel` |
| Card / sub-panel | `card-glass` |
| Primary button | `btn-primary` |
| Secondary / inactive toggle | `btn-muted` |
| Text field | `input-glass` + `label-glass` |
| Segmented control | `tab-glass*` |
| Status chip | `badge-glass*` or `Badge` |

## Verify

```bash
rg '-neo' frontend-dapp/src
python3 scripts/check_design_tokens.py
make lint-frontend
```
