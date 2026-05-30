# Workflow files (reference only)

This repository is hosted on **GitLab** and does **not** run GitHub Actions or GitLab CI pipelines today.

The YAML under this directory is a **portable checklist**: job names, service containers, and command order for local verification and agents. Run the equivalent via Makefile targets and `scripts/` (see [docs/testing.md § CI](../docs/testing.md#ci)).

**Invariants (GitLab [#234](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/234)):**

- **Execution path:** local / QA host — `make …`, `scripts/*.sh` — not a hosted runner.
- **Job names** (`e2e`, `frontend-charts-integration`, …) are **labels** for automation targets; prefer those names in docs when mapping to Make/scripts.
- **Wasm:** fast `cargo wasm` in `test.yml` is for dev checks only; **mainnet uploads** use `make build-optimized` (workspace-optimizer). See [docs/deployment-guide.md § Build Optimized WASM](../docs/deployment-guide.md#1-build-optimized-wasm).
- **Agents:** do not instruct third parties to wait for "GitHub Actions on `main`"; say **local checklist passed** or cite the Make target.

## `test.yml` — reference job → local command

| Reference job | Local command |
|---------------|---------------|
| `docs-fee-discount-tiers` | `make check-fee-discount-tier-docs` |
| `contracts-terra` | `make lint-contracts` && `make test-contracts` (optional LCOV: `make coverage-contracts`) |
| `localnet-trading-swarm` | `cd packages/localnet-trading-swarm && npm ci && npx tsc -p tsconfig.json && npm run test:run` |
| `frontend` | `bash scripts/with-node.sh --cwd frontend-dapp -- npx tsc --noEmit` && `make lint-frontend` && `make test-frontend` |
| `frontend-charts-vitest` | `make test-frontend-charts` |
| `frontend-charts-integration` | `make test-charts-integration` |
| `indexer` | Postgres up → `cd indexer && cargo fmt --check && cargo clippy -- -D warnings && cargo test` with `TEST_DATABASE_URL` (see [docs/testing.md § Indexer](../docs/testing.md#indexer-rust)) |
| `e2e` | LocalTerra + Postgres → `make wait-localterra` → `bash scripts/deploy-dex-local.sh` → `make qa-verify-deploy` → `bash scripts/e2e-start-indexer.sh` → `make test-e2e` |
| `frontend-e2e-indexer-outage` | `make test-e2e-indexer-outage` |

## `contracts-wasm-optimizer.yml`

| Reference job | Local command |
|---------------|---------------|
| `optimizer` | `make build-optimized` (artifacts under `smartcontracts/artifacts/`, checksums from `optimize.sh`) |

Agent playbooks: [`skills/AGENTS_E2E_STRICT_CHAIN.md`](../skills/AGENTS_E2E_STRICT_CHAIN.md), [`skills/AGENTS_E2E_INDEXER_OUTAGE.md`](../skills/AGENTS_E2E_INDEXER_OUTAGE.md), [`skills/AGENTS_LOCAL_POSTGRES_DEV.md`](../skills/AGENTS_LOCAL_POSTGRES_DEV.md).
