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
| **M590-4** | `limit_place` from `maker_fee_amount`; unwrap from wrap-mapper **`fee`** (legacy `fee_amount`) only (not InstantWithdraw burn tax; [#613](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/613)). |
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

## Post-merge !402 Create Token Coolify + LocalTerra (invariant Q9) {#post-merge-ops-602}

| Invariant | Check | On failure |
| --------- | ----- | ---------- |
| **Q9** Post-merge !402 Create Token / catalog is live-checked (Coolify env + 11614 free-create + LocalTerra retail); unit/docs already landed with #593 / #594 / #601 | **`make verify-issue-602`** → children **593, 594** plus live Coolify pins and optional LocalTerra smoke (**M602-1–M602-8**) | Non-zero exit; fix the failing child, Coolify pin, or smoke; do not treat green `verify-issue-593` as Coolify / LocalTerra clearance |

**M602** (GitLab **#602** — [`skills/AGENTS_POST_MERGE_OPS_602.md`](../skills/AGENTS_POST_MERGE_OPS_602.md)):

| ID | Rule |
|----|------|
| **M602-1** | `make verify-issue-602` runs children **593** and **594**. Unit/docs FAILs fail the stack. Live Coolify SKIP only with `VERIFY602_SKIP_LIVE=1`. LocalTerra SKIP only when the chain is down (unless `VERIFY602_REQUIRE_CHAIN=1`). |
| **M602-2** | **P402-1.** Coolify frontend bakes `VITE_COMMUNITY_TAX_CODE_ID=11619` and launcher `terra126pr5…` (code **11622**; store was **11614**). More menu shows **Create Token**. Do not bake unused **11612**. |
| **M602-3** | **P402-2.** Coolify indexer `COMMUNITY_TAX_CODE_ID` + `COMMUNITY_TOKEN_LAUNCHER` + `CMM_GOVERNANCE_ADDR`. `GET /api/v1/community-tokens` is `{ configured: true, … }` (empty list OK). |
| **M602-4** | **P402-3.** Free create lives on `terra126pr5…` (store `33F6A49F…` / instantiate `041E3C43…`; now code **11622**). **11612** unused. Do not whitelist 11612 / 11613 / 11614 / 11620 / 11622. |
| **M602-5** | **P402-4.** LocalTerra in-repo launcher smoke + UI: free create, paid SKU via #595, Manage Save 50 UST1, non-manager read-only, Unverified admin. |
| **M602-6** | **P402-5.** `/create` is copy-address + link only — no query prefill (**C542-11**). |
| **M602-7** | **P402-6.** Swap/Trade Max is extra-debit. Catalog lists without a pair; default `attested_cmm` only. Do not reopen #593 / #594 unless **C593** / **I594** is wrong. |
| **M602-8** | Playbook + this Q9 + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Post-merge !407/!408 Enable Feature + LocalTerra (invariant Q10) {#post-merge-ops-612}

| Invariant | Check | On failure |
| --------- | ----- | ---------- |
| **Q10** Post-merge !407/!408 Enable Feature is live-checked (columbus-5 11622 + `token_code_id` 11619 + LocalTerra launcher SKU unlock); unit/docs already landed with #606 / #607 | **`make verify-issue-612`** → children **606, 607** plus live pins and optional `verify-issue-601` smoke (**M612-1–M612-8**) | Non-zero exit; fix the failing child, Coolify pin, or smoke; do not treat green `verify-issue-606` as LocalTerra Enable Feature clearance |

**M612** (GitLab **#612** — [`skills/AGENTS_POST_MERGE_OPS_612.md`](../skills/AGENTS_POST_MERGE_OPS_612.md)):

| ID | Rule |
|----|------|
| **M612-1** | `make verify-issue-612` runs children **606** and **607**. Unit/docs FAILs fail the stack. Live Coolify SKIP only with `VERIFY612_SKIP_LIVE=1`. LocalTerra SKIP only when the chain is down (unless `VERIFY612_REQUIRE_CHAIN=1`). |
| **M612-2** | Columbus-5 launcher `terra126pr5…` is **11622** with `GetConfig` **11619** / **11621**. Do not whitelist 11612 / 11614 / 11620 / 11621 / 11622 / 8654. Keep 11611 listed until Refresh. |
| **M612-3** | Coolify frontend bakes `VITE_COMMUNITY_TAX_CODE_ID=11619` and launcher `terra126pr5…`. Single `communityTaxHint`. Do not bake unused **11612**. |
| **M612-4** | LocalTerra `verify-issue-601` smoke reports `sku_unlock_via_launcher: true`. Free create → Enable Feature `transfer_tax` is manager → launcher → token. |
| **M612-5** | Same smoke reports `paid_create_one_sku` + `sku_second_unlock_via_launcher`. Paid create one SKU, then Enable Feature a second SKU. |
| **M612-6** | Manage **Enable feature** chrome is launcher-payee; Minting is create-only. Do **not** run the stale option-1 disclose checklist — current copy is option 2; live copy QA is [#616](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/616). |
| **M612-7** | Do not reopen #606 / #607 for ops/QA. File a new ticket if **T606** / **T592-13** / **C593-14** is wrong. |
| **M612-8** | Playbook + this Q10 + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Post-merge !409–!413 option-2 / wrap / window / AutoLP / ranking (invariant Q11) {#post-merge-ops-616}

| Invariant | Check | On failure |
| --------- | ----- | ---------- |
| **Q11** Post-merge !409–!413 is live-checked (columbus-5 11622 + `token_code_id` 11619 + option-2 copy + children 607/610/613/614/615); Coolify window/wrap leftovers stay recorded | **`make verify-issue-616`** → children **607, 610, 613, 614, 615** plus live pins and optional leftovers (**M616-1–M616-8**) | Non-zero exit; fix the failing child, Coolify pin, or copy; do not treat green children as live wrap/window ingest |

**M616** (GitLab **#616** — [`skills/AGENTS_POST_MERGE_OPS_616.md`](../skills/AGENTS_POST_MERGE_OPS_616.md)):

| ID | Rule |
|----|------|
| **M616-1** | `make verify-issue-616` runs children **607, 610, 613, 614, 615**. Unit/docs FAILs fail the stack. Coolify leftover probes SKIP unless `VERIFY616_REQUIRE_LIVE_LEFTOVERS=1`. LocalTerra SKIP only when the chain is down (unless `VERIFY616_REQUIRE_CHAIN=1`). |
| **M616-2** | Columbus-5 launcher `terra126pr5…` is **11622** with `GetConfig` **11619** / **11621**. Do not whitelist 11612 / 11613 / 11614 / 11620 / 11621 / 11622 / 8654. Keep 11611 listed until Refresh. |
| **M616-3** | Coolify frontend bakes `VITE_COMMUNITY_TAX_CODE_ID=11619` and launcher `terra126pr5…`. Indexer catalog `code_id=11619`. Indexer `UST1_WINDOW_ADDRESS` is pinned (`ust1_window_configured: true`). Live wrap/unwrap and `ust1_mint` / `ust1_redeem` increment (2026-08-25). |
| **M616-4** | Swap/Trade pair-direct **and** hops show `Sell tax extra` / `Buy tax applies`. Create/Manage: `Buy/sell tax applies on every listed-pair swap.` No `Route skips buy/sell tax`. |
| **M616-5** | AutoLP skim floor (100 bps, cap 200) + factory-listed tax pair. Do not whitelist AutoLP. Crate **610** is the gate when LocalTerra is down. |
| **M616-6** | Ranking TAX→UST1 vs TAX→USTR. Unmigrated 11611 stays Honest hops until `COMMUNITY_TAX_OPTION2_*` (**R615-5**). Do not infer wrap/window fees from `amount × bps`. |
| **M616-7** | Do not reopen #607 / #610 / #613 / #614 / #615 for ops/QA. Live `ust1_mint` / `ust1_redeem` increment on `/protocol`. |
| **M616-8** | Playbook + this Q11 + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Post-merge !414 LocalTerra community-tax seed leftovers (invariant Q12) {#post-merge-ops-624}

| Invariant | Check | On failure |
| --------- | ----- | ---------- |
| **Q12** Post-merge !414 leftover live is checked (fresh volume + indexer catalog + Transfer funding + stamp skip + children 620/601/592/610/594) | **`make verify-issue-624`** → child **620** plus leftover probes and children **601, 592, 610, 594** (**M624-1–M624-8**) | Non-zero exit; fix the failing child, env pin, or Transfer fork; do not treat green #620 on a stale volume as leftover #1 |

**M624** (GitLab **#624** — [`skills/AGENTS_POST_MERGE_OPS_624.md`](../skills/AGENTS_POST_MERGE_OPS_624.md)):

| ID | Rule |
|----|------|
| **M624-1** | `make verify-issue-624` runs child **620** plus leftover live probes and children **601, 592, 610, 594**. Unit/docs FAILs fail the stack. Fresh-volume leftover SKIP unless `VERIFY624_FRESH=1`. Indexer leftover SKIP unless the indexer is up (FAIL when `VERIFY624_REQUIRE_LIVE=1`). |
| **M624-2** | Fresh `make reset && make start && make deploy-local` writes **local** `VITE_COMMUNITY_TAX_*` / SmokeUST1 / indexer `COMMUNITY_TAX_*`. LCD pair + AutoLP `pair` match env. Reserves ≥ **10M** raw / side. Stamp `git_sha` == `HEAD`. Never whitelist columbus-5 **11611** / **11619** / launcher / AutoLP / ALPHA **8654**. |
| **M624-3** | `GET /api/v1/community-tokens` is `configured: true` and lists the QA token as `attested_cmm`. Do not pin columbus-5 **11619** against local instances. `GET /tokens/{tax}` may embed `community_tax`. `route/solve` sees `buy_tax_bps` / `sell_tax_bps`. |
| **M624-4** | `e2e-provision-dev-wallet.sh` **Transfer**s the QA tax token from `test1` (fail-closed). Wrap skip. TCL8Y still **Mint**s. `DEPLOY_SKIP_COMMUNITY_TAX=1` is gems-only. |
| **M624-5** | Swarm `fundBotWallets` never **Mint**s the tax token. `--dry-run` skips funding and logs `swarm_funding_plan` only. |
| **M624-6** | Re-run without `--fresh`: `deploy_up_to_date` stamp skip (no second tax pair). Phase 4d always stores + paid-creates if the whole deploy runs. |
| **M624-7** | Do not reopen #620 / #592 / #601 / #610 / #594 for ops/QA. Do not enable `MintControl` or fall back to Mint. Do not implement pair/router FoT math (**H-01**). |
| **M624-8** | Playbook + this Q12 + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Post-merge !415–!417 tax swarm / e2e-tx / Layer B leftovers (invariant Q13) {#post-merge-ops-625}

| Invariant | Check | On failure |
| --------- | ----- | ---------- |
| **Q13** Post-merge !415–!417 leftover live is checked (tax-on seed buy from non-treasury, Playwright P0 extra-debit, swarm soak, OE-1 `pool_only`, children 621/622/623) | **`make verify-issue-625`** → children **621, 622, 623** plus leftover probes and **293** (**M625-1–M625-8**) | Non-zero exit; fix the failing child, buy wallet, seed treasury, or swarm exclude; do not treat green #621/#622/#623 docs-only as leftover live |

**M625** (GitLab **#625** — [`skills/AGENTS_POST_MERGE_OPS_625.md`](../skills/AGENTS_POST_MERGE_OPS_625.md)):

| ID | Rule |
|----|------|
| **M625-1** | `make verify-issue-625` runs children **621, 622, 623** plus leftover live (tax-on seed buy, Playwright P0, swarm soak, **293**). Unit/docs FAILs fail the stack. Fresh-volume leftover SKIP unless `VERIFY625_FRESH=1`. Live leftover SKIP unless LocalTerra + seed pins (FAIL when `VERIFY625_REQUIRE_LIVE=1`). |
| **M625-2** | Seed token treasury ≠ **test1** (e2e / swarm trader). CMM stand-in stays test1. Tax-on seed-path **buy** uses `pick_trader` (non-treasury / non-exempt). `LAYER_B_TAX_ON_FORCE_EPHEMERAL=1` skips seed pins so a #624 volume still proves instantiate + buy-from-trader. |
| **M625-3** | `VERIFY_ISSUE_622_CHAIN=1` P0 sell extra-debit + buy net + provide 1:1 + limit place honest / cancel buy-net refund. Missing pins fail closed. Gem pickers skip the pinned tax market. Playwright Vite Origin (default `http://127.0.0.1:3173`) must be in indexer `CORS_ORIGINS`. Place gas is batch `n=1` at **1.18M** (base **1M**); send-inner `place_limit_order` is **1.2M**. Cancel refund is buy-classified (not `userAfterCancel === userBefore` when ExemptionDirectory is off). |
| **M625-4** | Short swarm soak: `tax_listed` extra-debit + buy split + router `trader` + `tax_hybrid_skip`. `SWARM_TAX_WORKERS=0` is exclude-only. Python `tax-0` starts before gem workers and warms up `hybrid` then `sell`. |
| **M625-5** | `make verify-issue-293` stays OE-1 gem `pool_only`. Do not add tax/EMBER to hub symmetry. |
| **M625-6** | Prefer a fresh #620 seed so leftover #623 ephemeral tax pairs cannot steal `pairs[0]`. Tax-on pins are `VITE_TOKEN_COMMUNITY_TAX_*`. |
| **M625-7** | Do not reopen #621 / #622 / #623 / #620 for ops/QA. Do not merge tax-on into B-lt. Do not turn hybrid off. Do not `test.skip` e2e-tx. Never whitelist 11611 / 11619 / 8654. No pair/router FoT math. |
| **M625-8** | Playbook + this Q13 + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Post-merge !418 community-tax migrate leftovers (invariant Q14) {#post-merge-ops-628}

| Invariant | Check | On failure |
| --------- | ----- | ---------- |
| **Q14** Post-merge !418 leftover live is checked (adopt pin ≠ 11619, factory list without 8654/code 3, Coolify/indexer single-id, 6036 cw2, LocalTerra P3/P7/P11, children 626/592/593/594) | **`make verify-issue-628`** → children **626, 592, 593, 594** plus leftover probes (**M628-1–M628-8**) | Non-zero exit; fix the failing child, pin, or LocalTerra leftover; do not treat green #626 docs-only as leftover live |

**M628** (GitLab **#628** — [`skills/AGENTS_POST_MERGE_OPS_628.md`](../skills/AGENTS_POST_MERGE_OPS_628.md)):

| ID | Rule |
|----|------|
| **M628-1** | `make verify-issue-628` runs children **626, 592, 593, 594** plus leftover live (columbus-5 pins, 6036 cw2, Coolify/indexer, LocalTerra P3/P7/P11). Unit/docs FAILs fail the stack. Live leftover SKIP unless LCD / `dex.cl8y.com` answers (FAIL when `VERIFY628_REQUIRE_LIVE=1` or `VERIFY628_IID=628`). LocalTerra SKIP unless the chain + tax pins are up (FAIL when `VERIFY628_REQUIRE_LIVE=1` / `VERIFY628_IID=628` / `VERIFY628_REQUIRE_CHAIN=1`). |
| **M628-2** | Retail adopt target is the current listed tax pin, not **11619**. #628 stored + listed **11626**; #635 bump **11630** is the live factory / launcher / Coolify pin. 0 instances of 11619 — CMM same-crate migrate is N/A. |
| **M628-3** | Factory-list only a GO tax pin. Never whitelist **8654** or columbus-5 **code 3**. 11619 may be removed at 0 instances. Keep 6036 / 8266 / 10184 listed. |
| **M628-4** | Coolify `VITE_COMMUNITY_TAX_CODE_ID` + indexer `COMMUNITY_TAX_CODE_ID` match the current listed tax pin (**11630** after #635; **11626** was the #628 store). Migrate allowlist default **6036,10184,8266,8654**. Catalog is single-id. Do not bake 11619. |
| **M628-5** | Confirm LCD cw2 on a live 6036 instance. `crates.io:cw20-base` is adopt-go. `terraswap-token` stays page-go / chain-revert — do not append that name from this ticket. Either recorded outcome PASSes the leftover probe. |
| **M628-6** | LocalTerra: mintable adopt keeps balances / `total_supply`; inbound Transfer to a CL8Y pair is 1:1; admin CMM (**P3**). Pair fail-closes until `RefreshPairAssetCodeIds`; after Refresh, extra-debit sell works (**P7**). Manage shows tax SKUs when `code_id` matches the env pin (**P11**). |
| **M628-7** | Create Token retail copy stays code-id-free. Lead is “Already have a token? Migrate here”. 8654 is a normal migrate-allowlist entry in env/docs — never factory-list language. Do not reopen #626 / implement #627 / RegisterListedPair Terraport/GDEX / turn hybrid off / add pair-router FoT math. |
| **M628-8** | Playbook + this Q14 + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Post-merge !437–!458 leftover verify (invariant Q15) {#post-merge-ops-673}

| Invariant | Check | On failure |
| --------- | ----- | ---------- |
| **Q15** Post-merge !437–!458 leftover live is checked (Coolify migrations + `liquidity_usd`/`created_at`, pair leaderboard 404/400, volume grains, frontend rebuild `8af5563c+`, children 655–672) | **`make verify-issue-673`** → children **655–672** plus leftover probes (**M673-1–M673-8**) | Non-zero exit; fix the failing child or Coolify leftover; do not treat green child docs-only as leftover live; do not reopen #655–#672 |

**M673** (GitLab **#673** — [`skills/AGENTS_POST_MERGE_OPS_673.md`](../skills/AGENTS_POST_MERGE_OPS_673.md)):

| ID | Rule |
|----|------|
| **M673-1** | `make verify-issue-673` runs children **655–672** plus leftover live (Coolify indexer + frontend). Unit/docs FAILs fail the stack. Live leftover SKIP unless `indexer.dex.cl8y.com` / `dex.cl8y.com` answers (FAIL when `VERIFY673_REQUIRE_LIVE=1` or `VERIFY673_IID=673`). Leftover Playwright SKIP unless LocalTerra is up (FAIL when `VERIFY673_REQUIRE_CHAIN=1`). |
| **M673-2** | Coolify Postgres has `20260826150000_pair_liquidity_usd.sql` and `20260826180000_protocol_volume_hourly_monthly.sql`, then indexer redeploy. Live `GET /pairs` always includes `created_at`. `liquidity_usd` is present when stamped. `sort=liquidity_usd` is allowlisted. |
| **M673-3** | Live `GET /traders/leaderboard?pair=`: unknown pair **404**; `sort=best_trade_pnl` with `pair` **400**. Unscoped board unchanged. Charts pair board hides Best Trade. |
| **M673-4** | Live grains are `GET /protocol/volume/daily?grain=hourly\|daily\|monthly` + capped `limit`. No `/protocol/volume/hourly` path. `from`/`to` → **400**. |
| **M673-5** | Coolify frontend rebuild from `8af5563c+`. `/charts` has no `charts-overview-*`. `/protocol` keeps census + Hourly/Daily/Monthly. `/pool` four tabs + v2 LP + Created. Swap seam, footer Homepage/Bridge, Connect Wallet Close, Create Token width, migrate Unlock copy stay on the bundle. |
| **M673-6** | Do not restore Charts DEX-census tiles or a second `pair_liquidity.rs`. Do not commit `scripts/tmp-558-*`. Do not wait on GitLab CI. Do not reopen #655–#672 for ops/QA. |
| **M673-7** | Optional LocalTerra leftover Playwright is `e2e-smoke` at **5 workers** (pool Manage, footer, Charts smoke, trader-page). `e2e-tx` stays **1 worker**. Do not leak a non-3173 `PLAYWRIGHT_WEB_PORT` into children (CORS). |
| **M673-8** | Playbook + this Q15 + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Post-merge !459–!468 leftover verify (invariant Q16) {#post-merge-ops-686}

| Invariant | Check | On failure |
| --------- | ----- | ---------- |
| **Q16** Post-merge !459–!468 leftover live is checked (Coolify `NUMERIC(78,18)` + `economic_token_marks`, `HUB_CL8Y_ADDRESS`, hub four cells, CL8Y fee USD when a pair exists, frontend rebuild `36d64528+`, children 674–680 / 683) | **`make verify-issue-686`** → children **674, 675, 676, 677, 678, 679, 680, 683** plus leftover probes (**M686-1–M686-8**) | Non-zero exit; fix the failing child or Coolify leftover; do not treat green child docs-only as leftover live; do not reopen #674–#680 / #683 |

**M686** (GitLab **#686** — [`skills/AGENTS_POST_MERGE_OPS_686.md`](../skills/AGENTS_POST_MERGE_OPS_686.md)):

| ID | Rule |
|----|------|
| **M686-1** | `make verify-issue-686` runs children **674, 675, 676, 677, 678, 679, 680, 683** plus leftover live (Coolify indexer + frontend). Unit/docs FAILs fail the stack. Live leftover SKIP unless `indexer.dex.cl8y.com` / `dex.cl8y.com` answers (FAIL when `VERIFY686_REQUIRE_LIVE=1` or `VERIFY686_IID=686`). Leftover Playwright SKIP unless LocalTerra is up (FAIL when `VERIFY686_REQUIRE_CHAIN=1`). |
| **M686-2** | Coolify Postgres has `20260827120000_trader_positions_numeric_78.sql` and `20260827140000_economic_token_marks.sql`, then indexer redeploy. After migrate, UST1/USTR and CL8Y-cb/cUSTC `trade_count` on `/positions` matches `/trades` (else `rebuild-positions`). |
| **M686-3** | `HUB_CL8Y_ADDRESS` is pinned (columbus-5 CL8Y CW20). NULL CL8Y `fee_usd` backfills once; non-null stamps stay. GET `/hub-prices` stays four cells; extra ticker **400**. |
| **M686-4** | Live `/protocol` token mix shows CL8Y-cb human + `$` when a qualifying pair exists. Liquidity Δ% is 24h-only. UTC volume x-axis stays dense. |
| **M686-5** | Coolify frontend rebuild from `36d64528+`. `/portfolio` hide-gems + Mark/Unrealized. `/charts` UST1/USD hero. Swap Quote only / Get UST1. Mixed hybrid Network fee ~192 LUNC is the dApp source of truth. |
| **M686-6** | Do not reopen #674–#680 / #683 for ops/QA. Do not wait on GitLab CI. Do not turn hybrid off. Do not rewrite non-null `fee_usd` or expand `/hub-prices`. Do not touch #684. |
| **M686-7** | Optional LocalTerra leftover Playwright is `e2e-smoke` at **5 workers** (portfolio, protocol-page, Charts smoke). `e2e-tx` stays **1 worker**. Do not leak a non-3173 `PLAYWRIGHT_WEB_PORT` into children (CORS). |
| **M686-8** | Playbook + this Q16 + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Post-merge !474/!475 leftover verify (invariant Q17) {#post-merge-ops-698}

| Invariant | Check | On failure |
| --------- | ----- | ---------- |
| **Q17** Post-merge !474/!475 leftover live is checked (Coolify API4 caps + FE-01 production reject, children 694 / 695). [#699](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/699) / [#700](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/700) are duplicates of [#698](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/698) | **`make verify-issue-698`** → children **694, 695** plus leftover probes (**M698-1–M698-8**) | Non-zero exit; fix the failing child or Coolify leftover; do not treat green child docs-only as leftover live; do not reopen #694 / #695 |

**M698** (GitLab **#698** — [`skills/AGENTS_POST_MERGE_OPS_698.md`](../skills/AGENTS_POST_MERGE_OPS_698.md)):

| ID | Rule |
|----|------|
| **M698-1** | `make verify-issue-698` runs children **694, 695** plus leftover live (Coolify indexer + frontend). Unit/docs FAILs fail the stack. Live leftover SKIP unless `indexer.dex.cl8y.com` / `dex.cl8y.com` answers (FAIL when `VERIFY698_REQUIRE_LIVE=1` or `VERIFY698_IID` is `698`/`699`/`700`). Leftover Playwright SKIP unless LocalTerra is up (FAIL when `VERIFY698_REQUIRE_CHAIN=1`). |
| **M698-2** | Coolify indexer redeploy from `3c4060ab+` (no new migration). Live blacklist 17 tokens → **400**. At-cap still reaches LCD. |
| **M698-3** | Live `/gt/events` 2000-block window is bounded (not multi-MB; over-cap **400**). Progress JSON `stage` answers. GET does not `SELECT` `pair_reserves`. |
| **M698-4** | Coolify frontend rebuild from `489268eb+`. Production `VITE_DEV_MODE` unset. Live wallet chunk has no Simulated Wallet chrome. |
| **M698-5** | Progress stays advisory. Omit `trader` when discount is known. Back off after poll failures. Place / Swap ungated. Trade does not wire `useRouteSolveProgress`. |
| **M698-6** | Do not reopen #694 / #695 for ops/QA. Do not wait on GitLab CI. Do not trust `X-Forwarded-For`. Do not fail-open blacklist. Do not change CSP / WC / clickwrap / `devWallet.ts`. Do not commit `tmp-558-*`. |
| **M698-7** | Close RE-01/02/03 / FE-01 in the Grok46 audit only after prod evidence. Optional leftover Playwright is Swap e2e-smoke at **5 workers**. Do not leak a non-3173 `PLAYWRIGHT_WEB_PORT`. |
| **M698-8** | Playbook + this Q17 + child skills stay crosslinked. `#699` / `#700` alias this leftover. GitLab CI quota is not a substitute for local verify. |

## Post-merge !477 leftover verify (invariant Q18) {#post-merge-ops-701}

| Invariant | Check | On failure |
| --------- | ----- | ---------- |
| **Q18** Post-merge !477 leftover live is checked (Coolify `pair_volume_24h.volume_usd` migrate + list↔stats + `/pool` Vol USD, child 692) | **`make verify-issue-701`** → child **692** plus leftover probes (**M701-1–M701-8**) | Non-zero exit; fix the failing child or Coolify leftover; do not treat green child docs-only as leftover live; do not reopen #692 |

**M701** (GitLab **#701** — [`skills/AGENTS_POST_MERGE_OPS_701.md`](../skills/AGENTS_POST_MERGE_OPS_701.md)):

| ID | Rule |
|----|------|
| **M701-1** | `make verify-issue-701` runs child **692** plus leftover live. Unit/docs FAILs fail the stack. Live leftover SKIP unless hosts answer (FAIL when `VERIFY701_REQUIRE_LIVE=1` or `VERIFY701_IID=701`). |
| **M701-2** | Coolify Postgres has `20260829120000_pair_volume_24h_usd.sql`, then indexer redeploy. `refresh_pair_volumes` must run once. |
| **M701-3** | Live `GET /pairs?sort=volume_usd_24h` includes `volume_usd_24h`. `sort=volume_usd` → **400**. Priced list USD ≈ pair stats `volume_usd` (**PVol-5**). Unpriced → omit/`—`. |
| **M701-4** | Coolify frontend rebuild from `f4fa6788+`. `/pool` Vol is `$…` or **—**. No N+1 `/stats`. No quote-token Vol. |
| **M701-5** | Production gem hide unchanged. Do not change `liquidity_usd` / v2 LP USD. |
| **M701-6** | Do not reopen #692 / #678 for ops/QA. Do not wait on GitLab CI. Do not convert `volume_quote_24h` in the browser. |
| **M701-7** | Optional leftover Playwright is `e2e/pool-table-547.spec.ts` at **5 workers** (`:3173`). Do not leak a different `PLAYWRIGHT_WEB_PORT`. |
| **M701-8** | Playbook + this Q18 + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Post-merge !476 leftover verify (invariant Q19) {#post-merge-ops-702}

| Invariant | Check | On failure |
| --------- | ----- | ---------- |
| **Q19** Post-merge !476 leftover live is checked (Coolify `/trade` flatten rebuild, children 693 / 563 / 653) | **`make verify-issue-702`** → children **693, 563, 653** plus leftover probes (**M702-1–M702-8**) | Non-zero exit; fix the failing child or Coolify leftover; do not treat green child docs-only as leftover live; do not reopen #693 |

**M702** (GitLab **#702** — [`skills/AGENTS_POST_MERGE_OPS_702.md`](../skills/AGENTS_POST_MERGE_OPS_702.md)):

| ID | Rule |
|----|------|
| **M702-1** | `make verify-issue-702` runs children **693, 563, 653** plus leftover live. Unit/docs FAILs fail the stack. Live leftover SKIP unless `dex.cl8y.com` answers (FAIL when `VERIFY702_REQUIRE_LIVE=1` or `VERIFY702_IID=702`). |
| **M702-2** | Coolify frontend rebuild from `e6ddbf1d+`. Live Trade chunk has `trade-order-text-tab` / `trade-order-tab-market` / `trade-order-mode-docs`. No new indexer migration. |
| **M702-3** | Fresh `/trade/:pair` is Market. Compact underlined tabs. Heading logo + non-orange wash. No Side `TicketSection`. |
| **M702-4** | Limit reachable; book Edit / Place another stay Limit. Slippage chips only under Advanced; closed Advanced still 5% `max_spread`. `/limits` chips stay beside price. |
| **M702-5** | Light + dark at ~375px and ~1280px (issue **10.2.19**). Escrow, GET `/route/solve`, hybrid, footer dock unchanged. |
| **M702-6** | Do not reopen #693 for ops/QA. Do not wait on GitLab CI. Do not restyle `/limits` place card. |
| **M702-7** | Optional leftover Playwright is `e2e/trade-page-responsive.spec.ts` at **5 workers**. Expiry measurement opens Advanced first. P1 T527-1 overlap is clipped to **visible** `trade-order-ticket-scroll`. Do not leak a non-3173 `PLAYWRIGHT_WEB_PORT`. |
| **M702-8** | Playbook + this Q19 + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

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
- [`skills/AGENTS_POST_MERGE_OPS_602.md`](../skills/AGENTS_POST_MERGE_OPS_602.md) — post-merge !402 Coolify + 11614 launcher + LocalTerra Create Token ([#602](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/602), **Q9**)
- [`skills/AGENTS_POST_MERGE_OPS_612.md`](../skills/AGENTS_POST_MERGE_OPS_612.md) — post-merge !407/!408 Enable Feature migrate + LocalTerra QA ([#612](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/612), **Q10**)
- [`skills/AGENTS_POST_MERGE_OPS_616.md`](../skills/AGENTS_POST_MERGE_OPS_616.md) — post-merge !409–!413 option-2 / wrap / window / AutoLP / ranking ([#616](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/616), **Q11**)
- [`skills/AGENTS_POST_MERGE_OPS_624.md`](../skills/AGENTS_POST_MERGE_OPS_624.md) — post-merge !414 LocalTerra community-tax seed leftovers ([#624](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/624), **Q12**)
- [`skills/AGENTS_POST_MERGE_OPS_625.md`](../skills/AGENTS_POST_MERGE_OPS_625.md) — post-merge !415–!417 tax swarm / e2e-tx / Layer B leftovers ([#625](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/625), **Q13**)
- [`skills/AGENTS_POST_MERGE_OPS_628.md`](../skills/AGENTS_POST_MERGE_OPS_628.md) — post-merge !418 community-tax migrate leftovers ([#628](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/628), **Q14**)
- [`skills/AGENTS_POST_MERGE_OPS_673.md`](../skills/AGENTS_POST_MERGE_OPS_673.md) — post-merge !437–!458 leftover verify ([#673](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/673), **Q15**)
- [`skills/AGENTS_POST_MERGE_OPS_686.md`](../skills/AGENTS_POST_MERGE_OPS_686.md) — post-merge !459–!468 leftover verify ([#686](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/686), **Q16**)
- [`skills/AGENTS_POST_MERGE_OPS_698.md`](../skills/AGENTS_POST_MERGE_OPS_698.md) — post-merge !474/!475 leftover verify ([#698](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/698), **Q17**)
- [`skills/AGENTS_POST_MERGE_OPS_701.md`](../skills/AGENTS_POST_MERGE_OPS_701.md) — post-merge !477 leftover verify ([#701](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/701), **Q18**)
- [`skills/AGENTS_POST_MERGE_OPS_702.md`](../skills/AGENTS_POST_MERGE_OPS_702.md) — post-merge !476 leftover verify ([#702](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/702), **Q19**)
