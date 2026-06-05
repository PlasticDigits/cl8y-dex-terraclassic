# CL8Y DEX Documentation

## Production review bundle

The frozen **production review bundle** (executive summary, security review, release readiness, issue backlog) is under [`docs/reviews/20260409T030009Z/README.md`](./reviews/20260409T030009Z/README.md).

## Architecture & Design
- [Architecture Overview](./architecture.md) — system diagram, contract relationships, swap flow
- [Integrators](./integrators.md) — hybrid hooks (L7), limit-book fees, [on-chain book HTTP](./integrators.md#on-chain-limit-book-lcd-proxy)
- [Integrators — hybrid volume reconciliation](./integrators-hybrid-volume.md) — headline vs leg vs fill volumes, CG/CMC mapping ([GitLab #216](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/216)); agent playbook [`skills/AGENTS_INTEGRATOR_HYBRID_VOLUME.md`](../skills/AGENTS_INTEGRATOR_HYBRID_VOLUME.md)
- [Security Model](./security-model.md) — governance keys, treasury, hook safety
- [Contracts Security Audit & Invariants](./contracts-security-audit.md) — invariant matrix, attack paths, test mapping
- [Indexer Invariants & API Security](./indexer-invariants.md) — HTTP/indexing invariants, caps, caches, test mapping
- [Route solver guide](./route-solver.md) — global best-execution pipeline, glossary, `optimality_scope`, optimization theory ([GitLab #310](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/310))

## Smart Contracts
- [Contract Reference](./contracts-terraclassic.md) — Factory, Pair, Router message schemas. **LP CW20 shares** use **18** `decimals`; **`CreatePair` / empty-pool liquidity** rejects either asset CW20 with **`decimals > 18`** ([gitlab #124](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/124)).

## Frontend
- [Frontend Guide](./frontend.md) — tech stack, project structure, wallet integration, [Terra Classic swap gas limits](./frontend.md#terra-classic-gas-limits), [pool list: indexer vs factory](./frontend.md#liquidity-pools-list-indexer-vs-factory)

## Development
- [Local Development](./local-development.md) — Docker setup, deploy scripts, Makefile
- [Testing](./testing.md) — test philosophy, running unit/integration/E2E tests; **[§ CI — local automation only](./testing.md#ci)** (reference [`.github/workflows/README.md`](../.github/workflows/README.md), GitLab [#234](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/234)); indexer Postgres: [shared-DB parallelism](./testing.md#shared-postgres-and-test-parallelism); agent [`skills/AGENTS_LOCAL_POSTGRES_DEV.md`](../skills/AGENTS_LOCAL_POSTGRES_DEV.md)
- **Cursor:** the **Babysit PR** skill (in Cursor *Skills*) is useful for keeping topic branches merge-ready (local checklists, comments). Example frontend fix: [GitLab #113](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/113) / [price chart invariants](./frontend.md#trade-page--price-chart-invariants).
- **Agent players (3rd party):** [`skills/AGENTS_INTEGRATOR_HYBRID_VOLUME.md`](../skills/AGENTS_INTEGRATOR_HYBRID_VOLUME.md) — hybrid volume reconciliation for CG/CMC/Vyntrex ([#216](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/216)); [`skills/AGENTS_LOCAL_POSTGRES_DEV.md`](../skills/AGENTS_LOCAL_POSTGRES_DEV.md) — local Postgres for indexer integration tests (`cl8y_legal`, `make reset`, `setup-postgres-dev-databases.sh`); [`skills/AGENTS_FRONTEND_DEEP_ORDER_BOOK.md`](../skills/AGENTS_FRONTEND_DEEP_ORDER_BOOK.md) — paginated limit book on `/trade` and `/limits` ([GitLab #194](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/194)); [`skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md`](../skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md) — batch / ladder limit placement ([GitLab #206](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/206), escrow sum [#233](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/233)); [`skills/AGENTS_FRONTEND_COPY_BUTTON.md`](../skills/AGENTS_FRONTEND_COPY_BUTTON.md) — clipboard `CopyButton` primitive ([GitLab #183](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/183)); [`skills/AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md) — Terra Classic swap gas tuning and doc crosslinks ([GitLab #115](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/115): dApp `SWAP_GAS_BUFFER` + [`packages/localnet-trading-swarm/src/gas.ts`](../packages/localnet-trading-swarm/src/gas.ts) stay aligned; factory discount-registry batch [GitLab #123](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/123)); [`skills/AGENTS_LOCALNET_TRADING_SWARM.md`](../skills/AGENTS_LOCALNET_TRADING_SWARM.md) — localnet trading swarm ([GitLab #119](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/119), LP/bootstrap decimals [GitLab #124](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/124)); [`skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](../skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md) — maker UX for indexer **`parked_expired`** + **`ClaimExpiredLimitOrder`** ([GitLab #141](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/141); pair **pause blocks claim** per [GitLab #120](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120) / [`contracts-security-audit.md`](./contracts-security-audit.md) **L6**); [`skills/AGENTS_FRONTEND_USER_ERRORS.md`](../skills/AGENTS_FRONTEND_USER_ERRORS.md) — retail error humanization funnel ([GitLab #145](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/145)); indexer observability (tracing-only, no `/metrics`): [GitLab #200](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/200) / [`indexer-invariants.md`](./indexer-invariants.md).

## Deployment
- [Deployment Guide](./deployment-guide.md) — mainnet and testnet deployment steps

## Operations & runbooks
- [Environment matrix](./environment-matrix.md) — local, testnet, mainnet chain IDs and indexer expectations
- [Operator secrets](./operator-secrets.md) — DB, LCD, keys, `RUN_MODE`; tracing-only observability ([glab#200](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/200))
- [Runbook: indexer reorg, replay, dedup, backfill](./runbooks/indexer-reorg-replay-dedup.md)
- [Runbook: Wasm admin migration](./runbooks/wasm-admin-migration.md)
- [Incident template (DEX + indexer)](./templates/incident-dex-indexer.md)

## QA
- [QA stack invariants](./qa-invariants.md) — `make start-qa` vs `reset-qa` / `QA_FRESH_VOLUMES` ([GitLab #202](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/202)); agent playbook [`skills/AGENTS_QA_FRESH_VOLUMES.md`](../skills/AGENTS_QA_FRESH_VOLUMES.md)
- [QA redeploy decision guide](../skills/AGENTS_QA_REDEPLOY_DECISION.md) — when to `reset-qa` vs `deploy-local` vs no redeploy ([GitLab #325](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/325))
- [QA Onboarding](./qa-onboarding.md) — getting started with QA, test flows, device matrix
- [Bug Report Template](./qa-templates/frontend-bug.md)
- [Test Pass Template](./qa-templates/qa-test-pass.md)
