# Issue #578 Open Graph / Twitter cards

Static social preview for every dApp SPA route. Crawlers never run React.

| Artifact | Role |
|----------|------|
| [`frontend-dapp/public/og-image.png`](../../../frontend-dapp/public/og-image.png) | Shipped 1200×630 community medallion card |
| [`frontend-dapp/brand/community-opengraph-concept.png`](../../../frontend-dapp/brand/community-opengraph-concept.png) | Square source (not a crawler URL) |
| [`scripts/compose-og-image.py`](../../../scripts/compose-og-image.py) | Recompose without stretching |

**#488** shipped the first product-copy card. That artwork is **not** the live OG image — see [issue-488 README](../issue-488/README.md).

Verify (no chain): `make verify-issue-578`

Playbook: [`skills/AGENTS_FRONTEND_OPENGRAPH.md`](../../../skills/AGENTS_FRONTEND_OPENGRAPH.md) · invariants **OG-1–OG-8** in [`docs/frontend.md`](../../frontend.md#open-graph-social-cards).
