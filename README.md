# CL8Y DEX - Terra Classic

A decentralized exchange (DEX) built on Terra Classic, featuring an AMM (Automated Market Maker) with customizable fee hooks, CW20 token support, multi-hop routing, and tiered fee discounts for CL8Y token holders.

## Quick Start

```bash
# Start LocalTerra
make start

# Build optimized smart contracts
make build-optimized

# Deploy contracts to LocalTerra (uploads, instantiates, creates test pair)
make deploy-local

# Start frontend dev server
make dev
```

## Project Structure

```
cl8y-dex-terraclassic/
├── smartcontracts/          # CosmWasm smart contracts (Rust)
│   ├── contracts/
│   │   ├── factory/         # Factory - creates and manages trading pairs
│   │   ├── pair/            # Pair - AMM pool for two tokens
│   │   ├── router/          # Router - multi-hop swap routing
│   │   ├── fee-discount/    # Fee Discount - tiered swap fee discounts for CL8Y holders
│   │   └── hooks/           # Fee hooks (burn, tax, LP burn)
│   ├── packages/
│   │   └── dex-common/      # Shared types and messages
│   ├── scripts/
│   │   ├── optimize.sh      # Build optimized wasm via Docker
│   │   └── deploy.sh        # Deployment instructions for testnet/mainnet
│   └── artifacts/           # Optimized wasm output (generated)
├── frontend-dapp/           # React frontend (Vite + TypeScript)
│   ├── src/
│   │   ├── components/      # UI components
│   │   ├── pages/           # Swap, Pool, CreatePair, Tiers pages
│   │   ├── services/        # Terra Classic wallet & contract services
│   │   └── utils/           # Constants and helpers
│   └── patches/             # Cosmes library patches
├── scripts/                 # Deployment scripts
│   ├── deploy-dex-local.sh  # Full local deployment
│   ├── deploy-dex-testnet.sh
│   └── deploy-dex-mainnet.sh
├── docker-compose.yml       # LocalTerra infrastructure
├── Makefile                 # Build, test, deploy commands
└── docs/                    # Documentation
```

## Available Commands

| Command | Description |
|---------|-------------|
| `make start` | Start LocalTerra via Docker |
| `make stop` | Stop LocalTerra |
| `make reset` | Stop LocalTerra and delete all data |
| `make logs-terra` | Follow LocalTerra logs |
| `make build-optimized` | Build optimized wasm contracts |
| `make deploy-local` | Deploy all contracts to LocalTerra |
| `make deploy-testnet` | Print testnet deployment instructions |
| `make deploy-mainnet` | Print mainnet deployment instructions |
| `make dev` | Start frontend dev server |
| `make test` | Run all tests (contracts + frontend) |
| `make lint` | Run all linters |

## Networks

| Network | Chain ID | LCD |
|---------|----------|-----|
| Local | `localterra` | `http://localhost:1317` |
| Testnet | `rebel-2` | `https://terra-classic-lcd.publicnode.com` |
| Mainnet | `columbus-5` | `https://terra-classic-lcd.publicnode.com` |

## Documentation

Project docs (architecture, integrators, runbooks, testing) are indexed in [`docs/README.md`](docs/README.md). For **local, testnet, and mainnet** chain IDs, LCD defaults, and indexer expectations (`RUN_MODE`, `LCD_URLS`, etc.), see the [environment matrix](docs/environment-matrix.md).

## Development

### Git

The default branch is **`main`** (the only long-lived integration branch). Merge requests should target `main`.

### Prerequisites

- [Docker](https://docs.docker.com/get-docker/)
- [Rust](https://rustup.rs/) with `wasm32-unknown-unknown` target
- [Node.js](https://nodejs.org/) 24+ (see `.nvmrc`)

### Smart Contracts

```bash
make test-contracts    # Run contract unit tests
make lint-contracts    # Check formatting and clippy
make build-optimized   # Build production wasm
```

### Indexer (Rust)

The indexer has library tests (`cargo test --lib`, no Postgres) and integration tests (`cargo test --tests`) that require PostgreSQL and migrations. See [Testing — Indexer (Rust)](docs/testing.md#indexer-rust) and [Shared Postgres and test parallelism](docs/testing.md#shared-postgres-and-test-parallelism) for `TEST_DATABASE_URL` and how to avoid flaky runs when multiple tests share one database.

### Frontend

```bash
cd frontend-dapp
npm install
npm run dev            # Start dev server
npm run test:run       # Run tests
npm run lint           # Lint code
```
