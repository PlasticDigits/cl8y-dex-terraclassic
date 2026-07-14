# QuickSwap-inspired blue + gold system

Authoritative visual spec for the CL8Y DEX frontend ([GitLab #488](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/488)). Implementation lives in [`frontend-dapp/src/index.css`](../frontend-dapp/src/index.css) with theme tokens in [`theme-dark.css`](../frontend-dapp/src/theme-dark.css) and [`theme-light.css`](../frontend-dapp/src/theme-light.css). Dual theme is toggled via `data-theme` on `<html>`.

**Agent playbook:** [`skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md`](../skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md) · engineering invariants: [`docs/frontend.md`](./frontend.md) · QA checklist: [`QA_TEMPLATE.md`](../QA_TEMPLATE.md) §10 · docs alignment companion: [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489).

> **Supersedes** the warm amber “Cyberminimalist Glass” identity and the #416 “no blue primary” guardrail. Class names like `*-glass` / `shell-panel` remain; colors are cool navy + blue CTAs + gold brand.

## Principles

| Principle | Meaning |
|-----------|---------|
| Familiar DEX chrome | Centered trade cards, flat layered panels — QuickSwap-like, not a brown/amber page fade. |
| Blue CTAs, gold brand | Primary actions use **blue** (`#448aff`). Brand mark, network chip, CL8Y accents use **gold** (`#e8b84a`). |
| Token-first | Compose UI from CSS variables (`--ink`, `--line`, `--blue`, `--gold`, `--mint`→blue, …) and `@layer components` primitives. |
| Minimal on-card copy | Prefer short labels + optional single-word **Docs** links; keep blocking errors and required risk ack ([#488] note). |
| Keyboard focus | Interactive primitives use `:focus-visible` with `var(--focus-ring)` — see [frontend.md § WCAG 2.4.7](./frontend.md#keyboard-focus-visible-wcag-247). |

## Color & surface tokens

| Token | Role |
|-------|------|
| `--bg-0`, `--bg-1`, `--bg-2` | Page background stack (dark `#0d111c` / light `#f4f6fb`) |
| `--ink`, `--ink-dim`, `--ink-subtle` | Primary, secondary, tertiary text |
| `--line`, `--line-strong` | Borders and dividers |
| `--blue`, `--gold` | Primary CTA blue; brand / network gold |
| `--mint`, `--accent` | **Aliases of `--blue`** (legacy name retained for components) |
| `--mint-soft`, `--accent-surface`, `--gold-surface` | Soft fills for chips / active states |
| `--accent-gradient` | Blue CTA fill |
| `--panel-bg`, `--panel-bg-strong`, `--card-bg` | Shell / card surfaces |
| `--chrome-border`, `--chrome-highlight`, `--chrome-glow` | Shared chrome borders / glows |
| `--focus-ring` | Keyboard focus ring |
| `--color-positive`, `--color-negative`, `--color-warning` | Semantic chart / status (not button fills) |

Theme files override the above per `data-theme='dark'` | `'light'`.

### Invariants (#488)

1. Dark `--bg-0` is `#0d111c`; light `--bg-0` is `#f4f6fb` — **not** warm brown `#0e0908` / peach `#f4e0cb`.
2. `--blue` = `#448aff`; `--gold` = `#e8b84a`; `--mint` aliases `--blue`.
3. `trade-bootstrap.css` `--bg-0` must match theme files (no amber→blue FOUC on `/trade`).
4. Do not reintroduce Tailwind `primary` / `dex` hard-coded hex palettes; alias via CSS variables.
5. Safety gates and required legal/risk copy stay visible — copy minimization must not hide blockers.

## Tailwind color aliases

[`tailwind.config.js`](../frontend-dapp/tailwind.config.js) maps `theme.extend.colors` to the same CSS variables — **not** a parallel hex palette.

| Tailwind key | CSS variable | Notes |
|--------------|--------------|-------|
| `bg-0`, `bg-1`, `bg-2` | `--bg-0`, … | Page / surface stack |
| `ink`, `ink-dim`, `ink-subtle` | `--ink`, … | Text hierarchy |
| `line`, `line-strong` | `--line`, … | Borders |
| `mint`, `mint-soft` | `--mint`, `--mint-soft` | CTA blue (legacy name) |
| `blue`, `gold` | `--blue`, `--gold` | Explicit #488 accents |
| `accent` | `--accent` | Alias of `--mint` / blue |
| `surface-*` | `--surface-*` | Raised / inset surfaces |
| `positive`, `negative`, `warning` | `--color-*` | Semantic status only |

### Historical (#416)

| Entry | Status |
|-------|--------|
| `primary` / `dex` hard-coded blue/slate scales | **Still removed** — use CSS-variable aliases (`mint`/`blue`/`gold`) and primitives |
| “No blue primary” agent rule | **Superseded by #488** — blue CTAs are intentional |

### Trade bootstrap (critical path)

[`trade-bootstrap.css`](../frontend-dapp/public/bootstrap/trade-bootstrap.css) defines a **minimal** `:root` / `data-theme` token subset mirroring theme files so `/trade` first paint matches the hydrated app. See [frontend.md § Trade page — initial load](./frontend.md#trade-page-initial-load).

## Brand assets

| Asset | Path |
|-------|------|
| Header mark | `/assets/cl8y-dex-glass-logo.svg` (blue disc + gold “8”) |
| Favicons | `/favicon-16.png`, `/favicon-32.png`, `/favicon.ico` |
| Open Graph | `/og-image.png` (~1200×630) |

Canonical same-origin only — do not wire user-controlled OG URLs.

## Typography

| Use | Font | Notes |
|-----|------|-------|
| Headings (`h1`–`h6`) | **Chakra Petch** | Slight letter-spacing |
| Body, controls | **IBM Plex Sans** | Default `body` font |
| Monospace (addresses, order IDs) | Tailwind `font-mono` on fields | Pair with `input-glass` |

Labels use `.label-glass`: uppercase, `text-xs`, semibold, wide tracking.

## Component matrix

| Class | When to use | Pair with |
|-------|-------------|-----------|
| `shell-panel` / `shell-panel-strong` | Page-level sections | Default route chrome |
| `card-glass` | Nested content blocks | Order ticket, swap IO rows |
| `btn-primary` | Primary CTA (blue gradient) | Optional `btn-cta` |
| `btn-muted` | Secondary / toggle-off | Mode toggles |
| `tab-glass*` | Segmented controls | Side, slippage, order type |
| `input-glass` / `label-glass` | Fields | Always associate `htmlFor` |
| `badge-glass-accent` | Brand/gold chips | Prefer gold surface |
| `alert-*` | Inline notices | `role="alert"` when assertive |

## Limit place IA

Default `/limits` place card: **rate** (“When 1 {token0} is worth”) → **Pay** → **Expiry** (advanced/ladder progressive disclosure). No instructional paragraphs on the primary card; blocking errors stay visible. Details: [`skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](../skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md).

## Verification

```bash
# No legacy -neo classes
rg '-neo' frontend-dapp/src && exit 1 || echo OK

# Tailwind + trade-bootstrap + blue/gold alignment (#488)
python3 scripts/check_design_tokens.py

# Frontend lint + unit tests
make lint-frontend
make test-frontend
```

Manual matrix (both themes): Swap, Limit, Trade, Pool, Portfolio, Connect Wallet modal — cool navy/slate surfaces, blue CTAs, gold network/brand accents.
