# Agent playbook: Open Graph / Twitter cards

Use when changing dApp social previews, `index.html` meta, `/og-image.png`, or crawler-facing HTML for `https://dex.cl8y.com` ([GitLab **#578**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/578)). Related: [#488](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/488) shipped the first product OG card; **#578 replaces that typesetting artwork** with the community medallion and fixes relative image URLs that X/Twitter drops.

Crawlers do **not** run React. nginx `try_files` rewrites every public SPA path to the same `index.html`. One static tag set covers `/`, `/trade`, `/pool`, `/charts/:pair`, `/trader/:address`, and the rest.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Open Graph](../docs/frontend.md#open-graph-social-cards) | Invariants **OG-1–OG-8** |
| [docs/design-system.md § Brand assets](../docs/design-system.md) | Community medallion card vs header/favicon variants |
| [`index.html`](../frontend-dapp/index.html) | Source meta (relative image URLs for local Vite) |
| [`viteOg.ts`](../frontend-dapp/viteOg.ts) | Allowlisted origin + production HTML bake |
| [`vite.config.ts`](../frontend-dapp/vite.config.ts) | `og-absolute-meta` `transformIndexHtml` (production only) |
| [`public/og-image.png`](../frontend-dapp/public/og-image.png) | Shipped 1200×630 card |
| [`brand/community-opengraph-concept.png`](../frontend-dapp/brand/community-opengraph-concept.png) | Square source — **not** the crawler URL |
| [`scripts/compose-og-image.py`](../scripts/compose-og-image.py) | Recompose 1200×630 without stretching the square |
| [`docker/frontend/nginx.conf`](../docker/frontend/nginx.conf) | `/og-image.png` → real PNG; other paths → `index.html` |
| [docs/qa/issue-488/README.md](../docs/qa/issue-488/README.md) | #488 typesetting card is **not** live OG |

## Invariants (OG-1–OG-8)

1. **OG-1 Absolute https image** — Production `og:image` and `twitter:image` are `https://dex.cl8y.com/og-image.png` (or another origin baked from `PUBLIC_ORIGIN_ALLOWLIST`). Relative `/og-image.png` is OK only for non-production Vite.
2. **OG-2 Large card** — `twitter:card` stays `summary_large_image`. Do not invent `twitter:site` unless product supplies an official handle.
3. **OG-3 Community composition** — Shipped `/og-image.png` is **1200×630** (±1 px), community-medallion art (not a stretched square, not the #488 typesetting card). Regenerate with `python3 scripts/compose-og-image.py`.
4. **OG-4 File budget** — PNG or JPEG, **< 5 MB** (target < 1 MB). **No SVG.** Concept source stays out of `public/` so Coolify does not serve the 1.75 MB square as the card.
5. **OG-5 One shell** — All SPA routes return the same static meta. **No** `react-helmet`, prerender.io, or per-route titles for crawlers.
6. **OG-6 No request origin** — Do not build OG URLs from the HTTP host header, `X-Forwarded-Host`, `window.location`, query, hash, pair address, or wallet. `VITE_PUBLIC_ORIGIN` must be https and allowlisted in `viteOg.ts`; unknown values **fail the production build**.
7. **OG-7 Dimensions + alt** — `og:image:width` / `height` match the file; `og:image:alt` / `twitter:image:alt` describe the medallion (no wallet/pair data).
8. **OG-8 Docs** — Design-system + this playbook + #488 QA note describe the community card and absolute-URL rule. Verify: `make verify-issue-578`.

## Rules of thumb

1. **Same-origin file only** — `og:image` is `/og-image.png` on the baked origin. Do not hotlink token logos, CDNs, or user-supplied preview URLs.
2. **Do not stretch the square** — source is 1254×1254; pillarbox onto 1200×630 so the portrait and `CL8Y` wordmark stay intact.
3. **Do not weaken CSP / frame headers** “for crawlers.” Cards fetch the **image URL**, not an iframe of the dApp.
4. **Cache** — replace `og-image.png` in place (same URL). Query-string cache-bust (`?v=`) is discouraged. After deploy, reset the X/Twitter card cache.
5. **Copy** — title/description stay product (swaps / limits / Terra Classic). The image is brand art; do not put theme jargon in OG text.
6. **Staging** — add the staging origin to `PUBLIC_ORIGIN_ALLOWLIST` in code, then set `VITE_PUBLIC_ORIGIN`. Do not copy the request host into tags.

## Verification

```bash
make verify-issue-578
```

No LocalTerra, indexer, or wallet work. Manual after Coolify deploy: `curl -sL https://dex.cl8y.com/ | grep og:image` and `curl -sI https://dex.cl8y.com/og-image.png`; Telegram + X/Twitter large card (reset X cache if stale).

## Cross-links

- Design tokens / brand variants: [`AGENTS_FRONTEND_DESIGN_SYSTEM.md`](./AGENTS_FRONTEND_DESIGN_SYSTEM.md) ([#488](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/488))
- Production CSP / trust boundaries: [`AGENTS_FRONTEND_TRUST_BOUNDARIES.md`](./AGENTS_FRONTEND_TRUST_BOUNDARIES.md) ([#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378))
- Production build: [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md)
