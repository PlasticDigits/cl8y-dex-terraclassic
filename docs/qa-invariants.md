# QA stack invariants

Operator and agent reference for the **QA server** workflow (`scripts/qa/`, `make start-qa`). Fresh volumes: [GitLab **#202**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/202) — [`skills/AGENTS_QA_FRESH_VOLUMES.md`](../skills/AGENTS_QA_FRESH_VOLUMES.md). Deploy verification: [GitLab **#203**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203) — [`skills/AGENTS_QA_DEPLOY_VERIFY.md`](../skills/AGENTS_QA_DEPLOY_VERIFY.md). **Redeploy decision guide:** [GitLab **#325**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/325) — [`skills/AGENTS_QA_REDEPLOY_DECISION.md`](../skills/AGENTS_QA_REDEPLOY_DECISION.md).

**Test automation:** GitHub Actions are **reference spec only** (never executed). **GitLab CI** ([`.gitlab-ci.yml`](../.gitlab-ci.yml)) runs the `security` stage plus, since [#421](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/421), a `test` stage with Phase-1 functional gates (contracts + indexer-lib + frontend unit/lint + build) on default branch and change-gated MRs. The heavier rows (Postgres-backed indexer integration, Playwright E2E) still run locally via `make` / `scripts/` pending Phase 2; reference job names live in [docs/testing.md § CI](./testing.md#ci) ([#234](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/234)). Do not wait for “GitHub Actions green on `main`”.

## Deploy verification (invariant Q1)

| Invariant | Check | On failure |
| --------- | ----- | ---------- |
| **Q1** Deployed pair accepts current-schema LCD queries (`is_paused`, `expired_limit_refund`) and **`.qa-deploy-stamp`** `git_sha` matches **`HEAD`** | **`make qa-verify-deploy`** (runs inside **`make start-qa`** after **`deploy-local`**) | Non-zero exit; fix depends on probe result (see below) |

**Q1 failure modes** (GitLab **#203** — [`verify-deploy.sh`](../scripts/qa/verify-deploy.sh)):

| Probe result | Likely cause | Fix |
| ------------ | ------------ | --- |
| `unknown variant` on `is_paused` / `expired_limit_refund` | Stale on-chain wasm (reused **`localterra-data`** volume) | **`make reset-qa`** / **`QA_FRESH_VOLUMES=1 make start-qa`** ([#202](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/202)) |
| Schema probes **pass**, stamp `git_sha` ≠ **`HEAD`** | **`git pull`** without **`deploy-local`** | **`make deploy-local && make qa-verify-deploy`** — no volume wipe |

Unit checks (no Docker): **`make test-qa-verify-deploy`** → [`scripts/qa/test-verify-deploy.sh`](../scripts/qa/test-verify-deploy.sh).

Fresh wasm on disk does **not** guarantee fresh on-chain behaviour when volumes are reused ([#120](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120)).

## Post-deploy config verification (invariant Q2)

| Invariant | Check | On failure |
| --------- | ----- | ---------- |
| **Q2** Factory governance/treasury/default fee, whitelisted CW20 code IDs, fee-discount tiers, trusted router, pair hooks, and blacklist clean-wallet probe are queryable and pass assertions | **`make qa-verify-deploy-config`** → [`verify-deploy-config.sh`](../scripts/qa/verify-deploy-config.sh) | Non-zero exit; paste failing section on the release issue; fix on-chain config or redeploy |

Release sign-off: paste full script output on the launch tracking issue — [launch checklist Phase 3](runbooks/launch-checklist.md#phase-3--post-deploy-verification-pool-only) (SEC-H03, [GitLab **#441**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/441)). Agent playbook: [`skills/AGENTS_DEPLOY_CONFIG_VERIFY.md`](../skills/AGENTS_DEPLOY_CONFIG_VERIFY.md). Doc drift: **`make check-deploy-config-docs`**. Unit checks (no Docker): **`make test-qa-verify-deploy-config`**.

## Volume lifecycle

| Invariant | Default `make start-qa` | `make reset-qa` / `QA_FRESH_VOLUMES=1 make start-qa` |
| --------- | ----------------------- | ------------------------------------------------------ |
| Docker volumes `localterra-data`, `postgres-data` | **Preserved** (`docker compose down`) | **Removed** (`docker compose down -v`) |
| LocalTerra chain state | Reused from prior QA runs | Empty chain (re-init on next up) |
| Postgres `dex_indexer` data | Reused | Empty DB (init scripts on first up) |
| `make deploy-local` | Always runs; uploads/instantiates from **current tree** wasm | Same, on **fresh** chain/DB |
| `make qa-verify-deploy` | Runs after deploy; schema + stamp check (**Q1**) | Same |
| UX | Yellow SSH-tunnel reminder banner | **Red** fresh-volumes banner before teardown |

**Why it matters:** `deploy-local` redeploys wasm from the working tree but does **not** reset chain state. A reused LocalTerra volume can leave **old contract instances** at prior addresses while the indexer and frontend expect addresses from the latest deploy — invalidating contract walks and E2E ([#120](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120)). **`qa-verify-deploy`** fails loud when schema or stamp mismatch ([#203](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203)).

## When to use

| Scenario | Command |
| -------- | ------- |
| Fast restart, same contracts/genesis | `make start-qa` |
| After contract/genesis/indexer schema changes, or suspected stale chain | `make reset-qa` |
| Same as reset, explicit env | `QA_FRESH_VOLUMES=1 make start-qa` |
| Stop without wiping | `make stop-qa` |

Compose project name is fixed (`name: cl8y-dex-terraclassic` in `docker-compose.yml`) so git worktrees share volume names `cl8y-dex-terraclassic_localterra-data` and `cl8y-dex-terraclassic_postgres-data`.

**LocalTerra chain binary:** pinned **terrad v4.0.1 / SDK 0.53.6** — after upgrading the image digest ([GitLab **#292**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/292)), use **`make reset-qa`** / fresh volumes; see [`docs/localterra-sdk53.md`](./localterra-sdk53.md).

## LocalTerra host ports vs `docker exec`

| Consumer | RPC/LCD access |
| -------- | -------------- |
| Browser / `VITE_TERRA_*_URL` | Published host ports (`127.0.0.1:26657`, `:1317`) |
| `make qa-verify-deploy`, `make wait-localterra`, `scripts/status.sh`, LCD helpers | Host curl first, then **`docker exec`** into `localterra` ([`scripts/lib/localterra-host-curl.sh`](../scripts/lib/localterra-host-curl.sh)) |

Agents: **`make test-localterra-host-curl`** when compose is up; see [`skills/AGENTS_QA_DEPLOY_VERIFY.md`](../skills/AGENTS_QA_DEPLOY_VERIFY.md).

## Related docs

- [GitLab **#337**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/337) — master executable Local/QA verification checklist (Q1 maps to **INF-00-02** / **LR-00-01**)
- [`scripts/qa/README.md`](../scripts/qa/README.md) — server + laptop workflow
- [`docs/qa-onboarding.md`](./qa-onboarding.md) — human QA onboarding
- [`skills/AGENTS_QA_DEPLOY_VERIFY.md`](../skills/AGENTS_QA_DEPLOY_VERIFY.md) — post-deploy schema check ([#203](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203))
- [`skills/AGENTS_DEPLOY_CONFIG_VERIFY.md`](../skills/AGENTS_DEPLOY_CONFIG_VERIFY.md) — post-deploy config assertions ([#441](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/441))
- [`skills/AGENTS_LOCAL_POSTGRES_DEV.md`](../skills/AGENTS_LOCAL_POSTGRES_DEV.md) — dev `make reset` (non-QA compose)
