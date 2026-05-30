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

### Trading swarm (UI load / localnet only)

**Warning:** for **LocalTerra / local development only** — not for testnet, mainnet, or public RPCs. The process refuses wrong `chain_id` and non-local `VITE_NETWORK`.

After `docker compose up -d localterra` and `bash scripts/deploy-dex-local.sh`:

```bash
make swarm-local
# equivalent: ./scripts/localnet-trading-swarm.sh
```

Details, invariants, `--dry-run`, `--stats`, and env vars: [`packages/localnet-trading-swarm/README.md`](../packages/localnet-trading-swarm/README.md). Agent-oriented notes: [`skills/AGENTS_LOCALNET_TRADING_SWARM.md`](../skills/AGENTS_LOCALNET_TRADING_SWARM.md).

## Docker Setup

The `docker-compose.yml` at the repo root starts a LocalTerra node for development. Contract deployment scripts in `smartcontracts/scripts/` target this local node by default.

Images use **immutable digests** (LocalTerra + Postgres) for reproducible QA. To bump LocalTerra after a new `:latest` publish:

```bash
docker pull ghcr.io/plasticdigits/localterra-cl8y:latest
docker inspect ghcr.io/plasticdigits/localterra-cl8y:latest --format '{{index .RepoDigests 0}}'
```

Copy the `name@sha256:…` value into `docker-compose.yml` and update the adjacent YAML comment with the human-readable tag.

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

```bash
# Fresh volume after credential changes:
make reset && make start && make wait-healthy && make deploy-local

# Or ensure databases only:
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
| Frontend can't connect            | Verify `VITE_NETWORK=local` and LocalTerra is up   |
| Trade page shows `not implemented` / `-32701` under Pair | `VITE_FACTORY_ADDRESS` is empty and the app hit a malformed LCD URL (often **publicnode** when `VITE_TERRA_LCD_URL` is also unset). Run **`make deploy-local`** (writes **`frontend-dapp/.env.local`**). Delete or refresh stale **`frontend-dapp/.env`** so it does not override `.env.local`. Restart `npm run dev`. |
| `no such contract` on factory query | Chain was reset but env still has old addresses — re-run **`make deploy-local`** or **`make reset-qa`**. |
| Postgres auth / missing test DB   | See [`skills/AGENTS_LOCAL_POSTGRES_DEV.md`](../skills/AGENTS_LOCAL_POSTGRES_DEV.md); try `make reset` then redeploy |
| Stale deployed contracts (QA)    | [`scripts/qa/README.md`](../scripts/qa/README.md) § Stale deployed contracts; [`skills/AGENTS_QA_DEPLOY_VERIFY.md`](../skills/AGENTS_QA_DEPLOY_VERIFY.md) ([#203](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203)) |
| `node_modules` issues             | Delete `node_modules` and `package-lock.json`, re-run `npm ci` |
