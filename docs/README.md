# CL8Y DEX Documentation

## Production review bundle

The frozen **production review bundle** (executive summary, security review, release readiness, issue backlog) is under [`docs/reviews/20260409T030009Z/README.md`](./reviews/20260409T030009Z/README.md).

## Architecture & Design
- [Architecture Overview](./architecture.md) — system diagram, contract relationships, swap flow
- [Integrators](./integrators.md) — hybrid hooks (L7), limit-book fees, slippage semantics
- [Security Model](./security-model.md) — governance keys, treasury, hook safety
- [Contracts Security Audit & Invariants](./contracts-security-audit.md) — invariant matrix, attack paths, test mapping
- [Indexer Invariants & API Security](./indexer-invariants.md) — HTTP/indexing invariants, caps, caches, test mapping

## Smart Contracts
- [Contract Reference](./contracts-terraclassic.md) — Factory, Pair, Router message schemas. **LP CW20 shares** use **18** `decimals`; **`CreatePair` / empty-pool liquidity** rejects either asset CW20 with **`decimals > 18`** ([gitlab #124](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/124)).

## Frontend
- [Frontend Guide](./frontend.md) — tech stack, project structure, wallet integration, [Terra Classic swap gas limits](./frontend.md#terra-classic-gas-limits), [pool list: indexer vs factory](./frontend.md#liquidity-pools-list-indexer-vs-factory)

## Development
- [Local Development](./local-development.md) — Docker setup, deploy scripts, Makefile
- [Testing](./testing.md) — test philosophy, running unit/integration/E2E tests (includes indexer Postgres setup and [shared-DB parallelism](./testing.md#shared-postgres-and-test-parallelism))
- **Cursor:** the **Babysit PR** skill (in Cursor *Skills*) is useful for keeping topic branches merge-ready (CI, comments). Example frontend fix: [GitLab #113](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/113) / [price chart invariants](./frontend.md#trade-page--price-chart-invariants).
- **Agent players (3rd party):** [`skills/AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md) — Terra Classic swap gas tuning and doc crosslinks ([glab#115](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/115)); [`skills/AGENTS_LOCALNET_TRADING_SWARM.md`](../skills/AGENTS_LOCALNET_TRADING_SWARM.md) — localnet trading swarm ([glab#119](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/119), LP/bootstrap decimals [glab#124](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/124)).

## Deployment
- [Deployment Guide](./deployment-guide.md) — mainnet and testnet deployment steps

## Operations & runbooks
- [Environment matrix](./environment-matrix.md) — local, testnet, mainnet chain IDs and indexer expectations
- [Operator secrets](./operator-secrets.md) — DB, LCD, keys, `RUN_MODE`, optional Prometheus
- [Runbook: indexer reorg, replay, dedup, backfill](./runbooks/indexer-reorg-replay-dedup.md)
- [Runbook: Wasm admin migration](./runbooks/wasm-admin-migration.md)
- [Incident template (DEX + indexer)](./templates/incident-dex-indexer.md)

## QA
- [QA Onboarding](./qa-onboarding.md) — getting started with QA, test flows, device matrix
- [Bug Report Template](./qa-templates/frontend-bug.md)
- [Test Pass Template](./qa-templates/qa-test-pass.md)
