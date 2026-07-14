# Issue #489 docs/skills alignment

Source-of-truth update for anti-cognitive-overload retail copy and shared terminology ([#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489)). Complements the #488 visual redesign ([#488](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/488)).

Verified against local Vite with `VITE_INDEXER_URL=https://indexer.dex.cl8y.com` (soft-launch indexer).

## Canonical docs

| Doc | Role |
|-----|------|
| [`docs/design-system.md`](../../design-system.md) | Principles, Limit/Trade IA, [Terminology glossary](../../design-system.md#terminology-glossary) |
| [`skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](../../../skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) | Agent playbook — labels, errors, Docs link, safety boundaries |
| [`docs/frontend.md` § Retail copy & cognitive load](../../frontend.md#retail-copy-cognitive-load) | Engineering invariants |
| [`docs/qa/issue-488/`](../issue-488/) | Visual QA shots from #488 reopen |

## Screenshots (this folder)

| File | Checks |
|------|--------|
| `swap-dark.png` / `swap-light.png` | You Pay / You Receive; blue CTA; cool surfaces |
| `limits-dark.png` / `limits-light.png` | Rate → % chips → Buy/Sell → Pay → Receive → Expiry + Docs |
| `limits-place-*.png` | Place-card crop (IA order) |
| `trade-*.png` / `trade-ticket-*.png` | Ticket section titles **Side** / **Limit** (no essay chrome) |
| `header-*.png` | Gold hairline active nav; blue Connect |
| `risk-ack-modal-dark.png` | Required risk gate retained |
| `wallet-modal-dark.png` | Connect modal chrome |

## QA rows

Manual checks: [`QA_TEMPLATE.md`](../../../QA_TEMPLATE.md) §10.2.12–10.2.14.
