# Local Development

## Prerequisites

- **Rust** (stable) with `wasm32-unknown-unknown` target
- **Node.js 24** (see `.nvmrc`)
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

# 4. Start the frontend
cd ../../frontend-dapp
npm ci
# If you use the Simulated Wallet (VITE_DEV_MODE=true), set VITE_DEV_MNEMONIC so it matches docker/init-chain.sh. After `./deploy-dex-local.sh` from the deploy guide, that value is in `.env.development` (see docs/frontend.md, GitLab #118).
VITE_NETWORK=local npm run dev
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

## Troubleshooting

| Problem                           | Fix                                                |
|-----------------------------------|----------------------------------------------------|
| `wasm32-unknown-unknown` missing  | `rustup target add wasm32-unknown-unknown`         |
| LocalTerra won't start            | Ensure Docker is running, check port 1317/26657    |
| Contract upload fails             | Check gas settings in deploy script                |
| Frontend can't connect            | Verify `VITE_NETWORK=local` and LocalTerra is up   |
| `node_modules` issues             | Delete `node_modules` and `package-lock.json`, re-run `npm ci` |
