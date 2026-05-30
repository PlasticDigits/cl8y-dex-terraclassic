# Workflow files (reference only)

This repository is hosted on **GitLab** and does **not** run GitHub Actions or GitLab CI pipelines today.

The YAML under this directory is a **portable checklist**: job names, service containers, and command order for local verification and agents. Run the equivalent via Makefile targets and `scripts/` (see [docs/testing.md § CI](../docs/testing.md#ci)).

Example mappings:

| Workflow job | Local equivalent |
|--------------|------------------|
| `frontend` | `make test-frontend` (or `npm run test:run` in `frontend-dapp`) |
| `frontend-charts-vitest` | `make test-frontend-charts` — Node `canvas` OS deps; see [docs/testing.md § Real lightweight-charts](../docs/testing.md#real-lightweight-charts-in-vitest-gitlab-211) ([#230](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/230)) |
| `frontend-charts-integration` | `make test-charts-integration` |
| `frontend-e2e-indexer-outage` | `make test-e2e-indexer-outage` |
| `e2e` | LocalTerra + deploy + `scripts/e2e-start-indexer.sh` + `npm run test:e2e` |
