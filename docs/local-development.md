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
VITE_NETWORK=local npm run dev
```

## Makefile Commands

| Command               | Description                                    |
|-----------------------|------------------------------------------------|
| `make build`          | Build all contracts to WASM                    |
| `make test`           | Run `cargo test` for all contracts             |
| `make fmt`            | Run `cargo fmt` on all contracts               |
| `make clippy`         | Run clippy with `-D warnings`                  |
| `make optimize`       | Produce optimized WASM via cosmwasm/optimizer   |
| `make deploy-local`   | Deploy to LocalTerra                           |

## Docker Setup

The `docker-compose.yml` at the repo root starts a LocalTerra node for development. Contract deployment scripts in `smartcontracts/scripts/` target this local node by default.

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
