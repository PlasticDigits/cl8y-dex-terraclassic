# Workflow files (reference only)

This repository is hosted on **GitLab**. **GitLab CI** runs supply-chain and QA artifact jobs — see [`.gitlab-ci.yml`](../.gitlab-ci.yml) and [docs/supply-chain-security.md](../docs/supply-chain-security.md). The YAML under this directory is a **portable checklist** for the broader test matrix: job names, service containers, and command order for local verification and agents.

**Invariants (GitLab [#234](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/234)):**

- **Execution path:** GitLab CI for security + QA artifacts; full matrix still **local / QA host** — `make …`, `scripts/*.sh`.
- **Job names** (`e2e`, `frontend-charts-integration`, …) are **labels** for automation targets; prefer those names in docs when mapping to Make/scripts.
- **Wasm:** fast `cargo wasm` in `test.yml` is for dev checks only; **mainnet uploads** use `make build-optimized` (workspace-optimizer). See [docs/deployment-guide.md § Build Optimized WASM](../docs/deployment-guide.md#1-build-optimized-wasm).
- **Agents:** default-branch merges expect a **green GitLab pipeline** (gitleaks + audits). For contracts/frontend/indexer/E2E not in `.gitlab-ci.yml`, say **local checklist passed** or cite the Make target.

## `test.yml` — reference job → local command

| Reference job | Local command |
|---------------|---------------|
| `docs-fee-discount-tiers` | `make check-fee-discount-tier-docs` |
| `contracts-terra` | `make lint-contracts` && `make test-contracts` (optional LCOV: `make coverage-contracts`) |
| `localnet-trading-swarm` | `cd packages/localnet-trading-swarm && npm ci && npx tsc -p tsconfig.json && npm run test:run` |
| `frontend` | `bash scripts/with-node.sh --cwd frontend-dapp -- npx tsc --noEmit` && `make lint-frontend` && `make test-frontend` |
| `frontend-charts-vitest` | `make test-frontend-charts` — Node `canvas` OS deps; see [docs/testing.md § Real lightweight-charts](../docs/testing.md#real-lightweight-charts-in-vitest-gitlab-211) ([#230](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/230)) |
| `frontend-charts-integration` | `make test-charts-integration` |
| `indexer` | Postgres up → `cd indexer && cargo fmt --check && cargo clippy -- -D warnings && cargo test` with `TEST_DATABASE_URL` (see [docs/testing.md § Indexer](../docs/testing.md#indexer-rust)) |
| `e2e` | LocalTerra + Postgres → `make wait-localterra` → `bash scripts/deploy-dex-local.sh` → `make qa-verify-deploy` → `bash scripts/e2e-start-indexer.sh` → `make test-e2e` |
| `frontend-e2e-indexer-outage` | `make test-e2e-indexer-outage` |

## `contracts-wasm-optimizer.yml`

| Reference job | Local command |
|---------------|---------------|
| `optimizer` | `make build-optimized` (artifacts under `smartcontracts/artifacts/`, checksums from `optimize.sh`) |

Agent playbooks: [`skills/AGENTS_E2E_STRICT_CHAIN.md`](../skills/AGENTS_E2E_STRICT_CHAIN.md), [`skills/AGENTS_E2E_INDEXER_OUTAGE.md`](../skills/AGENTS_E2E_INDEXER_OUTAGE.md), [`skills/AGENTS_LOCAL_POSTGRES_DEV.md`](../skills/AGENTS_LOCAL_POSTGRES_DEV.md).
