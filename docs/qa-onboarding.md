# QA Onboarding

## Prerequisites

- **git** and **gh** CLI installed and authenticated
- **Node.js 24** (use `nvm use` in the repo root)
- **Docker** and **Docker Compose**
- **Rust** (stable) with `wasm32-unknown-unknown` target (for building contracts and running the indexer)
- A Terra Classic wallet (Station extension) with testnet LUNC
- Access to the repository — [GitLab](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic)

## Quick Start (Testnet Only)

```bash
git clone https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic.git && cd cl8y-dex-terraclassic
git config core.hooksPath .githooks
cd frontend-dapp && npm ci
VITE_NETWORK=testnet npm run dev
```

Open `http://localhost:3000` and connect your wallet.

## Local Infrastructure Setup

For full local testing (including the indexer, contracts, and swaps), use the
automated setup script. This spins up LocalTerra + Postgres via Docker, builds
and deploys all contracts, creates test tokens/pairs, executes sample swaps,
and writes the `.env` files for both the frontend and the indexer.

### One-command setup

```bash
make dev-full
```

This runs the full lifecycle: starts Docker containers, builds optimized WASM
artifacts, deploys everything via `scripts/deploy-dex-local.sh`, then starts
the indexer and frontend dev server.

### Step-by-step setup

```bash
# 1. Start LocalTerra and Postgres containers
make start

# 2. Build optimized WASM contracts
make build-optimized

# 3. Deploy contracts, tokens, pairs, and generate .env files
make deploy-local
#    (runs scripts/deploy-dex-local.sh — writes frontend-dapp/.env.local and indexer/.env)

# 4. Start the indexer
make indexer-dev

# 5. In a separate terminal, start the frontend
cd frontend-dapp && npm ci && npm run dev
```

### What the deploy script does

`scripts/deploy-dex-local.sh` automates the entire local deployment:

1. Waits for LocalTerra to be healthy
2. Uploads CW20, Factory, Pair, Router, and Fee Discount WASM contracts
3. Instantiates the Factory and Router
4. Creates 10 test tokens (EMBER, CORAL, JADE, etc.)
5. Creates 20 trading pairs with initial liquidity
6. Executes 60 test swaps to seed price history
7. Writes `frontend-dapp/.env.local` (LCD/RPC URLs, contract addresses)
8. Writes `indexer/.env` (Postgres connection, Factory address, API port)

### Verifying the indexer

Once the indexer is running (`make indexer-dev`), it indexes on-chain events
into Postgres and exposes an API on `http://localhost:3001`. The frontend
connects to this for charts, trade history, and leaderboard data.

### Useful Makefile commands

| Command           | Description                                     |
|-------------------|-------------------------------------------------|
| `make start`      | Start Docker containers (LocalTerra + Postgres) |
| `make stop`       | Stop Docker containers                          |
| `make restart`    | Restart Docker containers                       |
| `make reset`      | Stop containers and delete volumes              |
| `make status`     | Show container status                           |
| `make logs`       | Tail all container logs                         |
| `make deploy-local` | Run the local deploy script                   |
| `make indexer-dev`  | Start the indexer                              |
| `make dev-full`   | Full lifecycle: infra + build + deploy + run    |

## What to Test

### Swap Flow
1. Connect wallet
2. Select input/output tokens from the dropdown
3. Enter an amount — verify the estimated output updates
4. Click Swap — confirm the transaction in Station
5. Verify balances updated correctly
6. Verify the fee was deducted (check treasury balance or tx events)

### Pool / Liquidity
1. Navigate to `/pool`
2. Select a pair
3. Add liquidity — enter both token amounts, confirm
4. Verify LP tokens received
5. Remove liquidity — enter LP amount, confirm
6. Verify both tokens returned

### Create Pair
1. Navigate to `/pool/create`
2. Enter two valid CW20 token addresses
3. Submit — confirm the Factory transaction
4. Verify the new pair appears in the pool list

## Wallet Matrix

| Wallet          | Platform    | Priority |
|-----------------|-------------|----------|
| Station (ext)   | Chrome      | P0       |
| Station (ext)   | Firefox     | P1       |
| Station (mobile)| iOS/Android | P1       |

## CLI Workflow

### File a bug
```bash
./scripts/qa/new-bug.sh "swap fails with zero amount"
# or with evidence:
./scripts/qa/new-bug.sh --evidence /path/to/screenshot.png "swap fails with zero amount"
```

### File a test pass
```bash
./scripts/qa/new-test-pass.sh
```

## Security Escalation

If you discover a potential security issue (e.g., unauthorized access, fund loss, contract exploit):

1. **Do NOT** file a public GitHub issue
2. Contact `@PlasticDigits` directly via a private channel
3. Include: steps to reproduce, affected contract/function, potential impact

## Device Checklist

- [ ] Desktop Chrome (latest)
- [ ] Desktop Firefox (latest)
- [ ] Mobile Safari (iOS)
- [ ] Mobile Chrome (Android)
- [ ] Tablet (either platform)
