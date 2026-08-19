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
6. **Gold vs blue:** CTAs/tabs/focus = blue; brand mark, network chip text, and **hairline gold borders** = gold. Do **not** paint large `--gold-surface` / brown fills on nav, warnings, or page backgrounds — active nav uses `--accent-surface` + gold bottom border ([#488] reopen).
7. **Trade bootstrap:** [`trade-bootstrap.css`](../frontend-dapp/public/bootstrap/trade-bootstrap.css) must mirror theme `--bg-0` (cool navy / cool light) — no warm-brown FOUC.
8. **Copy (anti-cognitive-overload):** Write for humans, not agents. Labels ≤ ~5 words; blocking errors ≤ 1 short sentence; optional single-word **Docs** link. Never show `token0` / `token1` / raw `bid`/`ask` in retail UI — use symbols and **Buy** / **Sell**. Keep blocking errors, risk acknowledgement, and chain anchors. **No** always-on educational / cross-nav / gas-trivia paragraphs ([`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) **§9**). Agents read docs/skills; users should not drown in disclosure.
9. **Limit IA:** rate → % chips → Pay → Receive → Expiry; book/open orders below place card. **Swap IA:** Pay → flip (cool, not brown) → Receive → CTA; collapse verbose trade details.
10. **OG / social:** `/og-image.png` + `index.html` meta emphasize swaps, limits, Terra Classic — not theme colors.
11. **`.glass` is deprecated** — prefer `shell-panel` or `card-glass`.

## Quick matrix

| Need | Class / token |
|------|----------------|
| Page section | `shell-panel` |
| Card / sub-panel | `card-glass` |
| Primary button | `btn-primary` (blue) |
| Brand / network accent | `var(--gold)` text/border; `.badge-glass-accent` (cool fill + gold border) |
| Header brand mark | Circular `/logo.png` medallion, no chrome plate — **no** orange `rgba(249,115,22)` glow |
| Text field | `input-glass` + `label-glass` |
| Segmented control | `tab-glass*` |
| Warning surface | cool `--alert-warning-bg` + `--alert-warning-border` (not brown wash) |

## Verify

```bash
rg '-neo' frontend-dapp/src
python3 scripts/check_design_tokens.py
make lint-frontend
```

Cross-links: [`docs/design-system.md`](../docs/design-system.md) · [`AGENTS_FRONTEND_THEME_TOGGLE.md`](./AGENTS_FRONTEND_THEME_TOGGLE.md) · [`AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md) · [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) · QA shots [`docs/qa/issue-488/`](../docs/qa/issue-488/).