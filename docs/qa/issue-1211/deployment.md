> The API must remain serving. The stopped-runtime workflow below is retained as
> historical evidence and must not be used for the online transition. Start with
> the [compatibility-first checklist](compatibility.md); an online migration and
> traffic switch require separate proved continuity and exact-release evidence.

# Cohort schema rollout

The additive [cohort migration](../../../indexer/migrations/20261018110000_protocol_fee_cohorts.sql)
uses a separate migrator. First roll out the server that accepts both reviewed
67/68 schema markers, then use the [online migration flow](online-migration.md)
while the API remains serving. Cohort ingestion activates only after schema 68. Preserve
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

## Current rollout: online additive migration

Follow [the online migration checklist](online-migration.md). It requires the
useful exact-revision 67/68-compatible server before credential injection and
rechecks serving readiness after its concurrent snapshot, immediately before DDL.
The separate mode-bound job preserves existing history, roles and ACLs. It never
stops the serving application or restores its database. Its runtime credential
is shown-once and removed with readback; the serving environment stays unchanged.

The exact job/storage/deployment ownership bindings and finite current-run
receipt checks remain required. Normal starts refuse an unresolved prior
operation before API or credential access. Only a verified terminal status or
explicit cleanup resolves that owner. Explicit cleanup deliberately cancels only
the bound maintenance job and removes its private credential.

## Historical standalone evidence (superseded)

The historical standalone helper/worker and
[old local rehearsal](local-migration-rehearsal.json) remain for audit. That
rehearsal refused active runtime sessions and restored a snapshot into a second
owned database; those are historical offline controls, not the current rollout
procedure. The legacy `stop-serving` command now refuses before credential or
API access. Do not use a serving stop or database restore for the online flow.

The [first storage readback failure](production-storage-readback-fail-20261018.json)
is retained: its coordinator expected an unprefixed volume name and refused
before starting. Current job creation requires Coolify's exact scoped volume
name, storage UUID, owning application ID, one private `/state` mount, no host
mount and no file shadows. Current status also binds deployment UUID, application
UUID/name and numeric application ID before interpreting receipts.

The [interrupted earlier apply](production-worker-interrupted-20261018.json)
remains unverified. A finished Coolify build and stale exited-app status were not
a worker completion receipt. Preserve uncertain history and snapshots; never
repair SQLx history or infer success from container state. Current receipt owners
use finite 70-second waiting/inspect and 915-second apply/verify allowances;
polling never renews those deadlines. Only the current revision/action/run/mode
receipt may authorize verification and cleanup of the owned maintenance job.

At that historical checkpoint, the compatible release and live migration/API
acceptance remained pending. The [later dated release](../issues-1206-1512/live-release-20261018.md)
now records the actual compatible rollout, online migration/verification and
public API PASS, with bounded sampled health and its explicit timing limits.
[The current checklist](online-migration.md) distinguishes those completed checks
from a full continuous-time or exact DDL-interval claim.

The [subsequent read-only inspection and API recovery](production-api-recovery-20261018.json)
confirmed 67 historical migrations with only the cohort migration pending. The
prior backend `0a3c251755a88ee9cbbbbc222fc747811c9da08c` was restored through Coolify,
finished healthy at 06:41:38Z, and returned public HTTP200 at 06:42:00Z. Its
source pin was retained. The earlier unknown worker outcome describes the
interruption time; it does not override this later database inspection. This
recovery receipt does not claim schema68 or a compatibility rollforward.
