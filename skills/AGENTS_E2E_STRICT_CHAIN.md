# Agent skill: Strict on-chain Playwright E2E (GitLab #201 / #103)

## When to use

You are changing **CI Playwright wiring**, **`e2e/helpers/chain.ts`**, **global setup**, or **tx specs** (`*-tx.spec.ts`, `hybrid-swap`, `wrap-*`) that must not silently pass when LocalTerra, funds, routes, or pairs are missing.

## Policy (invariants)

| Mode | Env | Global setup | Tx spec on missing LCD/funds/pair |
|------|-----|--------------|-----------------------------------|
| **Strict (default CI/local)** | unset | `e2e/global-setup.ts` runs provision + hybrid seed | **`expect` / throw** — job fails |
| **UI-only local** | `PLAYWRIGHT_SKIP_CHAIN=1` or legacy `REQUIRE_LOCALTERRA=0` | skipped | documented `test.skip` in helpers only |

**Never** use `test.skip()` in tx spec bodies for funds/pair/pause/route on the default path — use helpers in `chain.ts`, `hybrid-e2e.ts`, `limit-e2e.ts`, `wrap-e2e.ts`, `fee-e2e.ts`.

## Playwright projects

| Project | Files | Purpose |
|---------|-------|---------|
| `e2e-smoke` | All except tx globs | UI/navigation without mandatory chain |
| `e2e-tx` | `*-tx.spec.ts`, `hybrid-swap`, `wrap-pool`, `wrap-swap` | On-chain paths; strict by default |
| `e2e-indexer-outage` | `*-indexer-outage.spec.ts` | Market-data-down; **separate CI job** — not part of default `npm run test:e2e` ([#219](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/219)) |
| `e2e-smoke` (strict) | `price-chart-smoke.spec.ts` | Canvas mount on `/charts` + `/trade`, fullscreen aria ([#228](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/228)); skipped when `PLAYWRIGHT_SKIP_CHAIN=1` |

## One-command repro

Node **24** via **nvm** (`.nvmrc`); local commands use [`scripts/with-node.sh`](../scripts/with-node.sh) — not bare system `npm`.

```bash
make test-e2e-tx
```

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
| [`frontend-dapp/playwright.config.ts`](../frontend-dapp/playwright.config.ts) | `e2e-smoke` / `e2e-tx` / `e2e-indexer-outage` projects, optional global setup |
| [`scripts/test-e2e-indexer-outage.sh`](../scripts/test-e2e-indexer-outage.sh) | Local/CI indexer stop + outage Playwright ([#219](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/219)) |
| [`frontend-dapp/e2e/helpers/chain.ts`](../frontend-dapp/e2e/helpers/chain.ts) | `isChainOptional()`, LCD + CTA assertions |
| [`frontend-dapp/e2e/global-setup.ts`](../frontend-dapp/e2e/global-setup.ts) | LCD wait + provision + hybrid book seed |
| [`scripts/e2e-provision-dev-wallet.sh`](../scripts/e2e-provision-dev-wallet.sh) | CW20 mint floor; CL8Y ≥ tier-1 via `E2E_DEV_MIN_CL8Y_U128` |
| [`scripts/with-node.sh`](../scripts/with-node.sh) | Local Node/npm via nvm (`.nvmrc`) |
| [`.github/workflows/test.yml`](../.github/workflows/test.yml) | LocalTerra + `make wait-localterra` + deploy + Playwright (CI uses `setup-node`) |

## Cross-links

- Umbrella issue: [GitLab **#201**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201)
- Policy parent: [GitLab **#103**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/103)
- Hybrid child: [`AGENTS_E2E_HYBRID_SWAP.md`](./AGENTS_E2E_HYBRID_SWAP.md) ([#193](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/193))
- Limit child: [`AGENTS_E2E_LIMIT_ORDERS_TX.md`](./AGENTS_E2E_LIMIT_ORDERS_TX.md) ([#195](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/195))
- Operator runbook: [`frontend-dapp/e2e/README.md`](../frontend-dapp/e2e/README.md)
- QA deploy verification (server): [`AGENTS_QA_DEPLOY_VERIFY.md`](./AGENTS_QA_DEPLOY_VERIFY.md) ([#203](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203))
- Postgres / indexer env: [`AGENTS_LOCAL_POSTGRES_DEV.md`](./AGENTS_LOCAL_POSTGRES_DEV.md)
- Testing matrix: [`docs/testing.md`](../docs/testing.md) § E2E Tests
- Indexer outage E2E (separate job): [`docs/testing.md`](../docs/testing.md#frontend-e2e-indexer-outage), [`AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md`](./AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md) ([#219](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/219))
- Price chart browser smoke: [`docs/testing.md`](../docs/testing.md#price-chart-playwright-smoke-gitlab-228), [`AGENTS_FRONTEND_PRICE_CHART.md`](./AGENTS_FRONTEND_PRICE_CHART.md) ([#228](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/228))
