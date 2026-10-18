# Online cohort migration (#1211)

Contract: [FC1211-12–FC1211-14](../../../skills/AGENTS_FEE_COHORTS.md).
[Deployment checklist/history](deployment.md),
[coordinator](../../../scripts/ops/cohort-online-migration-1211.py),
[worker](../../../scripts/ops/cohort-online-worker-1211.py).

Keep the serving API running. First deploy and verify the exact released server
that supports both reviewed schema markers (67 migrations through
`20261016120000`, then 68 through `20261018110000`). Cohort ingestion remains
gated until schema 68. The separate online job applies only the additive cohort
migration using the existing owner role; the runtime receives no migration
credential. This flow has no serving-stop, role transition or database restore.
The prior standalone procedure is superseded.

## Job sequence and boundaries

From the exact merged compatible-main checkout, resolve the public revision and
use a new private journal:

```bash
cohort_revision=$(git rev-parse HEAD)
bao-exec --only COOLIFY_URL,COOLIFY_WIRE_TOKEN -- \
  python3 scripts/ops/cohort-online-migration-1211.py plan \
  --revision "$cohort_revision" --journal /tmp/cohort-online-1211.json
```

The helper checks exact remote main. Run its `build`, `status`, `inspect`, `status`,
`apply`, `status`, `verify`, `status` phases with the same revision/journal.
Build is credential-free WAITING. Both the migration-bearing binary and worker
Python bytes come from the same exact reviewed release Git object, rather than
mutable operator working-tree files. Exact useful serving health is required before job credential injection and
after the concurrent snapshot immediately before DDL. A normal
start refuses unresolved prior work before any API access; it cannot implicitly
abort an active snapshot or migration. Explicit `cleanup` deliberately aborts
only the owned maintenance job. It never stops the serving API.

Job, recipe, private storage and deployment identities bind the action. Terminal
receipts also bind revision, action, run ID and online mode; polling has a finite
unchanged owner deadline. Online PASS requires serving compatibility, preserved
full history/semantic ACLs/roles, new-table runtime DML grants, four denials and
an immutable concurrent-MVCC snapshot that explicitly forbids automatic restore.
If a start POST is accepted but its acknowledgement is lost, retain the pending
owner/run ID and exclude every prior deployment UUID before `status` reconciles
the new owned deployment. A transport exception after dispatch does not abort the
worker or remove its current credential. The shown-once runtime credential is
removed with readback after the owned job reaches its bound terminal state.

Before a snapshot, obtain a fresh read-only capacity receipt on the exact
private `/state` mount and database resource. The release recorded database size
via `pg_database_size(current_database())`, free filesystem bytes and free inodes;
no directory scan or snapshot-file read is needed. Its conservative threshold was
`max(2 × database bytes + 8GiB, 20% filesystem bytes)` and 10,000 free inodes.
This is an operator check, not an automatic guard in the migration coordinator,
and cannot guarantee against concurrent host growth. Stop only the capacity job
and remove its credential afterward; never delete its shared snapshot volume.
The [live receipt](../issues-1206-1512/live-release-20261018.md) records the result.

The migrator DSN establishes the effective existing owner, with `lock_timeout`
250ms and `statement_timeout` 5s. A busy lock fails the transaction rather than
waiting indefinitely. Preserve the original snapshot and inspect the exact
failure before a new explicit run ID retries. No ledger checksum repair,
credential rotation or database restore is part of recovery.

## Owned offline evidence

[The real owned rehearsal](online-owned-rehearsal.json) passed in **9.613s** under
an outer 59-second ceiling. It used a separately built/pinned debug migrator and
baseline example, real SQLx baseline 67, a new labeled PostgreSQL container,
random private environment credentials and an injected local readiness callback.
Production HTTP was forbidden. Only that container and its anonymous volume
were removed afterward.

It observed **134 actual runtime SQL read/write iterations**, zero errors,
progress before/during/after migration, all 67 complete history rows unchanged,
unchanged credentials/memberships and semantic ACLs, new grants and four actual
denials. Reordered equivalent relation/schema/database ACL arrays compare equal.
A real conflicting lock failed in **0.311s** without history/ACL changes; retry
and idempotent replay reused the same original snapshot. Published fee values
and original rollup clock were preserved. No database was restored.

This proves SQL continuity on the owned fixture. It does **not** establish real
serving HTTP continuity, a rolling deployment, a release image or production
migration. Initial harness failures are retained separately; incorrect fixture
counter naming and lock-holder cancellation were corrected before this passing
case, without modifying worker logic to bypass a failed assertion.

Focused controls (each process capped separately):

```bash
timeout 59s python3 scripts/tests/test_cohort_online_worker_1211.py
timeout 59s python3 scripts/tests/test_cohort_online_ops.py
# Prepare reviewed credential-free base.py and immutable current binary/example
# pins separately before running the actual owned PostgreSQL fixture:
timeout 59s python3 scripts/tests/test_cohort_online_local.py
```

## Release acceptance

- [x] Worker controls cover readiness changing after snapshot, constructor-time
  read-only options, stopped-flag refusal, scoped owner subprocess and snapshot reuse.
- [x] Coordinator controls bind online mode, useful exact SHA before credentials,
  current operation ownership, protected API identities, storage/deployments,
  complete online proof flags and finite terminal receipts.
- [x] Real owned PostgreSQL online 67→68, runtime SQL progress, lock refusal,
  unchanged full history/roles/ACLs and idempotent snapshot reuse.
- [x] Exact compatible backend `5b623a04f35bb6b44d3e64df0014e458cad653fc` rolled out and served schema 67 before migration, then schema 68.
- [x] Bounded sampled useful serving/readiness checks passed around the online operation; 55 new-revision HTTP 200 samples/zero failures cover 09:15:46–09:16:40 UTC, including snapshot completion 09:16:26. The worker checked exact useful serving immediately before DDL and after migration. This is not a full continuous-time or exact DDL-interval attestation.
- [x] Production additive 67→68 migration and separate verification passed: complete original 67 history, semantic ACLs and roles/credentials/memberships unchanged; new runtime grants and four actual denials. Concurrent 68,458,132-byte MVCC snapshot retained; shown-once job credentials removed with cleanup PASS. No database restore.
- [x] Final schema/readiness plus the anonymous digest and all three cohort windows passed against the exact final backend; counts/priced USD reconcile and shares total 100%.

See [the published live release evidence](../issues-1206-1512/live-release-20261018.md)
and [public API receipt](../issues-1206-1512/live-api-20261018.json).
The privileged job did not perform a fresh runtime-password login
(`runtime_login_tested=false`); the existing actual API runtime remained serving
and role/password/membership state did not change. After cleanup the owned job's
logs returned 400, so its exact terminal timestamp is unavailable. These limits
do not replace the separately passed migration/verification receipts or create
an additional funded-mainnet acceptance requirement.

The issue body/comment acceptance is complete; #1211 is ready to close.
