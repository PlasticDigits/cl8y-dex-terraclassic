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
