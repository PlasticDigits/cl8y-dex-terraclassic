# Indexer database boundaries (#1396)

Apply this playbook when changing database startup, privileges, migrations, QA
seeding, Compose networking or integration-test helpers. Issue:
[#1396](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1396).

| Invariant | Required behavior |
|---|---|
| DB1396-1 | The server, rebuild and backfill use a non-owner runtime login with no superuser, CREATEDB, CREATEROLE, replication, bypass-RLS or role memberships. Runtime has application DML and sequence usage, no schema creation, TRUNCATE or migration-history access. |
| DB1396-2 | A separate one-shot `migrate` process owns schema changes. The runtime reads only the protected schema-version marker and refuses an unprepared/incompatible database before writes. Migration/bootstrap/QA credentials stay out of server configuration. |
| DB1396-3 | Runtime, migrator and QA identities differ. QA has CREATEDB only for disposable test databases and cannot connect to the application database. Database ACLs deny implicit PUBLIC access; other applications require equivalent CONNECT/schema isolation. |
| DB1396-4 | Default Compose publishes Postgres only on IPv4 loopback. Repo commands select the base Compose file explicitly. Host networking requires the named opt-in file and listens only on IPv4/IPv6 loopback; it never requires a firewall exemption. |
| DB1396-5 | Passwords are explicit and nondefault. DSNs/passwords never enter logs or public receipts. Private runtime/migration/QA env files have mode 0600. |
| DB1396-6 | `seed-qa` parses help/errors before config; mutations require an explicit local QA mode, exact QA database name, dedicated QA login and loopback/service host before connecting. Authenticated role/marker/schema checks precede seed/clean. Seed never migrates. |
| DB1396-7 | Integration helpers never migrate or clean the supplied admin-owned marked control DB. They create a random per-process QA-owned child DB and apply strict migration history there. No `set_ignore_missing`, shared migration checksum edits or shared truncation. Tests within a binary remain serial. |
| DB1396-8 | Legacy role/ownership transition is staged with a snapshot, verified grants and rollback; setup never silently promotes, rotates or adopts a legacy role/database. Never reset shared/production volumes as a test shortcut. |
| DB1396-9 | Real ACL/socket and no-contact binary tests establish local behavior; source checks and skipped deployment cannot establish production privilege or exposure claims. |
| DB1396-10 | Complete runnable checks first. Unavailable checks may be explicit SKIP with overall PASS WITH SKIPS; executed failures remain FAIL. Record revision/time/commands/outcomes and bounded evidence scope. |

Implementation: [database security](../indexer/src/database_security.rs),
[QA guard](../indexer/src/qa_guard.rs), [role setup](../scripts/setup-postgres-dev-databases.sh),
[test helper](../indexer/tests/common/database.rs),
[host opt-in](../docker-compose.postgres-host.yml).

Run `make verify-issue-1396` and follow the
[agent checklist](../docs/qa/issue-1396/README.md).
Provisioning/transition/rollback instructions:
[runbook](../docs/runbooks/indexer-database-isolation.md).
Local setup: [Postgres playbook](AGENTS_LOCAL_POSTGRES_DEV.md).
Production role transition receipt:
[docs/qa/issue-1385/production-maintenance.json](../docs/qa/issue-1385/production-maintenance.json)
(61 migrations, schema marker `20261006133000`, four permission denials).
That receipt is deployment evidence for [#1396](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1396);
release coordination is [REL1421-4](AGENTS_RELEASE_OPS_1421.md).
