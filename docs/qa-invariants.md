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

## Pre-deploy test evidence (invariant Q3)

| Invariant | Check | On failure |
| --------- | ----- | ---------- |
| **Q3** | Before production mainnet deploy, contract, indexer integration, frontend, and pool smoke test output is pasted or linked on the release issue at the deployed commit SHA — or a CI pipeline link shows green `test-contracts`, `test-indexer-integration`, and `test-frontend` for that SHA | Do not proceed to mainnet Phase 5 **GO**; run suites and paste output per [launch checklist Phase 0](runbooks/launch-checklist.md#phase-0--preconditions) (SEC-H08, [GitLab **#444**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/444)) |

| Suite | Command |
| ----- | ------- |
| Contracts | **`make test-contracts`** |
| Indexer integration | **`make test-indexer-integration`** (Postgres required) |
| Frontend unit | **`make test-frontend`** |
| Pool swap smoke | **`make smoke-pool-swap`** (after deploy; Phase 3) |

Agent playbook: [`skills/AGENTS_TEST_EVIDENCE_GATE.md`](../skills/AGENTS_TEST_EVIDENCE_GATE.md). Doc drift: **`make check-test-evidence-gate-docs`**. Issue acceptance: **`make verify-issue-444`**.

## Env/chain address cross-check (invariant Q4)

| Invariant | Check | On failure |
| --------- | ----- | ---------- |
| **Q4** Indexer and frontend env contract addresses match each other; router on-chain `config.factory` equals env `FACTORY_ADDRESS`; factory and fee-discount `config` respond at configured addresses | **`make qa-verify-env-addresses`** → [`verify-env-addresses.sh`](../scripts/qa/verify-env-addresses.sh) (also inside **`make qa-verify-deploy`** after Q1) | Non-zero exit; fix env drift or redeploy; paste failing section on the release issue |

Release sign-off: paste full script output on the launch tracking issue — [launch checklist Phase 4](runbooks/launch-checklist.md#phase-4--off-chain-stack-if-applicable) (SEC-H04, [GitLab **#442**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/442)). Agent playbook: [`skills/AGENTS_DEPLOY_ENV_ADDRESSES_VERIFY.md`](../skills/AGENTS_DEPLOY_ENV_ADDRESSES_VERIFY.md). Doc drift: **`make check-deploy-env-addresses-docs`**. Unit checks (no Docker): **`make test-qa-verify-env-addresses`**.

## Indexer FACTORY_ADDRESS guard (invariant Q5)

| Invariant | Check | On failure |
| --------- | ----- | ---------- |
| **Q5** Indexer refuses to start with empty or whitespace-only `FACTORY_ADDRESS` in every `RUN_MODE`; post-deploy QA asserts the env var is set before schema checks | **`make verify-issue-451`**; unit test `empty_factory_address_rejected_in_dev`; **`make qa-verify-deploy`** (Q1 pre-flight) | Fix `indexer/.env` and redeploy; do not run indexer until `FACTORY_ADDRESS` is set |

Release sign-off: confirm Phase 0 **Indexer FACTORY_ADDRESS (SEC-I02)** on the launch tracking issue — [launch checklist Phase 0](runbooks/launch-checklist.md#phase-0--preconditions) ([#451](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/451)). Agent playbook: [`skills/AGENTS_FACTORY_ADDRESS_GUARD.md`](../skills/AGENTS_FACTORY_ADDRESS_GUARD.md). Doc drift: **`make check-factory-address-docs`**.

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

## Post-merge Coolify + indexer stack (invariant Q6) {#post-merge-stack-573}

| Invariant | Check | On failure |
| --------- | ----- | ---------- |
| **Q6** Stacked frontend/indexer MRs (!368–!377) are locally verified together and Coolify/indexer env rules are documented | **`make verify-issue-573`** → child verifies **557, 560, 561, 562, 563, 564, 565, 566, 567** plus logo + Coolify env docs (**M573-1–M573-8**) | Non-zero exit; fix the failing child or docs; do not treat local pass as columbus-5 hardware/production smoke |

**M573** (GitLab **#573** — [`skills/AGENTS_POST_MERGE_STACK.md`](../skills/AGENTS_POST_MERGE_STACK.md)):

| ID | Rule |
|----|------|
| **M573-1** | `make verify-issue-573` runs the child `make verify-issue-*` list. Unit/docs FAILs fail the stack. Optional Playwright SKIP is allowed only when LocalTerra / `.env.local` is absent. |
| **M573-2** | Coolify frontend rebuild ships the stack together. Production: `VITE_NETWORK=mainnet`, **`VITE_SHOW_TEST_TOKENS` unset**, **`VITE_FAUCET_ADDRESS` unset**. |
| **M573-3** | Indexer restart for additive #557 trade/fill decimals (optional #560 position decimals). JSON amounts stay raw. Confirm #553 / #556 migrations if not live. |
| **M573-4** | LocalTerra Swap pay still lists **EMBER** (`e2e/retail-test-tokens-562.spec.ts`). Gems hide only on mainnet (**P562-3**). |
| **M573-5** | Favicons use simplified C+8 (`favicon-16.png` / `favicon-32.png`); never `/logo.png` as a tab icon. |
| **M573-6** | Android Station/Cosmostation WC (#566) and Keplr+Ledger Nano (#567) stay operator hardware AC. |
| **M573-7** | Production smoke after Coolify: tape vs LCD, Charts Vol (USD), `/trade` layout + **Buy {base}** heading, hub P&amp;L header, no gemstone tickers + faucet Pause. |
| **M573-8** | Playbook + this Q6 + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Post-merge fees / gas / 8266 ops (invariant Q7) {#post-merge-ops-590}

| Invariant | Check | On failure |
| --------- | ----- | ---------- |
| **Q7** Stacked indexer-fees + Swap gas + CW20 audit MRs (!394–!396) are locally verified together; Coolify migrate and 8266 listing status are documented | **`make verify-issue-590`** → child verifies **586, 587, 589** plus L7/unwrap classification and optional A-lcd/B-lt + wrap-swap E7/E8 (**M590-1–M590-8**) | Non-zero exit; fix the failing child or harness stub; do not treat multi-test #589 as 8266 execution |

**M590** (GitLab **#590** — [`skills/AGENTS_POST_MERGE_OPS_590.md`](../skills/AGENTS_POST_MERGE_OPS_590.md)):

| ID | Rule |
|----|------|
| **M590-1** | `make verify-issue-590` runs children **586, 587, 589**. Unit/docs FAILs fail the stack. Optional A-lcd/B-lt and wrap-swap E7/E8 SKIP only when LocalTerra is absent (unless `VERIFY590_REQUIRE_CHAIN=1`). |
| **M590-2** | Coolify indexer migrate + `WRAP_MAPPER_ADDRESS` + dApp Protocol fees / Swap Network fee ship together. Empty mapper omits wrap/unwrap. |
| **M590-3** | Hybrid fee = pool `commission_amount` (`swap_amm`) + fill `commission_amount` (`book_take`) once — never swap `book_commission_amount` (**L7**). |
| **M590-4** | `limit_place` from `maker_fee_amount`; unwrap from wrap-mapper `fee_amount` only (not InstantWithdraw burn tax). |
| **M590-5** | Playwright `e2e/wrap-swap.spec.ts` **E7/E8** `--project=e2e-tx` (1 worker): success, no OOG. 0 USTC still allows LUNC-funded swap. |
| **M590-6** | `LAYER_B_LT=1` executes pinned LCD wasm (`layer-a-lcd.sh` + `layer-b-lt.sh`). Stub PASS is a harness bug. |
| **M590-7** | 8266 `codeids/8266/REPORT.md` is **GO**. Columbus-5 `AddWhitelistedCodeId 8266` **RAN 2026-08-22** (height **30060600**, `GetWhitelistedCodeIds` **`[6036, 8266, 10184]`**). Do **not** whitelist a LocalTerra store id. Do **not** whitelist ALPHA **8654**. SpaceUSD/UST1 create+provide stays on [#558](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/558). |
| **M590-8** | Playbook + this Q7 + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Post-merge unwrap+≥2hop chain QA (invariant Q8) {#post-merge-ops-600}

| Invariant | Check | On failure |
| --------- | ----- | ---------- |
| **Q8** Post-merge !400 unwrap+≥2hop envelope is live-checked (LocalTerra E9 + optional columbus-5 hash); unit/docs already landed with #599 | **`make verify-issue-600`** → children **599, 587** plus wrap-swap E9/E7 and optional `VERIFY600_COLUMBUS_TX` (**M600-1–M600-8**) | Non-zero exit; fix the failing child or E9 OOG; do not treat green `verify-issue-599` as E9/columbus-5 clearance |

**M600** (GitLab **#600** — [`skills/AGENTS_POST_MERGE_OPS_600.md`](../skills/AGENTS_POST_MERGE_OPS_600.md)):

| ID | Rule |
|----|------|
| **M600-1** | `make verify-issue-600` runs children **599** and **587**. Unit/docs FAILs fail the stack. E9 SKIP only when LocalTerra is absent (unless `VERIFY600_REQUIRE_CHAIN=1`). |
| **M600-2** | Playwright `e2e/wrap-swap.spec.ts` **E9** `--project=e2e-tx` (1 worker): USTR or JADE/RUBY → USTC, one submit, no OOG, LCD `gas_used < gas_wanted`. |
| **M600-3** | LUNC→USTR and USTC→USTR stay wrap+combo one-tx (#587). E7 when chain is up. |
| **M600-4** | Direct mapper unwrap stays **800k**. Do not raise `UNWRAP_GAS_LIMIT` for the hub InstantWithdraw path. |
| **M600-5** | Envelope is **3,110,000** (~88 LUNC class). If captured columbus-5 `gasUsed` ≥ 3.11M, open a new ticket — do not silently bump `UNWRAP_GAS_LIMIT`. |
| **M600-6** | Columbus-5 `/` USTR→USTC is operator-run. Record hash + gas via `VERIFY600_COLUMBUS_TX`. No hybrid / `book_input` (**H596-7**). |
| **M600-7** | Do **not** reopen #599 unless 3.11M still OOGs. File a new envelope ticket if the named combo must rise. |
| **M600-8** | Playbook + this Q8 + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Related docs

- [GitLab **#337**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/337) — master executable Local/QA verification checklist (Q1 maps to **INF-00-02** / **LR-00-01**)
- [`scripts/qa/README.md`](../scripts/qa/README.md) — server + laptop workflow
- [`docs/qa-onboarding.md`](./qa-onboarding.md) — human QA onboarding
- [`skills/AGENTS_QA_DEPLOY_VERIFY.md`](../skills/AGENTS_QA_DEPLOY_VERIFY.md) — post-deploy schema check ([#203](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203))
- [`skills/AGENTS_DEPLOY_CONFIG_VERIFY.md`](../skills/AGENTS_DEPLOY_CONFIG_VERIFY.md) — post-deploy config assertions ([#441](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/441))
- [`skills/AGENTS_DEPLOY_ENV_ADDRESSES_VERIFY.md`](../skills/AGENTS_DEPLOY_ENV_ADDRESSES_VERIFY.md) — env/chain address cross-check ([#442](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/442))
- [`skills/AGENTS_LOCAL_POSTGRES_DEV.md`](../skills/AGENTS_LOCAL_POSTGRES_DEV.md) — dev `make reset` (non-QA compose)
- [`skills/AGENTS_POST_MERGE_STACK.md`](../skills/AGENTS_POST_MERGE_STACK.md) — post-merge Coolify + indexer stack !368–!377 ([#573](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/573), **Q6**)
- [`skills/AGENTS_POST_MERGE_OPS_590.md`](../skills/AGENTS_POST_MERGE_OPS_590.md) — post-merge !394–!396 fees / wrap gas / 8266 A-lcd ([#590](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/590), **Q7**)
- [`skills/AGENTS_POST_MERGE_OPS_600.md`](../skills/AGENTS_POST_MERGE_OPS_600.md) — post-merge !400 LocalTerra E9 + columbus-5 USTR→USTC unwrap gas ([#600](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/600), **Q8**)
