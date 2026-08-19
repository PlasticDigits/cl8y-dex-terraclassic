# QA server + laptop (Yield Omega DEX)

**Master checklist:** [GitLab **#337**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/337) — executable verification scenarios for LocalTerra and this QA stack (**INF-00-03**, **Q1** / **INF-00-02**).

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

This stops any prior QA indexer and runs **`docker compose down`** (volumes **preserved**), then starts **localterra** + **postgres**, waits for health, then deploys:

- **Skip deploy** when **`.qa-deploy-stamp`** `git_sha` == **`HEAD`**, env factory matches, wasm artifacts exist, and the factory LCD probe passes (**Q1** — GitLab [**#325**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/325)).
- Otherwise **`make deploy-local-no-build`** when optimizer wasm is already on disk, or **`make deploy-local`** (optimizer + deploy).

Optional: **`QA_FETCH_CI_ARTIFACTS=1 make start-qa`** tries GitLab generic packages for wasm/indexer before deploy. Set **`INDEXER_QA_BIN`** to a prebuilt **`cl8y-dex-indexer`** to skip release compile.

Then **`make qa-verify-deploy`** (schema + deploy-stamp + env/chain address cross-check — [#203](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203), SEC-H04 / [#442](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/442)), optional **`make qa-verify-deploy-config`** (on-chain config assertions — SEC-H03 / [#441](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/441)), indexer background start (pidfile **`.indexer-qa.pid`**, log **`.indexer-qa.log`**), **`/health`**, and laptop steps (**`make qa-tunnel-help`**).

**Which reset level after code changes?** See [`skills/AGENTS_QA_REDEPLOY_DECISION.md`](../../skills/AGENTS_QA_REDEPLOY_DECISION.md) ([#325](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/325)).

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

### Wallet verification scope (laptop QA via tunnel)

Browser wallet checks over an SSH tunnel follow the same matrix as [`docs/qa-onboarding.md`](../../docs/qa-onboarding.md) § Wallet Matrix ([GitLab **#235**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/235)):

| Environment | P0 wallets | Station |
|-------------|------------|---------|
| **LocalTerra** (tunnel to QA server) | **Keplr (extension)**, **dev/simulated wallet** | **N/A** — Station’s built-in `localterra` gas step cannot match node ante minimum; dApp overrides do not fix it |
| **columbus-5** | **Station (extension)** | P0 for swap, limits, bids, wrap/unwrap |

**Keplr + Ledger Nano** is columbus-5 **P1** (not LocalTerra): [`docs/qa-onboarding.md`](../../docs/qa-onboarding.md) § Wallet Matrix, [`skills/AGENTS_FRONTEND_KEPLR_LEDGER.md`](../../skills/AGENTS_FRONTEND_KEPLR_LEDGER.md) ([#567](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/567)).

Canonical root cause: [`docs/frontend.md` § Station extension signing](../../docs/frontend.md#station-extension-signing). Agent playbooks: [`skills/AGENTS_FRONTEND_STATION_SIGNING.md`](../../skills/AGENTS_FRONTEND_STATION_SIGNING.md), [`skills/AGENTS_TERRACLASSIC_GAS.md`](../../skills/AGENTS_TERRACLASSIC_GAS.md).

### Pair search — token name relevance (LocalTerra)

After **`make deploy-local`** (or QA deploy) and indexer sync, verify indexer **tier 2** (token **name** substring) and degraded combobox typed search ([#328](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/328), [#314](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/314)):

| Step | Expected |
| ---- | -------- |
| `./scripts/qa/verify-issue-328.sh` | Vitest PASS; live `q=Ember` returns pairs with `Ember` in `asset_*.name` or `EMBER` symbol when indexer is up |
| Stop indexer → `/trade` or `/limits` → type `EMBER` in pair combobox | Options list matching factory pairs (not “No pairs match”); **Offline search** hint |
| Indexer up → type `EMBER` | Indexer relevance results (unchanged healthy path) |

Deploy seeds CW20 **`name`** / **`symbol`** in Phase 2 (`scripts/deploy-dex-local.sh`). If `assets.name` is empty in Postgres, restart indexer after deploy so pair discovery re-resolves metadata.

### Fee-discount TCL8Y proxy (LocalTerra, GitLab #383)

Trading tokens (EMBER, CORAL, …) use **6** decimals. Fee-discount `min_cl8y_balance` values assume **18** decimals. Deploy creates **TCL8Y** and sets `VITE_CL8Y_TOKEN_ADDRESS` to it (not EMBER).

| Step | Expected |
| ---- | -------- |
| `make verify-issue-383` | TCL8Y `decimals=18`; tier-1 register + deregister on-chain (FT-3 / FT-4) |
| `make verify-issue-384` | `getGasLimitForTx` register/deregister limits; optional live `gas_used` check |
| `make verify-issue-475` | Retail execute-msg gas inventory / `BASE_GAS_LIMIT` guardrail; faucet `drip` (#474); optional live drip `gas_used` |
| `/tiers` + Keplr or Simulated Wallet | Register tier 1 succeeds when wallet holds ≥ 1 TCL8Y (requires #384 gas limits) |

---

## Makefile reference

| Target              | Purpose                                      |
| ------------------- | -------------------------------------------- |
| `make start-qa`     | Full QA bring-up (skips deploy when stamp + LCD probe match HEAD); runs post-deploy smoke after verify |
| `QA_SKIP_SMOKE=1 make start-qa` | Skip post-deploy `smoke-pool-swap` (indexer-only debugging) |
| `make smoke-pool-swap` | Pool LCD smoke using deploy stamp pair ([#368](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/368)) |
| `make deploy-local-no-build` | Deploy only — wasm artifacts must already exist |
| `QA_DEPLOY_SEED=minimal\|charts\|wallet\|full` | Lighter **`deploy-dex-local`** seed profiles ([#325](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/325)) |
| `QA_FETCH_CI_ARTIFACTS=1 make start-qa` | Try GitLab wasm/indexer packages before deploy |
| `make fetch-qa-ci-artifacts` | Download wasm/indexer for current **`HEAD`** sha |
| `make build-indexer-release` | Build **`indexer/target/release/cl8y-dex-indexer`** for **`INDEXER_QA_BIN`** |
| `make qa-start`     | Same as `start-qa`                           |
| `make reset-qa`     | Wipe LocalTerra + Postgres volumes, then `start-qa` |
| `QA_FRESH_VOLUMES=1 make start-qa` | Same as `reset-qa`              |
| `make stop-qa`      | Stop indexer + compose (volumes kept)      |
| `make test-qa-fresh-volumes` | Unit checks for fresh-volumes toggle (no Docker) |
| `make test-qa-verify-deploy` | Unit checks for LCD helpers + host-curl wiring (`test-verify-deploy`, `test-localterra-host-curl`) |
| `make test-localterra-host-curl` | Exec fallback wiring; live RPC probe when `localterra` is up |
| `make qa-tunnel-help` | Reprint SSH + laptop steps               |
| `make qa-verify-deploy` | Post-deploy schema + stamp + env address cross-check (also runs inside `start-qa`) |
| `make qa-verify-env-addresses` | Env/chain address cross-check only (SEC-H04 / [#442](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/442)) |
| `make qa-verify-deploy-config` | On-chain config assertions (SEC-H03 / [#441](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/441)) |
| `make status`       | Health summary                               |
| `make compose-ps`   | `docker compose ps` only                     |

---

## Troubleshooting

- **Indexer health fails** — Read **`.indexer-qa.log`**; confirm Postgres is up and **`indexer/.env`** **`DATABASE_URL`** matches compose (**`postgres://cl8y_legal:cl8y_legal@127.0.0.1:5432/dex_indexer`** by default (override via repo-root `.env` / `scripts/lib/postgres-dev.env`)). If the host Postgres only has **`postgres:postgres`**, run **`./scripts/setup-postgres-dev-databases.sh`** to bootstrap **`cl8y_legal`** (GitLab [**#245**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/245); [`skills/AGENTS_LOCAL_POSTGRES_DEV.md`](../../skills/AGENTS_LOCAL_POSTGRES_DEV.md)).
- **LocalTerra not ready** — `docker compose logs localterra`; on port conflicts set **`QA_SHARED_HOST=1`** or free host ports.
- **Host `curl` to `127.0.0.1:26657` / `:1317` hangs** — Chain may still be healthy in-container. **`make qa-verify-deploy`** and **`make wait-localterra`** use [`scripts/lib/localterra-host-curl.sh`](../lib/localterra-host-curl.sh) (`docker exec` fallback). Run **`make test-localterra-host-curl`** when compose is up. Frontend/browser still need working published ports or Docker daemon **`userland-proxy: false`** — see [`docs/local-development.md`](../../docs/local-development.md) troubleshooting.
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
