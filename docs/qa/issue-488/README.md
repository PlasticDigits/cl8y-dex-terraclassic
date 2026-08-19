# Issue #488 visual QA

QuickSwap-inspired blue + gold redesign verification screenshots. Docs/skills copy alignment companion: [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489) · [`skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](../../../skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) · [`docs/qa/issue-489/`](../issue-489/).

Full set is also attached on [GitLab #488](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/488).

## Reopen follow-up (post-!305)

After MR !305, reopen feedback required:

| Fix | Notes |
|-----|--------|
| Gold = hairline only | No dirty brown/gold fills on nav, warnings, page wash |
| Minimal copy | Swap/Limit on-card disclosures trimmed; footer shortened |
| Product OG | `/og-image.png` + `index.html` meta emphasize swaps/limits/Terra Classic |
| Limit IA | rate → % chips → Pay → Receive → Expiry; book below place card |
| Swap IA | Cool flip button; collapsed trade/signing details |
| Remote indexer QA | Capture with `VITE_INDEXER_URL=https://indexer.dex.cl8y.com` (soft-launch env) |

## Files

| File | What |
|------|------|
| `header-brand-dark.png` | Logo + blue Connect CTA + cool active nav + gold hairline |
| `swap-*.jpg` / `limits-*.jpg` | Dark/light page chrome |
| `wallet-modal-dark.jpg` / `risk-ack-modal-dark.jpg` | Modal chrome |
| `og-image.png` (in `frontend-dapp/public/`) | Product-focused social preview |

Promoted brand assets live under `frontend-dapp/public/` (favicon / OG / `logo.png`).

Spec: [`docs/design-system.md`](../../design-system.md) · agent playbook: [`skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md`](../../../skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md) · theme: [`skills/AGENTS_FRONTEND_THEME_TOGGLE.md`](../../../skills/AGENTS_FRONTEND_THEME_TOGGLE.md) · limit price: [`skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](../../../skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md) · copy: [`skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](../../../skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) ([#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489)).
