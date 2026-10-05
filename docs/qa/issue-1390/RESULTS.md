# Issue #1390 results

Implementation verification: **PASS with explicit production skip**, 2026-10-05
UTC. Baseline revision: `9944ada10ee711ff8ffaa91f4c19b0f292de3924`;
implementation source hashes, completion timestamps and verifier summaries are
in [verification-manifest.json](verification-manifest.json). The PR commit
identifies the final source revision.

Policy and reviewer instructions: [LG1390](../../../skills/AGENTS_ROUTE_LAUNCH_GUARDS.md),
[acceptance checklist](README.md), [RT1381](../../../skills/AGENTS_ROUTE_LOCAL_TAX.md).

## Completed acceptance criteria

- [x] **AC1:** repeated capable assets are rejected in execute, forward/reverse
  quotes, solver eligibility and client preflight. The logical path is first
  offer plus each ask; intermediate buy/sell is one visit. Tests cover initial,
  intermediate and final recurrence, cooldown 0/1/>1, disabled launch features,
  zero rates and exemptions. Endpoint-based structural solves cannot accept
  caller-injected cyclic paths or operations.
- [x] **AC2:** normal intermediate funding succeeds; a second independent route
  in the cooldown window fails. Prior direct buy/sell history, expiry, manager
  directory exemptions and atomic rollback are covered. Existing gross funding,
  dust isolation, pair credit and tax accounting regressions pass.
- [x] **AC3:** original nonzero residue restores owned input above a lowered cap;
  zero residue and repeated refunds also succeed without increasing holdings.
  Purchased intermediate residue, final bought output and delivery to another
  recipient remain capped. Failed settlement reverts wallets, pools, supply and
  tax sinks. Old contexts without the refund label retain cap enforcement.
- [x] **AC4:** protocol-exempt traders are rejected in token previews and routed
  buy/sell execution. Forward/exact-output APIs retain sanitized rejection, while
  valid alternatives remain eligible. HTTP 422 stops Swap, wrapped Swap, Trade
  and invoices without client fallback or network retry.
- [ ] **Production acceptance:** no Columbus-5 rollout, production code/version
  inventory or bounded production execution was performed. Local verification
  does not establish production deployment.

## Commands and outcomes

Host toolchain: Rust 1.99.0, Node 24.21.0. Contracts used a separate worktree
target/cache; the indexer used a dedicated PostgreSQL instance and serial tests.
The test database ran on port 55490 with tmpfs storage and durability disabled;
these tests verify API/accounting behavior, not PostgreSQL crash durability.
LocalTerra used a private instance of the repository's pinned SDK 0.53 image,
without shared volume resets. All chain accounts and token balances are synthetic.

```bash
# Common cache/resource settings for the final verifier invocations:
export CARGO_HOME=/tmp/cl8y-dex-1390-cargo
export CARGO_NET_OFFLINE=true CARGO_BUILD_JOBS=2
export INDEXER_CARGO_TEST_DEBUG=0 INDEXER_CARGO_INCREMENTAL=0

# Reproduction equivalent to the recorded local runner in verification-manifest.json:
export VERIFY_ISSUE_1390_CHAIN_COMMAND='python3 scripts/qa/localterra-route-tax-1381.py --container cl8y-dex-issue1390-localterra --isolated-owner --artifacts smartcontracts/artifacts --code-cache /tmp/dex-1390-isolated-codes.json --evidence-file /tmp/dex-1390-funded.json && python3 scripts/qa/localterra-route-guards-1390.py --container cl8y-dex-issue1390-localterra --fixture /tmp/dex-1390-funded.json --evidence-file /tmp/dex-1390-guards.json'
export VERIFY_ISSUE_1390_SKIP_LIVE_REASON='Production deployment is outside this implementation PR; no updated production contract or bounded live execution receipt is claimed.'
make verify-issue-1390

export VERIFY_ISSUE_1381_CONTAINER=cl8y-dex-issue1390-localterra
export VERIFY_ISSUE_1381_CODE_CACHE=/tmp/dex-1390-isolated-codes.json
export VERIFY_ISSUE_1381_CHAIN_EVIDENCE=/tmp/dex-1390-regression-funded.json
export VERIFY_ISSUE_1381_FLOW_EVIDENCE=/tmp/dex-1390-regression-flows.json
export VERIFY_ISSUE_1381_LEGACY_ARTIFACTS=/tmp/dex-1390-pre-v1-artifacts
export VERIFY_ISSUE_1381_WRAP_STACK_ENV=/tmp/dex-1390-wrap.env
export VERIFY_ISSUE_1381_SKIP_LIVE_REASON='No production rollout performed for this implementation PR; production code/version and bounded execution acceptance remain unverified.'
make verify-issue-1381
```

| Check | Outcome |
|---|---|
| `make verify-issue-1390` | **PASS=15 FAIL=0 SKIP=1**, including tests and retests. |
| `make verify-issue-1381` | **PASS=13 FAIL=0 SKIP=1**, including full workspace contracts, full serial indexer library, four API suites, frontend funding/gas regressions, real migration and native flows. |
| Focused contract matrix | 10 integration cases plus legacy-context unit check, passed and retested. |
| Indexer | 473 library tests passed serially and retested; exact-output 13 and funded API 9 passed and retested, plus existing solve/hybrid regressions. |
| Frontend | Issue verifier: 246 tests across 14 files, passed twice. Funding verifier: 269 tests across 19 files, passed twice. |
| Static/build | Contract formatting, changed indexer formatting, frontend lint/types/production build, helper syntax and invariant crosslinks passed. Existing lint/compiler warnings remain visible. |
| Optimized Wasm | Pinned `cosmwasm/workspace-optimizer:0.16.1`, dedicated target volume; actual bytes stored and executed on LocalTerra. |

## Synthetic execution evidence

- [First funded run](localterra-first-funded.json): six successful/failure branches;
  [first guard run](localterra-first-guards.json): fifteen checks.
- [Final funded run](localterra-funded.json): six checks with fresh contracts;
  [final guard run](localterra-guards.json): sixteen checks, including actual
  purchased-intermediate residue cap rejection and atomic rollback.
- [Migration/funding regression](localterra-regression-funded.json): six funded
  checks and four migration checks. Reviewed pre-v1 artifact hashes are linked
  in [legacy provenance](legacy-fixture-provenance.json). Router/token upgrades
  preserve configuration/balances; factory pair pin refresh preserves reserves.
- [Native stack bootstrap](localterra-wrap-bootstrap.json) and
  [invoice/native execution](localterra-regression-flows.json): exact invoice
  success and rollback, native wrap success and rollback, and native unwrap.

The first and final fixtures belong to separate ephemeral LocalTerra instances;
their transaction hashes are test receipts, not production or public-chain
receipts. Router/token version is 1.2.1; funding protocol remains version 1.
Deployment still requires the [coordinated rollout](../../runbooks/route-local-tax-1381.md).

## Resolved test/setup failures and evidence limits

An early ordinary same-pair cycle control assumed equality with a static quote.
Executed hops change reserves, so that control now reconciles actual output
events, minimum receive and dust instead. A parallel library run encountered an
existing negative-cache fixture collision; serial runs pass. Shared chain stalls,
setup sequence rejection and a terminated ephemeral node were resolved using
dedicated infrastructure. Failed/unfinished attempts were superseded by fresh
successful verification, rather than counted as PASS.

A standalone Rust 1.99 Wasm diagnostic needed explicit linker permission for
CosmWasm imports; its corrected build passed. The chain used the pinned optimizer
artifacts. An offline optimizer recheck lacked its ephemeral Git checkout cache;
the final build uses a dedicated Git cache. None of these setup observations is
deployment acceptance, anti-Sybil protection or third-party audit certification.

Agents may skip unavailable checks while recording overall **PASS with explicit
skips**, following [the checklist](README.md). A failed executed assertion stays
FAIL until fixed and retested. Here the sole final skip is production acceptance.

Public transaction hashes use lowercase hexadecimal, preserving their bytes.
Source/artifact digests use separate filename and SHA-256 fields so secret
scanning does not mistake public token-named hashes for API credentials. No
secret-scan rule or allowlist was weakened.

Pre-commit Clippy caught two redundant comparison references; they were removed
and Clippy passed. Focused contract tests passed and retested after that cleanup.
The final optimized rebuild has identical router/token/launcher hashes to the
executed fixture bytes. Secret scanning passed on the staged patch.
