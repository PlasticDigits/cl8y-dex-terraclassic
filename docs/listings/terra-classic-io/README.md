# terra-classic.io — CL8Y DEX directory listing (#1261)

List the live [CL8Y DEX](https://dex.cl8y.com) on [terra-classic.io](https://terra-classic.io) under **Markets - DEX** and **Applications**. Tracking: [#1261](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1261). Parent catalog #639 stays closed.

Submitted 2026-10-04: [terra-classic-io/website#55](https://github.com/terra-classic-io/website/pull/55). Awaiting upstream maintainer review and merge; the live listing is not yet verified. Reuse this PR for follow-up.

## Exact upstream files

| Pack file | Destination in [terra-classic-io/website](https://github.com/terra-classic-io/website) |
|-----------|-------------|
| [`projects.fragment.ts`](./projects.fragment.ts) | Insert the single object into `src/data/projects.ts`'s `projects: ProjectLink[]`, alphabetically between ChangeNOW and Coinhall |
| [`cl8y.png`](./cl8y.png) | `public/logos/dex/cl8y.png` |

The PNG is a byte-for-byte copy of [`tokenlist/images/CL8Y.png`](../../../tokenlist/images/CL8Y.png): 256×256 RGBA. The `logo` value includes `/public/`, matching upstream's convention. No external image host or generated artwork is needed.

Checked upstream `main` at [`d31436523eb7c41de874802740eef00163aefe56`](https://github.com/terra-classic-io/website/tree/d31436523eb7c41de874802740eef00163aefe56) on 2026-10-04: no CL8Y name or `dex.cl8y.com` URL; no existing CL8Y pull request. Both categories and the `onchain` indicator match the current `ProjectLink` schema and [contribution instructions](https://github.com/terra-classic-io/website/blob/main/CONTRIBUTE.md).

## Invariants

| ID | Rule |
|----|------|
| **TC1261-1** | One project: `CL8Y DEX`, URL equal to catalog `pins.dapp` (`https://dex.cl8y.com`). |
| **TC1261-2** | `description: 'Native DEX'`, `indicator: 'onchain'`, categories exactly `['dex', 'applications']`, with DEX first. |
| **TC1261-3** | Logo `/public/logos/dex/cl8y.png`; copy the repository PNG bytes to the upstream destination. |
| **TC1261-4** | No token rows, gems, ALPHA, USTRIX, SpaceUSD, tax templates, BSC addresses, or invented CoinGecko IDs. |
| **TC1261-5** | No price or peg claims. UST1 is an unstablecoin; USTR is not a stablecoin. |
| **TC1261-6** | Catalog venue `terra-classic-io` and child #1261; permanent six, forms, skip list and wallet pins stay unchanged. |
| **TC1261-7** | L639-7: **Do not open upstream PRs from CI.** Operator/agent submits the GitHub PR through `gh` outside CI. |
| **TC1261-8** | `make verify-issue-1261` and `make verify-issue-639`; no indexer, wasm, factory, wallet or deployment changes. |

## Submit and follow up

1. Check current upstream data and all CL8Y PRs with `gh`; reuse an existing submission rather than opening a duplicate.
2. Fork/clone the website, insert the object alphabetically, and copy `cl8y.png` to `public/logos/dex/cl8y.png`.
3. Run the website's type check and build. Open one GitHub PR against `main`, titled **Add CL8Y DEX to Markets - DEX**, linking #1261.
4. Record the PR URL in this pack, the catalog venue/child, and #1261 using `fj`. Upstream maintainers approve and merge; check the live directory after deployment.

No CAPTCHA, wallet, signing or treasury action is involved. A submitted PR is awaiting upstream review until merged; do not report the live listing as complete before checking deployment.

## Verify locally

```bash
make verify-issue-1261
make verify-issue-639
```

The verifier is offline: it checks the single object, catalog identity, logo bytes, and submission boundaries. It does not submit a PR or prove upstream merge/deployment.

Skill: [`AGENTS_TERRA_CLASSIC_IO.md`](../../../skills/AGENTS_TERRA_CLASSIC_IO.md). QA: [`issue-1261`](../../qa/issue-1261/README.md).
