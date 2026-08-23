# QuickSwap-inspired blue + gold system

Authoritative visual spec for the CL8Y DEX frontend ([GitLab #488](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/488)). Implementation lives in [`frontend-dapp/src/index.css`](../frontend-dapp/src/index.css) with theme tokens in [`theme-dark.css`](../frontend-dapp/src/theme-dark.css) and [`theme-light.css`](../frontend-dapp/src/theme-light.css). Dual theme is toggled via `data-theme` on `<html>`.

**Agent playbook:** [`skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md`](../skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md) · Open Graph: [`skills/AGENTS_FRONTEND_OPENGRAPH.md`](../skills/AGENTS_FRONTEND_OPENGRAPH.md) ([#578](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/578)) · engineering invariants: [`docs/frontend.md`](./frontend.md) · QA checklist: [`QA_TEMPLATE.md`](../QA_TEMPLATE.md) §10 · docs alignment companion: [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489).

> **Supersedes** the warm amber “Cyberminimalist Glass” identity and the #416 “no blue primary” guardrail. Class names like `*-glass` / `shell-panel` remain; colors are cool navy + blue CTAs + gold brand.

## Principles

| Principle | Meaning |
|-----------|---------|
| Familiar DEX chrome | Centered trade cards, flat layered panels — QuickSwap-like, not a brown/amber page fade. |
| Blue CTAs, gold brand | Primary actions use **blue** (`#448aff`). Brand mark, network chip, and **hairline** gold borders/text use **gold** (`#e8b84a`). |
| Token-first | Compose UI from CSS variables (`--ink`, `--line`, `--blue`, `--gold`, `--mint`→blue, …) and `@layer components` primitives. |
| Gold is not a fill | Do **not** use gold/amber as large dirty-brown backgrounds. Gold = tiny emphasis (borders, text, logo). Active nav uses cool `--accent-surface` + gold bottom border. |
| Minimal on-card copy | **Anti-cognitive-overload:** labels ≤ ~5 words; errors ≤ 1 short sentence; optional **Docs** link. No `token0`/`token1` or raw bid/ask in retail UI — use symbols + Buy/Sell. Keep blocking errors and required risk ack ([#488] reopen). **Reject** always-on educational, cross-nav (“use Swap/UST1”), and gas/burn-tax trivia under CTAs — page = title + controls + live status + CTA ([`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](../skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) invariant **9**, [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489)). |
| One chrome layer per region | Do **not** wrap `shell-panel` / `shell-panel-strong` / `card-glass` in another of the same family for the same visual region. Page background → one section surface → content. Nested `card-glass` inside a page `shell-panel` is OK for distinct inner blocks (Swap IO cards). First applied on `/trade` ([#561](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/561)). |
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
| `--color-positive`, `--color-negative`, `--color-warning` | Semantic chart / status. **Exception (#563):** Buy/Sell **side** controls may use these hues via `--side-buy*` / `--side-sell*` fills. **Not** for primary money CTAs (`btn-primary`). |

Theme files override the above per `data-theme='dark'` | `'light'`.

### Invariants (#488)

1. Dark `--bg-0` is `#0d111c`; light `--bg-0` is `#f4f6fb` — **not** warm brown `#0e0908` / peach `#f4e0cb`.
2. `--blue` = `#448aff`; `--gold` = `#e8b84a`; `--mint` aliases `--blue`.
3. `trade-bootstrap.css` `--bg-0` must match theme files (no amber→blue FOUC on `/trade`).
4. Do not reintroduce Tailwind `primary` / `dex` hard-coded hex palettes; alias via CSS variables.
5. Safety gates and required legal/risk copy stay visible — copy minimization must not hide blockers.
6. Gold accents are border/text only; warning surfaces use cool slate + amber border (`--alert-warning-border`), not brown washes.
7. Open Graph `/og-image.png` is the **community medallion** card (1200×630). Meta title/description still describe **product** (swaps, limits, Terra Classic) — not theme jargon. Production image URLs are absolute `https://dex.cl8y.com/og-image.png` ([#578](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/578)).

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
| `positive`, `negative`, `warning` | `--color-*` | Semantic status; side-control fills (#563) may reuse the hue |

### Historical (#416)

| Entry | Status |
|-------|--------|
| `primary` / `dex` hard-coded blue/slate scales | **Still removed** — use CSS-variable aliases (`mint`/`blue`/`gold`) and primitives |
| “No blue primary” agent rule | **Superseded by #488** — blue CTAs are intentional |

### Trade bootstrap (critical path)

[`trade-bootstrap.css`](../frontend-dapp/public/bootstrap/trade-bootstrap.css) defines a **minimal** `:root` / `data-theme` token subset mirroring theme files so `/trade` first paint matches the hydrated app. See [frontend.md § Trade page — initial load](./frontend.md#trade-page-initial-load).

## Brand assets

Two circular medallions. Pick by **rendered size and whether the name is shown in type next to the mark** — not by page.

| Variant | What it shows | Canonical file |
|---------|-----------------|----------------|
| **Full scene** | Classical bust, scales, waves/fish, and **CL8Y DEX** lettering in the coin | `/logo.png` (also `/assets/cl8y-logo.png`, `/assets/cl8y-dex-header-logo.png`) |
| **Simplified C+8** | Gold **C** wrapping a mint-green **8** on a dark teal field, gold rim — no character scene | `/logo-simplified-variant.png` |

**Use the simplified C+8 mark** when the circle is small or stands alone (no “CL8Y DEX” wordmark beside it). The full scene turns into mud below ~64px; the C and 8 stay readable as a tab icon. That includes:

- Favicons and browser / OS chrome: `/favicon-16.png`, `/favicon-32.png`, `/favicon.ico` (16/32/48), `/favicon.png` (256)
- Apple touch / PWA / app icons, social avatars, and any mark at **≤ ~64px** without adjacent product type
- Anywhere a 16–32px “home” glyph is needed

**Use the full scene** when the medallion is large enough to read the figure and lettering, or when it sits beside the product name in chrome:

- Header brand mark (`.app-brand-logo`, 44px / 40px on narrow viewports) — `/logo.png` next to the **CL8Y DEX** title
- Marketing, splash, about, and print where the coin is shown at **≳ 96px**
- Open Graph is the **community medallion** landscape card `/og-image.png` (1200×630, [#578](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/578)) — not a favicon dump and not the old #488 product-copy card. Production crawlers must fetch `https://dex.cl8y.com/og-image.png`. Square source: [`frontend-dapp/brand/community-opengraph-concept.png`](../frontend-dapp/brand/community-opengraph-concept.png) (do not serve as `og:image`).

Do **not** downscale `/logo.png` into a favicon. Source for the simplified family is `/logo-simplified-variant.png` (1024², transparent). `/logo-simplified-variant-lowquality.png` is the original 145² capture only — do not ship it as a favicon or header mark.

Canonical same-origin only — do not wire user-controlled OG URLs, and do not build `og:image` from the request host header or the browser location object. Meta: `frontend-dapp/index.html` `og:description` / `twitter:description`; production bake: [`viteOg.ts`](../frontend-dapp/viteOg.ts). Favicon `<link>` tags: `favicon-32.png` / `favicon-16.png` (browsers also fetch `/favicon.ico`). Agent playbook: [`skills/AGENTS_FRONTEND_OPENGRAPH.md`](../skills/AGENTS_FRONTEND_OPENGRAPH.md) (invariants **OG-1–OG-8**).

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
| `tab-glass*` | Segmented controls | Limit/Market, slippage, order type — **not** Buy/Sell side |
| `side-control` + `side-buy-*` / `side-sell-*` | Buy/Sell side radiogroup | `/trade` + `/limits` only ([#563](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/563)) |
| `input-glass` / `label-glass` | Fields | Always associate `htmlFor` |
| `badge-glass-accent` | Brand/gold chips | Cool surface + gold border/text |
| `alert-*` | Inline notices | `role="alert"` when assertive |

## Limit place IA

Default `/limits` place card (#488 reopen): **rate** (“When 1 {token0} is worth”) → **% chips** (side-aware maker offsets: bid `0%−`/`−1%`/`−5%`/`−10%`, ask `0%+`/`+1%`/`+5%`/`+10%` — [#495](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/495)) → **Pay** → **Receive** → **Expiry** (advanced/ladder progressive disclosure). **Single / Ladder** tabs: Ladder create fields must render when a pair is selected even if the wallet is disconnected — Connect Wallet CTA on the place button, same as Single ([#494](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/494); invariant §14 in [`skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md`](../skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md)). Order book and open placements sit **below** the place card. No instructional paragraphs on the primary card; blocking errors stay visible. Details: [`skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](../skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md) · copy rules: [`skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](../skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) · agent playbook: [`skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md`](../skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md). On `/trade`, the sticky **Place limit** CTA must stay opaque and keep validation banners in normal flow ([#500](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/500); [`skills/AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md`](../skills/AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md)).

Default `/` Swap card: centered Pay → flip → Receive → CTA; route + min-received on card; verbose trade/signing details behind progressive disclosure. Cool flip control (no brown).

Default `/trade` ticket sections (#489): short titles only — **Side**, **Limit**, **Market** — no “Maker side / Choose direction / Resting order” essays. Action panels use cool blue wash (not amber). Buy/Sell **side** controls use green/red semantic fills; Limit/Market tabs stay blue `tab-glass*` ([#563](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/563); [`skills/AGENTS_FRONTEND_TRADE_TICKET_HEADING.md`](../skills/AGENTS_FRONTEND_TRADE_TICKET_HEADING.md)).

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
| 24h volume | Trailing window (`now − 24h`), not a midnight reset. `$0` means no priced swaps in that window (idle), not a calendar-day close. Same for Protocol **7d** / **30d** (`now − N days`). Playbook: [`AGENTS_FRONTEND_TRAILING_WINDOW.md`](../skills/AGENTS_FRONTEND_TRAILING_WINDOW.md) ([#576](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/576)). |
| Pay with any token | Shared invoice checkout: user picks any routable token; the payee still receives a **canonical invoice** (e.g. 50 UST1). CTA is **Pay** / **Enable**, not Swap. Playbook: [`AGENTS_FRONTEND_PAY_INVOICE.md`](../skills/AGENTS_FRONTEND_PAY_INVOICE.md) ([#595](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/595)). |
| Invoice (protocol fee) | Exact CW20 amount a feature charges (SKU unlock or settings batch). Not a DEX swap fee and not a community token tax. |
| Community tax token | Leader-created CW20 from the #592 template. Buy/sell/wallet tax is **not** the DEX swap fee. Manager cannot migrate wasm (CMM-only). Playbook: [`AGENTS_FRONTEND_CREATE_TOKEN.md`](../skills/AGENTS_FRONTEND_CREATE_TOKEN.md) ([#593](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593)). |
| Extra exemptions | Paid SKU: manager-chosen wallets skip **buy, sell, and transfer** tax. Launch guards still apply. Playbook: [`AGENTS_COMMUNITY_TAX_EXEMPT.md`](../skills/AGENTS_COMMUNITY_TAX_EXEMPT.md) ([#609](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/609)). |
| Create Token | More-menu flow to instantiate a community tax token (`/token/create`). Not faucet **Mint** and not **Create Pair**. |

## Verification

```bash
# No legacy -neo classes
rg '-neo' frontend-dapp/src && exit 1 || echo OK

# Tailwind + trade-bootstrap + blue/gold alignment (#488)
python3 scripts/check_design_tokens.py

# Frontend lint + unit tests
make lint-frontend
make test-frontend

# Open Graph / Twitter cards (#578)
make verify-issue-578
```

Manual matrix (both themes): Swap, Limit, Trade, Pool, Portfolio, Connect Wallet modal — cool navy/slate surfaces, blue CTAs, gold network/brand accents. Connect Wallet rows include circular brand logos ([#490](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/490); [docs/frontend.md § logos](./frontend.md#connect-modal-wallet-logos)).
