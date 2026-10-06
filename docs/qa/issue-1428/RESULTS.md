# Verification receipt: #1428


Implementation: [PR #1432](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1432).
Issue: [#1428](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1428).
Invariants and repeatable agent procedure: [README.md](README.md),
[GR1428](../../../skills/AGENTS_GREEDY_REVERSE_SUPPORT.md).
Worktree: `/home/answorld/repos/worktrees/cl8y-dex-1428`, based on
`e7b54989c21fc411141c4c6c3af553c03dea1bdd`, verified 2026-10-06 UTC.
The issue and all comments were read through openbao-tools `bao-exec` with `fj`;
there were no comments. All chain accounts, contracts and liquidity are synthetic.

## Change and qualified scope

Router **1.5.0** deterministically rejects reverse routes containing any greedy
hop when the route has four total hops. `GREEDY_REVERSE_ROUTE_UNSUPPORTED`
returns before capability, factory or pair queries. Mixed and empty-book routes
follow the same policy. Four-hop pool reverse and four-hop forward/execution stay
available; fresh forward sufficiency, original-trader/gross funding, net hook
floors and simulation/maker/scan ceilings are preserved.

The support ceiling of three total greedy reverse hops is admission policy,
not a promise that every admitted book fits a provider. The measured populated
successes are explicitly **one hop, maker cap 8, one raw ask unit both directions**,
plus **one human ask unit forward**. They are independent requests, not fallback
quotes replacing rejected original operations. Empty three-hop greedy boundary
cases also pass in both directions.

## Evidence and checks

| Check | Result and scope |
| --- | --- |
| `make build-optimized` and native provenance verifier | PASS; pinned optimizer, fresh executable volumes, locked inputs and unchanged source hashes verified. Only router release hash changes. |
| `make verify-issue-1428` | **PASS: 9 groups, 0 failures, 0 skips.** Frozen integrated reproduction plus first/retest router tests (24), funded contract regressions (19), client tests (107), harness tests (8), format/syntax/diff checks and isolated matrix. |
| Final book matrix first/retest | PASS; **22 checks and 15 fresh sufficient forward validations each**. Original populated four-hop request rejected in both directions. Empty/pool/declared behavior and scoped populated successes retained. |
| `make verify-issue-1422`, router version 1.5.0 | PASS with one explicit historical-baseline skip; **10 groups, 0 failures**. Six funded scenarios and **18 actual 3M smart-query successes per run**, repeated from new fixtures; probe transactions and accounting/rollback also pass. |
| `make verify-issue-1398` | PASS; **7 groups, 0 failures, 0 skips**. Focused net-floor/hook/client tests repeated, full contract workspace, formatting and crosslinks. |
| `make verify-issue-1389` | PASS with one separate-chain-command skip; **8 groups, 0 failures**. Contract/API/client checks repeated; chain scope is covered by the final #1428 matrix rather than a second verifier shell command. |
| `make verify-issue-1381` | PASS with explicit skips; **13 groups, 0 failures**. Contract/indexer/client checks repeated; lint/types/build, actual six funded accounting scenarios, four migration assertions and two atomic invoice flows pass. Native wrap/unwrap and production execution remain unverified. |
| `make verify-issue-1390` | PASS with one production-execution skip; **15 groups, 0 failures**. Source checks repeated; lint/types/build and all **16 isolated chain guard checks** pass on migrated router 1.5.0. |
| Independent review | Router tests (24), all-target router Clippy with warnings denied, harness integrity tests (8), saved fuzz regression and first chain/provenance receipts pass. Final disposition: [independent review](review.md). |

All executed local final acceptance failures were repaired and retested. Earlier failed
attempts and exploratory failures are retained/disclosed below. This receipt does
not claim that every command ever executed passed.

Commands used the standard repository verifier targets with task-owned artifact,
container, cache and evidence paths. Contract tests used the existing local target;
indexer test profiles used debug=0/incremental=0 and the dedicated QA control
credential exported privately from `/tmp/dex1396-qa.env`. Each integration binary
created its own disposable database under the #1396 helper. No credential values
were published. Node 24 dependencies and existing repository hooks were retained.

### Raw chain evidence

- [Integrated baseline](evidence/book-matrix-before-fix.json) and
  [baseline reproduction/retest](evidence/book-matrix-baseline-retest.json):
  router **1.4.0**, pair **1.21.0**, existing **15,000,000** query envelope.
  Fourteen preceding checks pass; populated four-hop queries in both directions
  remain raw **FAIL**, `baseline_regression=REPRODUCED`. Their full provider
  errors and independent failed 12M probe receipts remain visible.
- [Final first matrix](evidence/book-matrix-first.json) and
  [fresh-fixture retest](evidence/book-matrix-retest.json): router **1.5.0**,
  unchanged pair and **15M** envelope. Both original populated four-hop queries
  return the stable admission marker. Probe gas is **207,757** in both directions
  in the first run, including transaction/wrapper overhead. This is a measured
  rejection receipt, not a simulation count or an exact LCD meter reading.
- [3M funded first](evidence/funded-first.json) and
  [3M funded retest](evidence/funded-retest.json): exact node image/config,
  stored checksums, cw2, complete queries/responses and separate probe gas.
  Both router 1.5.0 runs pass all six accounting scenarios and eighteen queries.
- [Funding/migration](evidence/route-tax-1381.json),
  [atomic invoice flows](evidence/route-tax-flows-1381.json) and
  [launch guards](evidence/route-guards-1390.json): actual isolated
  router **1.4.0→1.5.0** migration preserves funding/configuration and pair pins.
- [Node initialization](evidence/node-init.json),
  [build provenance](evidence/build-provenance.json),
  [test dependencies](evidence/test-dependencies.json), and command summary
  preserve scope and immutable hashes. [Command summary and compressed logs](evidence/verification.json) record exits, counts and log hashes. The independently rebuilt test-only probe
  has a distinct hash from the historical probe because registry paths are
  embedded in the binary; each matrix records its actual stored probe checksum.

The native build receipt records a dirty source snapshot at the base revision,
then verifies inputs unchanged throughout the successful build. After checkout of
the PR, reproduce its recorded `source_commit` in a detached worktree and overlay
the exact files named in `source_inputs` from this PR, verifying each SHA-256.
Copy the matching local optimized outputs named in `artifacts`, restore the native receipt from public `metadata`, converting each `path`/`sha256`
entry in `source_inputs` and `artifacts` back into a dictionary, then run
`python3 scripts/ci/build-provenance.py --root /path/to/snapshot --verify /path/to/receipt.json`.
This follows the existing #1422 public receipt format. Transaction hashes in public
JSON use lowercase hexadecimal; the full raw command logs preserve their original
case. Hash bytes, provider error strings and gas receipts are unchanged.
A later documentation/implementation commit is not the receipt's build revision.
An unsigned receipt does not independently authenticate a compiler or deployment.

## Failed attempts, limits and explicit skips

The first baseline run hit an unrelated uninitialized-oracle Observe assertion
([raw incomplete receipt](evidence/fixture-repair-first.json)). The harness now
seeds one oracle observation **after** all reverse measurements, so book/state
reproduction is unchanged; the repaired baseline and both final runs pass Observe.

The first build's source-provenance check rejected a test file changed during
compilation. A subsequent build was stopped to fix the saved fuzz fixture below.
The final frozen-source cold build passes provenance and produces the same router
hash as the initial optimized output; only the accepted final receipt qualifies it.

Initial #1381/#1389 integration invocations failed because the dedicated QA
credential was not exported. The same checks pass after privately exporting the
correct QA environment, with disposable owned databases. No shared migration
history or privilege policy was altered.

The broad contract run found a pre-existing slippage property whose floored
second deposit was called exactly proportional at zero tolerance. The fixture
now uses whole units of the reduced integer reserve ratio and asserts the exact
cross-product before checking arbitrary tolerance. The saved failure seed remains;
focused test/retest, independent review and complete workspace retest pass.
Runtime liquidity or slippage rules were not weakened.

[Shorter-route exploratory receipts](evidence/shorter-route-limitations.json)
retain populated mixed-decimal **one-to-three-hop failures** on the integrated
1.4.0 fixture: 100-human/maker100 requests fail at 15M in both directions;
one-human/maker8 requests still exhibit work-budget/provider failures outside the
successful forward one-hop case; raw-one/maker8 multi-hop requests also fail.
These executed failures do not qualify as supported quotes and are not hidden as
skips. The admission repair does not optimize those paths or promise their gas
feasibility; broader populated-book optimization/qualification is a relevant
separate followup. Do not enlarge limits or substitute weaker executable requests.

Explicit unavailable/separate scope: the optional #1422 historical pre-optimization
artifacts were not supplied again (its committed historical evidence is intact;
#1428's integrated baseline is fully reproduced); the #1389 shell gas command is
substituted by the actual #1428 first/retest matrix; no compatible owned native
wrap-stack environment was supplied; production migration/provider acceptance
remains #1384/#1386/#1421. These skips do not mark their criteria complete and do
not create an implementation hold. Runtime protection remains enforced.

## Acceptance checklist

- [x] Read issue/comments and reproduce integrated 1.4.0/1.21.0 populated four-hop
  failure on the pinned isolated node; preserve raw errors and preceding checks.
- [x] Implement deterministic bounded admission with zero external queries,
  including empty/mixed four-hop greedy paths and both orientations.
- [x] Test/retest exact successful scopes and fresh net forward sufficiency;
  preserve pool-only/funded execution, budgets, trader/gross semantics and floors.
- [x] Verify stale executable Pay/operations invalidate while Receive survives;
  no automatic retry, weaker fallback or resend.
- [x] Recheck six funded pool scenarios at unchanged 3M and actual migration guards.
- [x] Document invariants, integration behavior, raw evidence and third-party agent
  procedure; permit explicit skips with overall PASS when all executed final checks pass.
- [ ] Production deployment and real-provider acceptance: separate existing rollout
  issues; no production transactions were submitted in this implementation.

## Remote CI disposition

[Woodpecker pipeline 332](https://ci.cl8y.com/repos/7/pipeline/332/1) remains
**FAIL** at the secret-scan gate. [Finding metadata](evidence/remote-ci.json)
identifies three unchanged public Venus address literals in `oracle_safety.py`
and two existing QA fixtures; their bytes match `origin/main`. The repository
secret-scan configuration allows this public identifier, while the separate trusted
hosted policy flags it. Contracts/frontend/indexer CI jobs did not execute after
the gate failure. Local full verification and repository-configured secret scans
pass; that does not establish remote CI success. Reconcile the trusted hosted
policy separately without broad exemptions or changing the fixed oracle identity.
The user explicitly permitted unavailable/separate gates to be overridden for
implementation publication; the PR is open, and the actual CI failure stays visible.
