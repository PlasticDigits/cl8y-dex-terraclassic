# Frontend — QuickSwap-inspired blue + gold system

Use when adding or restyling dApp UI so new work matches cool blue chrome + gold brand accents ([GitLab #488](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/488)).

## Source of truth

- Spec: [`docs/design-system.md`](../docs/design-system.md)
- CSS: [`frontend-dapp/src/index.css`](../frontend-dapp/src/index.css)
- Themes: [`theme-dark.css`](../frontend-dapp/src/theme-dark.css), [`theme-light.css`](../frontend-dapp/src/theme-light.css)
- Focus/a11y: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md)
- Theme toggle: [`AGENTS_FRONTEND_THEME_TOGGLE.md`](./AGENTS_FRONTEND_THEME_TOGGLE.md)
- Docs companion: [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489)

## Rules

1. **No `*-neo` classes** — renamed to `*-glass` in [#415](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/415). Grep `frontend-dapp/src` for `-neo` before merging.
2. **Buttons:** `btn-primary` (blue CTA), `btn-muted` (secondary/toggle-off).
3. **Panels:** `shell-panel` / `shell-panel-strong` for page sections; `card-glass` for nested blocks.
4. **Tabs/segments:** `tab-glass`, `tab-glass-active`, `tab-glass-inactive`.
5. **Tokens:** Use `var(--ink)`, `var(--line)`, `var(--blue)`, `var(--gold)`. Legacy `--mint` / `--accent` **alias blue** — do **not** restore warm amber page fades or a hard-coded Tailwind `primary`/`dex` hex scale.
6. **Gold vs blue:** CTAs/tabs/focus = blue; brand mark, network chip, selective active chips = gold (`--gold` / `--gold-surface`).
7. **Trade bootstrap:** [`trade-bootstrap.css`](../frontend-dapp/public/bootstrap/trade-bootstrap.css) must mirror theme `--bg-0` (cool navy / cool light) — no warm-brown FOUC.
8. **Copy:** Minimize on-card instructional prose; keep blocking errors, risk acknowledgement, and chain anchors. Prefer a single-word **Docs** link over helper paragraphs.
9. **`.glass` is deprecated** — prefer `shell-panel` or `card-glass`.

## Quick matrix

| Need | Class / token |
|------|----------------|
| Page section | `shell-panel` |
| Card / sub-panel | `card-glass` |
| Primary button | `btn-primary` (blue) |
| Brand / network accent | `var(--gold)` / `.badge-glass-accent` |
| Text field | `input-glass` + `label-glass` |
| Segmented control | `tab-glass*` |

## Verify

```bash
rg '-neo' frontend-dapp/src
python3 scripts/check_design_tokens.py
make lint-frontend
```
