# Agent playbook: local Postgres for indexer dev & tests

Use this skill when running **indexer integration tests**, **`make deploy-local`**, **`make wait-healthy`**, or debugging **`TEST_DATABASE_URL` / `DATABASE_URL`** failures.

## Defaults (do not assume `postgres:postgres`)

Local dev uses **`cl8y_legal` / `cl8y_legal`** (not the old Docker default user):

| Variable | Default |
|----------|---------|
| `POSTGRES_USER` | `cl8y_legal` |
| `POSTGRES_PASSWORD` | `cl8y_legal` |
| `POSTGRES_DB` | `dex_indexer` |
| `POSTGRES_TEST_DB` | `dex_indexer_test` |
| `POSTGRES_SUPERUSER` | `postgres` (bootstrap only) |
| `POSTGRES_SUPERUSER_PASSWORD` | `postgres` (bootstrap only) |
| `DATABASE_URL` | `postgres://cl8y_legal:cl8y_legal@127.0.0.1:5432/dex_indexer` |
| `TEST_DATABASE_URL` | `postgres://cl8y_legal:cl8y_legal@127.0.0.1:5432/dex_indexer_test` |

Canonical source: [`scripts/lib/postgres-dev.env`](../scripts/lib/postgres-dev.env). Override via repo-root [`.env`](../.env.example) (gitignored) or exported env vars before deploy/setup.

## Stack prerequisite: `cl8y_legal` role

The indexer and integration tests connect as **`cl8y_legal`**. Every stack must satisfy one of:

| Stack | How `cl8y_legal` appears |
|-------|---------------------------|
| **Repo Docker Compose** (fresh volume) | Compose `POSTGRES_USER` creates `cl8y_legal` on first init — no manual step |
| **Legacy / external Postgres** (only `postgres:postgres`) | Run [`scripts/setup-postgres-dev-databases.sh`](../scripts/setup-postgres-dev-databases.sh) — it **bootstraps** `cl8y_legal` via `POSTGRES_SUPERUSER` when superuser creds are available ([GitLab **#245**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/245)) |
| **Locked-down host** (no superuser for automation) | **Manual one-time:** create role + DBs (QA example below) |

Manual QA-stack provisioning (when bootstrap cannot run):

```sql
-- as postgres superuser (local QA only; do not grant SUPERUSER in production)
CREATE ROLE cl8y_legal WITH LOGIN CREATEDB PASSWORD 'cl8y_legal';
CREATE DATABASE dex_indexer OWNER cl8y_legal;
CREATE DATABASE dex_indexer_test OWNER cl8y_legal;
```

After role exists, `./scripts/setup-postgres-dev-databases.sh` is idempotent (ensures DBs + syncs `indexer/.env`).

## One-time / fresh stack

**If you still have an old Postgres Docker volume** (created when compose used `postgres:postgres`), reset and redeploy:

```bash
make reset && make start && make wait-healthy && make deploy-local
```

That recreates the volume (see [`docker/postgres-init/`](../docker/postgres-init/)), runs [`scripts/setup-postgres-dev-databases.sh`](../scripts/setup-postgres-dev-databases.sh), and writes [`indexer/.env`](../indexer/.env) with both URLs.

**Manual DB ensure** (idempotent; safe to re-run):

```bash
./scripts/setup-postgres-dev-databases.sh
```

Requires a reachable Postgres on `127.0.0.1:5432`. Uses host **`psql`** when installed; otherwise **`docker compose exec postgres psql`** (Cloud Agent VMs often lack `postgresql-client`). The script **creates `cl8y_legal` via superuser when missing** (see [Stack prerequisite](#stack-prerequisite-cl8y_legal-role)), then ensures **`dex_indexer`** + **`dex_indexer_test`** and **upserts `DATABASE_URL` / `TEST_DATABASE_URL` into `indexer/.env`** (so `cargo test` from `indexer/` loads the test DB via `dotenvy`, not the live indexer DB). `make start` runs this after compose up (best-effort if Postgres is not ready yet; re-run after `make wait-healthy`).

### Cursor Cloud Agent (Postgres-only, no wasm deploy)

On Cloud Agent VMs, use the lightweight bootstrap ([#335](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/335)) — does **not** start LocalTerra or run `deploy-dex-local`:

```bash
make setup-indexer-postgres
make test-indexer-integration    # serialized full integration suite
make verify-issue-324            # #324 lib + route_solve_get_cache HTTP tests
```

Full stack (frontend `.env.local`, chain LCD, indexer tmux): `make setup-cloud-localterra` — see [`AGENTS.md`](../AGENTS.md) § LocalTerra.

Regression: `make test-setup-postgres` (static + optional live Docker bootstrap).

## What agents should expect on disk

| Path | Written by | Contents |
|------|------------|----------|
| `indexer/.env` | [`scripts/deploy-dex-local.sh`](../scripts/deploy-dex-local.sh) phase 6.2 | `DATABASE_URL`, `TEST_DATABASE_URL`, factory/LCD/CORS, etc. |
| Repo-root `.env` | You (copy from [`.env.example`](../.env.example)) | Optional `POSTGRES_*` overrides for all scripts |

Integration tests load `indexer/.env` via `dotenvy` when run from `indexer/` ([`indexer/tests/common/mod.rs`](../indexer/tests/common/mod.rs)).

## Run indexer integration tests

Postgres must be up; `dex_indexer_test` must exist (setup script or deploy handles this).

```bash
docker compose up -d postgres   # if not already running
./scripts/setup-postgres-dev-databases.sh

cd indexer
cargo test --tests -j 1 -- --test-threads=1
```

- **`-j 1`** — one integration test binary at a time (shared DB).
- **`--test-threads=1`** — one test at a time inside each binary.

Explicit override (only if `indexer/.env` is missing):

```bash
export TEST_DATABASE_URL="${TEST_DATABASE_URL:-postgres://cl8y_legal:cl8y_legal@127.0.0.1:5432/dex_indexer_test}"
cd indexer && cargo test --tests -j 1 -- --test-threads=1
```

Library tests need **no** Postgres: `cd indexer && cargo test --lib`.

## Makefile / automation hooks

| Command | Postgres behavior |
|---------|-------------------|
| `make setup-indexer-postgres` | Cloud Agent: dockerd + `postgres` only + `indexer/.env` ([#335](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/335)) |
| `make test-indexer-integration` | Runs `setup-indexer-postgres`, then `cargo test --tests -j 1 -- --test-threads=1` |
| `make verify-issue-324` | #324 lib + `api_route_solve` cache tier integration (needs Postgres bootstrap) |
| `make wait-healthy` | Waits for Postgres, runs `setup-postgres-dev-databases.sh` |
| `make deploy-local` | Sources postgres env, ensures DBs, writes `indexer/.env` |
| `make test-charts-integration` | Ensures target DB, migrates, seeds charts fixtures, runs Vitest integration (indexer HTTP; **stubbed** chart library) — [#205](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/205), [#230](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/230) |
| `make test-frontend-charts` | **No Postgres** — real `lightweight-charts` / Node `canvas` only (`npm run test:charts`; reference job `frontend-charts-vitest`) — [#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211), [#230](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/230) |
| `make reset` | `docker compose down -v` — **wipes volumes**; use when credentials/volume are stale |

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| `password authentication failed for user "postgres"` | Old volume or wrong URL | `make reset && make start && make deploy-local` |
| `password authentication failed for user "cl8y_legal"` | Postgres not up, user not created, or wrong password | `docker compose up -d postgres`; run `./scripts/setup-postgres-dev-databases.sh` (auto-bootstrap via superuser when available); see [Stack prerequisite](#stack-prerequisite-cl8y_legal-role) |
| `database "dex_indexer_test" does not exist` | Skipped setup | `./scripts/setup-postgres-dev-databases.sh` |
| Duplicate-key / FK flakes in tests | Parallel tests on one DB | Always use `-j 1 -- --test-threads=1` |
| Tests pass locally but env unset on another host | Reference `indexer` job sets `TEST_DATABASE_URL` | Export `TEST_DATABASE_URL` per [docs/testing.md § CI](../docs/testing.md#ci); spec in [`.github/workflows/test.yml`](../.github/workflows/test.yml) (reference only) |

## Cross-links

- [GitLab **#245**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/245) — off-chain fee-discount quotes; **Postgres bootstrap** note (this skill)
- [`docs/testing.md`](../docs/testing.md) — test types, shared-DB parallelism, [§ CI](../docs/testing.md#ci) (local automation, [#234](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/234))
- [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) — integration test matrix
- [`docs/local-development.md`](../docs/local-development.md) — full local stack
- [`docs/localterra-sdk53.md`](../docs/localterra-sdk53.md) — LocalTerra **terrad v4 / SDK 0.53** ([#292](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/292)); **`make reset`** with Postgres when bumping chain image
- [`skills/AGENTS_TESTING_P2_EPIC.md`](./AGENTS_TESTING_P2_EPIC.md) — charts layers: HTTP integration vs real-library Vitest ([#230](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/230))
- [`skills/AGENTS_E2E_STRICT_CHAIN.md`](./AGENTS_E2E_STRICT_CHAIN.md) — Playwright chain setup (Postgres via deploy)
- [`skills/AGENTS_LOCALNET_TRADING_SWARM.md`](./AGENTS_LOCALNET_TRADING_SWARM.md) — localnet bots after deploy
