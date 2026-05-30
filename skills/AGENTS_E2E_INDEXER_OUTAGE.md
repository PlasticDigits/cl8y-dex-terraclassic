# Agent skill: Playwright indexer-outage E2E (GitLab #219)

## When to use

You are changing **market-data-down** Playwright specs (`*-indexer-outage.spec.ts`), CI job **`frontend-e2e-indexer-outage`**, or [`scripts/test-e2e-indexer-outage.sh`](../scripts/test-e2e-indexer-outage.sh).

## Invariants

| Rule | Why |
|------|-----|
| **Separate from strict `e2e` job** | Stopping the indexer must not flake [#201](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201) on-chain specs |
| **`E2E_INDEXER_OUTAGE=1`** | Specs `test.skip` without it — default `npm run test:e2e` unchanged; global setup waits for LCD only (no mint/seed) |
| **Sanity on `:3001`, browser on dead port** | Script checks `/api/v1/overview` on real indexer, stops it, then sets `VITE_INDEXER_URL` to `OUTAGE_E2E_INDEXER_URL` (default `:39991`) for Playwright — avoids false greens when QA auto-restarts `:3001` |
| **`E2E_TRADE_PAIR` from deploy** | Use [`scripts/lib/e2e-trade-pair-from-deploy.sh`](../scripts/lib/e2e-trade-pair-from-deploy.sh) — hardcoded bech32 drifts per `deploy-dex-local.sh` |
| **Playwright workers = 5** | Do not raise without stability review ([`.cursor/rules/playwright-workers.mdc`](../.cursor/rules/playwright-workers.mdc)) |
| **No `VITE_INDEXER_URL` in retail DOM** | Outage specs assert banner copy — see [`AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md`](./AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md) |

## Playwright project

- **`e2e-indexer-outage`** — `testMatch: **/*-indexer-outage.spec.ts`
- Excluded from **`e2e-smoke`** / default `npm run test:e2e`

## Repro

```bash
docker compose up -d localterra postgres   # if needed
make wait-localterra
bash scripts/deploy-dex-local.sh
cd indexer && cargo build --release
make test-e2e-indexer-outage
```

Or CI-equivalent:

```bash
bash scripts/test-e2e-indexer-outage.sh
```

## Cross-links

- [docs/testing.md § Frontend E2E — indexer outage](../docs/testing.md#frontend-e2e-indexer-outage)
- [docs/frontend.md § Market data loading & outage](../docs/frontend.md#market-data-loading-outage)
- Strict chain (do not merge jobs): [`AGENTS_E2E_STRICT_CHAIN.md`](./AGENTS_E2E_STRICT_CHAIN.md)
- Product copy / testids: [`AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md`](./AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md)
- Gap: [`gaps/GAP_1780023683.md`](../gaps/GAP_1780023683.md) §5.2
