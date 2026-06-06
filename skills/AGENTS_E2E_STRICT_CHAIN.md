# Agent skill: Strict on-chain Playwright E2E (GitLab #201 / #103)

## When to use

You are changing **strict Playwright automation wiring**, **`e2e/helpers/chain.ts`**, **global setup**, or **tx specs** (`*-tx.spec.ts`, `hybrid-swap`, `wrap-*`) that must not silently pass when LocalTerra, funds, routes, or pairs are missing.

## Policy (invariants)

| Mode | Env | Global setup | Tx spec on missing LCD/funds/pair |
|------|-----|--------------|-----------------------------------|
| **Strict (default automation/local)** | unset | `e2e/global-setup.ts` runs provision + hybrid seed | **`expect` / throw** — run fails |
| **UI-only local** | `PLAYWRIGHT_SKIP_CHAIN=1` or legacy `REQUIRE_LOCALTERRA=0` | skipped | documented `test.skip` in helpers only |

**Never** use `test.skip()` in tx spec bodies for funds/pair/pause/route on the default path — use helpers in `chain.ts`, `hybrid-e2e.ts`, `limit-e2e.ts`, `wrap-e2e.ts`, `fee-e2e.ts`, `pool-ui.ts`.

**On-chain worker count:** `e2e-tx` runs with **1 Playwright worker** (config detects `--project=e2e-tx`). All tx specs share the LocalTerra test mnemonic; parallel workers cause `account sequence mismatch`. `npm run test:e2e` runs `e2e-smoke` (5 workers) then `e2e-tx` (1 worker) sequentially.

## Playwright projects

| Project | Files | Purpose |
|---------|-------|---------|
| `e2e-smoke` | All except tx globs | UI/navigation without mandatory chain |
| `e2e-tx` | `*-tx.spec.ts`, `hybrid-swap`, `wrap-pool`, `wrap-swap` | On-chain paths; strict by default; **1 worker** (shared dev account) |
| `e2e-indexer-outage` | `*-indexer-outage.spec.ts` | Market-data-down; **separate reference job** `frontend-e2e-indexer-outage` → `make test-e2e-indexer-outage` — not part of default `npm run test:e2e` ([#219](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/219)) |
| `e2e-smoke` (strict) | `price-chart-smoke.spec.ts` | Canvas mount on `/charts` + `/trade`, fullscreen aria ([#228](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/228)); skipped when `PLAYWRIGHT_SKIP_CHAIN=1` |

## One-command repro

Node **24** via **nvm** (`.nvmrc`); local commands use [`scripts/with-node.sh`](../scripts/with-node.sh) — not bare system `npm`.

```bash
make deploy-local
bash scripts/e2e-start-indexer.sh
make test-e2e
```

**CI reference job `e2e`** uses the order above (`deploy` → `e2e-start-indexer` → `make test-e2e`). **`make test-e2e-tx`** runs `deploy-dex-local.sh` again without restarting the indexer — only use it on a fresh chain or after `make reset-qa` ([#292](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/292) **LT11**).

**Cloud Agent VM:** run `make test-e2e` (or the three-step order above) inside `sg docker -c '…'` (docker group) — global setup calls deploy/e2e shell scripts that need `docker exec` when host `:1317` hangs. Build indexer release before `e2e-start-indexer.sh` on a fresh VM: `(cd indexer && cargo build --release)`. Install browsers once via the **locked** Playwright version (**LT12** in [`docs/localterra-sdk53.md`](../docs/localterra-sdk53.md)): `bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright install chromium` — bare `npx playwright install` can fetch the wrong Chromium revision. Use `CI=1` to avoid the HTML report server hanging on exit. Tx project stays at **1 worker** (not 5) — shared dev account sequence ([#201](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201)).

Or manually:

```bash
docker compose up -d localterra
make wait-localterra
bash scripts/deploy-dex-local.sh
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:e2e:tx
```

UI-only (no chain):

```bash
bash scripts/with-node.sh --cwd frontend-dapp -- env PLAYWRIGHT_SKIP_CHAIN=1 npm run test:e2e:smoke
```

## Files

| Path | Role |
|------|------|
| [`frontend-dapp/e2e/helpers/pool-ui.ts`](../frontend-dapp/e2e/helpers/pool-ui.ts) | Pool card expand vs submit locators (GitLab #201 strict mode) |
| [`frontend-dapp/playwright.config.ts`](../frontend-dapp/playwright.config.ts) | `e2e-smoke` / `e2e-tx` / `e2e-indexer-outage` projects, optional global setup, **1 worker for `e2e-tx`** |
| [`scripts/test-e2e-indexer-outage.sh`](../scripts/test-e2e-indexer-outage.sh) | Local indexer stop + outage Playwright ([#219](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/219)) |
| [`frontend-dapp/e2e/helpers/chain.ts`](../frontend-dapp/e2e/helpers/chain.ts) | `isChainOptional()`, LCD + CTA assertions |
| [`frontend-dapp/e2e/global-setup.ts`](../frontend-dapp/e2e/global-setup.ts) | LCD wait + provision + hybrid book seed |
| [`scripts/e2e-provision-dev-wallet.sh`](../scripts/e2e-provision-dev-wallet.sh) | CW20 mint floor; CL8Y ≥ tier-1 via `E2E_DEV_MIN_CL8Y_U128` |
| [`scripts/with-node.sh`](../scripts/with-node.sh) | Local Node/npm via nvm (`.nvmrc`) |
| [`.github/workflows/test.yml`](../.github/workflows/test.yml) | **Reference only** — step order for `e2e`; run via `make test-e2e` / [docs/testing.md § CI](../docs/testing.md#ci) ([#234](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/234)) |

## Cross-links

- LocalTerra **SDK 0.53 / terrad v4** stack: [`docs/localterra-sdk53.md`](../docs/localterra-sdk53.md) ([GitLab **#292**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/292)) — **`make reset`** after image digest bump before strict E2E; E2E global setup uses **docker exec LCD fallback** when host `:1317` hangs (**LT9**)
- Umbrella issue: [GitLab **#201**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201)
- Policy parent: [GitLab **#103**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/103)
- Hybrid child: [`AGENTS_E2E_HYBRID_SWAP.md`](./AGENTS_E2E_HYBRID_SWAP.md) ([#193](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/193))
- Limit child: [`AGENTS_E2E_LIMIT_ORDERS_TX.md`](./AGENTS_E2E_LIMIT_ORDERS_TX.md) ([#195](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/195))
- Operator runbook: [`frontend-dapp/e2e/README.md`](../frontend-dapp/e2e/README.md)
- QA deploy verification (server): [`AGENTS_QA_DEPLOY_VERIFY.md`](./AGENTS_QA_DEPLOY_VERIFY.md) ([#203](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203))
- Postgres / indexer env: [`AGENTS_LOCAL_POSTGRES_DEV.md`](./AGENTS_LOCAL_POSTGRES_DEV.md)
- Testing matrix: [`docs/testing.md`](../docs/testing.md) § E2E Tests
- Indexer outage E2E (separate automation target): [`docs/testing.md`](../docs/testing.md#frontend-e2e-indexer-outage), [`AGENTS_E2E_INDEXER_OUTAGE.md`](./AGENTS_E2E_INDEXER_OUTAGE.md), [`AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md`](./AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md) ([#219](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/219))
- Price chart browser smoke: [`docs/testing.md`](../docs/testing.md#price-chart-playwright-smoke-gitlab-228), [`AGENTS_FRONTEND_PRICE_CHART.md`](./AGENTS_FRONTEND_PRICE_CHART.md) ([#228](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/228))
