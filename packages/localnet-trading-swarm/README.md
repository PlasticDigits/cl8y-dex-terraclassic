# `@cl8y-dex/localnet-trading-swarm`

**LocalTerra only.** Five concurrent bot wallets (distinct strategy profiles) that broadcast swaps, hybrid swaps, router multi-hop swaps, limit orders, and LP add/remove on a schedule derived from a **Poisson process** with mean **20 seconds between actions per bot** (exponential inter-arrival; optional small jitter in `scheduler.ts`).

Spec and acceptance criteria: [GitLab #119](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/119).

## Safety invariants

1. **Chain guard:** LCD `/cosmos/base/tendermint/v1beta1/node_info` must report `default_node_info.network === "localterra"`. Wrong chain → non-zero exit.
2. **Env guard:** `VITE_NETWORK` in `.env.local`, if set, must be `local`. Any other value → exit.
3. **Infra:** `docker compose ps -q localterra` must return a container ID; funding uses `docker exec … terrad` like `scripts/deploy-dex-local.sh` / `scripts/e2e-provision-dev-wallet.sh`.
4. **No committed secrets:** Bot keys are **not** stored in git. Default: generate a fresh 12-word mnemonic at startup and print it **once** on stderr (set `SWARM_BOT_MNEMONIC` to reproduce the same five addresses via indices `0…4`). This differs from the public `TEST_MNEMONIC` in `docker/init-chain.sh`, which is a **dev-only** test vector for the simulated wallet / Playwright (see `docs/frontend.md`, GitLab #118).
5. **CW20 funding:** Every CW20 that appears in **any** factory pair is enumerated (paginated `pairs` query) and bots receive idempotent `Mint` top-ups when below a floor — same pattern as `scripts/e2e-provision-dev-wallet.sh`, not only the “primary” `VITE_*` token trio.

## Liquidity / sizing invariants (bots vs `deploy-dex-local.sh`)

On-chain truth for first LP mint remains `MINIMUM_LIQUIDITY` (1000 micro-units locked forever) on the pair contract; the dApp mirrors this as `PAIR_MINIMUM_LIQUIDITY` in `frontend-dapp/src/utils/provideLiquidityEstimate.ts`.

This package encodes **conservative heuristics** in `src/liquidityGuards.ts` so bots prefer pools that already have deep reserves from `scripts/deploy-dex-local.sh`:

| Constant | Role |
|----------|------|
| `MIN_RESERVE_PER_SIDE_FOR_SWAP` | Skip swaps when either reserve is too thin (default **1e7** raw units — far below typical seeded pools). |
| `MIN_PROVIDE_LIQUIDITY_LEG` | Skip tiny `provide_liquidity` legs that would round to dust vs the 1000 LP lock. |
| `MIN_SWAP_OR_ESCROW_AMOUNT` | Avoid limit orders and hybrid legs that fail **maker-fee** dust checks on-chain. |

Limit orders have **no** extra “minimum pool liquidity” gate in the pair contract; failures are usually wrong-side token, bad price, or hybrid book empty — see `docs/limit-orders.md`.

## Prerequisites

1. LocalTerra up: `docker compose up -d localterra`
2. Contracts + `frontend-dapp/.env.local`: `bash scripts/deploy-dex-local.sh`
3. Node **24+** and npm **11+** (see repo `.nvmrc`)

## Commands

From the **repository root**:

```bash
chmod +x scripts/localnet-trading-swarm.sh
./scripts/localnet-trading-swarm.sh
# or:
make swarm-local
```

From this package directory:

```bash
npm ci
npm run start -- --dry-run    # validate only
npm run start -- --stats      # print JSON inter-tx stats on SIGINT/SIGTERM
npm run test:run
```

Flags:

- `--dry-run` — no `bank send` / `Mint` funding and no bot txs; still requires LocalTerra + `.env.local` + LCD chain check.
- `--stats` — on shutdown, emit `kind: "swarm_stats"` JSON with per-bot mean gap vs target (20s).

Stop: **Ctrl+C** (SIGINT). The process exits after printing optional stats.

## Environment variables

| Variable | Purpose |
|----------|---------|
| `SWARM_REPO_ROOT` | Repo root (auto-detected via `docker-compose.yml` + `frontend-dapp/`). |
| `SWARM_BOT_MNEMONIC` | Optional mnemonic; five wallets use HD indices `0…4`, `coinType=330`. |
| `SWARM_ULUNA_TOPUP` / `SWARM_UUSD_TOPUP` / `SWARM_CW20_MINT_TOPUP` / `SWARM_MIN_CW20_BALANCE` / `SWARM_MINT_SLEEP_MS` | Funding tuning (see `src/funding.ts`). |
| `VITE_*` | Read from `frontend-dapp/.env.local`; **process environment overrides** the file (same as CI / shell exports). |

## Logs

Structured **JSON lines** to stdout: `profile`, `bot`, `action`, `txHash`, `note`, `error`, timestamps.

## Manual verification (after deploy)

1. Run the swarm for several minutes.
2. Confirm logs include all action kinds: `router_multihop`, `pair_swap`, `hybrid_swap`, `limit_order`, `add_liquidity`, `remove_liquidity` (field `action`).
3. Run with `--stats`, stop with SIGINT, confirm per-bot `meanGapSec` is near **20** (large sample).
4. Temporarily point `VITE_TERRA_LCD_URL` at a public LCD and observe a **clean refusal** (wrong `network`).

## Agent docs

Third-party / automation agents: [`../../skills/AGENTS_LOCALNET_TRADING_SWARM.md`](../../skills/AGENTS_LOCALNET_TRADING_SWARM.md).
