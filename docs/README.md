# CL8Y DEX Documentation

## Architecture & Design
- [Architecture Overview](./architecture.md) — system diagram, contract relationships, swap flow
- [Security Model](./security-model.md) — governance keys, treasury, hook safety
- [Contracts Security Audit & Invariants](./contracts-security-audit.md) — invariant matrix, attack paths, test mapping
- [Indexer Invariants & API Security](./indexer-invariants.md) — HTTP/indexing invariants, caps, caches, test mapping

## Smart Contracts
- [Contract Reference](./contracts-terraclassic.md) — Factory, Pair, Router message schemas

## Frontend
- [Frontend Guide](./frontend.md) — tech stack, project structure, wallet integration

## Development
- [Local Development](./local-development.md) — Docker setup, deploy scripts, Makefile
- [Testing](./testing.md) — test philosophy, running unit/integration/E2E tests (includes indexer Postgres setup and [shared-DB parallelism](./testing.md#shared-postgres-and-test-parallelism))

## Deployment
- [Deployment Guide](./deployment-guide.md) — mainnet and testnet deployment steps

## QA
- [QA Onboarding](./qa-onboarding.md) — getting started with QA, test flows, device matrix
- [Bug Report Template](./qa-templates/frontend-bug.md)
- [Test Pass Template](./qa-templates/qa-test-pass.md)
