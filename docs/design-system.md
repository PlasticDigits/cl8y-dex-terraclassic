# QuickSwap-inspired blue + gold system

Authoritative visual spec for the CL8Y DEX frontend ([GitLab #488](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/488)). Implementation lives in [`frontend-dapp/src/index.css`](../frontend-dapp/src/index.css) with theme tokens in [`theme-dark.css`](../frontend-dapp/src/theme-dark.css) and [`theme-light.css`](../frontend-dapp/src/theme-light.css). Dual theme is toggled via `data-theme` on `<html>`.

**Agent playbook:** [`skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md`](../skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md) · engineering invariants: [`docs/frontend.md`](./frontend.md) · QA checklist: [`QA_TEMPLATE.md`](../QA_TEMPLATE.md) §10 · docs alignment companion: [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489).

> **Supersedes** the warm amber “Cyberminimalist Glass” identity and the #416 “no blue primary” guardrail. Class names like `*-glass` / `shell-panel` remain; colors are cool navy + blue CTAs + gold brand.

## Principles

| Principle | Meaning |
|-----------|---------|
| Familiar DEX chrome | Centered trade cards, flat layered panels — QuickSwap-like, not a brown/amber page fade. |
| Blue CTAs, gold brand | Primary actions use **blue** (`#448aff`). Brand mark, network chip, and **hairline** gold borders/text use **gold** (`#e8b84a`). |
| Token-first | Compose UI from CSS variables (`--ink`, `--line`, `--blue`, `--gold`, `--mint`→blue, …) and `@layer components` primitives. |
| Gold is not a fill | Do **not** use gold/amber as large dirty-brown backgrounds. Gold = tiny emphasis (borders, text, logo). Active nav uses cool `--accent-surface` + gold bottom border. |
| Minimal on-card copy | **Anti-cognitive-overload:** labels ≤ ~5 words; errors ≤ 1 short sentence; optional **Docs** link. No `token0`/`token1` or raw bid/ask in retail UI — use symbols + Buy/Sell. Keep blocking errors and required risk ack ([#488] reopen). **Reject** always-on educational, cross-nav (“use Swap/UST1”), and gas/burn-tax trivia under CTAs — page = title + controls + live status + CTA ([`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](../skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) invariant **9**, [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489)). |
| Keyboard focus | Interactive primitives use `:focus-visible` with `var(--focus-ring)` — see [frontend.md § WCAG 2.4.7](./frontend.md#keyboard-focus-visible-wcag-247). |

## Color & surface tokens

| Token | Role |
|-------|------|
| `--bg-0`, `--bg-1`, `--bg-2` | Page background stack (dark `#0d111c` / light `#f4f6fb`) |
| `--ink`, `--ink-dim`, `--ink-subtle` | Primary, secondary, tertiary text |
| `--line`, `--line-strong` | Borders and dividers |
| `--blue`, `--gold` | Primary CTA blue; brand / network gold |
| `--mint`, `--accent` | **Aliases of `--blue`** (legacy name retained for components) |
| `--mint-soft`, `--accent-surface` | Soft **cool** fills for chips / active states |
| `--gold-surface` | Hairline / border accent only (~6–8% opacity) — **not** for large fills |
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
6. Gold accents are border/text only; warning surfaces use cool slate + amber border (`--alert-warning-border`), not brown washes.
7. Open Graph `/og-image.png` and `index.html` meta describe **product** (swaps, limits, Terra Classic) — not theme jargon.

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
| Header mark | `/logo.png` (circular teal/cream medallion) |
| Favicons | `/favicon-16.png`, `/favicon-32.png`, `/favicon.ico` |
| Open Graph | `/og-image.png` (~1200×630) — product copy (swap / limits / Terra Classic), not palette marketing |

Canonical same-origin only — do not wire user-controlled OG URLs. Meta: `frontend-dapp/index.html` `og:description` / `twitter:description`.

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
| `badge-glass-accent` | Brand/gold chips | Cool surface + gold border/text |
| `alert-*` | Inline notices | `role="alert"` when assertive |

## Limit place IA

Default `/limits` place card (#488 reopen): **rate** (“When 1 {token0} is worth”) → **% chips** (side-aware maker offsets: bid `0%−`/`−1%`/`−5%`/`−10%`, ask `0%+`/`+1%`/`+5%`/`+10%` — [#495](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/495)) → **Pay** → **Receive** → **Expiry** (advanced/ladder progressive disclosure). **Single / Ladder** tabs: Ladder create fields must render when a pair is selected even if the wallet is disconnected — Connect Wallet CTA on the place button, same as Single ([#494](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/494); invariant §14 in [`skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md`](../skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md)). Order book and open placements sit **below** the place card. No instructional paragraphs on the primary card; blocking errors stay visible. Details: [`skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](../skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md) · copy rules: [`skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](../skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) · agent playbook: [`skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md`](../skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md). On `/trade`, the sticky **Place limit** CTA must stay opaque and keep validation banners in normal flow ([#500](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/500); [`skills/AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md`](../skills/AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md)).

Default `/` Swap card: centered Pay → flip → Receive → CTA; route + min-received on card; verbose trade/signing details behind progressive disclosure. Cool flip control (no brown).

Default `/trade` ticket sections (#489): short titles only — **Side**, **Limit**, **Market** — no “Maker side / Choose direction / Resting order” essays. Action panels use cool blue wash (not amber).

## Terminology glossary

Retail copy terms used across docs and skills ([#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489)):

| Term | Meaning |
|------|---------|
| Swap / Best Trade (concept) | Aggregated market swap on `/` (nav: **Swap**). Charts/trader **Best Trade** = best historical P&L trade — not the Swap route. |
| Limit | Resting limit place flow (`/limits`, `/trade` Limit tab). |
| Market | Immediate pair swap on `/trade` Market tab (not the `/` Swap multihop card). |
| Pay / Receive | Limit IO labels. Swap card uses **You Pay** / **You Receive** (same roles). |
| Buy / Sell {base} | Retail side labels on `/trade` and `/limits` (on-chain bid/ask escrow mapping unchanged). |
| Blue primary | CTA / active tab / select-token (`--blue` / `--mint` alias). |
| Gold accent | Logo, network chip, hairline borders/text only — not large fills. |
| Docs link | Optional single-word control for optional depth — never replaces blocking errors. |
| Pool / Provide / Withdraw | v2 AMM LP on `/pool`. Both tokens required. LP tokens = share. |
| Use native LUNC (auto-wrap) | Pool checkbox: bank LUNC wraps into cLUNC in the provide tx. |
| Limit (maker) | Resting escrow on `/trade` or `/limits` — **not** pool LP and **not** a farm. |
| Bid / Ask (book) | Order-book column titles and on-chain side enums only; not retail place-card button copy. |

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

Manual matrix (both themes): Swap, Limit, Trade, Pool, Portfolio, Connect Wallet modal — cool navy/slate surfaces, blue CTAs, gold network/brand accents. Connect Wallet rows include circular brand logos ([#490](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/490); [docs/frontend.md § logos](./frontend.md#connect-modal-wallet-logos)).
