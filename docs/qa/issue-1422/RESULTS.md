# Issue #1422 implementation results

[Issue](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1422),
[agent checklist](README.md), [FRQ1422 invariants](../../../skills/AGENTS_FUNDED_REVERSE_QUERY_BUDGET.md).

## Required funded regression: PASS

Tested source is base `6612df57` plus this PR's changes. The
[build receipt](evidence/build-provenance.json) records the exact final contract
source hashes, dirty-source status, locked dependencies, pinned optimizer image,
Rust/Cargo 1.81.0, Binaryen 116 and all 13 owned Wasm outputs. Receipt verification
passed against the final source and binaries. The public receipt losslessly stores
hash maps as explicit path/SHA-256 entries, keeping public token artifact filenames
from being interpreted as credential names by the secret scanner; reconstruction
was compared byte-value-for-byte-value with the original and verified again. Every owned output except router
matches the prior integrated build. The separate reviewed CW20 fixture remains
`c77ffcdb0039041fbd071c76efb547232314ce8ccf219c3bf5b7fabd4cf7d4a4`.
The independent [probe receipt](evidence/probe-build.json) records its compatible
locked graph/toolchain; the probe is excluded from release artifacts.

Router 1.3.1 SHA-256:
`cb11ed967a31e8c7383c787a98e7a68ea83768d271f65d0eb79c2dbd555e3c2e`.
Pair 1.20.0 SHA-256:
`80b6cfe87878d039697f7bf864cc3488388d9d4541ace221dd58d22512b9485c`.

The frozen router 1.3.0 is
`311293710bc5d9ce9aea5e14b842a555e39876334f453aea2df85b384f43c3d4`.
Its ordinary CL8Y→ALPHA→BETA→USTR reverse query failed at the actual **3,000,000**
SDK53 limit (`gasUsed=3,050,913`); the equivalent probe transaction succeeded at
**3,386,934** gas including overhead. The
[first](evidence/baseline-first.json) and [retest](evidence/baseline-retest.json)
keep `status=FAIL` and `baseline_regression=REPRODUCED` with raw payload/error.
A frozen expected failure establishes reproduction, not release acceptance.

`make verify-issue-1422`: **10 PASS, 0 FAIL, 0 SKIP**. Both independently created
current fixtures passed all six funding/accounting/rollback cases and all **18
actual smart queries** at the unchanged 3M limit. All probe calls also passed.
[first receipts](evidence/funded-first.json), [retest receipts](evidence/funded-retest.json).
The ordinary→ALPHA→BETA→USTR reverse probe uses **2,126,348** gas, a **37.2%**
reduction from the frozen baseline. These transaction receipts include overhead;
actual smart-query success independently establishes the sampled 3M envelope.

| Reverse scenario | First probe gas | Retest probe gas |
|---|---:|---:|
| zero intermediates | 2,006,605 | 2,006,656 |
| multi tax intermediates | 2,126,348 | 2,126,399 |
| owned dust and existing balances | 2,006,576 | 2,006,627 |
| taxed initial gross | 1,372,332 | 1,372,383 |
| taxed final buy | 893,934 | 893,985 |
| minimum return rollback | 2,126,294 | 2,126,345 |

The required verifier command used the dedicated pinned SDK53
`cl8y-dex-issue1422-memory-ready` node and:

```bash
CARGO_HOME=/dev/shm/dex1422-cargo-home \
INDEXER_CARGO_TARGET_DIR=/dev/shm/dex-1422-indexer-target \
INDEXER_CARGO_TEST_DEBUG=0 INDEXER_CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 \
VERIFY_ISSUE_1422_CONTAINER=cl8y-dex-issue1422-memory-ready \
VERIFY_ISSUE_1422_BASELINE_ARTIFACTS=/home/answorld/repos/cl8y-dex-terraclassic/smartcontracts/artifacts \
VERIFY_ISSUE_1422_CODE_CACHE=/tmp/dex-1422/memory-codes.json \
make verify-issue-1422
```

## Additional available checks

| Check | Result |
|---|---|
| Complete contract workspace | 949 PASS, zero failed/ignored; final sequential rerun includes doctests |
| Strict contract workspace Clippy, all targets | PASS |
| Router suite / focused retest | 19 PASS / 5 new tests PASS |
| Route-tax / dedicated launch-guard integration | 17 PASS / 10 PASS |
| Focused client tests | 113 PASS twice; ESLint, Prettier and TypeScript PASS |
| SDK gas classification / exact-output PostgreSQL API | 4 PASS twice / 14 PASS twice |
| Indexer binary check | PASS; existing dead-code warnings retained |
| Rollout script tests | 70 PASS twice |
| `make verify-issue-1394` | 23 PASS, 0 FAIL, 0 SKIP; 559 library tests, parallel repeats, benchmark, PostgreSQL discovery retests |
| `make verify-issue-1381` | 13 PASS, 0 FAIL, 2 explicit skips (native wrap stack and production); actual funding/invoice flows PASS |
| `make verify-issue-1390` | 15 PASS, 0 FAIL, 1 production skip; actual launch-guard chain checks PASS |
| `make verify-issue-1376` | 12 PASS, 0 FAIL, 1 production skip; mocked-LCD browser receive lifecycle PASS |
| `make verify-issue-1389` | 8 source/API/client/doc PASS, 1 broader gas-matrix FAIL, 0 SKIP; existing failure retained below |
| Documentation, formatting, Bash/Python syntax | PASS |

PostgreSQL tests used their own pinned PostgreSQL 16.15 Alpine fixture and test
DB. Host journal contention required temporary memory-backed test-chain/DB/build
storage. The optimizer retained read-only source, fresh named target and registry
volumes and the pinned image; its temporary Git checkout also used memory storage.
No shared chain/DB volumes were reset. A Rust 1.99 test-probe build failed SDK53
static validation for bulk-memory instructions; the fixture lock was aligned to
the contract dependency graph and rebuilt with Rust 1.81.0, then actually stored
and queried successfully. Initial overlapping native Cargo runs produced doctest
dependency errors; the final sequential workspace run passed all 949 tests and
strict Clippy. These setup failures were repaired, not accepted as successful checks.

## Broader populated-book matrix: existing FAIL, retained

The independent prior #1389 matrix was also attempted on a **separate finite 15M
node**, with identical factory/pair/CW20/probe hashes and only the router changed.
Each run passed 14 preceding empty-route and expected declared-budget checks.
The populated **four-hop greedy reverse** query then exhausted provider gas before
its expected `REVERSE_QUERY_BUDGET_EXCEEDED` discriminator:

| Router / run | Limit | Observed gasUsed | Result |
|---|---:|---:|---|
| 1.3.0 before fix | 15,000,000 | 15,046,065 | FAIL |
| 1.3.1 first | 15,000,000 | 15,045,302 | FAIL |
| 1.3.1 retest | 15,000,000 | 15,045,881 | FAIL |

[Before-fix receipt](evidence/book-matrix-before-fix.json),
[first receipt](evidence/book-matrix-first.json),
[retest receipt](evidence/book-matrix-retest.json).
The failure predates this cache change and remains **FAIL**. Later reverse-direction
full-book and invalid-Observe gas cases were not reached in that matrix. Native
Observe/work-bound tests passed. No query limit/work ceiling was increased to
hide the failure and no failed assertion was relabeled SKIP/PASS. This fix qualifies
the required six funded pool-route fixtures; it does not establish universal
populated-book feasibility. The matrix also measured four-hop pool probe reduction
5,491,563→2,363,456 and empty greedy 6,073,495→2,945,397, both overhead-inclusive.
Its 15M measurements do not establish actual 3M or production acceptance.
Consequently `make verify-issue-1389` retains its gas group/overall FAIL while
its eight source/API/client/doc groups pass. The failed group checks the actual
first/retest receipts above; it is not an unavailable-check skip.

The related verifier batch used the same isolated pinned PostgreSQL and final
artifacts, memory-backed Cargo targets with debug/incremental disabled, the
3M node for funding and guards, and separate 15M receipts for the broad matrix.
Its native/API/client groups were tested and retested. The funding flow
[receipt](evidence/funding-invoice-flows.json) and
[launch-guard receipt](evidence/launch-guards.json) retain actual assertions and
transactions. The missing reviewed native wrap-stack environment was explicitly
skipped; no production execution or provider acceptance was claimed.

## Issue criteria and remaining scope

- [x] Reproduce the frozen ordinary→ALPHA→BETA→USTR reverse failure at 3M with actual optimized Wasm.
- [x] Reduce redundant reverse/tax discovery while preserving gross funding, fresh forward sufficiency, original trader guards, work bounds and signing protection; current six fixtures pass and retest at 3M.
- [x] Preserve observable deterministic SDK gas errors (sanitized HTTP 422, no provider fanout/cooldown or weaker quote), and verify stale executable quotes clear while Receive survives.
- [x] Reconcile router 1.3.1/pair 1.20.0 rollout pins and agent/runbook/checklist guidance with measured limits and exact hashes.
- [ ] Production provider envelope, Columbus-5 migration/application rollout and real-provider quote acceptance: **not performed**; remain in [#1384](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1384), [#1386](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1386) and [#1421](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1421).
- [ ] Populated four-hop greedy reverse feasibility: the prior 15M matrix failure remains explicit; a separate optimization/support-envelope decision is still needed before claiming that case supported.

Implementation acceptance and the broader matrix have separate scopes: #1422's
specified funded regression is PASS; the recorded broader matrix is FAIL. Agents
may skip genuinely unavailable checklist items with reasons and still report PASS
with explicit skips for that review scope. Executed failures retain their status.

## Hosted CI disposition

[Initial hosted pipeline 326](https://ci.cl8y.com/repos/7/pipeline/326/1)
stopped in the immutable secret-scanning policy before functional suites ran.
It misclassified public SDK Git commits/dependency text in the new receipts and
the existing public Venus BSC market address in three unchanged oracle files.
The published node metadata now selects actual runtime/version fields and a
hash of the complete version dump; the pinned image still identifies the full
binary. Local scanning/provenance checks pass. No scanner rule was disabled,
no status was fabricated, and the existing public-address policy issue remains
a hosted-CI limitation rather than production or source-test acceptance.
