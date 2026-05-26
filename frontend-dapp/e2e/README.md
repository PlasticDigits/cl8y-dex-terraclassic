# Playwright E2E

Default CI and `make test-e2e-tx` use **strict on-chain** mode: missing LCD, funds, routes, or paused pairs **fail** the job instead of `test.skip` ([GitLab **#201**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201), policy [**#103**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/103)).

Agent playbook: [`skills/AGENTS_E2E_STRICT_CHAIN.md`](../../skills/AGENTS_E2E_STRICT_CHAIN.md).

## Projects

| Project | Command | Chain |
|---------|---------|-------|
| `e2e-tx` | `npm run test:e2e:tx` | **Required** — global setup + strict helpers |
| `e2e-smoke` | `npm run test:e2e:smoke` | Optional (`PLAYWRIGHT_SKIP_CHAIN=1`) |
| Both | `npm run test:e2e` | Tx strict; smoke skips when chain off |

## One-command strict tx E2E

From repo root:

```bash
make test-e2e-tx
```

Manual equivalent:

```bash
docker compose up -d localterra
make wait-healthy
bash scripts/deploy-dex-local.sh
cd frontend-dapp && npm run test:e2e:tx
```

`playwright.config.ts` runs **`e2e/global-setup.ts`** (unless chain is optional), which waits for the LCD and runs:

1. **`scripts/e2e-provision-dev-wallet.sh`** — idempotent CW20 mint (factory tokens + CL8Y ≥ tier-1 for fee-tier tx)
2. **`scripts/e2e-seed-hybrid-book.sh`** — resting bid on first dual-CW20 pair

## Strict vs optional chain

| Variable | Behavior |
|----------|----------|
| unset (default) | **Strict** — global setup required; tx helpers **fail** on missing preconditions |
| `PLAYWRIGHT_SKIP_CHAIN=1` | **Optional** — no global setup; helpers may `test.skip` (local UI dev only) |
| `REQUIRE_LOCALTERRA=0` | Legacy alias for `PLAYWRIGHT_SKIP_CHAIN=1` |

**Do not** set `PLAYWRIGHT_SKIP_CHAIN=1` in CI.

### Minimum balances (raw CW20 units)

| Token / use | Env | Default |
|-------------|-----|---------|
| Factory pair CW20s | `E2E_DEV_MIN_CW20_U128` | `1000000000000` (\(10^6\) @ 6 decimals) |
| CL8Y (fee tier Register) | `E2E_DEV_MIN_CL8Y_U128` | `1000000000000000000` (tier 1 min) |

Native **uluna** / **uusd** for gas come from LocalTerra genesis on the dev mnemonic.

Workers are fixed at **5** in `playwright.config.ts`; funding runs **once** in global setup.

## Pool transaction tests (`pool-tx.spec.ts`)

On the default path, pool liquidity tests **fail** if the LCD is down, the submit control is still blocked after provisioning, or no tx result alert appears.

```bash
cd frontend-dapp && pnpm exec playwright test e2e/pool-tx.spec.ts --project=e2e-tx
```

## Hybrid swap tests (`hybrid-swap.spec.ts`)

Strict failures for LCD down, missing dual-CW20 pair, paused pair, blocked swap CTA, or missing `limit_order_fill` / `book_return_amount` ([GitLab **#193**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/193)).

| Env var | Default | Purpose |
|---------|---------|---------|
| `E2E_HYBRID_SEED_BID_ESCROW` | `50000000` | Raw CW20 on seeded bid |
| `E2E_HYBRID_SEED_BID_PRICE` | `1` | Bid limit price |

Playbook: [`skills/AGENTS_E2E_HYBRID_SWAP.md`](../../skills/AGENTS_E2E_HYBRID_SWAP.md).

## Limit order transaction tests (`limit-orders-tx.spec.ts`)

First **unpaused** dual-CW20 pair via LCD `is_paused`; wasm `place_limit_order` / `cancel_limit_order` required ([GitLab **#195**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/195)).

Playbook: [`skills/AGENTS_E2E_LIMIT_ORDERS_TX.md`](../../skills/AGENTS_E2E_LIMIT_ORDERS_TX.md).

UI smoke (no chain): `e2e/limit-orders.spec.ts` in `e2e-smoke` project.

## Wrap / swap tx (`wrap-swap.spec.ts`, `wrap-pool.spec.ts`, `swap-tx.spec.ts`)

Native LUNC/USTC and CW20 routes must exist after `deploy-dex-local.sh`; helpers in `e2e/helpers/wrap-e2e.ts` fail in strict mode when tokens or native-wrap pool cards are missing.

## Fee tier tx (`fee-tier-tx.spec.ts`)

Requires self-service **Register** buttons (tiers 1–9) and dev-wallet CL8Y ≥ tier-1 minimum (provision script).
