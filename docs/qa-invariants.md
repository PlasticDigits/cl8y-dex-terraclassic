# QA stack invariants

Operator and agent reference for the **QA server** workflow (`scripts/qa/`, `make start-qa`). Implementation: [GitLab **#202**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/202). Agent playbook: [`skills/AGENTS_QA_FRESH_VOLUMES.md`](../skills/AGENTS_QA_FRESH_VOLUMES.md).

## Volume lifecycle

| Invariant | Default `make start-qa` | `make reset-qa` / `QA_FRESH_VOLUMES=1 make start-qa` |
| --------- | ----------------------- | ------------------------------------------------------ |
| Docker volumes `localterra-data`, `postgres-data` | **Preserved** (`docker compose down`) | **Removed** (`docker compose down -v`) |
| LocalTerra chain state | Reused from prior QA runs | Empty chain (re-init on next up) |
| Postgres `dex_indexer` data | Reused | Empty DB (init scripts on first up) |
| `make deploy-local` | Always runs; uploads/instantiates from **current tree** wasm | Same, on **fresh** chain/DB |
| UX | Yellow SSH-tunnel reminder banner | **Red** fresh-volumes banner before teardown |

**Why it matters:** `deploy-local` redeploys wasm from the working tree but does **not** reset chain state. A reused LocalTerra volume can leave **old contract instances** at prior addresses while the indexer and frontend expect addresses from the latest deploy — invalidating contract walks and E2E ([#120](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120)). Detection/warnings: [#203](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203).

## When to use

| Scenario | Command |
| -------- | ------- |
| Fast restart, same contracts/genesis | `make start-qa` |
| After contract/genesis/indexer schema changes, or suspected stale chain | `make reset-qa` |
| Same as reset, explicit env | `QA_FRESH_VOLUMES=1 make start-qa` |
| Stop without wiping | `make stop-qa` |

Compose project name is fixed (`name: cl8y-dex-terraclassic` in `docker-compose.yml`) so git worktrees share volume names `cl8y-dex-terraclassic_localterra-data` and `cl8y-dex-terraclassic_postgres-data`.

## Related docs

- [`scripts/qa/README.md`](../scripts/qa/README.md) — server + laptop workflow
- [`docs/qa-onboarding.md`](./qa-onboarding.md) — human QA onboarding
- [`skills/AGENTS_LOCAL_POSTGRES_DEV.md`](../skills/AGENTS_LOCAL_POSTGRES_DEV.md) — dev `make reset` (non-QA compose)
