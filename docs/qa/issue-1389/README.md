# Reverse-query availability review (#1389)

Implementation: [PR #1407](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1407).

Issue [#1389](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1389).
Invariants: [RQ1389-1–RQ1389-8](../../route-solver.md#reverse-query-availability-1389).
Third-party agent protocol: [AGENTS_REVERSE_QUERY_BUDGET.md](../../../skills/AGENTS_REVERSE_QUERY_BUDGET.md).
Execution results: [verification.md](verification.md).
Prior receive protections: [#1376 checklist](../issue-1376/README.md).
Implementation: [router metering](../../../smartcontracts/contracts/router/src/reverse_query.rs),
[pair inverse/search](../../../smartcontracts/contracts/pair/src/hybrid_reverse.rs),
[API admission](../../../indexer/src/api/route_exact_output_admission.rs),
[gas harness](../../../scripts/qa/localterra-reverse-1389.py).

Agents own this checklist. Record revision, UTC, exact commands, exit status and
scope. Run every available check first; fix and retest failures. Agents may mark
overall implementation review **PASS with explicit skips** when every completed
check passes and unavailable checks have a precise reason. Mark skipped rows
**SKIP**, never checked off as completed; executed assertions remain **FAIL**
until fixed and rerun. Unavailable holds, gates, signing, deployment or device
work do not block opening/reviewing the PR. Runtime protections still apply.
No human verification assignment is required. Production acceptance and skipped
gas/load evidence cannot be inferred from this review status.

If Cargo's post-build bookkeeping is unavailable but the exact final-source
executables were built, the verifier accepts `VERIFY_ISSUE_1389_CONTRACTS_COMMAND`
and `VERIFY_ISSUE_1389_INDEXER_COMMAND` for equivalent direct execution. Record
the actual commands, source/executable hashes, compiler arguments and asserted
test counts; never substitute stale binaries or an empty success command.
The standard Cargo path remains the default. Executable failure still makes
the verifier fail, including in this alternate mode.

1. Read the issue and all comments through `bao-exec --only FORGEJO_PLASTICDIGTS_PAT -- fj issue view 1389` and `... fj issue view 1389 comments`. Review the final diff and invariants, including conservative reservation and proxy/NAT restrictions.
2. Provision Node 24 dependencies and an isolated Postgres test database (see [local Postgres](../../../skills/AGENTS_LOCAL_POSTGRES_DEV.md)). Run `make verify-issue-1389` and `make verify-issue-1376`. The new verifier runs contracts, API/DB and frontend checks twice. Record skip reason variables only for actual unavailability: `VERIFY_ISSUE_1389_SKIP_CONTRACTS_REASON`, `..._INDEXER_REASON`, `..._FRONTEND_REASON`, `..._CHAIN_REASON`.
3. Run `make test-contracts`, `make lint-contracts`, `make lint-frontend`, frontend `tsc -b`, and affected admission/API lint. Inspect fixture assertions instead of assuming a test name proves the entire matrix. Use mocks for abuse/load; never stress public LCD.
4. For chain gas, run `make has-localterra`; provision `make setup-cloud-localterra` when needed. Build final optimized artifacts using `make build-optimized`. Build the independent **test-only** query-probe with the command below, then run the isolated harness twice. Do not replace shared pools/contracts or reset shared volumes. `--code-cache` reuses only checksum-verified stores, and each run creates new synthetic pools/books.
5. Attach actual evidence and dispositions to the PR. Pair cw2 1.20.0/router 1.3.1 require migration for direct-LCD production protection. Production inventory/rollout is separate from this task; no public stress, real wallet activity, or production migration is part of the synthetic verifier.

```bash
RUSTFLAGS='-C link-arg=--allow-undefined -C target-cpu=mvp' cargo +1.81.0 build \
  --manifest-path scripts/qa/fixtures/reverse-query-probe/Cargo.toml \
  --release --target wasm32-unknown-unknown --locked
python3 scripts/qa/localterra-reverse-1389.py \
  --container cl8y-dex-issue1428-matrix --router-version 1.5.0 \
  --expect-four-hop-greedy-rejection \
  --code-cache /tmp/issue1389-code-cache.json \
  --evidence-file /tmp/issue1389-gas-first.json
python3 scripts/qa/localterra-reverse-1389.py \
  --container cl8y-dex-issue1428-matrix --router-version 1.5.0 \
  --expect-four-hop-greedy-rejection \
  --code-cache /tmp/issue1389-code-cache.json \
  --evidence-file /tmp/issue1389-gas-retest.json
```

For current artifacts, provision the dedicated pinned 15M node described in
[#1428 QA](../issue-1428/README.md) before these commands; do not raise a shared
provider limit. #1428 supersedes the earlier four-hop greedy support claim.

Each run funds a unique LocalTerra test signer, avoiding shared-account nonce
contention after bootstrap. Only a rejected bootstrap sequence mismatch may be
retried, at most five times; accepted transactions are never resent.

The probe invokes a smart query inside a transaction and captures deterministic
query errors in an event. `gas_used` includes wrapper/transaction overhead and
therefore bounds that measured query, not every possible book state. It executes
no downstream messages and never changes the queried contracts. The harness
refuses chain IDs other than `localterra`; its 12M test envelope is not a dApp or
production gas recommendation. Keep its fixture Wasm out of release artifacts.

## Reusable acceptance and review checklist

- [ ] **AC1:** Inspect charge-before-call and reserved final-validation paths. Run `issue1389` counters for pool/greedy/declared, both orientations, zero/1/6/18/Uint128, failed calls, oversize and four-hop mixtures. Confirm ≤33 per hop and ≤96 per request, cached offers and opaque 32-call reservations. Include legacy1.18/missing/malformed/future cw2 identities: pool/greedy seeds reserve32 unless the exact known optimized version is verified. Inspect unchanged maker/scan caps.
- [ ] **AC2:** Run declared reverse tests for large empty/exhausted/active books, both orientations and integer plateaus. Supported successes must be sufficient, with insufficient predecessor where minimum is claimed. Impossible or unprovable targets must terminate with stable errors within 32 calls. Record the support limits; universal book feasibility is not promised.
- [ ] **AC3:** Run exact-output admission tests and real API tests: one noisy socket peer plus distinct peers, shared endpoint capacity, header spoofing, mapped IPv6, missing metadata, cancellation/timeout/error recovery and bounded retained state. Observe invalid horizons must error without panic. Review proxy/NAT deployment restriction.
- [ ] **AC4:** Run receive-state/service tests while disconnected and after previously executable connected quotes. Budget/429/504 errors preserve Receive, clear Pay and block cached operations/signing. Confirm no deterministic retry/fallback or wallet resend, and unchanged fair-reference, spread/minimum, tax, pause and balance protections.
- [ ] **Gas:** Run the isolated harness twice on final optimized pair/router artifacts; record hashes, versions, receipt gas and scope. Four-hop pool cases must remain sufficient. Any four-hop reverse containing greedy (empty or populated) now returns `GREEDY_REVERSE_ROUTE_UNSUPPORTED` before external queries; shorter admitted routes retain sufficiency/budget checks. Do not claim the synthetic samples cover every 500-scan book shape.
- [ ] **Regression:** Run both verifiers, contract workspace and lint/types; read docs and agent crosslinks. Record all PASS/FAIL/SKIP and any pre-existing unrelated failures honestly.
- [ ] **Production:** Separately verify deployed revisions, pair inventory/factory pointer, migrations and proxy peer configuration before claiming production acceptance. Skip with explicit reason when no deployment is performed.

Funded tax-route provider gas compatibility is additionally covered by
[#1422](../issue-1422/README.md) and [FRQ1422](../../../skills/AGENTS_FUNDED_REVERSE_QUERY_BUDGET.md).
Its unmodified 3M funded fixture is separate from these broader synthetic book
measurements and from production acceptance.

## Current gas-matrix limitation (#1422)

[The funded regression results](../issue-1422/RESULTS.md) qualify the six
pool-route fixtures at the actual 3M SDK53 limit on router 1.3.1. The separate
prior full-book four-hop greedy matrix fails at a finite 15M provider limit on
both router 1.3.0 and 1.3.1 before its expected work-budget discriminator. These
executed failures remain FAIL; later gas cases were not reached. Work-count
bounds and successful empty-book samples cannot establish populated-book
feasibility. Preserve the raw receipts, do not increase ceilings or accept a
failed assertion as a skip, and leave that support envelope unverified.

The router 1.5.0 support policy and integrated 1.4.0 reproduction are documented in [#1428](../issue-1428/README.md). Historical failures above remain FAIL; bounded deterministic admission supersedes the unsupported four-hop greedy shape.
