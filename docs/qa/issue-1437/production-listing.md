# #1437 production rollout and GeckoTerminal submission

[Issue](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1437)
· [implementation receipt](verification.md)
· [agent checklist](README.md)
· [LV1437 playbook](../../../skills/AGENTS_LISTING_VIRTUAL_PAIRS.md)
· [form pack](../../listings/forms/geckoterminal.md)
· [database isolation runbook](../../runbooks/indexer-database-isolation.md).

## Migration evidence (2026-10-08)

The merged implementation is
[`4572e8779801c5c90b63ec95a0c7d5c38ce50bbe`](https://git.cl8y.com/code/cl8y-dex-terraclassic/commit/4572e8779801c5c90b63ec95a0c7d5c38ce50bbe),
following [implementation PR #1441](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1441)
and [treasury custody correction #1442](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1442).
Both passed mandatory CI before normal merge.

Production read-only preflight completed at **01:37:04 UTC** with 61 existing
migrations. The separate exact-revision native migrator completed backup,
additive migration and verification at **01:41:04 UTC**, with **63 migrations**
and schema marker **`20261007133000`**. The old indexer was stopped; the helper
also checked actual runtime database sessions before taking the snapshot.

- [x] Strict original migration history/checksums preserved; only the two #1437
  migrations were pending and applied.
- [x] Existing role attributes, credential verifiers and memberships unchanged;
  no ownership transition or credential rotation.
- [x] New listing table DML and sequence grants verified under the effective
  runtime role, with four real permission-denial probes.
- [x] Private structurally validated snapshot retained: 54,148,161 bytes,
  SHA-256 `d265cdc8413fcd24355ffee4455a6a2ed0f4495bb42a66edfb21ff9764850117`.

- [x] Completed migrator stopped at **01:45:47 UTC**. Both production and
  preview bootstrap credential fields were removed and absence read back;
  the private backup volume remains. The job defaults to idle.

The maintenance job authenticated using the unchanged existing bootstrap
capability and executed native migration statements as the existing migrator.
It did not authenticate as runtime; the deployed indexer's successful guarded
startup must establish that separately. Privileged credentials never entered
indexer server/build configuration or the public receipt.

## Deployed indexer

The indexer deployment finished at **01:46:56 UTC** on the recorded revision.
Public `/health` returned `status: ok`, exact git SHA `4572e877...`, expected chain
`columbus-5`, and a verified matching LCD endpoint. Guarded runtime startup
succeeded with the unchanged runtime credential and compatible schema; the
maintenance job's effective-role tests are separate evidence.

- [x] Exact merged revision deployed and healthy.
- [x] Runtime authentication and guarded schema startup established by the
  deployed server; no bootstrap/migration capability was injected into it.
- [x] Existing application configuration and runtime environment metadata
  preserved apart from the explicit commit pin.

## Public live checks and retest

First verification ran **01:48:41–01:49:06 UTC**; the unchanged public probe
retested **01:49:44–01:50:08 UTC**, after the ticker cache interval.
Each run reported **59 PASS, 0 FAIL, 4 SKIP** across 53 public GETs.
Overall live verification is **PASS WITH SKIPS**, with the limits below.
The indexed tip advanced from **30740080** to **30740093** between snapshots.

- [x] All five documented opaque markets published in CG and resolved through
  GT with exact native denoms and pinned wrapper/USDT identities.
- [x] Positive plain-decimal prices, consolidated pool fields, one-level books
  and historical-trade response shapes verified.
- [x] Native CoinGecko ids correct; wrappers do not inherit them. Public CMC and
  ordinary pair lists omit opaque ids; ordinary detail guards reject them.
- [x] Real cLUNC/cUSTC factory event at block **30739739**, transaction
  `B8FE66FF558DDDA970C6FA99CCE64368608FDFDE0750024ED83BFB15CBA2D4A0`,
  preserved alongside LUNC/USTC and cLUNC/USTC projections. Amounts, price,
  post-event reserves, maker and block match the source; event indexes are unique.
- [ ] Historical cLUNC/LUNC and cUSTC/USTC principal prints: no eligible indexed
  history available; historical mapper replay was not performed.
- [ ] Historical LUNC/USDT prints and sampled GT source projection: no eligible
  prints. The sampled factory GT event exposes zero post-event reserves, so
  emitting a projected swap would invent history; the projection is omitted.

Those three empty histories plus the USDT GT provenance check account for the
four explicit skips. Current market metadata does not prove historical trade
availability. No wallet transaction, artificial trade or funding was used to
manufacture evidence. Public APIs cannot independently establish SQL row counts,
custody provenance columns or zero outbound LCD calls; the implementation tests
and scoped migration receipts supply separate evidence for those boundaries.
The optional #655 browser acceptance remains unverified as documented in the
original implementation receipt; this rollout does not claim it passed.

## GeckoTerminal submission

Submitted **once on 2026-10-08 at 01:54:54 UTC** through the
[official DEX/chain listing form](https://about.geckoterminal.com/dex-chain-listing).
Google displayed the literal confirmation **“Your response has been recorded.”**
The workstation retained the exact submitted answers, confirmation text,
screenshot and receipt privately under
`~/.local/state/cl8y-dex/release-1437/forms/`.

- [x] Existing **`terra` / TerraClassic** network; **DEX Addition**, **Non-EVM**,
  **Not a Fork**, regular listing with Express declined.
- [x] Adapter base `https://indexer.dex.cl8y.com/gt`, key `cl8y`, public Forgejo
  logo, integrator documentation and adapter documentation links.
- [x] Refreshed highest positive USD-liquidity real CG factory ticker at
  **01:54:45 UTC**: **UST1/cUSTC**, approximately **$97,885.58**, pair
  `terra1ceprjsxp86ggftf5e38wwt34l83e5gq7penkdnv4wsatkwcs8v6qccw55f`.
  This is a dated ranking, not a USD peg or promise of a future ranking.
- [x] All five currently published opaque ids described as listing projections,
  with treasury/source prerequisites and historical verification limits stated.
- [x] Submission confirmation saved; no duplicate attempt.
- [ ] GeckoTerminal listing approval/crawler acceptance: pending external review.

The relevant follow-up is historical source/custody replay using an attested
archive when available, followed by actual peg and USDT GT event verification.
Do not invent missing post-state, fund the mapper or produce trades solely to
make a checklist pass. Listing approval is a separate external result.

## Agent verification procedure

1. Read the implementation receipt and LV1437 invariants. Run
   `make verify-issue-1437` in an isolated worktree with disposable PostgreSQL,
   including its same-executable retest, when verifying changed implementation.
   Existing local test results do not prove production deployment.
2. Read `https://indexer.dex.cl8y.com/health`; require the recorded deployed
   revision, healthy status and verified `columbus-5` chain identity. Confirm
   the separate migrator receipt establishes schema `20261007133000`, all 63
   checksummed migrations, preserved original history, unchanged credentials
   and ownership, and least-privilege runtime grants. Runtime startup proves
   actual runtime authentication; owner-session ACL probes alone do not.
3. Read `/gt/latest-block` twice and confirm progress. Read `/cg/pairs` and
   `/cg/tickers`, then `/gt/pair?id=` for each published opaque market and
   `/gt/asset?id=` for its pinned legs. Retest after cache expiry. Compare
   native CoinGecko ids and exact contract/denom identities with the contract.
   Record absent markets without assuming all five must always publish.
4. Read a bounded `/gt/events?fromBlock=H&toBlock=H+N` window (at most 2000
   blocks). Compare observed projected swaps with real indexed source
   transaction ids and factory events, including post-event reserves and unique
   combined event indexes. An empty recent window cannot establish historical
   mapper principal replay. Do not transact or move funds to manufacture proof.
5. Read ordinary `/api/v1/pairs`, CMC and DeFiLlama outputs; ensure opaque
   markets stay out of retail/economic responses. Public API readback cannot
   independently prove unchanged database accounting; preserve the narrower
   scope of local integration and migration evidence.
6. Verify the saved final form answers and literal confirmation text. Request
   DEX Addition on existing network `terra`, Non-EVM, Not a Fork, regular listing.
   Feature the highest positive-liquidity real factory ticker at submission;
   virtual reserves are not a factory pool. A recorded response proves receipt
   only. Check GeckoTerminal separately for eventual listing approval.

Record revision, UTC time, commands, outcomes and evidence scope. Complete every
runnable item first. An unavailable item may be **SKIP** after investigating its
prerequisites; include the attempted command, unavailable capability and criterion
left unverified. When all executed checks pass, overall verification may be
**PASS WITH SKIPS**. Keep skipped boxes unchecked. An executed failure remains
**FAIL** until corrected and retested. Neither skips nor a form receipt establish
GeckoTerminal acceptance.
