# Agent playbook: pre-deploy test evidence gate (SEC-H08)

Use when verifying **release checklist gates** that require contract, indexer, frontend, and LocalTerra smoke test output on the launch tracking issue before production mainnet deploy ([#444](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/444)).

## Problem

CI runs `test-contracts`, `test-indexer-integration`, and `test-frontend` on every default-branch commit and MR, but operators deploying **outside CI** (local wasm build) had no explicit runbook step requiring pasted test evidence on the release issue.

## Required evidence (Phase 0)

Before **production mainnet** deploy, record on the launch / release tracking issue:

| Suite | Command | Notes |
|-------|---------|-------|
| Contracts | `make test-contracts` | No chain required |
| Indexer integration | `make test-indexer-integration` | Requires Postgres — `make setup-indexer-postgres` or full LocalTerra stack |
| Frontend unit | `make test-frontend` | Node 24 on `PATH` |
| Pool swap smoke | `make smoke-pool-swap` | After deploy; also Phase 3 post-deploy step |

Include **git SHA** (`git rev-parse HEAD`) and **date UTC** with each paste or link.

## CI-built artifacts

When deploying wasm built by GitLab CI at commit `<git-sha>`, **link the pipeline URL** showing green `test-contracts`, `test-indexer-integration`, and `test-frontend` jobs instead of re-running and re-pasting locally.

## Canonical runbook

[`docs/runbooks/launch-checklist.md`](../docs/runbooks/launch-checklist.md) — **Phase 0** (test evidence gate) and **Phase 5** (P0 deploy/runbook category references SEC-H08).

Deploy trace template: [`docs/templates/deploy-trace.md`](../docs/templates/deploy-trace.md) § Test results.

## Regression

```bash
make verify-issue-444
# or doc-only:
make check-test-evidence-gate-docs
```

No LocalTerra or Postgres required for doc regression. Run the test commands themselves when producing evidence for a real release.

## Related

- Go/no-go gate: [`AGENTS_LAUNCH_GO_NO_GO.md`](./AGENTS_LAUNCH_GO_NO_GO.md) ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391))
- Post-deploy config: [`AGENTS_DEPLOY_CONFIG_VERIFY.md`](./AGENTS_DEPLOY_CONFIG_VERIFY.md) (SEC-H03, [#441](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/441))
- Test matrix: [`docs/testing.md`](../docs/testing.md)
- QA invariants Q3: [`docs/qa-invariants.md`](../docs/qa-invariants.md)
