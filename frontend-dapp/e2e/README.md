# Playwright E2E

Default CI and `make test-e2e-tx` use **strict on-chain** mode: missing LCD, funds, routes, or paused pairs **fail** the job instead of `test.skip` ([GitLab **#201**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201), policy [**#103**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/103)).

Agent playbook: [`skills/AGENTS_E2E_STRICT_CHAIN.md`](../../skills/AGENTS_E2E_STRICT_CHAIN.md).

## Projects

| Project | Command | Chain |
|---------|---------|-------|
| `e2e-tx` | `make test-e2e` / `bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:e2e:tx` | **Required** — global setup + strict helpers |
| `e2e-smoke` | `bash scripts/with-node.sh --cwd frontend-dapp -- env PLAYWRIGHT_SKIP_CHAIN=1 npm run test:e2e:smoke` | Optional chain |
| Both | `bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:e2e` | Smoke (5 workers) then tx strict (1 worker) |

**Worker invariant (#201):** On-chain specs share one LocalTerra account. `e2e-tx` uses **1 worker**; `e2e-smoke` keeps **5**. Pool tx helpers: [`e2e/helpers/pool-ui.ts`](./helpers/pool-ui.ts) (expand vs submit buttons).

## One-command strict tx E2E

From repo root (Node via **nvm** — `scripts/with-node.sh` / `.nvmrc`):

```bash
make test-e2e-tx
```

Manual equivalent:

```bash
docker compose up -d localterra
make wait-localterra
bash scripts/deploy-dex-local.sh
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:e2e:tx
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
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/pool-tx.spec.ts --project=e2e-tx
```

## Hybrid swap tests (`hybrid-swap.spec.ts`)

Strict failures for LCD down, missing dual-CW20 pair, paused pair, blocked swap CTA, or missing `limit_order_fill` / `book_return_amount` ([GitLab **#193**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/193)).

| Env var | Default | Purpose |
|---------|---------|---------|
| `E2E_HYBRID_SEED_BID_ESCROW` | `50000000` | Raw CW20 on seeded bid |
| `E2E_HYBRID_SEED_BID_PRICE` | `1` | Bid limit price |

Playbook: [`skills/AGENTS_E2E_HYBRID_SWAP.md`](../../skills/AGENTS_E2E_HYBRID_SWAP.md).

## Limit order transaction tests (`limit-orders-tx.spec.ts`)

First **unpaused** dual-CW20 pair via LCD `is_paused`; wasm `place_limit_order` / `cancel_limit_order` required ([GitLab **#195**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/195)). Tests use `fillValidLimitPrice()` so bid/ask prices pass the reference place gate (default UI price `1` is often invalid for bids).

Playbook: [`skills/AGENTS_E2E_LIMIT_ORDERS_TX.md`](../../skills/AGENTS_E2E_LIMIT_ORDERS_TX.md).

UI smoke (no chain): `e2e/limit-orders.spec.ts` in `e2e-smoke` project.

## Trade book Edit smoke (GitLab #338) {#trade-book-edit-smoke-gitlab-338}

Smoke E2E for order-book **Edit** prefill, `trade-limit-edit-context` banner, non-price block, and cancel-from-book ([GitLab **#338**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/338), unblocks [#292](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/292) AC-3).

| Invariant | Meaning |
|-----------|---------|
| **Seeded pair** | EMBER/CORAL via `e2eTradePairFromDeploy()` + global `e2e-seed-hybrid-book.sh` dev-wallet bids |
| **No stale pre-Edit assertion** | Cancel-first copy appears only **after** Edit (or after amount drift), not on workspace load |
| **Both viewports** | Desktop ≥1440px (`trade-desktop-workspace`) and sub-desktop &lt;1024px (`trade-sub-lg-workspace`) |

```bash
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/trade-book-edit-178.spec.ts --project=e2e-smoke
```

Playbook: [`skills/AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md`](../../skills/AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md).

## Claim all parked tx E2E (`limit-orders-claim-all-tx.spec.ts`, GitLab **#259**)

End-to-end **place → expire → hybrid park → indexer `parked_expired` → Claim all parked → `claim_expired_limit_orders_batch`**. The spec calls **`scripts/e2e-seed-expired-parked-claim-all.sh`** (terrad: two expired bids, wait for `block_time`, hybrid swap parks) then drives the `/limits` UI confirm (must include **est. LUNC gas** copy). Pure-book park swap sets **`min_return: "1"`** on the hook to satisfy execute slippage floor ([#334](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/334)).

**Prerequisites:** same stack as other `e2e-tx` specs — LocalTerra, deploy, **indexer running** with CORS for the Vite origin ([`docs/frontend.md` § Local dev indexer CORS](../../docs/frontend.md)).

| Env var | Default | Purpose |
|---------|---------|---------|
| `E2E_EXPIRED_PARK_EXPIRY_LEAD_SEC` | `45` | `expires_at` = block time + lead |
| `E2E_EXPIRED_PARK_EXPIRY_WAIT_MAX_SEC` | `120` | Max wait for chain time ≥ `expires_at` |
| `E2E_EXPIRED_PARK_HYBRID_BOOK_INPUT` | `5000` | Hybrid swap book leg (raw token0) |

```bash
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/limit-orders-claim-all-tx.spec.ts --project=e2e-tx
```

Playbooks: [`skills/AGENTS_E2E_LIMIT_ORDERS_TX.md`](../../skills/AGENTS_E2E_LIMIT_ORDERS_TX.md), [`skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](../../skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md).

## Price chart smoke (`price-chart-smoke.spec.ts`)

Browser checks for **lightweight-charts** canvas mount on `/charts` and `/trade`, interval switch stability, and **fullscreen** `aria-label` toggles with a mocked Fullscreen API ([GitLab **#228**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/228)). Helpers: `e2e/helpers/price-chart.ts`.

| Mode | Behavior |
|------|----------|
| Strict (default CI) | Canvas tests require indexer + deploy (same stack as `trade-page-responsive.spec.ts`) |
| `PLAYWRIGHT_SKIP_CHAIN=1` | Entire `price-chart-smoke.spec.ts` **skipped** (needs indexer + deploy like other trade E2E) |

```bash
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/price-chart-smoke.spec.ts --project=e2e-smoke
```

Playbook: [`skills/AGENTS_FRONTEND_PRICE_CHART.md`](../../skills/AGENTS_FRONTEND_PRICE_CHART.md); matrix: [`docs/testing.md`](../../docs/testing.md#price-chart-playwright-smoke-gitlab-228).

## Wrap / swap tx (`wrap-swap.spec.ts`, `wrap-pool.spec.ts`, `swap-tx.spec.ts`)

Native LUNC/USTC and CW20 routes must exist after `deploy-dex-local.sh`; helpers in `e2e/helpers/wrap-e2e.ts` fail in strict mode when tokens or native-wrap pool cards are missing.

**Pool list pagination (#340):** `/pool` loads **20 pairs per page** from the indexer. Wrap-pool **tx** specs locate the seeded **LUNC-C** card via `e2e/helpers/pool-nav.ts` (indexer search by symbol or `VITE_LUNC_C_TOKEN_ADDRESS`, then paginate fallback) — not by assuming page 1 contains LUNC-C. UI smoke tests still use the first visible pair on the default list.

## Fee tier tx (`fee-tier-tx.spec.ts`)

Requires self-service **Register** buttons (tiers 1–9) and dev-wallet CL8Y ≥ tier-1 minimum (provision script).
