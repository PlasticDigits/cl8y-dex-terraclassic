# Indexer role transition and isolated QA (#1396)

Preserve [DB1396-1–DB1396-10](../../skills/AGENTS_INDEXER_DATABASE_ISOLATION.md).
[Acceptance/checklist](../qa/issue-1396/README.md) distinguishes implementation
verification from production acceptance.

## Fresh local provisioning

Use a dedicated local cluster. Create a private root `.env` from
[the template](../../.env.example); supply four distinct nondefault passwords
(runtime, migrator, QA and bootstrap). The bootstrap password must have at least
16 characters and cannot be a known default/placeholder. Keep these values out of
logs and commits. Setup derives its local URLs from explicit POSTGRES_* role/password/host/database
configuration instead of reusing an inherited DATABASE_URL. Setup requires an explicit administrator and never automatically
changes existing passwords, privileged roles or database ownership.

```sh
COMPOSE_FILE=docker-compose.yml docker compose up -d postgres
./scripts/setup-postgres-dev-databases.sh
make indexer-migrate
# deploy-local supplies the remaining runtime chain configuration
make indexer-dev
```

The administrator creates the runtime database owned by the migrator, restricts
PUBLIC database/schema access and grants runtime DML on application tables and
sequence usage. Runtime cannot read/write `_sqlx_migrations`, change the schema
marker, create roles/databases or access the admin-owned QA control database.
Migrator owns the application schema and has no cluster administration privileges.
QA has LOGIN/CREATEDB without cluster administration or role memberships; it may
own disposable child databases, and cannot connect to the runtime database.
Unrelated applications must revoke PUBLIC CONNECT and avoid granting runtime
schema/table/function access: PostgreSQL's PUBLIC defaults are not isolation.

The separate private files are `indexer/.env` (runtime only), `.env.migrate`
(migrator) and `.env.qa` (QA), mode 0600. `scripts/indexer-migrate.sh` launches a
one-shot process with a minimal environment. The local `scripts/indexer-run.sh` launcher strips inherited migration/bootstrap/QA
capabilities before spawning server/recovery commands. The server never applies migrations
and rejects privileged credential variables, unsafe authenticated role attributes,
or an incompatible schema readiness marker before indexing/recovery writes.
Production must also keep bootstrap and QA credentials out of the server process.

## Existing deployment transition

1. Stop the old indexer and snapshot the database and role/grant metadata. Record
   old owner/login and revision without passwords. Retain the snapshot and old
   login privately for rollback; do not reset volumes or edit migration checksums.
2. Create distinct least-privilege roles, revoke all memberships and audit current
   database/schema/object ownership. Transfer only the dedicated application's
   database/schema/tables/sequences to the migrator under the administrator.
   Do not run a cluster-wide REASSIGN OWNED or adopt unrelated applications.
3. Run setup with the intended owners/credentials, then the new binary's `migrate`
   command in a separate job using the migrator's DATABASE_URL. Reconcile grants
   after migrations. Verify runtime DML succeeds and DDL, ledger changes, role
   creation, cross-schema and cross-database access fail under actual ACLs.
4. Inject only runtime DATABASE_URL/application config into the server. Start it,
   verify schema/role checks and indexing/API behavior, then disable the legacy
   administrative login after confirming it has no other consumers.

A production migration job uses the same image's `cl8y-dex-indexer migrate`
command with only its scoped DATABASE_URL and INDEXER_RUNTIME_ROLE; do not install
migration secrets in the server's Coolify resource. Local setup is not a production
migration runner. Production role/exposure acceptance remains a separately recorded
execution against that deployment.

## Coolify deployment rejected by schema readiness

`database schema version differs from this binary` means the database marker does
not match the binary's required schema version. For an upgrade with pending
migrations, prepare the database through the release's standalone migration step.
If the database is ahead of the binary, follow the [rollback decision](rollback-decision.md)
instead of lowering the marker. Coolify can retain
the previous healthy container after rejecting the new container; check both the
failed deployment's commit and the public `/health` revision before diagnosing an
outage. A healthy old revision does not establish acceptance of the new release.

1. Prepare a separate maintenance job for the exact failed commit. Compare all
   applied forward migration checksums with that release and restrict its pending
   migration allowlist to the reviewed changes. Keep database credentials runtime
   only in that job, with no build arguments or serving-app migration secrets.
2. Run read-only inspection while the old indexer serves. Prepare and review the
   snapshot, migration and ACL verification steps before stopping it. Retain the
   private snapshot on a persistent maintenance volume.
3. Stop the serving indexer and confirm its database sessions have closed. Take
   and validate the snapshot, then run the release's `cl8y-dex-indexer migrate`
   with the existing migrator's effective identity and the actual runtime role.
   Verify the schema marker, full original migration history, new runtime grants
   and runtime permission denials. Preserve role identities, credentials and
   ownership; never edit migration checksums or update the marker by hand.
4. Deploy the exact release, check its public `/health` revision and indexing
   progress, then stop the maintenance job and remove its database credential.
   Retain the snapshot and secret-free receipt. Restore the serving application's
   original source setting after exact revision acceptance if it was temporarily
   pinned for this operation.

Each release that adds migrations needs this separate migration step before its
server rollout. Adding startup migration privileges to the server would break
DB1396-1 and DB1396-2.

The [2026-10-10 deployment receipt](../qa/indexer-deployment-20261010.json)
records recovery of the failed `c5cc674f` rollout through migration
`20261009145300`, with the snapshot retained and the maintenance credential removed.

## Rollback

Stop the new process. If no incompatible schema change occurred, restore the
previous binary and private credential configuration from the snapshot, validate
its actual grants and start it with auto-deployment disabled. Prefer keeping the
new least-privilege roles if the previous binary supports them; a previous binary
that insists on startup migrations needs an explicitly scoped maintenance process.
Restore ownership/grants only for the inventoried application objects if necessary.
If migrations changed schema incompatibly, use the repository's
[rollback decision](rollback-decision.md) with the snapshot; never repair checksum
history as a test shortcut. Revoke unused new logins after rollback confirmation.

## QA seed and worktree tests

Seed/clean requires explicit `RUN_MODE=dev|qa|test|local`, `DATABASE_URL` using
`dex_indexer_qa` at loopback or the `postgres` service, and `QA_DATABASE_NAME`
matching a `dex_indexer_test`/`dex_indexer_qa` name with a safe underscore suffix.
The authenticated database must carry `cl8y-dex:isolated-qa:v1` and have the current
protected schema marker. Prepare a dedicated QA database/schema with the migration
command before seed/clean; the seed command never migrates. Help/invalid arguments
and production/unknown targets never connect. `--clean` cannot bypass these checks.
`seed-qa --check` performs these identity/readiness checks without mutation and
is used by the charts fixture runner, which requires an explicit QA target and
never creates/migrates databases. The integration control `dex_indexer_test` remains unmigrated; use a different
explicitly provisioned QA database for seed demonstrations.

Integration helpers instead use `.env.qa`'s TEST_DATABASE_ADMIN_URL (QA capability),
verify the admin-owned marked control DB, and create fresh random child databases.
Children remain for inspection, and strict migration checks never ignore missing
versions. Serial tests within each binary avoid fixture races; independent binaries
and worktrees can run concurrently without shared migration histories.

## Network verification

Default publication is `127.0.0.1:5432`. Use explicit base selection even for bare
Docker CLI calls; move legacy ignored host overrides aside after reviewing other
services. The opt-in [host file](../../docker-compose.postgres-host.yml) removes
port mappings and sets `listen_addresses=127.0.0.1,::1`; no non-loopback listen or
firewall exception is required. A connection from another Docker namespace to a
host interface must fail; default bridge-network service peers can access the DB
using their own credentials. Local namespace probes are not an internet exposure
assessment or a receipt of production rollout.

The production maintenance receipt is
[production-maintenance.json](../qa/issue-1385/production-maintenance.json):
61 preserved migrations, schema marker `20261006133000`, and four runtime
permission denials. Local probes do not replace that receipt. See
[DB1396](../../skills/AGENTS_INDEXER_DATABASE_ISOLATION.md) and
[REL1421-4](../../skills/AGENTS_RELEASE_OPS_1421.md).
