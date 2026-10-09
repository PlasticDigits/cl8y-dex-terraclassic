# PR consolidation, 2026-10-09

The user explicitly requested merging every open PR, including WIP #1455,
skipping CI, overriding unavailable holds/gates, performing feasible local sanity
checks, consolidating all local work into main, and preserving other workers' work.
No successful CI status was fabricated and branch protection was not weakened.

## Included code and merge resolution

- #1450: pinned Go builder digest; Docker manifest resolves.
- #1451: pinned Postgres digest in both CI definitions; Docker manifest resolves.
- #1454 / #1453: independent pool/hybrid observations and bounded provider failover.
- #1455 / #1452: cached local quotes and pinned operations/floors through signing.

The integration checkout started at `b26b762c`. PR source heads were `3ea7d9f1`,
`93296518`, `91c26fa9` and `30ae2024`. Combined-source implementation was committed
as `a2119a66`; subsequent sync commits changed ancestry/dependency pins only.
The tested source is included in the consolidation head recorded in
[local-results.json](local-results.json).

`Makefile` conflicted at the verifier targets: both were retained. The LCD client
conflicted at request admission, response limits and test modules: retain provider
circuits/failover/deadlines and quote-ingestion pacing/body caps, with both test
modules. Every paced background attempt remains bounded by the total deadline.

Inspection found an additional semantic integration problem: the new local mirror
reader used independently advanced pool reserves for book loading, erasing a usable
book on height mismatch. It now consumes a fresh compatible hybrid generation at
its original timestamp, falling back to current pool-only reserves when the hybrid
is missing/stale. Both minimum and maximum book heights must match. The new
`local_mirror_preserves_hybrid_age_and_falls_back_to_independent_pool` disposable-DB
regression passes twice, covering independent pool advancement and empty-book age.

Normal merges were attempted with `bao-exec --only FORGEJO_PLASTICDIGTS_PAT -- fj
pr merge`. The CLI has no force option; HTTP 405 merge holds require the explicit
user-authorized Forgejo API `force_merge` override. #1455 was made ready with `fj
pr edit ... title`. Issue creation/comments/closure use `fj` under `bao-exec`.
Every newly created integration/merge commit uses `[skip ci]`.

## Local sanity: mixed results, not full acceptance

UTC window: 2026-10-09 11:35–11:56. Dedicated integration worktree and fresh
loopback-only Postgres container; runtime/migrator/QA credentials are separate,
private and excluded from Git. No shared database/chain data was changed.

- `make verify-issue-1452`: **13 PASS / 1 FAIL / 0 SKIP**. 632 library tests and
  the explicit manual benchmark run; four socket tests and 181 focused frontend
  tests pass on both runs. Types/formatting/docs/source stability pass.
- Socket timing: misses max 12.819 ms and warm/concurrent max 55.060 ms on the
  second run, with zero quote-stage provider calls.
- First controlled browser run passes. The second records a **3,866.9 ms**
  desktop Swap outlier and fails the 250 ms requirement. Its zero-provider-call
  and zero-page-error evidence is retained in
  [browser-timing-failed.json](browser-timing-failed.json).
- An independent controlled-browser retest passes all 40 quotes; maximum
  **100.5 ms**, zero provider calls and zero page errors. See
  [browser-timing-retest.json](browser-timing-retest.json). This does not erase
  the failed run or establish production latency.
- `make verify-issue-1453`: **10 PASS / 2 FAIL / 2 SKIP**. Provider/book/valuation/
  discovery library groups, 73 dedicated liquidity/feed DB tests, 97 frontend
  tests per pass and four browser checks pass. The two failed groups contain the
  old `api_route_solve_db_hybrid` fixture suite: **2 PASS / 8 FAIL per pass**.
  It constructs uninitialized request-time solver fixtures that now correctly
  fail closed with 503; the historical live-fidelity answers are no longer the
  current quote contract. These executed fixture failures remain FAIL.
- `npm run test:unit`: **2,948 PASS / 137 FAIL**, 346 passing and 11 failing files.
  Many fixtures still expect factory loading rather than `getAllPairsPaged`,
  LCD/direct-pair quoting/execution, or removed acquisition copy. An isolated
  `TokenSearchSelect` retest passes all 11 tests; the separate acquisition-copy
  assertion still fails. Full frontend unit status remains FAIL.
- `make verify-issue-1396`: **24 PASS / 4 FAIL / 2 SKIP**. Database/runtime isolation, socket/ACL and launcher
  checks were attempted. The legacy `api_health` wrong-chain test fails twice:
  its malformed query omits `token_in`, expecting an RPC-attestation 503;
  the current local request parser returns 400 before RPC. See the final counts
  in `local-results.json`. Two FAIL groups are the health fixture and two are the interrupted unavailable bridge waits. The published-bridge loopback wait was interrupted
  after an independent authenticated probe timed out; original failures are kept.
- The optional authenticated runtime fixture first fails because the new
  disposable cluster also gives the runtime its synthetic application-DB CONNECT
  grant. Revoking that exact synthetic grant produces two passing retests.
  No shared grants were changed.
- Supplementary network probes explicitly skip the unavailable published-bridge
  host query and host IPv6 (kernel disabled). Internal bridge TCP, rejection of
  external namespace access and opt-in host IPv4 confinement pass. Fresh ACL
  probes run against their own ephemeral cluster.
- Commit hooks (contract formatting/clippy, staged frontend checks and secret
  scanning) pass. Full tracked-file gitleaks scan passes. No hosted CI was requested or awaited.

The 11 failing frontend files are `TokenSearchSelect.test.tsx`,
`SwapPage.acquireGuidance.test.tsx`, `SwapPage.feeDiscountRegistryBanner.test.tsx`,
`TradeMarketOrderPanel.extraDebitSell.test.tsx`, `SwapPage.assetDecimals.test.tsx`,
`SwapPage.extraDebitSell.test.tsx`, `TradeMarketOrderPanel.acquireGuidance.test.tsx`,
`TradeMarketOrderPanel.slippagePresets.test.tsx`, `swapAutoGasFeeHint.test.tsx`,
`SwapPage.queryParams.test.tsx` and `TradePage.test.tsx`.

## Remaining work and cleanup scope

[#1452](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1452) remains open
for implementation: community-tax forwards, deducting-hook inverse, explicit
manual splits, zero-hop wrap/unwrap, incremental large-population admission and
complete authoritative actor-inventory recovery. Existing route/page/health
fixtures also require migration to the current contract and product regressions
must be separated from stale fixture expectations. Failures and the explicit WIP
merge override are documented there; this is not a verification-only hold.

[#1456](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1456) tracks separate
migration/deployment, live external quote latency, connected-wallet/device scope,
provider failover/recovery, supported coverage/history reconciliation and the
trusted-policy-service digest follow-up. #1453's implementation can close with
that remaining operational scope tracked. Existing DeFiLlama operations stay in
[#1449](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1449).

Initial and final stash inventories are empty. All local branch heads and the
GitLab mirror main are ancestors of the consolidated main. The three existing
worktrees/checked-out branches belong to other issue workers and are retained;
the temporary merge worktree, synthetic services, private env files and merged
Renovate remote branch refs are cleaned up. Origin worker branches are preserved.


Final consolidation audit: remote merge `4e84ee5e3b947714f33cbe5fe05a1e0fe734443e`
contains all four PR heads and the resolved integration/evidence commits. Local
main was pulled from origin, the GitLab main was already included, and all
remaining local/remote refs are ancestors. There are zero open PRs and zero
stashes. The final source fingerprint exactly matches the tested source. Both
merged Renovate remote branches and the owned temporary integration worktree/
branch are removed; original worker branches/worktrees remain clean and preserved.
