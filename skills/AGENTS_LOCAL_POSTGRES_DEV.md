# Agent playbook: local Postgres for indexer dev and tests

Use for integration tests, local deployment and database startup. Preserve
[DB1396-1–DB1396-10](AGENTS_INDEXER_DATABASE_ISOLATION.md) and run
`make verify-issue-1396`. The old `cl8y_legal` administrative runtime/shared-test
configuration from [#245](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/245)
is legacy; never restore it or reset shared volumes to bypass migration drift.

The bootstrap login defaults to `postgres`, runtime to `dex_indexer_runtime`,
migrator to `dex_indexer_migrator`, and QA to `dex_indexer_qa`. Passwords have no
defaults; configure four distinct private credentials through root `.env` or
an explicitly scoped `bao-exec` command. Never print URLs or secret values.
See [.env.example](../.env.example) and the
[provisioning and rollback runbook](../docs/runbooks/indexer-database-isolation.md).

```sh
COMPOSE_FILE=docker-compose.yml docker compose up -d postgres
./scripts/setup-postgres-dev-databases.sh
make indexer-migrate
# After chain deployment has provided FACTORY_ADDRESS/LCD/chain configuration:
make indexer-dev
```

Setup uses host `psql`, with the existing Compose-exec fallback for unreachable
published TCP. Existing databases must already have the intended owner; otherwise
stage the reviewed ownership transition from the runbook. No automatic reset,
password replacement, privilege promotion or migration-checksum surgery.

| Private file | Scope |
|---|---|
| `indexer/.env` | Runtime DATABASE_URL and application configuration only |
| `indexer/.env.migrate` | Migrator DATABASE_URL and runtime grant identity, consumed only by a one-shot process |
| `indexer/.env.qa` | Dedicated QA control credentials, QA database identity and local QA mode |

`make setup-indexer-postgres` provisions Postgres only, without chain deployment.
Use `make setup-cloud-localterra` when the task needs LocalTerra/front-end contracts.
Keep QA/migration env files out of the running server. Production migration runs
in a separately configured job, never as server startup.

```sh
cd indexer
cargo test --lib
cargo test --tests -j 1 -- --test-threads=1
```

Integration helpers load `.env.qa` without replacing explicitly injected test
credentials. They verify the dedicated QA role and admin-owned marked
`dex_indexer_test` control database, then create a fresh random database per test
binary/process. Only that child is migrated/cleaned. Binaries/worktrees can run
concurrently; tests within one binary still share fixtures and remain serial.
The child databases remain for investigation. Dispose of the entire dedicated
QA cluster after a run, or drop only individually verified QA-owned/marked child
databases after their processes exit. Never clean the control/application DB.

Repo commands explicitly select the base Compose file, so an ignored automatic
`docker-compose.override.yml` cannot silently enable host networking. Direct CLI
users must also specify `COMPOSE_FILE=docker-compose.yml` (or `-f`). If required:
`COMPOSE_FILE=docker-compose.yml:docker-compose.postgres-host.yml` opts into
Postgres host networking with IPv4/IPv6 loopback listen addresses only.

Run Cargo on the host. Never bind-mount `indexer/` into root containers for Cargo;
see [AGENTS.md](../AGENTS.md) and
[ownership helper](../scripts/lib/docker-indexer-bind-mount.sh).

Cross-links: [testing](../docs/testing.md), [local development](../docs/local-development.md),
[indexer invariants](../docs/indexer-invariants.md),
[issue checklist and skip policy](../docs/qa/issue-1396/README.md).
