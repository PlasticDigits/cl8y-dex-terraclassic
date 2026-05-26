# Playwright E2E

## Pool transaction tests (`pool-tx.spec.ts`)

On the **default** path (full LocalTerra + deployed contracts), pool liquidity tests **fail** if the LCD is down, the submit control is still blocked after provisioning, or no tx result alert appears. This avoids silent `test.skip` masking regressions.

### Prerequisites

1. **LocalTerra** — from repo root: `docker compose up -d localterra`
2. **Contracts + `.env.local`** — `bash scripts/deploy-dex-local.sh` (writes `frontend-dapp/.env.local` and funds genesis dev account `terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v` with CW20 balances).
3. **Optional: indexer** — not required for `pool-tx`; the pool page works with LCD + factory env.

### Single-command pool E2E

From `frontend-dapp`:

```bash
pnpm exec playwright test e2e/pool-tx.spec.ts
```

or:

```bash
npx playwright test e2e/pool-tx.spec.ts
```

`playwright.config.ts` runs **`e2e/global-setup.ts`**, which waits for the LCD and executes **`scripts/e2e-provision-dev-wallet.sh`** to **idempotently mint** factory-listed CW20s to the dev wallet when balances fall below the configured floor (see script env vars).

### Strict vs optional chain

| `REQUIRE_LOCALTERRA` | Behavior |
|----------------------|----------|
| unset / `1` / other  | **Strict** — global setup requires LCD + `.env.local` + docker `localterra`; on-chain helpers **fail** instead of skipping when preconditions are missing. |
| `0`                  | **Optional** — global setup is skipped; `skipIfLcdUnreachable` and pool CTAs fall back to **`test.skip`** where documented (for jobs without a chain). |

### Minimum balances (raw CW20 units)

Provisioning targets **`E2E_DEV_MIN_CW20_U128`** (default `1000000000000`, i.e. \(10^{12}\) raw = \(10^6\) tokens at 6 decimals) per factory pair token. Native **uluna** / **uusd** for gas come from LocalTerra genesis (`docker/init-chain.sh`) on the same mnemonic as the simulated wallet.

Workers are fixed at **5** in `playwright.config.ts`; funding runs **once** in global setup to avoid per-worker races.

## Hybrid swap tests (`hybrid-swap.spec.ts`)

On the **default** path (full LocalTerra + deployed contracts), hybrid swap E2E **fail** when the LCD is down, there is no dual-CW20 pair, hybrid Settings controls are missing, the pair is paused, the swap CTA stays blocked after provisioning, or the on-chain tx does not emit `limit_order_fill` / positive `book_return_amount`. This replaces conditional `test.skip` for those environment gaps ([GitLab **#193**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/193)).

### Prerequisites

Same as pool tx, plus a **resting bid** on the first dual-CW20 factory pair (global setup runs **`scripts/e2e-seed-hybrid-book.sh`** after wallet provisioning). Hybrid swaps paying token0 with a book leg match bid-side liquidity.

| Env var | Default | Purpose |
|---------|---------|---------|
| `E2E_HYBRID_SEED_BID_ESCROW` | `50000000` | Raw CW20 units escrowed on the seeded bid (token1 of the pair). |
| `E2E_HYBRID_SEED_BID_PRICE` | `1` | Bid limit price (token1 per token0, CosmWasm `Decimal` string). |

The on-chain spec still places an additional limit in-test so the fill path is exercised end-to-end; the seed guarantees book depth even when that step is skipped in optional mode.

### Single-command hybrid E2E

```bash
cd frontend-dapp
pnpm exec playwright test e2e/hybrid-swap.spec.ts
```

Agent playbook: [`skills/AGENTS_E2E_HYBRID_SWAP.md`](../skills/AGENTS_E2E_HYBRID_SWAP.md).
