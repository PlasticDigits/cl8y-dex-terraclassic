# #1261 — terra-classic.io directory evidence

Issue: [#1261](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1261). Pack: [`terra-classic-io`](../../listings/terra-classic-io/README.md). Invariants: **TC1261-1–TC1261-8**.

## Automated verification

```bash
make verify-issue-1261
make verify-issue-639
```

Checks one exact `ProjectLink` object, dApp/catalog lockstep, DEX first, onchain indicator, byte-identical repository logo, and documentation of **L639-7** (no upstream PRs from CI). The exact object rejects extra fields/rows, forbidden URLs, token/gem names and price claims. Parent verification preserves the permanent six, required forms, wallet pins and skip list.

Local negative checks also rejected an indexer URL, CEX category, hybrid indicator, second project, ALPHA name, positive peg copy, extra price-ID field, and changed logo bytes. Existing catalog tokens, pins, owned surfaces, forms, wallet rows, skip list and children were compared against the base commit and are unchanged.

## Upstream evidence

On 2026-10-04, `terra-classic-io/website` main at `d31436523eb7c41de874802740eef00163aefe56` has no CL8Y DEX name or dApp URL. `gh pr list --state all --search CL8Y` returned no prior submission. `CONTRIBUTE.md` requires alphabetical placement and DEX as the first category. Repository logo is already a 256×256 RGBA PNG.

Submitted 2026-10-04: [terra-classic-io/website#55](https://github.com/terra-classic-io/website/pull/55), commit `fb3cf71`. Only `src/data/projects.ts` and `public/logos/dex/cl8y.png` are changed.

- `make verify-issue-1261`: passed, including parent #639 checks.
- `make verify-issue-639`: passed (4 checks).
- Upstream `yarn install --frozen-lockfile`, `yarn type-check`, and `yarn build`: passed (client and SSR).
- Built `dist/client/logos/dex/cl8y.png`: byte-identical to the submitted PNG.
- `git diff --check`: passed in both repositories.

Upstream maintainer review, merge and deployed directory verification remain pending. The issue stays open while the live listing is pending; no wallet, CAPTCHA or operator signing work is required.
