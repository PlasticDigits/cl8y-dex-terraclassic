# Local Development

## Prerequisites

- **Rust** (stable) with `wasm32-unknown-unknown` target
- **[nvm](https://github.com/nvm-sh/nvm)** with Node **24** (`nvm use` at repo root — `.nvmrc`). Prefer `make dev` / `scripts/with-node.sh` so the correct Node is always on `PATH`.
- **Docker** and **Docker Compose** (for LocalTerra)
- **gh** CLI (for QA scripts)

## Quick Start

```bash
# 1. Install git hooks
git config core.hooksPath .githooks

# 2. Start LocalTerra
docker compose up -d

# 3. Build and deploy contracts
cd smartcontracts
cargo build --release --target wasm32-unknown-unknown
cd scripts
./deploy-dex-local.sh

# 4. Start the frontend (from repo root — Node via nvm)
bash scripts/with-node.sh --cwd frontend-dapp -- npm ci
# Simulated wallet: VITE_DEV_MNEMONIC in `.env.development` after deploy (docs/frontend.md, GitLab #118).
make dev
# or: bash scripts/with-node.sh --cwd frontend-dapp -- env VITE_NETWORK=local npm run dev
```

## Makefile Commands

| Command               | Description                                    |
|-----------------------|------------------------------------------------|
| `make build-contracts` | Build all contracts to WASM (cargo, not optimizer) |
| `make test`           | Run `cargo test` for all contracts             |
| `make fmt`            | Run `cargo fmt` on all contracts               |
| `make clippy`         | Run clippy with `-D warnings`                  |
| `make build-optimized` | Produce optimized WASM via workspace-optimizer |
| `make deploy-local`   | Deploy to LocalTerra                           |
| `make dev`            | Start Vite (`scripts/dev-frontend-local.sh` — requires `.env.local`) |
| `make swarm-local`    | Run the **localnet-only** trading bot swarm ([GitLab #119](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/119)) — requires LocalTerra + `deploy-dex-local` first |
| `make verify-issue-620` | LocalTerra community-tax seed + Transfer funding ([GitLab #620](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/620)) |
| `make verify-issue-621` | Tax-aware localnet swarm gem exclude + tax workers ([GitLab #621](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/621)) |

### Trading swarm (UI load / localnet only)

**Warning:** for **LocalTerra / local development only** — not for testnet, mainnet, or public RPCs. The process refuses wrong `chain_id` and non-local `VITE_NETWORK`.

After `docker compose up -d localterra` and `bash scripts/deploy-dex-local.sh`:

```bash
make swarm-local
# equivalent: ./scripts/localnet-trading-swarm.sh
```

Details, invariants, `--dry-run`, `--stats`, and env vars: [`packages/localnet-trading-swarm/README.md`](../packages/localnet-trading-swarm/README.md). Agent-oriented notes: [`skills/AGENTS_LOCALNET_TRADING_SWARM.md`](../skills/AGENTS_LOCALNET_TRADING_SWARM.md). After the #620 tax seed, gem workers exclude the tax token and a dedicated `tax_listed` / `--worker tax` path sizes extra-debit correctly — [`skills/AGENTS_LOCALNET_SWARM_TAX.md`](../skills/AGENTS_LOCALNET_SWARM_TAX.md).

### Community-tax seed (GitLab #620)

Default `make deploy-local` stores the current community-tax / launcher / AutoLP wasm, instantiates a **SmokeUST1** invoice stand-in, paid-creates **QATax** (`auto_v2_lp`, buy/sell 500 bps, no MintControl), `CreatePair` vs EMBER, `RegisterListedPair`, seeds LP above the swarm floor, and binds AutoLP `pair`. Writes local pins into `frontend-dapp/.env.local` and `indexer/.env` (catalog `configured: true` after ingest).

This is **not** a license to whitelist columbus-5 **11611** / **11619** from LocalTerra. CMM stand-in is `test1`. Funding tops up the tax token with **Transfer** from `test1` — never Mint.

```bash
DEPLOY_SKIP_COMMUNITY_TAX=1 make deploy-local   # gems only
make verify-issue-620
```

Playbook: [`skills/AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md`](../skills/AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md) (**L620-1–L620-8**). `#601` smoke stays ephemeral.

## Docker Setup

The `docker-compose.yml` at the repo root starts a LocalTerra node for development. Contract deployment scripts in `smartcontracts/scripts/` target this local node by default.

Images use **immutable digests** (LocalTerra + Postgres) for reproducible QA. LocalTerra tracks **Terra Classic terrad v4 / Cosmos SDK 0.53** ([GitLab **#292**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/292) — invariants: [`docs/localterra-sdk53.md`](./localterra-sdk53.md)). To bump LocalTerra after a new `:latest` publish:

```bash
docker pull ghcr.io/plasticdigits/localterra-cl8y:latest
docker inspect ghcr.io/plasticdigits/localterra-cl8y:latest --format '{{index .RepoDigests 0}}'
terrad version   # inside container: docker compose exec localterra terrad version
```

Copy the `name@sha256:…` value into `docker-compose.yml`, sync [`docker/init-chain.sh`](../docker/init-chain.sh) with the image’s `/usr/local/bin/init-chain.sh` if genesis/CLI changed, then **`make reset`** before redeploy. Genesis `test1` balances are **11M LUNC** (10× + deploy headroom) + **100M USTC** (GitLab [#372](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/372)); stale volumes keep old funding — see **LT4** in [`docs/localterra-sdk53.md`](./localterra-sdk53.md).

```bash
# Start
docker compose up -d

# View logs
docker compose logs -f

# Stop
docker compose down
```

## Postgres (indexer + integration tests)

Docker Compose Postgres defaults to user **`cl8y_legal`** (not `postgres`). Deploy writes `indexer/.env` with `DATABASE_URL` and `TEST_DATABASE_URL`.

**Stack prerequisite:** Every host running the indexer or `cargo test --tests` must have the **`cl8y_legal`** role (see [`skills/AGENTS_LOCAL_POSTGRES_DEV.md`](../skills/AGENTS_LOCAL_POSTGRES_DEV.md) § Stack prerequisite; [GitLab **#245**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/245)). [`scripts/setup-postgres-dev-databases.sh`](../scripts/setup-postgres-dev-databases.sh) bootstraps the role via superuser when missing.

```bash
# Fresh volume after credential changes:
make reset && make start && make wait-healthy && make deploy-local

# Or ensure role + databases only:
./scripts/setup-postgres-dev-databases.sh
```

Agent playbook (setup, test commands, troubleshooting): [`skills/AGENTS_LOCAL_POSTGRES_DEV.md`](../skills/AGENTS_LOCAL_POSTGRES_DEV.md). Human docs: [Testing — local Postgres](./testing.md#local-postgres-setup-agents).

## Troubleshooting

| Problem                           | Fix                                                |
|-----------------------------------|----------------------------------------------------|
| `wasm32-unknown-unknown` missing  | `rustup target add wasm32-unknown-unknown`         |
| LocalTerra won't start            | Ensure Docker is running, check port 1317/26657    |
| Host `curl` to `127.0.0.1:26657` / `:1317` hangs (TCP connects, no HTTP) | Known **docker userland-proxy** issue on some Linux hosts. Chain is often healthy **inside** the container. Scripts (`make wait-localterra`, `make qa-verify-deploy`) use **`docker exec` fallback** via [`scripts/lib/localterra-host-curl.sh`](../scripts/lib/localterra-host-curl.sh). Browser/frontend still need published ports or fix proxy (`userland-proxy: false` in Docker daemon). |
| Contract upload fails             | Check gas settings in deploy script                |
| `make deploy-local` aborts at Phase 4 with `InsufficientPairCreationFee` | Each `create_pair` consumes the factory's `pair_creation_fee_uluna` (default **100 LUNC**; treasury = deploy address, so it returns to you). Deploy attaches it automatically and pre-flights `test1`'s uluna balance. Re-fund `test1` if the pre-flight fails, or run with `LOCAL_PAIR_CREATION_FEE_ULUNA=0` for a fee-free local chain (GitLab [#318](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/318)). |
| Frontend can't connect            | Verify `VITE_NETWORK=local` and LocalTerra is up   |
| Trade page shows `not implemented` / `-32701` under Pair | `VITE_FACTORY_ADDRESS` is empty and the app hit a malformed LCD URL (often **publicnode** when `VITE_TERRA_LCD_URL` is also unset). Run **`make deploy-local`** (writes **`frontend-dapp/.env.local`**). Delete or refresh stale **`frontend-dapp/.env`** so it does not override `.env.local`. Restart `npm run dev`. |
| `no such contract` on factory query | Chain was reset but env still has old addresses — re-run **`make deploy-local`** or **`make reset-qa`**. |
| Postgres auth / missing test DB   | See [`skills/AGENTS_LOCAL_POSTGRES_DEV.md`](../skills/AGENTS_LOCAL_POSTGRES_DEV.md); try `make reset` then redeploy |
| `Permission denied` on `indexer/target/debug/.cargo-build-lock` | Cargo ran as **root** (almost always `docker run -v indexer:/… rust:… cargo`). Compile on the host; cleanup `sudo chown -R $(whoami) indexer/target`. Do not bind-mount `indexer/` to “fix” hung `:5432` — use compose exec. [AGENTS.md § Rust / Docker gotchas](../AGENTS.md) |
| Stale deployed contracts (QA)    | [`scripts/qa/README.md`](../scripts/qa/README.md) § Stale deployed contracts; [`skills/AGENTS_QA_DEPLOY_VERIFY.md`](../skills/AGENTS_QA_DEPLOY_VERIFY.md) ([#203](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203)) |
| `node_modules` issues             | Delete `node_modules` and `package-lock.json`, re-run `npm ci` |
