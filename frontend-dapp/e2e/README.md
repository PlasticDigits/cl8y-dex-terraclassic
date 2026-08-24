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

1. **`scripts/e2e-provision-dev-wallet.sh`** — gems/TCL8Y **Mint**; wrap skip; tax tokens **Transfer** (or skip leftover `test1→test1`; pinned token below floor fails — GitLab #620 / #622)
2. **`scripts/e2e-seed-hybrid-book.sh`** — resting bid on first dual-CW20 pair

## Strict vs optional chain

| Variable | Behavior |
|----------|----------|
| unset (default) | **Strict** — global setup required; tx helpers **fail** on missing preconditions |
| `PLAYWRIGHT_SKIP_CHAIN=1` | **Optional** — no global setup; helpers may `test.skip` (local UI dev only) |
| `REQUIRE_LOCALTERRA=0` | Legacy alias for `PLAYWRIGHT_SKIP_CHAIN=1` |
| `PLAYWRIGHT_WEB_PORT` | Dedicated Vite port (and `PLAYWRIGHT_BASE_URL`) so a worktree does not reuse another checkout on `:3000`. Indexer `CORS_ORIGINS` must include that Origin (default `http://127.0.0.1:3173`) or `/pool` catalog fetch fails ([#625](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/625)). `scripts/e2e-start-indexer.sh` merges it. |

**Do not** set `PLAYWRIGHT_SKIP_CHAIN=1` in CI.

### Minimum balances (raw CW20 units)

| Token / use | Env | Default |
|-------------|-----|---------|
| Factory pair CW20s | `E2E_DEV_MIN_CW20_U128` | `10000000000000` (\(10^7\) @ 6 decimals; GitLab #372) |
| CL8Y (fee tier Register) | `E2E_DEV_MIN_CL8Y_U128` | `1000000000000000000` (tier 1 min) |

Native **uluna** / **uusd** for gas come from LocalTerra genesis on the dev mnemonic (**11M LUNC** after GitLab #372; run `make reset` after genesis changes).

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
| `E2E_HYBRID_SEED_ASK_ESCROW` | `50000000` | Raw token0 on seeded **ask** (multihop CORAL→* hop 1; GitLab #422) |
| `E2E_HYBRID_SEED_ASK_PRICE` | `1` | Ask limit price |

Playbook: [`skills/AGENTS_E2E_HYBRID_SWAP.md`](../../skills/AGENTS_E2E_HYBRID_SWAP.md).

## Multihop hybrid router tx (`multihop-hybrid-tx.spec.ts`, GitLab #422)

Strict **≥2-hop** router swap with indexer hybrid quote (CORAL→IRON through seeded EMBER/CORAL **ask** book; pay ≥600 CORAL raw). Asserts `limit_order_fill`, return within slippage vs `simulate_swap_operations`, and attaches a success screenshot. Receive token defaults to IRON (liquid EMBER/IRON hop 2); COBALT fails hop-spread preflight on seed liquidity.

```bash
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/multihop-hybrid-tx.spec.ts --project=e2e-tx
```

Playbook: [`skills/AGENTS_TESTING_MULTIHOP_HYBRID.md`](../../skills/AGENTS_TESTING_MULTIHOP_HYBRID.md).

## Swap route display vs on-chain ops (`swap-route-alignment-tx.spec.ts`, SEC-E07 / GitLab #428)

Strict Playwright tx spec confirming the **Route** row on `/` matches wasm `swap` hop sequence after submit:

| Case | Route display | On-chain |
|------|---------------|----------|
| Direct dual-CW20 | 2 symbols, 1 arrow | 1 wasm `swap` (`offer_asset` → `ask_asset`) |
| Multihop CORAL→IRON | ≥3 symbols, ≥2 arrows | ≥2 wasm `swap` hops in order |

Helpers: `e2e/helpers/route-alignment-e2e.ts`, `txJsonWasmSwapHops` in `e2e/helpers/lcd.ts`. Playbook: [`skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](../../skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md).

```bash
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/swap-route-alignment-tx.spec.ts --project=e2e-tx
```

## Trader / Protocol / blacklist smoke (#422)

| Spec | Project | Notes |
|------|---------|-------|
| `trader-page.spec.ts` | `e2e-smoke` | Indexer positions section for connected dev wallet |
| `protocol-page.spec.ts` | `e2e-smoke` | Factory + router audit addresses |
| `blacklist-swap.spec.ts` | `e2e-smoke` | LCD mock via `helpers/blacklist-lcd-mock.ts` — disabled **Trading restricted** CTA |

Indexer required for `trader-page.spec.ts` (`bash scripts/e2e-start-indexer.sh`).

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

**Wrap safety CTA (SEC-A02 / GitLab #389):** `wrap-swap.spec.ts` asserts exact submit copy and `disabled` for wrap-mapper **pause** and **rate limit** in isolated tests (LCD route mocks in `e2e/helpers/wrap-mapper-lcd-mock.ts`; `wrapMapperAddressFromEnv` reads `frontend-dapp/.env.local`). Rate-limit path also asserts inline `swap-wrap-rate-limit-banner` with retry guidance (SEC-I05 F-04 / GitLab #463). Vitest parity: `SwapPage.test.tsx` `SEC-A02` describe. Run: `--project=e2e-tx` (spec is in `txSpecGlobs`, not `e2e-smoke`). Playbook: [`skills/AGENTS_FRONTEND_SWAP_SAFETY_CTA.md`](../../skills/AGENTS_FRONTEND_SWAP_SAFETY_CTA.md).

**Pool list pagination (#340):** `/pool` loads **20 pairs per page** from the indexer. Wrap-pool **tx** specs locate the seeded **cLUNC** card via `e2e/helpers/pool-nav.ts` (indexer search by symbol or `VITE_LUNC_C_TOKEN_ADDRESS`, then paginate fallback) — not by assuming page 1 contains cLUNC. UI smoke tests still use the first visible pair on the default list.

## Community-tax pair tx (`community-tax-tx.spec.ts`, GitLab #622)

Strict Playwright on the LocalTerra **QATax / EMBER** seed pair (`VITE_PAIR_COMMUNITY_TAX_EMBER` from `.env.local`). Missing tax pins **fail** — no `test.skip`. Gem specs stay on the first dual-CW20 pair (EMBER/CORAL).

| Case | Assert |
|------|--------|
| Sell QTAX → EMBER | Max is extra-debit; small sell: user debit == `TaxPreview.debit`; pair credit == Send amount |
| Buy EMBER → QTAX | You Receive is **net** (#615); user credit + sink == pair debit |
| Provide / withdraw | `TransferFrom` pair delta == declared |
| Limit place / cancel | Place Send honest (`declared - maker_fee`); cancel pair debit = remaining; refund is buy-net unless directory skip |
| Trade Market (P1) | Default `GET /route/solve`; option-2 copy |

Helpers: `e2e/helpers/community-tax-e2e.ts`, `e2e/helpers/community-tax-env.ts` (tx reads `.env.local`; smoke `/token/create` may bake columbus-5). Provision never Mints tax and never Transfers `test1→test1` (leftover launcher-origin tokens are skipped). Playbook: [`skills/AGENTS_E2E_COMMUNITY_TAX_TX.md`](../../skills/AGENTS_E2E_COMMUNITY_TAX_TX.md) (**E622-1–E622-8**).

```bash
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/community-tax-tx.spec.ts --project=e2e-tx
```

## Fee tier tx (`fee-tier-tx.spec.ts`)

Requires self-service **Register** buttons (tiers 1–9) and dev-wallet CL8Y ≥ tier-1 minimum (provision script).
