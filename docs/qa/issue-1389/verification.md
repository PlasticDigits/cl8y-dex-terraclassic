# Implementation verification receipt

Issue [#1389](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1389).
[Agent checklist and skip policy](README.md),
[invariants](../../route-solver.md#reverse-query-availability-1389),
[source/executable hashes](test-receipt.json).

Scope: implementation working tree based on
`9944ada10ee711ff8ffaa91f4c19b0f292de3924`, verified on **2026-10-05 UTC**.
The receipt records tested source hashes rather than claiming the unchanged base
revision contains the fix. Pair cw2 is **1.20.0**, router cw2 **1.3.0**.
Overall implementation review: **PASS with explicit skips**. No production
deployment, migration acceptance, funded trade or measured query gas is claimed.

## Completed issue criteria

- [x] **AC1:** Charge-before-call, cached greedy probes, exact-version pool cost,
  opaque 32-call reservations and reserved fresh validation bound every supported
  branch to 33 simulations/hop and 96/request. Counters exercise all 81 four-hop
  mode combinations, both directions, failed queries, legacy/missing/malformed/
  future identities, zero/dust/large/Uint128 targets and oversize routes.
- [x] **AC2:** Large pool/declared and pure-book plateau fixtures validate
  sufficient output and insufficient predecessor within the existing 32 calls.
  Real matcher fixtures cover both canonical asset orderings, 6/18 and 18/6,
  active/exhausted/expired/maker-capped books and missing/wrong-side hints.
  Unprovable/impossible targets terminate with stable errors. Supported cases
  and conservative rejection limits are documented; unrestricted hybrid
  feasibility or a universal gas ceiling is not claimed.
- [x] **AC3:** Six admission tests demonstrate four distinct peers, repeated
  monopolization rejection, bounded state, 20 cancellations, timeout/error/
  success recovery, unpolled rejected work, missing-peer bucketing and spoofed
  forwarding/mapped-IPv6/socket-port handling. Observe invalid horizons error
  without panic. Full exact-output unit/API regressions pass.
- [x] **AC4:** Connected refetch failures for budget/429/504 preserve Receive,
  clear executable Pay, disable signing and stop retries; disconnected failures
  remain finite. Existing route/fee/tax/spread/minimum guards pass the workspace
  and frontend suites. No execute-message or signing-protection relaxation.

## Commands and results

| Check | Actual command / execution | Result |
| --- | --- | --- |
| Router | `cargo test --offline --manifest-path smartcontracts/Cargo.toml -p cl8y-dex-router`, then preserved test executable `--test-threads=1` | 12 passed twice; strict all-target Clippy passed |
| Pair | `cargo test --offline --manifest-path smartcontracts/Cargo.toml -p cl8y-dex-pair --lib`, then `issue1389` focused retest | 96 passed; five focused tests passed; strict all-target Clippy passed |
| Full contracts | `make test-contracts`/workspace compilation completed, but Cargo post-build bookkeeping stalled; final test executables copied to tmpfs and each run with `--test-threads=1` | **888 passed** across 15 crates; both `issue1376`/`issue1389` focused retest scopes passed, 24 total |
| Exact-output unit | `cargo test --offline --lib route_exact_output -- --test-threads=1`; final formatted source rebuilt directly with `rustc --test` to tmpfs and retested | 19 passed twice on final source at **07:52:24Z** |
| API | `python3 /dev/shm/issue1389-api-build/build.py`, then fresh `api_route_exact_output --test-threads=1` against isolated native PostgreSQL and wiremock | 12 passed twice; no public LCD |
| Admission | Fresh `rustc --test` harness importing the production module with `#[path]`, then `--test-threads=1` | Six passed twice on final formatted source |
| Frontend | `bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- src/pages/SwapPage.queryParams.test.tsx src/utils/cw20RouteSolveQuote.test.ts src/services/terraclassic/router.reverse.test.ts src/services/indexer/__tests__/client.test.ts` | 134 passed twice; verifier repeats also passed |
| #1389 verifier | `make verify-issue-1389` with equivalent direct-execution command overrides below | PASS contracts/API/frontend/docs twice; gas SKIP |
| #1376 verifier | `make verify-issue-1376` with explicit normal-wrapper/browser/chain skip reasons | PASS frontend 199 tests twice, docs, lint/types; normal Cargo invocations covered separately above; browser/chain/live unverified |
| #1381 verifier | `make verify-issue-1381` with explicit unavailable Cargo/chain/live scope | Frontend 268 passed twice, docs, contract formatting, changed-indexer formatting, frontend lint and production types/build passed after correction |
| Static | `make lint-contracts`, `make lint-frontend`, frontend `tsc -b`, `python3 scripts/qa/check-issue-1381-format.py`, `python3 scripts/qa/check-issue-1389-docs.py`, `git diff --check` | PASS; frontend retains five existing warnings, zero errors |
| Optimized Wasm | `make build-optimized` / `cosmwasm/workspace-optimizer:0.16.1` | Completed; pair/router hashes recorded in receipt and tracked checksums |
| Test probe | QA README's locked MVP/allow-undefined build command | PASS; test-only Wasm compiled, not VM-executed |

The normal Cargo API invocation blocked in kernel `wait_on_buffer`; the fresh
tmpfs fallback compiled the **actual final production library**, not an old
cached indexer library. Its coherent dependency fingerprint IDs came from the
successful unit build. Exact argv are preserved in the workspace's
`/dev/shm/issue1389-api-build/{lib,test,unit}-command.json`; the receipt publishes
source/library/executable hashes without credentials or workstation identities.
The standard Cargo commands remain the portable reproduction path.

For the verifier run, contracts used
`VERIFY_ISSUE_1389_CONTRACTS_COMMAND='python3 /tmp/issue1389-direct-contracts.py'`.
That runner checks hashes from the preserved workspace manifest and executes
each final test binary with `issue1389 --test-threads=1`. Indexer used
`VERIFY_ISSUE_1389_INDEXER_COMMAND='/dev/shm/issue1389-indexer-lib-tests route_exact_output --test-threads=1 && cd indexer && /dev/shm/issue1389-api-build/api_route_exact_output --test-threads=1'`.
Both execute assertions and propagate failures. Rebuild replacements from the
current source before reusing this method; ephemeral paths are not release
artifacts or a substitute for provenance.

The first #1381 verifier returned **FAIL** for the new module's Rust 2021
formatting. Applying repository-required Rust 2024 formatting corrected it;
the formatting check and complete selected verifier checks were rerun. Fresh
post-format API/unit/admission rebuilds all passed. No failed assertion was
converted to a skip or a pass.

## Explicit unavailable scope

- [ ] **Gas / LocalTerra execution — SKIP:** `make has-localterra` initially
  reported a running chain. Optimized artifacts and the test-only query probe
  were built. Two `localterra-reverse-1389.py` setup attempts failed with rejected
  account-sequence mismatches before any VM assertion. Read-only RPC diagnosis
  found height **838695** frozen since **2026-10-05T06:49:55.135267141Z**, one
  pending transaction and `catching_up=false`. Independent test signers and
  bounded rejected-bootstrap retry support are prepared, but cannot advance a
  frozen shared chain. No shared volumes/state were reset. Run the QA harness
  twice after infrastructure recovers; its arbitrary-state gas coverage remains
  limited to the measured fixtures.
- [ ] **Browser replay — SKIP:** the dedicated mocked-LCD Playwright attempt on
  port 3039 failed navigation setup at 120 seconds, before application
  assertions. That failed attempt is retained as a setup failure, never a
  passing browser test. Vite cold navigation under the same host I/O contention
  could not load; the selected #1376 verifier explicitly skips this unavailable
  replay. RTL/service results above independently establish the tested UI guards.
- [ ] **Remaining broad indexer #1381 suites — SKIP:** shared-target I/O blocked
  the normal broader Cargo invocation. The changed exact-output unit/API suites
  are verified separately; unchanged tax/discovery integration suites are not
  claimed as executed by this task.
- [ ] **Production — SKIP:** no deployment, pair/factory migration inventory,
  proxy configuration acceptance, live signing or public load test. Existing
  production pair rollout belongs to
  [#1386](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1386).

Agents may complete the independently available review while leaving these
rows unchecked and recording **PASS with explicit skips**. Production/runtime
protections remain enforced. Followups are agent-owned infrastructure recovery
and these specific replays; no verification work is assigned to the human.
