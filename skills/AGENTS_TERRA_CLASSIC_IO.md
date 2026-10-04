# Agent playbook: terra-classic.io CL8Y DEX directory (#1261)

Use for the CL8Y DEX listing in [terra-classic-io/website](https://github.com/terra-classic-io/website), `src/data/projects.ts`. This is one product on one directory, tracked by [#1261](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1261).

Pack and invariants **TC1261-1–TC1261-8**: [`docs/listings/terra-classic-io/README.md`](../docs/listings/terra-classic-io/README.md). Parent: [`AGENTS_LISTINGS.md`](./AGENTS_LISTINGS.md), **L639-1 / L639-2 / L639-5 / L639-7**.

- **TC1261-1 / TC1261-2:** Submit one `CL8Y DEX` object; URL `https://dex.cl8y.com`, `Native DEX`, `onchain`, categories `['dex', 'applications']`. Insert alphabetically in `projects: ProjectLink[]`.
- **TC1261-3:** Copy pack `cl8y.png` unchanged to `public/logos/dex/cl8y.png`; source `tokenlist/images/CL8Y.png`. Object logo includes `/public/`.
- **TC1261-4 / TC1261-5:** No token/gem rows or invented prices. UST1 is an unstablecoin; USTR is not a stablecoin. No BSC URL, indexer URL, WalletConnect URI, or CoinGecko ID in the object.
- **TC1261-6:** Keep the catalog's permanent six, skip list, form packs, and wallet PR pins unchanged. Append the venue/child; do not reopen #639, #631, #629 or #644.
- **TC1261-7:** **Do not open upstream PRs from CI.** An operator/agent uses `gh` for GitHub submission outside CI, and `fj` for git.cl8y.com tracking. Check for existing CL8Y PRs before submitting.
- **TC1261-8:** Run `make verify-issue-1261` and `make verify-issue-639`. No chain/deployment work. Validate the upstream website's types and build before submitting.

Live submission: [terra-classic-io/website#55](https://github.com/terra-classic-io/website/pull/55) (submitted 2026-10-04). Follow this PR; do not open a duplicate. Record any status changes in the pack and catalog. Maintainers control upstream merge; verify the deployed directory after merge before claiming the listing is live. Do not batch Bridge, OTC, token or other directory listings into this PR.
