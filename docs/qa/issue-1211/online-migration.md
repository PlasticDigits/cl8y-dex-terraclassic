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
- [ ] Released exact-revision compatible server and actual rolling deployment.
- [ ] Real serving API continuity/readiness before, during and after live DDL.
- [ ] Production online migration with complete current-run receipt and cleanup.
- [ ] Final schema/readiness plus digest and all three cohort API windows against
  the deployed final backend revision.

Keep #1211 open until its remaining production and live criteria pass.
