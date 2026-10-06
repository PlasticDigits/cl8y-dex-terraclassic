# Issue #1398: agent review and verification

[#1398](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1398) addresses
post-hook receive protection, explicit hook pricing, bounded registration and
atomic AutoLP provision. Read invariants **NE1398-1–NE1398-10** in
[the agent playbook](../../../skills/AGENTS_NET_EXECUTION_FLOORS.md) and
[the integrator contract](../../integrators.md#net-execution-floors-and-hook-pricing-1398).

Implementation versions are pair **1.21.0**, factory **1.12.0**, router
**1.5.0** and community-tax AutoLP **1.1.0**. Source tests exercise these versions; deployment acceptance
must verify the actually deployed code IDs and versions separately. Router
1.4.0 → 1.5.0 migration preserves route funding/settlement version 1 and existing
configuration/nonce state; [#1428](../issue-1428/README.md) adds bounded greedy reverse admission; the existing historical route-tax rollout plan still
targets its explicit 1.3.0 artifact and is not a deployment plan for this PR.

## Reproduce verification

```bash
make verify-issue-1398
```

The [script](../../../scripts/verify-issue-1398.sh) runs common hook tests,
pair/factory admission tests, router reverse-budget regressions, AutoLP and issue
integration tests twice; frontend
quote, route preflight and Swap/Trade submit tests also run twice. Then run the complete
contract workspace, Rust formatting and documentation crosslinks. It exits
nonzero on executed failure. Optional `CONTRACTS_CARGO_HOME` and
`CONTRACTS_CARGO_TARGET_DIR` select an existing compatible dependency/build cache.
Run from this checkout/worktree so the tested source is the reviewed source.

For each row below, record PASS, FAIL or SKIP, its exact command and evidence.
Check off only verified rows. Review the actual balance/reserve/escrow assertions
and error cause, rather than counting a generic transaction rejection as evidence.
Retest after fixes. Save revision, UTC time, test counts and source evidence scope.

## Issue criteria and review checklist

- [x] **AC1 — net protection:** Direct, declared hybrid, greedy and router hooked
  execution reject absent/zero `min_return` and a floor exceeding post-hook ask
  output. A valid floor succeeds; L9 pool/hybrid bounds and belief validation
  remain intact. Router final `minimum_receive` cannot bypass a per-hop net floor.
  Swap/Trade direct pool and hybrid signing derive a fresh positive post-hook
  floor; Swap reverse signing keeps the stronger target/fresh floor. Zero/dust
  receive blocks signing.
- [x] **AC2 — quote/execute parity:** Forward quotes apply matching ask-token
  tax/burn config and per-hook rounding exactly once. Leg and L7 commission
  meanings stay stable. Unknown/malformed/unqueryable pricing fails explicitly.
  Reverse matching nonzero fee-hook pricing fails explicitly instead of returning
  a gross sufficiency claim; supported hook-free reverse regressions pass.
  Router recognizes reviewed pair cw2 1.21.0 for its established reverse-probe
  accounting; unknown future versions retain the conservative budget.
- [x] **AC3 — bounded admission and recovery:** All list-setting paths reject
  ninth and duplicate validated hooks before state/message mutation. Exactly
  eight and empty lists succeed in caller order. Invalid legacy lists fail before
  config queries/callbacks; authorized full replacement restores bounded state.
  Verify no implicit truncation/deduplication or governance-gate bypass.
- [x] **AC4 — AutoLP rollback:** Missing/stale manager-approved LP economics,
  insufficient minted LP and failed provide ratio/asset checks revert swap/provide.
  Snapshot AutoLP, pair, quote and LP recipient balances around failure; all
  changes roll back. Permissionless skim never refreshes mutable spot policy,
  and the lock remains held through provide completion. Successful skims pass.
  For migration, query sister `GetConfig.manager` and `GetLpPolicy`; use the stored
  manager for explicit same-pair `UpdateConfig`. Current launcher uses a human
  manager. Unchanged token settings are a no-op; record token-owned legacy proxy
  recovery separately instead of claiming a human can impersonate that contract.
- [x] Run the source verifier, resolve executed failures and complete its focused
  retest. Inspect public regression coverage for pause, blacklist, frozen code-id,
  route funding and transaction rollback in the full contract workspace.
- [ ] If available, build optimized wasm, deploy/migrate an isolated LocalTerra
  stack and reproduce synthetic net-floor, replacement and AutoLP-rollback scenarios.
  Provision with `make setup-cloud-localterra` before declaring local chain
  unavailable. Record SKIP with an observed reason when unavailable.
- [ ] Separately record deployed code IDs, migration receipts and version-specific
  checks before production acceptance. Source verification is not deployment.

## Explicit skip policy

Agents may skip infrastructure, signing, deployment, device or other checks they
cannot complete and still mark overall verification **PASS WITH SKIPS**, provided
all executed checks pass. Complete runnable work first. List each unavailable
item and the observed reason; leave its checkbox unchecked. A failed executed
check remains **FAIL** until corrected and retested; do not erase it by skipping
a later run. Omitted checks do not create an implementation hold or certify
production deployment.

Automated unavailable groups accept explicit reasons through
`VERIFY_ISSUE_1398_SKIP_CONTRACTS_REASON`,
`VERIFY_ISSUE_1398_SKIP_FULL_CONTRACTS_REASON`,
`VERIFY_ISSUE_1398_SKIP_FRONTEND_REASON`,
`VERIFY_ISSUE_1398_SKIP_FORMAT_REASON`, or
`VERIFY_ISSUE_1398_SKIP_DOCS_REASON`. For example, an unavailable Cargo toolchain
may use `VERIFY_ISSUE_1398_SKIP_CONTRACTS_REASON='Cargo unavailable: <observed error>'`.
These controls permit honest skips, never relabeling executed failures as passes.

## Evidence receipt

Verified on 2026-10-06 at **04:42 UTC**, against base
`6612df5704abcdc129aa7492113ed0b3b1641b4c` plus this PR's implementation in branch
`fix/issue-1398-net-floors`. The checked AC1–AC4 rows above are source/multitest
criteria; the unchecked chain and production rows remain unverified.

Canonical reproduction from the worktree root:

```bash
make verify-issue-1398
cargo clippy --manifest-path smartcontracts/Cargo.toml --workspace --all-targets -- -D warnings
bash scripts/with-node.sh --cwd frontend-dapp -- npm run build
bash scripts/with-node.sh --cwd frontend-dapp -- npm run lint
```

Optional dependency/build-cache overrides are documented above; they do not
change the source under test.

| Check | Outcome |
| --- | --- |
| Focused contracts, first run and retest | PASS each: 12 common hook, 2 factory admission, 1 pair admission/recovery, 11 router reverse-budget, 29 AutoLP and 6 integration tests |
| Net-flow integration matrix | PASS: pool/book/mixed in both directions; zero, single, multiple and full-rate hooks; direct/router missing/zero/exceeding/exact floors; taxed three-hop and actual greedy-maker execution; rollback |
| Frontend quotes and signing, first run and retest | PASS: 8 files, 167 tests each; direct pool/mixed/dust floors, route quotes, Swap and Trade submit behavior |
| Full contract workspace | PASS: 977 tests, including 567 integration tests |
| Rust formatting and documentation crosslinks | PASS |
| Automated source verifier | **7 PASS, 0 FAIL, 0 SKIP; overall PASS (source scope)** |
| Strict workspace Clippy, all targets | PASS with `-D warnings` |
| Frontend production build and lint | PASS; lint has 0 errors and 5 existing warnings outside this change |
| Skip/failure accounting probes | PASS: unavailable groups permit PASS WITH SKIPS; injected executed failure remains FAIL/nonzero |
| Independent implementation review | PASS: settlement/raw reverse, bounded replacement, AutoLP policy/mint/lock and fresh net signing |
| Isolated optimized-wasm/chain/max-eight-hook gas | SKIP after bounded provisioning attempts, described below |
| Production migration/signing and token-owned legacy policy refresh | SKIP: outside source PR; actual manager authority and deployed code must be verified separately |

Initial workspace runs exposed token mock `PoolResponse` omissions, callback
fixtures missing their now-required signed net floors, and an outdated reviewed
pair-version classification in router reverse accounting. They were corrected,
then the focused tests and full workspace passed. The related reverse-query
frontend regression initially lacked a fresh-floor quote mock; its clean rerun
passed. Interrupted verifier runs were discarded and the immutable final script
completed. No executed failure was relabeled as SKIP.

Related `make verify-issue-1389` completed **6 PASS, 0 FAIL, 3 SKIP** and
`make verify-issue-1390` completed **12 PASS, 0 FAIL, 4 SKIP**. Their skips retain
the same infrastructure/deployment limitations, and are not production receipts.
Postgres provisioning was attempted with `make setup-indexer-postgres` (60-second
timeout); a fresh isolated fallback remained `Created` after 30 seconds and was
removed. Indexer source is unchanged by this PR. Record any further related
verification receipts separately rather than assuming omitted groups passed.

Overall agent review is **PASS WITH SKIPS** for the source implementation and
the explicitly unavailable rollout scope. Production deployment was not performed.

### Unavailable isolated-chain evidence

On 2026-10-06 at 04:18–04:28 UTC, an isolated provisioning attempt used the
repository's pinned LocalTerra image, a dedicated
`cl8y-dex-issue1398-localterra-data` volume and host ports 27657/1417. The chain
initialized, but its RPC repeatedly returned connection refused/reset for more
than six minutes after Docker reported the container running. In-container RPC
also failed. Docker `inspect`, `kill` and `rm` operations timed out at 15–30
seconds while other host workloads were active (observed load approximately
48–61). No shared-daemon restart or volume reset was performed.

`CARGO_BUILD_JOBS=2 make build-optimized` was attempted concurrently. A captured
input receipt detected source changes during the active implementation; this
obsolete build was canceled (exit 137) before an artifact was accepted. This
interruption is not a compiler/test failure or a valid final-source build receipt.
Final optimized-wasm deployment, maximum-eight-hook on-chain gas measurement,
and isolated-chain AutoLP execution are **SKIP: Docker/chain unavailable after
bounded provisioning attempts**. Unit/multitest behavior and query-count bounds
are separate source evidence. The isolated containers and optimizer volumes
were subsequently removed; no production deployment was performed.
