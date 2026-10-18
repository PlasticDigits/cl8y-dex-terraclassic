# Cohort schema rollout

The additive [cohort migration](../../../indexer/migrations/20261018110000_protocol_fee_cohorts.sql)
needs a separate migrator before the new serving binary starts. Preserve
[DB1396](../../../skills/AGENTS_INDEXER_DATABASE_ISOLATION.md) and the
[role/snapshot runbook](../../runbooks/indexer-database-isolation.md).
Never put migration credentials in the serving application or its build inputs.

The [production preflight receipt](production-preflight-20261018.json) records a
SELECT-only isolated job: 67 complete historical checksums match, schema marker
`20261016120000`, only `20261018110000` pending, active runtime-role sessions and
safe runtime ACLs. PostgreSQL enforced read-only mode and statement/lock timeouts.
This is identity/readiness evidence, not a migration or deployment receipt.
The runtime DSN is hidden by Coolify; effective role/session checks do not claim
a fresh authenticated runtime login test.

[The preflight helper](../../../scripts/ops/cohort-preflight-1211.py) reads an
exact-hash-pinned historical maintenance guard in memory. It creates a new job,
binds its returned UUID/name/description/recipe before injecting credentials,
and limits the worker to 55 seconds. A saved terminal receipt prevents database
work repeating when Coolify restarts an exited container. The remaining original
window permits receipt collection, after which the helper stops only its own
job and removes the shown-once runtime credential with readback. Terminal
worker/cleanup failures return nonzero and retain sanitized failure evidence.
No Dockerfile or local journal contains a database credential.

```bash
bao-exec --only COOLIFY_URL,COOLIFY_WIRE_TOKEN -- \
  python3 scripts/ops/cohort-preflight-1211.py plan \
  --revision FULL_PUBLIC_SHA --journal /tmp/cohort-preflight.json
```

`create`, `status` and `cleanup` are separate explicit phases. `plan` performs
GETs only. This helper has no migrate, role-change, serving-stop or serving-deploy
operation. It uses the worktree's full SHA384 migration manifest and labels that
scope explicitly. Validate against the final merged release before migrating.
Tests: `timeout 59s python3 scripts/tests/test_cohort_preflight_1211.py`.

Coolify's [create controller](https://github.com/coollabsio/coolify/blob/v4.x/app/Http/Controllers/Api/ApplicationsController.php)
requires base64 Dockerfile content. Its
[deployment job](https://github.com/coollabsio/coolify/blob/v4.x/app/Jobs/ApplicationDeploymentJob.php)
generates the container restart policy, so a custom restart flag alone does not
establish one-shot execution. Exact recipe/guard hashes, explicit production role
bindings and finite receipt ownership are required.

Remaining rollout steps require the exact merged main binary, private persistent
snapshot, unchanged full SQLx history, one allowed migration, effective migrator
execution, runtime permission denials, credential cleanup, exact service revision
and bounded live API checks. No checksum/marker repair, password rotation or host
recovery is authorized by this preflight.

After the reviewed revision serves, run the bounded public smoke:

```sh
timeout 59s python3 scripts/qa/live-evidence-1206-1211.py --revision "$DEPLOYED_REVISION" --output /tmp/evidence-live.json
```

It uses at most fourteen anonymous requests, a fifty-second owner, exact revision
checks before/after, digest UTC/privacy/query/method/OpenAPI checks and all three
cohort windows with plain-decimal USD/shares. It exports booleans/counts rather
than response bodies. It does not establish historical cohort reconstruction,
actual-chain payer coverage or the long native acceptance matrix.

## Exact-release standalone migration

The [migration helper](../../../scripts/ops/cohort-migration-1211.py) and
[worker](../../../scripts/ops/cohort-migration-worker-1211.py) prepare a distinct
job using a public Git fetch of the exact merged main SHA and the release's
pinned Rust image. The compiled HEAD is checked before build and retained in
the image. All forward SQLx SHA384 checksums come from that exact Git object;
only `20261018110000` is allowed to be pending. The reviewed historical guard's
security conditions remain in force, with complete-prefix checks and explicit
new-table/runtime permission verification.

`plan` is GET-only. `build` creates the isolated job and a unique private named
snapshot volume, then compiles in credential-free WAITING mode. No migration
can start in WAITING. `inspect` temporarily injects only the job's shown-once
runtime credential and enforces PostgreSQL read-only mode. `status` accepts only
the current job/revision/action/run-ID receipt, stops that job and removes the
credential with readback. Prior or conflicting receipts cannot authorize a later
action. The serving application's environment never changes.

After exact-main hosted checks and operator authorization, `stop-serving`
requests the existing indexer's stop. `apply` requires stopped application status;
the worker additionally checks real database sessions before taking its private
custom snapshot. It invokes the **new compiled binary's** `migrate` command using
DSN `options` to establish the existing effective migrator, retains all original
SQLx rows, verifies the new marker/table grants and four actual runtime permission
denials, and checks unchanged roles/credentials/memberships. No checksum/marker
SQL repair or password rotation exists. Failed states remain failed; a new
explicit run ID owns any retry. A filesystem lock prevents concurrent job work.

```bash
bao-exec --only COOLIFY_URL,COOLIFY_WIRE_TOKEN -- \
  python3 scripts/ops/cohort-migration-1211.py plan \
  --revision FULL_MERGED_MAIN_SHA --journal /tmp/cohort-migration.json
```

Run `build`, `status`, `inspect`, `status`, then the explicitly authorized
`stop-serving`, `apply`, `status` phases. `verify` and `cleanup` are separate
bounded operations. Preserve the private snapshot volume after cleanup. A
migration receipt does not deploy applications: deploy that same SHA through
Coolify, then run [exact frontend/backend artifact attestation](../../../scripts/qa/attest-trading-quotes-1473.py)
and relevant live API checks. Restore source settings only after exact revision
acceptance. Do not start an incompatible older binary against an advanced marker.

The [owned local rehearsal](local-migration-rehearsal.json) passes in 11.957s:
real filtered SQLx baseline 67, actual new debug binary to 68, full original
history preservation, priced/unpriced/idle bootstrap rows and original clock,
actual runtime login/four permission denials, immutable snapshot restored into a
second owned database, and actual binary replay. Wrong owner, altered expected
checksum and active runtime sessions are refused. A fresh labeled PostgreSQL
container and its anonymous volume are removed afterward. No shared QA migration
history was used. The snapshot/permission engine is exercised directly; only
filesystem locations and the local debug binary path differ from production.

This rehearsal used checkout head `47ebbc2d` **plus working-tree seed/depositor
changes**, identified by the recorded migration SHA384 and debug-binary SHA256.
It does not attest a release image from that commit. Production build must pin
the final merged main revision. Retained setup/teardown failures remain separate
from the passing final rehearsal; none was converted into a skip or ledger edit.

Build the baseline fixture separately, then cap each test invocation:

```bash
cargo build --manifest-path indexer/Cargo.toml --example cohort_baseline_1211
timeout 59s python3 scripts/tests/test_cohort_migration_ops.py
timeout 59s bao-exec --only COOLIFY_URL,COOLIFY_WIRE_TOKEN -- \
  python3 scripts/tests/test_cohort_migration_local.py
```
