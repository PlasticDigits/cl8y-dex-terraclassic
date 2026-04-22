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
