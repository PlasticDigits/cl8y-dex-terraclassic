# QA Onboarding

**Master executable checklist:** [GitLab **#337**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/337) — Automation Verification Engineer master Local/QA checklist (INF/CFG/WL/NAV/SW/TR/LT/PL/CP/FT/PF/CH/PR/IDX/ONC/ERR/CON/XBR/RES/A11Y/PERF/SEC/OBS/REG/EDGE/INT/LR sections). Use for launch sign-off; pair with [`QA_TEMPLATE.md`](../QA_TEMPLATE.md) and [`docs/runbooks/launch-checklist.md`](./runbooks/launch-checklist.md).

## Prerequisites

- **git** and **gh** CLI installed and authenticated
- **nvm** + Node **24** (`nvm use` at repo root, or `bash scripts/with-node.sh -- …`)
- **Docker** and **Docker Compose**
- **Rust** (stable) with `wasm32-unknown-unknown` target (for building contracts and running the indexer)
- A Terra Classic wallet with testnet LUNC — **Terra Classic Keplr (extension)** for local QA; **Terra Station** for **columbus-5** P0 only (Station cannot verify fees/signing on LocalTerra — [GitLab #235](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/235))
- Access to the repository — [GitLab](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic)

## Git Workflow

- **Default branch:** `main` is the only long-lived integration branch in this repository. Open merge requests and base releases on `main`.

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

1. **Staleness guard** — checks that WASM artifacts are newer than their source; exits with an error and tells you to run `make build-optimized` if anything is stale
2. Waits for LocalTerra to be healthy
3. Uploads CW20, Factory, Pair, Router, and Fee Discount WASM contracts
4. Instantiates the Factory and Router
5. Uploads and instantiates Treasury and Wrap-Mapper (builds from ustr-cmm source if `.wasm` not present)
6. Creates wrapped-native CW20 tokens (LUNC-C, USTC-C) and registers denom mappings
7. Registers the Wrap-Mapper on the Router (enables native denom swap path)
8. Funds the Treasury with LUNC + USTC
9. Creates 10 whitelisted test tokens (EMBER, CORAL, JADE, etc.)
10. Creates 2 non-whitelisted tokens (ROGUE, BOGUS) under a separate code ID
11. Creates 3 unpaired/minimally-paired tokens (ZINC, IRON, NEON)
12. Instantiates the Fee Discount contract with 11 tiers
13. Creates 23 trading pairs + 3 unpaired-token pairs, all with initial liquidity
14. Executes ~60 test swaps to seed price history
15. Writes `frontend-dapp/.env.local` (LCD/RPC URLs, all contract addresses including Treasury/Wrap-Mapper/wrapped tokens)
16. Writes `indexer/.env` (Postgres connection, Factory address, API port)

### Verifying the indexer

Once the indexer is running (`make indexer-dev`), it indexes on-chain events
into Postgres and exposes an API on `http://localhost:3001`. The frontend
connects to this for charts, trade history, and leaderboard data.

### Seeding historical data for chart / candle testing

A fresh LocalTerra only has a few minutes of block history, so longer candle
intervals (1h, 4h, 1d, 1w) will be empty. The indexer includes a `seed-qa`
command that inserts synthetic swap events with timestamps spread across
several weeks and rebuilds all candle intervals from them.

**Prerequisites:** contracts must be deployed first (`make deploy-local`) so
that pairs exist in the database, and the indexer must have run at least once
to apply migrations.

```bash
# Seed 4 weeks of history (default), 24 swaps/pair/day
cd indexer && cargo run -- seed-qa

# Customise the time span and density
cargo run -- seed-qa --weeks 6 --swaps-per-day 40

# Remove all seeded data and rebuild candles from real swaps only
cargo run -- seed-qa --clean
```

After seeding, restart the indexer (`make indexer-dev`) and open the charts
page — you should see populated candles for all intervals including 1w.

> **Note:** seeded swaps use a `SEEDQA_` tx-hash prefix and a fixed sender
> address, so they are easy to distinguish from real on-chain activity. The
> `--clean` flag removes only seeded data; real swap events are preserved.

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

### QA server (remote host)

For Yield Omega DEX QA on a shared server, see [`scripts/qa/README.md`](../scripts/qa/README.md). Volume behaviour is documented in [`docs/qa-invariants.md`](./qa-invariants.md) ([#202](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/202)): use **`make start-qa`** for fast restarts; **`make reset-qa`** (or **`QA_FRESH_VOLUMES=1 make start-qa`**) after contract changes or when [#203](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203) stale-contract warnings apply.

## What to Test

### Swap Flow
1. Connect wallet
2. Select input/output tokens from the dropdown
3. Enter an amount — verify the estimated output updates
4. Click Swap — confirm the transaction in your wallet (**Keplr** or **dev/simulated wallet** on LocalTerra; **Station** on columbus-5 only — [#235](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/235))
5. Verify balances updated correctly
6. Verify the fee was deducted (check treasury balance or tx events)

### Pool / Liquidity
1. Navigate to `/pool`
2. Select a pair
3. Add liquidity — enter both token amounts, confirm
4. Verify LP tokens received
5. Remove liquidity — enter LP amount, confirm
6. Verify both tokens returned

### Indexer pool list API & Pool/Charts (when shipping indexer or pool-list UI)

For releases that change **liquidity pool discovery** (indexer-backed list, search, sort, pagination) or **Charts** pair selection, run the dedicated checklist in **`QA_TEMPLATE.md` §23** (Indexer pool list API & Pool/Charts UI). It covers:

- **`GET /api/v1/pairs`** — envelope `{ items, total, limit, offset }`, pagination defaults, `q` / `asset` / `sort` / `order`, `volume_quote_24h`, validation errors
- **`GET /api/v1/pairs/{addr}`** — detail response vs list-only fields
- **Integration tests:** `cd indexer && cargo test --test api_pairs` (requires Postgres and `TEST_DATABASE_URL`)
- **Pool page** — `VITE_INDEXER_URL`, error handling, total count formatting, UI controls, E2E `frontend-dapp/e2e/pool.spec.ts`
- **Charts** — “Find pair” filter behavior and safe selection when the list changes

The short **`docs/qa-templates/qa-test-pass.md`** template includes condensed bullets for the same areas when filing a test pass.

### Create Pair
1. Navigate to `/pool/create`
2. Enter two valid CW20 token addresses
3. Submit — confirm the Factory transaction
4. Verify the new pair appears in the pool list

## Wallet Matrix

### LocalTerra (`VITE_NETWORK=local`)

| Wallet              | Platform | Priority        |
|---------------------|----------|-----------------|
| Keplr (ext)         | Chrome   | P0              |
| Dev / simulated     | Chrome   | P0 (when enabled) |
| Station (ext)       | —        | **N/A** — use columbus-5 for Station P0 ([#235](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/235)) |

### columbus-5 (mainnet / staging)

| Wallet          | Platform    | Priority |
|-----------------|-------------|----------|
| Station (ext)   | Chrome      | P0       |
| Station (ext)   | Firefox     | P1       |
| Station (mobile)| iOS/Android | P1       |
| Keplr (ext)     | Chrome      | P1       |

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
