# Compatibility-first cohort transition

[FC1211-11](../../../skills/AGENTS_FEE_COHORTS.md) keeps serving active while the
separate additive migration is prepared. The serving process never receives
migration credentials or performs DDL. Do not stop the API or reuse a maintenance
job's stopped-runtime flag to manufacture online safety.

The compatibility server accepts only the reviewed 67/68 markers
(`20261016120000` / `20261018110000`) when its compiled latest migration is68.
Every existing runtime privilege check remains mandatory. Strict maintenance/QA
entry points retain exact-version checks; a future compiled migration disables
the older-version exception. Original migration files/checksums are unchanged.

On 67, each block owner caches its cohort decision once and skips all attribution
SQL. Economic ingestion, existing aggregates and the anonymous digest continue.
Fee publication and GET read their marker inside their existing repeatable-read
transaction. A publication that observed 67 cannot start cohort work after a
concurrent migration commits. The next owner sees 68 normally; earlier missing
attribution stays unknown and is never rewritten.

`GET /api/v1/protocol/fees` returns `cohort_status=pending_schema` with no
`by_cohort` on 67. On 68 it remains `pending_publication` until a real joint refresh
has stamped the existing `indexer_state` key `protocol_fee_cohort_publication`
with version 68 and the transaction timestamp. Global, source, token and all three
cohort rows must carry that same timestamp in one read snapshot. Missing rows,
migration bootstrap rows and legacy publisher interleavings cannot produce fake
idle cohorts. Available output retains the static three rows, exact decimal/null
semantics and aggregate-only privacy. The existing 60-second whole-response cache
retains an internally consistent earlier response; it cannot combine generations.

The reorg helper previews registry rows only on 68 and decides registry cleanup
inside its maintenance transaction. This does not authorize a live reorg or any
serving outage.

Build first, separately from bounded tests:

```sh
CARGO_TARGET_DIR=/path/to/shared-target cargo test --manifest-path indexer/Cargo.toml \
  --test cohort_online_compat --test indexer_fee_cohorts \
  --test indexer_protocol_fees --no-run
timeout 59s python3 scripts/qa/test-cohort-online-compat-1211.py \
  --test-binary /path/to/shared-target/debug/deps/cohort_online_compat-HASH
```

The owned-local fixture provisions a unique loopback PostgreSQL container and
fresh roles/database, runs the genuine 67 SQLx prefix without ledger rewrites,
then applies 68 while actual HTTP requests and production block parsing continue.
It holds the real 67 fee publisher across migration, checks pending/available and
mixed-generation behavior, restarts the API on both schemas, refuses older/newer
unknown markers, verifies strict-version entry points, preserves all 67 history
rows, checks runtime DDL/history/role denials and unchanged role attributes.
Only its labeled container/volume are removed. Its internal 48-second owner
includes provisioning; cleanup is bounded separately within the 59-second test.

- [x] Source build and focused owned transition pass.
- [x] Existing cohort and protocol-fee regressions pass.
- [ ] Exact compatibility release serves67 with useful APIs before any migration.
- [ ] Separately reviewed online backup/migration proves bounded locks and latency.
- [ ] Parallel replacement warms useful native quotes before traffic moves.
- [ ] Joint 68 publication, runtime denials and exact live revisions are verified.

Local/source evidence does not check off production migration, native provider
continuity, rollback or traffic-switch acceptance. Original failed maintenance
receipts remain failed/unverified and the private snapshot volume is preserved.

[Executed local receipt](compatibility-local-results.json): the final transition
passed in 3.67 seconds including provisioning/cleanup (30 HTTP samples, seven
parsed blocks). The cohort 10 and protocol-fee 31 regression targets passed in
2.83/9.10 seconds. The initial fixture-only down-migration counting failure is
retained; its owned container was removed before the corrected fresh runs.
