# QA server + laptop (Yield Omega DEX)

## On the QA server

### One-time

- **Docker** with compose v2. LocalTerra / Postgres image **digests** are pinned in root `docker-compose.yml`; bump them after pulling a new tag (see [`docs/local-development.md`](../../docs/local-development.md) § Docker Setup).
- **Rust + Cargo** on the PATH for `cargo run --release` in `indexer/` (or set **`INDEXER_QA_BIN`** to a prebuilt binary path in the environment before `make start-qa`).
- Optional repo-root **`.env`**: set **`QA_SSH_HOST`** (hostname as seen from your laptop) so `make qa-tunnel-help` prints a useful `ssh` destination; **`QA_SSH_PORT`** if SSH is not 22.

### Bring up the stack

```bash
make start-qa
# alias:
make qa-start
```

This stops any prior QA indexer and runs **`docker compose down`** (volumes **preserved**), then starts **localterra** + **postgres**, waits for health, runs **`make deploy-local`** (optimizer wasm + **`scripts/deploy-dex-local.sh`**), runs **`make qa-verify-deploy`** (schema + deploy-stamp check — GitLab [**#203**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203)), starts the **indexer** in the background (pidfile **`.indexer-qa.pid`**, log **`.indexer-qa.log`**), checks indexer **`/health`**, and prints **laptop** steps (same as **`make qa-tunnel-help`**).

### Fresh volumes (empty chain + Postgres)

After **contract or genesis changes**, or when **`qa-verify-deploy`** reports **stale deployed contracts** ([#203](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203)), wipe **`localterra-data`** and **`postgres-data`** before bring-up:

```bash
make reset-qa
# equivalent:
QA_FRESH_VOLUMES=1 make start-qa
```

A **red banner** prints when volumes are removed. Default **`make start-qa`** is unchanged (fast restarts when chain state is still valid). See [`docs/qa-invariants.md`](../../docs/qa-invariants.md) and [GitLab **#202**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/202).

### Shared host with cl8y-bridge-monorepo

Bridge uses LocalTerra on **26658/1318** by default. To avoid port clashes, run DEX QA with:

```bash
export QA_SHARED_HOST=1   # or set in repo-root .env
make start-qa
```

This merges **`docker-compose.qa-shared-host.yml`** and defaults DEX LocalTerra to **26659/1319** (and remapped gRPC ports). Override with **`DEX_TERRA_RPC_PORT`**, **`DEX_TERRA_LCD_PORT`**, etc., if needed.

### Stop

```bash
make stop-qa
```

Stops the QA indexer (from pidfile) and **`docker compose down`**.

### Status

```bash
make status
```

Checks Docker, LocalTerra RPC, Postgres, indexer **`/health`**, and the indexer pidfile.

---

## On your laptop

1. **SSH** — Run the `ssh -4 -N ... -L ...` block from **`make qa-tunnel-help`** (forwards LocalTerra RPC/LCD and indexer API to local loopback).
2. **Env** — `scp` **`frontend-dapp/.env.local`** from the server into your clone (URLs use `localhost` and match forwarded ports).
3. **Optional** — **`./scripts/qa/write-frontend-env-local.sh`** refreshes **`VITE_TERRA_*`** / **`VITE_INDEXER_URL`** from **`scripts/qa/qa-host.env`** (useful if ports differ between machines).
4. **Vite** — `cd frontend-dapp && npm ci && npm run dev` — do **not** tunnel the Vite port; run the dev server locally.

---

## Makefile reference

| Target              | Purpose                                      |
| ------------------- | -------------------------------------------- |
| `make start-qa`     | Full QA bring-up on the server (keep volumes) |
| `make qa-start`     | Same as `start-qa`                           |
| `make reset-qa`     | Wipe LocalTerra + Postgres volumes, then `start-qa` |
| `QA_FRESH_VOLUMES=1 make start-qa` | Same as `reset-qa`              |
| `make stop-qa`      | Stop indexer + compose (volumes kept)      |
| `make test-qa-fresh-volumes` | Unit checks for fresh-volumes toggle (no Docker) |
| `make qa-tunnel-help` | Reprint SSH + laptop steps               |
| `make qa-verify-deploy` | Post-deploy schema + stamp check (also runs inside `start-qa`) |
| `make status`       | Health summary                               |
| `make compose-ps`   | `docker compose ps` only                     |

---

## Troubleshooting

- **Indexer health fails** — Read **`.indexer-qa.log`**; confirm Postgres is up and **`indexer/.env`** **`DATABASE_URL`** matches compose (**`postgres://cl8y_legal:cl8y_legal@127.0.0.1:5432/dex_indexer`** by default (override via repo-root `.env` / `scripts/lib/postgres-dev.env`)).
- **LocalTerra not ready** — `docker compose logs localterra`; on port conflicts set **`QA_SHARED_HOST=1`** or free host ports.
- **Stale wasm on disk** — `make build-optimized` then re-run deploy ( **`make deploy-local`** ).
- **Stale deployed contracts (reused Docker volumes)** — GitLab [**#203**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203). **`make start-qa`** runs **`make qa-verify-deploy`** after deploy; it queries the deployed pair for **`is_paused`** and **`expired_limit_refund`** and compares **`.qa-deploy-stamp`** (`git_sha`) to **`HEAD`**. Failure exits non-zero with volume-reset instructions.

  | Symptom | Likely cause | Fix |
  | ------- | ------------ | --- |
  | `unknown variant` on `is_paused` / `expired_limit_refund` | On-chain pair wasm older than repo tree | **`make reset-qa`** (or below) |
  | `qa-verify-deploy`: stamp `git_sha` ≠ `HEAD` | Pulled new commits without redeploying | `make deploy-local` or full `make start-qa` |
  | Live QA walks pass UI but contract queries fail | Laptop **`.env.local`** from an old server deploy | Re-`scp` **`frontend-dapp/.env.local`** after server deploy |

  **Reset chain + Postgres state** (when reuse is unsafe — after contract schema changes or failed verification):

  ```bash
  make reset-qa
  # or manually:
  make stop-qa
  docker compose down -v
  docker volume rm cl8y-dex-terraclassic_localterra-data cl8y-dex-terraclassic_postgres-data
  make start-qa
  ```

  Agent playbook: [`skills/AGENTS_QA_DEPLOY_VERIFY.md`](../../skills/AGENTS_QA_DEPLOY_VERIFY.md). Fresh-volumes toggle: [`skills/AGENTS_QA_FRESH_VOLUMES.md`](../../skills/AGENTS_QA_FRESH_VOLUMES.md) ([#202](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/202)).

  **When volume reuse is safe:** infra-only changes, frontend/indexer-only work, or no pair query/schema changes since last verified deploy on the same **`git_sha`** stamp.
