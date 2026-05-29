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
| `DATABASE_URL` | `postgres://cl8y_legal:cl8y_legal@127.0.0.1:5432/dex_indexer` |
| `TEST_DATABASE_URL` | `postgres://cl8y_legal:cl8y_legal@127.0.0.1:5432/dex_indexer_test` |

Canonical source: [`scripts/lib/postgres-dev.env`](../scripts/lib/postgres-dev.env). Override via repo-root [`.env`](../.env.example) (gitignored) or exported env vars before deploy/setup.

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

Requires `psql` and a reachable Postgres listening as `cl8y_legal` on `127.0.0.1:5432`. The script creates **`dex_indexer`** + **`dex_indexer_test`** and **upserts `DATABASE_URL` / `TEST_DATABASE_URL` into `indexer/.env`** (so `cargo test` from `indexer/` loads the test DB via `dotenvy`, not the live indexer DB). `make start` runs this after compose up (best-effort if Postgres is not ready yet; re-run after `make wait-healthy`).

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

## Makefile / CI hooks

| Command | Postgres behavior |
|---------|-------------------|
| `make wait-healthy` | Waits for Postgres, runs `setup-postgres-dev-databases.sh` |
| `make deploy-local` | Sources postgres env, ensures DBs, writes `indexer/.env` |
| `make test-charts-integration` | Ensures target DB, migrates, seeds charts fixtures, runs Vitest integration (indexer HTTP; **stubbed** chart library) — [#205](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/205), [#230](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/230) |
| `make test-frontend-charts` | **No Postgres** — real `lightweight-charts` / Node `canvas` only (`npm run test:charts`; CI job `frontend-charts-vitest`) — [#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211), [#230](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/230) |
| `make reset` | `docker compose down -v` — **wipes volumes**; use when credentials/volume are stale |

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| `password authentication failed for user "postgres"` | Old volume or wrong URL | `make reset && make start && make deploy-local` |
| `password authentication failed for user "cl8y_legal"` | Postgres not up or user not created | `docker compose up -d postgres`; fresh volume runs init SQL |
| `database "dex_indexer_test" does not exist` | Skipped setup | `./scripts/setup-postgres-dev-databases.sh` |
| Duplicate-key / FK flakes in tests | Parallel tests on one DB | Always use `-j 1 -- --test-threads=1` |
| Tests pass locally but env unset in CI | CI sets `TEST_DATABASE_URL` in workflow | Match [`/.github/workflows/test.yml`](../.github/workflows/test.yml) |

## Cross-links

- [`docs/testing.md`](../docs/testing.md) — test types, shared-DB parallelism
- [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) — integration test matrix
- [`docs/local-development.md`](../docs/local-development.md) — full local stack
- [`skills/AGENTS_TESTING_P2_EPIC.md`](./AGENTS_TESTING_P2_EPIC.md) — charts layers: HTTP integration vs real-library Vitest ([#230](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/230))
- [`skills/AGENTS_E2E_STRICT_CHAIN.md`](./AGENTS_E2E_STRICT_CHAIN.md) — Playwright chain setup (Postgres via deploy)
- [`skills/AGENTS_LOCALNET_TRADING_SWARM.md`](./AGENTS_LOCALNET_TRADING_SWARM.md) — localnet bots after deploy
