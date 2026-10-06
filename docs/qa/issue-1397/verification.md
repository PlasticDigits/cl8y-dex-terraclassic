# Issue #1397 verification receipt

[Review checklist and explicit skip policy](README.md).
[Invariants E1397](../../../skills/AGENTS_EVIDENCE_EXPORT_PRIVACY.md).

Verified 2026-10-06 04:37:29 UTC on branch `fix/issue-1397-evidence-privacy`, based on
`6612df5704abcdc129aa7492113ed0b3b1641b4c` (the implementation was tested as working-tree changes before commit).
The published PR/commit identifies the final revision. SHA-256 of the sorted
relative filename, NUL, file content, NUL for the seven files listed below:
`114846f008eddc1b6622c1dde487cf07d66e395d59e02bd87131b1a2ff9cd726`.
Files: `indexer/src/api/evidence.rs`, `indexer/src/api/mod.rs`,
`indexer/src/db/queries/evidence_daily.rs`,
`indexer/migrations/20261006133000_evidence_daily_ordered_indexes.sql`,
`indexer/tests/api_evidence_daily.rs`, `indexer/tests/evidence_daily_budget.rs`,
`indexer/Cargo.lock`.

## Executed checks

| Command / check | Outcome |
| --- | --- |
| `INDEXER_CARGO_TARGET_DIR=/home/answorld/repos/cl8y-dex-terraclassic/indexer/target make verify-issue-1397` | PASS; evidence-filtered library 5/5 twice, API 14/14 twice, DB budget 6/6 twice; shell syntax, crosslinks and diff checks passed. No focused skips. |
| `CARGO_TARGET_DIR=/home/answorld/repos/cl8y-dex-terraclassic/indexer/target cargo test --lib evidence -- --nocapture` from `indexer/` | Initial 5/5 PASS; middleware deadline tested with paused Tokio time. |
| `CARGO_TARGET_DIR=/home/answorld/repos/cl8y-dex-terraclassic/indexer/target make verify-issue-1205` | PASS, 8/8 accounting/structure checks including 14/14 API tests. |
| `./scripts/ci/gitleaks-scan-tracked.sh --exit-code 1` | PASS; no leaks found across all staged/tracked files. |
| Rebuilt API executable `api_evidence_daily-4e03048e2654e4ff --test-threads=1 --nocapture` from `indexer/` | Independent agent run/retest 14/14 both times. |
| Rebuilt DB executable `evidence_daily_budget-48a1e8e66f31fe3b --test-threads=1 --nocapture` from `indexer/` | Independent agent run/retest 6/6 both times; each measured 24 query-plan scenarios. |
| `CARGO_TARGET_DIR=/home/answorld/repos/cl8y-dex-terraclassic/indexer/target cargo test --test security -j 2 -- --test-threads=1` from `indexer/`, then `EXPECTED_CHAIN_ID=columbus-5 /home/answorld/repos/cl8y-dex-terraclassic/indexer/target/debug/deps/security-30e581edb9ec9b8d --test-threads=1` | Final 42/42 PASS, including socket-peer rate limits, spoofed headers and sanitized errors. The first completed run had 40/42 because two config tests lacked the required test chain variable; the corrected environment was retested in full. |

Cargo-built executables were reused for repeat runs. Shared Cargo build caching
was serialized by Cargo's lock. Library/code warnings were existing unrelated
warnings; no failing assertions remain.

## Environment and measured scope

PostgreSQL 16 ran in an exclusive disposable container `dex-1397-postgres-memory`
on loopback port 15449, with a 1 GiB tmpfs data directory, `fsync=off` and
`synchronous_commit=off`. The worktree used an isolated database and
`TEST_DB_LOCK_FILE=/tmp/cl8y-dex-1397-memory-test.seed.lock`. This durability
configuration applies only to synthetic tests, not production. Initial disk-backed
fixture setup stalled on filesystem sync/extension operations; those interrupted
attempts were replaced by the responsive disposable fixture and fully rerun.
No production activity, stress, migration, credentials or addresses were used.

Dense fixtures contain 20,000 events for each of the six source types. Each DB
run measures dense/empty dates with initial and late cursors: 24 `EXPLAIN
(ANALYZE, BUFFERS)` scenarios, without disabling sequential scans or forcing
indexes. Corrected run: 0.030–0.173 ms; repeat: 0.028–0.262 ms. Actual source
rows read: 0–101; shared blocks: 2–5. Empty initial pages use the new UTC-day
indexes. PostgreSQL can choose an equally efficient existing ordered index for
late dense pages. These are synthetic in-memory measurements, not production
latency/scale guarantees; runtime SQL limits remain the enforceable bound.

Initial plan tests incorrectly demanded a particular index name even when the
optimizer read 101 rows in 0.143 ms using another ordered index. Initial lock
observers searched text beyond PostgreSQL's 1024-byte activity-query truncation.
Assertions now verify bounded indexed reads and the actual query prefix. Both
changes were rebuilt and retested; executed failures were not converted to skips.

Actual PostgreSQL failure codes `55P03` (lock timeout) and `57014` (statement
timeout), acquisition exhaustion, an aborted executing SQL future, and reuse of
a one-connection pool all passed. Reused sessions report their original
`statement_timeout=0` and `lock_timeout=0`. Socket overload tests use distinct
spoofed forwarding headers and verify a DB-backed ordinary pair query still
succeeds. A canceled reqwest client task remains counted until bounded work
settles; the middleware lifecycle is also verified directly with paused time.
This tests caller-future cancellation, without claiming packet-level FIN tracing.

## Acceptance result

- [x] AC1: Explicit public-chain-linked privacy policy; no actor fields/digests;
  synthetic dictionary and transaction-join tests preserve the stated limitation.
- [x] AC2: Actor grouping removed from response, schema and SQL. Public transaction
  linkage remains explicitly documented; HMAC key rotation is not applicable.
- [x] AC3: All surfaces, dense/empty dates, bounded pages/cursors, 24 measured plan
  scenarios, real SQL/acquisition budgets, HTTP deadline and admission passed.
- [x] AC4: Cancellation, timeout cleanup, ordinary DB API continuation and pooled
  session recovery passed, then passed again.
- [x] Invariants, migration behavior, historical #1205 guidance and agent checklist
  updated and crosslinked. Independent review found no remaining blocking defect.
- [ ] Production rollout/readback: unverified. This task publishes a source PR;
  no deployed revision, live migration or production performance is claimed.

Source verification: **PASS**. Overall review including production:
**PASS WITH SKIPS** (production deployment/readback outside this implementation
run). Agents may use the explicit skip policy for unavailable checks; an executed
failure remains FAIL until corrected and retested.
