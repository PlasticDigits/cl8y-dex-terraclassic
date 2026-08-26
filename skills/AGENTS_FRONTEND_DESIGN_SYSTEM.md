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
3. **Panels:** `shell-panel` / `shell-panel-strong` for page sections. **One chrome layer per region** ([#561](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/561) **L561-2**, [#653](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/653)): do not wrap `shell-panel*` in another `shell-panel*`, and do not drop a **grid of** `card-glass` / default `StatBox` into a section panel. Metric tiles use `StatBox variant="flat"`. Nested `card-glass` is OK **only** for the short allowlist (Swap IO cards; a single table/chart well; Trade sibling panels) — see [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md). `/protocol` Global stats + fees keep inline Δ% on flat tiles ([#652](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/652)).
4. **Tabs/segments:** `tab-glass`, `tab-glass-active`, `tab-glass-inactive` for Limit/Market, slippage, order type. **Exception (#563):** Buy/Sell **side** controls use `side-control` + `side-buy-*` / `side-sell-*` (semantic fills from `--color-positive` / `--color-negative`). Do **not** paint `btn-primary` green/red.
5. **Tokens:** Use `var(--ink)`, `var(--line)`, `var(--blue)`, `var(--gold)`. Legacy `--mint` / `--accent` **alias blue** — do **not** restore warm amber page fades or a hard-coded Tailwind `primary`/`dex` hex scale.
6. **Gold vs blue:** CTAs/tabs/focus = blue; brand mark, network chip text, and **hairline gold borders** = gold. Do **not** paint large `--gold-surface` / brown fills on nav, warnings, or page backgrounds — active nav uses `--accent-surface` + gold bottom border ([#488] reopen).
7. **Trade bootstrap:** [`trade-bootstrap.css`](../frontend-dapp/public/bootstrap/trade-bootstrap.css) must mirror theme `--bg-0` (cool navy / cool light) — no warm-brown FOUC.
8. **Copy (anti-cognitive-overload):** Write for humans, not agents. Labels ≤ ~5 words; blocking errors ≤ 1 short sentence; optional single-word **Docs** link. Never show `token0` / `token1` / raw `bid`/`ask` in retail UI — use symbols and **Buy** / **Sell**. Keep blocking errors, risk acknowledgement, and chain anchors. **No** always-on educational / cross-nav / gas-trivia paragraphs ([`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) **§9**). Agents read docs/skills; users should not drown in disclosure.
9. **Limit IA:** rate → % chips → Pay → Receive → Expiry; book/open orders below place card. **Swap IA:** Pay → flip (cool, not brown) → Receive → CTA; collapse verbose trade details.
10. **OG / social:** `/og-image.png` is the **community medallion** 1200×630 card ([#578](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/578)). Production `og:image` / `twitter:image` are absolute `https://dex.cl8y.com/og-image.png`. Meta text still emphasizes swaps, limits, Terra Classic. Do not restore the #488 product-copy card or relative image URLs in production. Playbook: [`AGENTS_FRONTEND_OPENGRAPH.md`](./AGENTS_FRONTEND_OPENGRAPH.md).
11. **`.glass` is deprecated** — prefer `shell-panel` or `card-glass`.
12. **Logo variants:** `/logo.png` is the **full character-scene** medallion — header (beside **CL8Y DEX** type) and large marketing only. Favicons, PWA/touch icons, and isolated marks **≤ ~64px** use the simplified **C+8** `/logo-simplified-variant.png` (wired as `favicon-16.png` / `favicon-32.png` / `favicon.ico` / `favicon.png`). Never downscale the full scene into a tab icon. See [`docs/design-system.md`](../docs/design-system.md) § Brand assets.

## Quick matrix

| Need | Class / token |
|------|----------------|
| Page section | `shell-panel` |
| Card / sub-panel | `card-glass` |
| Primary button | `btn-primary` (blue) |
| Brand / network accent | `var(--gold)` text/border; `.badge-glass-accent` (cool fill + gold border) |
| Header brand mark | Circular `/logo.png` **full scene** medallion beside **CL8Y DEX** type, no chrome plate — **no** orange `rgba(249,115,22)` glow |
| Favicon / small isolated mark | Simplified **C+8** `/logo-simplified-variant.png` (not the full character scene). Wired as `/favicon-16.png`, `/favicon-32.png`, `/favicon.ico`, `/favicon.png`. See [`docs/design-system.md`](../docs/design-system.md) § Brand assets |
| Text field | `input-glass` + `label-glass` |
| Segmented control | `tab-glass*` |
| Buy/Sell side control | `side-control` + `side-buy-*` / `side-sell-*` (#563) |
| Warning surface | cool `--alert-warning-bg` + `--alert-warning-border` (not brown wash) |

## Verify

```bash
rg '-neo' frontend-dapp/src
python3 scripts/check_design_tokens.py
python3 scripts/check_chrome_nesting.py
make lint-frontend
make verify-issue-578
make verify-issue-653
make verify-issue-659
```

Cross-links: [`docs/design-system.md`](../docs/design-system.md) · [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md) ([#653](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/653)) · [`AGENTS_FRONTEND_SWAP_DIRECTION_SEAM.md`](./AGENTS_FRONTEND_SWAP_DIRECTION_SEAM.md) ([#659](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/659) Swap flip plate) · [`AGENTS_FRONTEND_PRODUCT_LINKS.md`](./AGENTS_FRONTEND_PRODUCT_LINKS.md) ([#663](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/663)) · [`AGENTS_FRONTEND_OPENGRAPH.md`](./AGENTS_FRONTEND_OPENGRAPH.md) ([#578](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/578)) · [`AGENTS_FRONTEND_THEME_TOGGLE.md`](./AGENTS_FRONTEND_THEME_TOGGLE.md) · [`AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md) · [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) · [`AGENTS_FRONTEND_TRADE_TICKET_HEADING.md`](./AGENTS_FRONTEND_TRADE_TICKET_HEADING.md) (#563 side-fill exception) · [`AGENTS_FRONTEND_TIERS_PHONE.md`](./AGENTS_FRONTEND_TIERS_PHONE.md) ([#651](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/651) `/tiers` phone cards, no `*-neo`) · QA shots [`docs/qa/issue-488/`](../docs/qa/issue-488/).
Cross-links: [`docs/design-system.md`](../docs/design-system.md) · [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md) ([#653](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/653)) · [`AGENTS_FRONTEND_OPENGRAPH.md`](./AGENTS_FRONTEND_OPENGRAPH.md) ([#578](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/578)) · [`AGENTS_FRONTEND_THEME_TOGGLE.md`](./AGENTS_FRONTEND_THEME_TOGGLE.md) · [`AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md) · [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) · [`AGENTS_FRONTEND_TRADE_TICKET_HEADING.md`](./AGENTS_FRONTEND_TRADE_TICKET_HEADING.md) (#563 side-fill exception) · [`AGENTS_FRONTEND_TIERS_PHONE.md`](./AGENTS_FRONTEND_TIERS_PHONE.md) ([#651](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/651) `/tiers` phone cards, no `*-neo`) · [`AGENTS_FRONTEND_CREATE_TOKEN_LAYOUT.md`](./AGENTS_FRONTEND_CREATE_TOKEN_LAYOUT.md) ([#669](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/669) `/token/create` desktop grid, no `*-neo`) · QA shots [`docs/qa/issue-488/`](../docs/qa/issue-488/).
