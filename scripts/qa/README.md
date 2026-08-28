# QA server + laptop (Yield Omega DEX)

**Master checklist:** [GitLab **#337**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/337) — executable verification scenarios for LocalTerra and this QA stack (**INF-00-03**, **Q1** / **INF-00-02**).

## On the QA server

### One-time

- **Docker** with compose v2. LocalTerra / Postgres image **digests** are pinned in root `docker-compose.yml`; bump them after pulling a new tag (see [`docs/local-development.md`](../../docs/local-development.md) § Docker Setup).
- **Rust + Cargo** on the PATH for `cargo run --release` in `indexer/` (or set **`INDEXER_QA_BIN`** to a prebuilt binary path in the environment before `make start-qa`).
- Optional repo-root **`.env`**: set **`QA_SSH_HOST`** (hostname as seen from your laptop) so `make qa-tunnel-help` prints a useful `ssh` destination; **`QA_SSH_PORT`** if SSH is not 22.

### Bring up the stack

```bash
make start-qa
# alias:
make qa-start
```

This stops any prior QA indexer and runs **`docker compose down`** (volumes **preserved**), then starts **localterra** + **postgres**, waits for health, then deploys:

- **Skip deploy** when **`.qa-deploy-stamp`** `git_sha` == **`HEAD`**, env factory matches, wasm artifacts exist, and the factory LCD probe passes (**Q1** — GitLab [**#325**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/325)).
- Otherwise **`make deploy-local-no-build`** when optimizer wasm is already on disk, or **`make deploy-local`** (optimizer + deploy).

Optional: **`QA_FETCH_CI_ARTIFACTS=1 make start-qa`** tries GitLab generic packages for wasm/indexer before deploy. Set **`INDEXER_QA_BIN`** to a prebuilt **`cl8y-dex-indexer`** to skip release compile.

Then **`make qa-verify-deploy`** (schema + deploy-stamp + env/chain address cross-check — [#203](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203), SEC-H04 / [#442](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/442)), optional **`make qa-verify-deploy-config`** (on-chain config assertions — SEC-H03 / [#441](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/441)), indexer background start (pidfile **`.indexer-qa.pid`**, log **`.indexer-qa.log`**), **`/health`**, and laptop steps (**`make qa-tunnel-help`**).

**Which reset level after code changes?** See [`skills/AGENTS_QA_REDEPLOY_DECISION.md`](../../skills/AGENTS_QA_REDEPLOY_DECISION.md) ([#325](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/325)).

### Fresh volumes (empty chain + Postgres)

After **contract or genesis changes**, or when **`qa-verify-deploy`** reports **stale deployed contracts** ([#203](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203)), wipe **`localterra-data`** and **`postgres-data`** before bring-up:

```bash
make reset-qa
# equivalent:
QA_FRESH_VOLUMES=1 make start-qa
```

A **red banner** prints when volumes are removed. Default **`make start-qa`** is unchanged (fast restarts when chain state is still valid). See [`docs/qa-invariants.md`](../../docs/qa-invariants.md) and [GitLab **#202**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/202).

### Shared host with cl8y-bridge-monorepo

Bridge uses LocalTerra on **26658/1318** by default. To avoid port clashes, run DEX QA with:

```bash
export QA_SHARED_HOST=1   # or set in repo-root .env
make start-qa
```

This merges **`docker-compose.qa-shared-host.yml`** and defaults DEX LocalTerra to **26659/1319** (and remapped gRPC ports). Override with **`DEX_TERRA_RPC_PORT`**, **`DEX_TERRA_LCD_PORT`**, etc., if needed.

### Stop

```bash
make stop-qa
```

Stops the QA indexer (from pidfile) and **`docker compose down`**.

### Status

```bash
make status
```

Checks Docker, LocalTerra RPC, Postgres, indexer **`/health`**, and the indexer pidfile.

---

## On your laptop

1. **SSH** — Run the `ssh -4 -N ... -L ...` block from **`make qa-tunnel-help`** (forwards LocalTerra RPC/LCD and indexer API to local loopback).
2. **Env** — `scp` **`frontend-dapp/.env.local`** from the server into your clone (URLs use `localhost` and match forwarded ports).
3. **Optional** — **`./scripts/qa/write-frontend-env-local.sh`** refreshes **`VITE_TERRA_*`** / **`VITE_INDEXER_URL`** from **`scripts/qa/qa-host.env`** (useful if ports differ between machines).
4. **Vite** — `cd frontend-dapp && npm ci && npm run dev` — do **not** tunnel the Vite port; run the dev server locally.

### Wallet verification scope (laptop QA via tunnel)

Browser wallet checks over an SSH tunnel follow the same matrix as [`docs/qa-onboarding.md`](../../docs/qa-onboarding.md) § Wallet Matrix ([GitLab **#235**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/235)):

| Environment | P0 wallets | Station |
|-------------|------------|---------|
| **LocalTerra** (tunnel to QA server) | **Keplr (extension)**, **dev/simulated wallet** | **N/A** — Station’s built-in `localterra` gas step cannot match node ante minimum; dApp overrides do not fix it |
| **columbus-5** | **Station (extension)** | P0 for swap, limits, bids, wrap/unwrap |

**Keplr + Ledger Nano** is columbus-5 **P1** (not LocalTerra): [`docs/qa-onboarding.md`](../../docs/qa-onboarding.md) § Wallet Matrix, [`skills/AGENTS_FRONTEND_KEPLR_LEDGER.md`](../../skills/AGENTS_FRONTEND_KEPLR_LEDGER.md) ([#567](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/567)).

Canonical root cause: [`docs/frontend.md` § Station extension signing](../../docs/frontend.md#station-extension-signing). Agent playbooks: [`skills/AGENTS_FRONTEND_STATION_SIGNING.md`](../../skills/AGENTS_FRONTEND_STATION_SIGNING.md), [`skills/AGENTS_TERRACLASSIC_GAS.md`](../../skills/AGENTS_TERRACLASSIC_GAS.md).

### Pair search — token name relevance (LocalTerra)

After **`make deploy-local`** (or QA deploy) and indexer sync, verify indexer **tier 2** (token **name** substring) and degraded combobox typed search ([#328](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/328), [#314](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/314)):

| Step | Expected |
| ---- | -------- |
| `./scripts/qa/verify-issue-328.sh` | Vitest PASS; live `q=Ember` returns pairs with `Ember` in `asset_*.name` or `EMBER` symbol when indexer is up |
| Stop indexer → `/trade` or `/limits` → type `EMBER` in pair combobox | Options list matching factory pairs (not “No pairs match”); **Offline search** hint |
| Indexer up → type `EMBER` | Indexer relevance results (unchanged healthy path) |

Deploy seeds CW20 **`name`** / **`symbol`** in Phase 2 (`scripts/deploy-dex-local.sh`). If `assets.name` is empty in Postgres, restart indexer after deploy so pair discovery re-resolves metadata.

### Fee-discount TCL8Y proxy (LocalTerra, GitLab #383)

Trading tokens (EMBER, CORAL, …) use **6** decimals. Fee-discount `min_cl8y_balance` values assume **18** decimals. Deploy creates **TCL8Y** and sets `VITE_CL8Y_TOKEN_ADDRESS` to it (not EMBER).

| Step | Expected |
| ---- | -------- |
| `make verify-issue-383` | TCL8Y `decimals=18`; tier-1 register + deregister on-chain (FT-3 / FT-4) |
| `make verify-issue-384` | `getGasLimitForTx` register/deregister limits; optional live `gas_used` check |
| `make verify-issue-475` | Retail execute-msg gas inventory / `BASE_GAS_LIMIT` guardrail; faucet `drip` (#474); optional live drip `gas_used` |
| `make verify-issue-599` | Unwrap+≥2hop USTR→USTC gas combo (`UNWRAP_ROUTER_COMBO_OVERHEAD_GAS`); inventory `send_2hop_unwrap_ustc`; E9 needs LocalTerra |
| `make verify-issue-600` | Post-merge !400 LocalTerra E9 + columbus-5 unwrap gas (`Q8` / **M600-1–M600-8**); children 599 + 587 |
| `/tiers` + Keplr or Simulated Wallet | Register tier 1 succeeds when wallet holds ≥ 1 TCL8Y (requires #384 gas limits) |

### Post-merge stack !368–!377 (GitLab #573)

After stacking logo / WC / Charts stats / hub P&amp;L / tape / gem hide / trade layout / ticket heading / Ledger copy, run the umbrella verify then Coolify+indexer together:

| Step | Expected |
| ---- | -------- |
| `make verify-issue-573` | Child verifies **557, 560, 561, 562, 563, 564, 565, 566, 567** plus C+8 favicon and Coolify env docs (**M573-1–M573-8**) |
| Coolify frontend rebuild | `VITE_NETWORK=mainnet`, `VITE_SHOW_TEST_TOKENS` unset, `VITE_FAUCET_ADDRESS` unset |
| Indexer restart | Additive #557 decimals; confirm #553 / #556 migrations if not live |

Agent playbook: [`skills/AGENTS_POST_MERGE_STACK.md`](../../skills/AGENTS_POST_MERGE_STACK.md). QA invariant **Q6**: [`docs/qa-invariants.md`](../../docs/qa-invariants.md#post-merge-stack-573).

### Post-merge !394–!396 (GitLab #590)

After stacking protocol fees, wrap+≥2hop gas, and the CW20 audit harness:

| Step | Expected |
| ---- | -------- |
| `make verify-issue-590` | Children **586, 587, 589** plus L7/unwrap classification (**M590-1–M590-8**) |
| `CODE_ID=8266 LAYER_B_LT=1 make verify-issue-589` | A-lcd/B-lt **execute** pinned wasm (not a stub) |
| Coolify indexer + dApp | Fee migration + `WRAP_MAPPER_ADDRESS`; `/protocol` fees + Swap Network fee |
| 8266 | REPORT **GO**; columbus-5 listed 2026-08-22. Do **not** whitelist a LocalTerra store id. ALPHA **8654** stays off. |
| 11611 | Community tax (#601) REPORT **GO**; listed 2026-08-23. Live whitelist **`[6036, 8266, 10184, 11611, 11619]`**. Canonical launcher `terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze` is code **11622**; `GetConfig` **11619** / **11621**. Unused 11612 `terra1af9xm…` has no `CreateToken`. Do **not** whitelist 11612/11613/11614/11620/11621/11622. Gate: `make verify-issue-601` (A-lcd/B-lt + LocalTerra smoke). Tax-on DEX (router / AutoLP) is `make verify-issue-623` — **not** B-lt. Post-merge Coolify: `make verify-issue-602` (**Q9**). Enable Feature remainder: `make verify-issue-612` (**Q10**). |

Agent playbook: [`skills/AGENTS_POST_MERGE_OPS_590.md`](../../skills/AGENTS_POST_MERGE_OPS_590.md). QA invariant **Q7**: [`docs/qa-invariants.md`](../../docs/qa-invariants.md#post-merge-ops-590).

### Post-merge !400 unwrap gas (GitLab #600)

After !400 landed unit/docs for `UNWRAP_ROUTER_COMBO_OVERHEAD_GAS` without chain QA:

| Step | Expected |
| ---- | -------- |
| `make verify-issue-600` | Children **599, 587** plus Q8 / **M600-1–M600-8** |
| `VERIFY600_REQUIRE_CHAIN=1 make verify-issue-600` | Playwright **E9** (and **E7**) e2e-tx; `gas_used < gas_wanted` |
| `VERIFY600_COLUMBUS_TX=<hash> make verify-issue-600` | Columbus-5 LCD `gasUsed < gasWanted` and used < 3.11M |

Agent playbook: [`skills/AGENTS_POST_MERGE_OPS_600.md`](../../skills/AGENTS_POST_MERGE_OPS_600.md). QA invariant **Q8**: [`docs/qa-invariants.md`](../../docs/qa-invariants.md#post-merge-ops-600).

### Post-merge !402 Create Token (GitLab #602)

After !402 landed Create Token + catalog without Coolify bake / LocalTerra retail:

| Step | Expected |
| ---- | -------- |
| `make verify-issue-602` | Children **593, 594** plus Q9 / **M602-1–M602-8** |
| Coolify frontend | `VITE_COMMUNITY_TAX_CODE_ID=11619` + launcher `terra126pr5…` (code **11622**; not unused 11612) |
| Coolify indexer | `GET /api/v1/community-tokens` `{ configured: true, … }` |
| LocalTerra | `VERIFY602_REQUIRE_CHAIN=1` smoke + `/token/create` retail |

Agent playbook: [`skills/AGENTS_POST_MERGE_OPS_602.md`](../../skills/AGENTS_POST_MERGE_OPS_602.md). QA invariant **Q9**: [`docs/qa-invariants.md`](../../docs/qa-invariants.md#post-merge-ops-602).

### Post-merge !407/!408 Enable Feature (GitLab #612)

After !407/!408 landed Enable Feature + unique SKUs without LocalTerra smoke / launcher migrate:

| Step | Expected |
| ---- | -------- |
| `make verify-issue-612` | Children **606, 607** plus Q10 / **M612-1–M612-8** |
| Columbus-5 launcher | `terra126pr5…` code **11622**, `GetConfig` **11619** / **11621** |
| Coolify frontend | `VITE_COMMUNITY_TAX_CODE_ID=11619` + launcher `terra126pr5…` (not unused 11612) |
| LocalTerra | `VERIFY612_REQUIRE_CHAIN=1` → `verify-issue-601` `sku_unlock_via_launcher` + paid create + second SKU |
| Disclose | Do **not** run option-1 `Route skips buy/sell tax` — that is [#616](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/616) |

Agent playbook: [`skills/AGENTS_POST_MERGE_OPS_612.md`](../../skills/AGENTS_POST_MERGE_OPS_612.md). QA invariant **Q10**: [`docs/qa-invariants.md`](../../docs/qa-invariants.md#post-merge-ops-612).

### Post-merge !409–!413 option-2 / wrap / window / AutoLP / ranking (GitLab #616)

After !409–!413 landed option-2 classify, wrap ingest, window pin, AutoLP floor, and tax ranking:

| Step | Expected |
| ---- | -------- |
| `make verify-issue-616` | Children **607, 610, 613, 614, 615** plus Q11 / **M616-1–M616-8** |
| Columbus-5 launcher | `terra126pr5…` code **11622**, `GetConfig` **11619** / **11621**; 11611+11619 listed |
| Coolify frontend | `VITE_COMMUNITY_TAX_CODE_ID=11619` + option-2 copy (not `Route skips buy/sell tax`) |
| Coolify indexer | `UST1_WINDOW_ADDRESS=terra1zxwpz…h3rh2` (Vite is not enough; live pin + mint/redeem ingest as of 2026-08-25); wrap `event_count ≥ 1`; `COMMUNITY_TAX_OPTION2_CODE_IDS=11619` after a 11619 instance |
| Disclose | Pair-direct **and** hops: `Sell tax extra` / `Buy tax applies` |

Agent playbook: [`skills/AGENTS_POST_MERGE_OPS_616.md`](../../skills/AGENTS_POST_MERGE_OPS_616.md). QA invariant **Q11**: [`docs/qa-invariants.md`](../../docs/qa-invariants.md#post-merge-ops-616).

### Post-merge !414 LocalTerra community-tax seed leftovers (GitLab #624)

After !414 landed the #620 seed + Transfer funding fork:

| Step | Expected |
| ---- | -------- |
| `make verify-issue-624` | Child **620** plus leftover live + children **601, 592, 610, 594** / **M624-1–M624-8** |
| Fresh volume | `VERIFY624_FRESH=1` → `make reset && make start && make deploy-local`; stamp `git_sha` == `HEAD` |
| Indexer | `GET /api/v1/community-tokens` `configured: true` + QA token `attested_cmm` (local code id, not 11619) |
| Funding | `e2e-provision-dev-wallet.sh` Transfer; swarm `--dry-run` logs `swarm_funding_plan` and does **not** call `fundBotWallets` |
| Disclose | Do **not** reopen #620 / #592 / #601 / #610 / #594; siblings #621 / #622 / #623 / #625 stay their tickets |

Agent playbook: [`skills/AGENTS_POST_MERGE_OPS_624.md`](../../skills/AGENTS_POST_MERGE_OPS_624.md). QA invariant **Q12**: [`docs/qa-invariants.md`](../../docs/qa-invariants.md#post-merge-ops-624).

### Post-merge !415–!417 tax swarm / e2e-tx / Layer B leftovers (GitLab #625)

After !415–!417 landed tax-aware swarm, named tax-on Layer B, and Playwright e2e-tx:

| Step | Expected |
| ---- | -------- |
| `make verify-issue-625` | Children **621, 622, 623** plus leftover live + **293** / **M625-1–M625-8** |
| Tax-on seed buy | `LAYER_B_TAX_ON=1` buy from `pick_trader` (non-treasury); artifact `buy_user` set |
| Tax-on ephemeral | `LAYER_B_TAX_ON_FORCE_EPHEMERAL=1` on a #624 seed volume; artifact `source=ephemeral` |
| Playwright P0 | `VERIFY_ISSUE_622_CHAIN=1` sell extra-debit + buy net + provide/limit; indexer `CORS_ORIGINS` includes `http://127.0.0.1:3173` |
| Seed treasury | `GetConfig.treasury` ≠ test1 (CMM stand-in stays test1) |
| Swarm soak | `tax_listed` extra-debit + `tax_hybrid_skip`; `SWARM_TAX_WORKERS=0` exclude-only |
| Disclose | Do **not** reopen #621 / #622 / #623 / #620; do not merge tax-on into B-lt |

Agent playbook: [`skills/AGENTS_POST_MERGE_OPS_625.md`](../../skills/AGENTS_POST_MERGE_OPS_625.md). QA invariant **Q13**: [`docs/qa-invariants.md`](../../docs/qa-invariants.md#post-merge-ops-625).

### Post-merge !418 community-tax migrate leftovers (GitLab #628)

After !418 landed free `/token/migrate` + `adopt.rs`:

| Step | Expected |
| ---- | -------- |
| `make verify-issue-628` | Children **626, 592, 593, 594** plus leftover live / **M628-1–M628-8** / **Q14** |
| Adopt pin | Current listed tax pin (**11630** after #635; **11626** was the #628 store) — not 11619 |
| Factory list | `[6036, 8266, 10184, 11630]` — never 8654 or code 3 |
| Coolify / indexer | Single-id bake of the listed tax pin; `VITE_COMMUNITY_MIGRATE_CODE_IDS` includes 8654 |
| 6036 cw2 | Live instance is `crates.io:cw20-base` (do not append `terraswap-token` from this ticket) |
| LocalTerra | [`localterra-628-migrate-leftover.sh`](./localterra-628-migrate-leftover.sh) P3 / P7 / P11 |
| Create Token | Code-id-free lead + **Migrate here** — 8654 is allowlist/docs only |
| Disclose | Do **not** reopen #626 / implement #627 / factory-list 8654 or code 3 |

Agent playbook: [`skills/AGENTS_POST_MERGE_OPS_628.md`](../../skills/AGENTS_POST_MERGE_OPS_628.md). QA invariant **Q14**: [`docs/qa-invariants.md`](../../docs/qa-invariants.md#post-merge-ops-628).

### Post-merge !437–!458 leftover verify (GitLab #673)

After !437–!458 landed pool/charts/protocol/trader leftover UI on `main` (`8af5563c+`):

| Step | Expected |
| ---- | -------- |
| `make verify-issue-673` | Children **655–672** plus leftover live / **M673-1–M673-8** / **Q15** |
| Coolify Postgres | Migrations `20260826150000_pair_liquidity_usd.sql` + `20260826180000_protocol_volume_hourly_monthly.sql`, then indexer redeploy |
| Live pairs | `created_at` always; `liquidity_usd` when stamped; `sort=liquidity_usd` allowlisted |
| Live leaderboard | `?pair=` unknown **404**; `sort=best_trade_pnl` with `pair` **400**; unscoped board unchanged |
| Live volume | `GET /protocol/volume/daily?grain=hourly\|daily\|monthly` (not a `/hourly` path) |
| Frontend | Rebuild from `8af5563c+`; no `charts-overview-*`; `/protocol` keeps census |
| Disclose | Do **not** reopen #655–#672; do not restore Charts census or a second `pair_liquidity.rs`; do not commit `tmp-558-*` |

Agent playbook: [`skills/AGENTS_POST_MERGE_OPS_673.md`](../../skills/AGENTS_POST_MERGE_OPS_673.md). QA invariant **Q15**: [`docs/qa-invariants.md`](../../docs/qa-invariants.md#post-merge-ops-673).

### Post-merge !459–!468 leftover verify (GitLab #686)

After !459–!468 landed portfolio/protocol/charts/gas/fee leftover UI on `main` (`36d64528+`):

| Step | Expected |
| ---- | -------- |
| `make verify-issue-686` | Children **674–680, 683** plus leftover live / **M686-1–M686-8** / **Q16** |
| Coolify Postgres | Migrations `20260827120000_trader_positions_numeric_78.sql` + `20260827140000_economic_token_marks.sql`, then indexer redeploy |
| Live hub | Four cells only; `ticker=cl8y` **400**; `HUB_CL8Y_ADDRESS` pin |
| Live fees | `/protocol` token mix CL8Y-cb human + `$` when a qualifying pair exists |
| Frontend | Rebuild from `36d64528+`; `/portfolio` hide-gems + Mark/Unrealized; Charts UST1/USD; Swap Quote only |
| Disclose | Do **not** reopen #674–#680 / #683; do not expand `/hub-prices`; do not touch #684 |

Agent playbook: [`skills/AGENTS_POST_MERGE_OPS_686.md`](../../skills/AGENTS_POST_MERGE_OPS_686.md). QA invariant **Q16**: [`docs/qa-invariants.md`](../../docs/qa-invariants.md#post-merge-ops-686).

### Listed-pair autoregister + migrate inventory (GitLab #633 / #634)

LocalTerra-only live rungs (columbus-5 factory/token migrate and Open/ALPHA LCD stay on [#635](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/635) / [#636](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/636)):

| Step | Expected |
| ---- | -------- |
| `VERIFY633_REQUIRE_CHAIN=1 make verify-issue-633` | Crates + Vitest + docs, then [`localterra-633-autoregister.sh`](./localterra-633-autoregister.sh): seed registered; factory `CreatePair` tax/UST1 autoregisters; honest/honest create does not revert; manager Honest; retail extra-debit |
| `VERIFY634_REQUIRE_CHAIN=1 make verify-issue-634` | Vitest + docs, then [`localterra-634-migrate-inventory.sh`](./localterra-634-migrate-inventory.sh): mintable + CL8Y pair → adopt does not register → F6 freeze → ops Refresh → factory-only register |

Host Postgres on `:5432` can block compose `postgres`; LocalTerra + `make deploy-local` is enough for these LCD rungs.

Agent playbooks: [`AGENTS_COMMUNITY_TAX_AUTOREGISTER.md`](../../skills/AGENTS_COMMUNITY_TAX_AUTOREGISTER.md), [`AGENTS_FRONTEND_TOKEN_MIGRATE.md`](../../skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md).

---

## Makefile reference

| Target              | Purpose                                      |
| ------------------- | -------------------------------------------- |
| `make start-qa`     | Full QA bring-up (skips deploy when stamp + LCD probe match HEAD); runs post-deploy smoke after verify |
| `QA_SKIP_SMOKE=1 make start-qa` | Skip post-deploy `smoke-pool-swap` (indexer-only debugging) |
| `make smoke-pool-swap` | Pool LCD smoke using deploy stamp pair ([#368](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/368)) |
| `make deploy-local-no-build` | Deploy only — wasm artifacts must already exist |
| `QA_DEPLOY_SEED=minimal\|charts\|wallet\|full` | Lighter **`deploy-dex-local`** seed profiles ([#325](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/325)) |
| `QA_FETCH_CI_ARTIFACTS=1 make start-qa` | Try GitLab wasm/indexer packages before deploy |
| `make fetch-qa-ci-artifacts` | Download wasm/indexer for current **`HEAD`** sha |
| `make build-indexer-release` | Build **`indexer/target/release/cl8y-dex-indexer`** for **`INDEXER_QA_BIN`** |
| `make qa-start`     | Same as `start-qa`                           |
| `make reset-qa`     | Wipe LocalTerra + Postgres volumes, then `start-qa` |
| `QA_FRESH_VOLUMES=1 make start-qa` | Same as `reset-qa`              |
| `make stop-qa`      | Stop indexer + compose (volumes kept)      |
| `make test-qa-fresh-volumes` | Unit checks for fresh-volumes toggle (no Docker) |
| `make test-qa-verify-deploy` | Unit checks for LCD helpers + host-curl wiring (`test-verify-deploy`, `test-localterra-host-curl`) |
| `make test-localterra-host-curl` | Exec fallback wiring; live RPC probe when `localterra` is up |
| `make qa-tunnel-help` | Reprint SSH + laptop steps               |
| `make qa-verify-deploy` | Post-deploy schema + stamp + env address cross-check (also runs inside `start-qa`) |
| `make qa-verify-env-addresses` | Env/chain address cross-check only (SEC-H04 / [#442](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/442)) |
| `make qa-verify-deploy-config` | On-chain config assertions (SEC-H03 / [#441](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/441)) |
| `make status`       | Health summary                               |
| `make compose-ps`   | `docker compose ps` only                     |

---

## Troubleshooting

- **Indexer health fails** — Read **`.indexer-qa.log`**; confirm Postgres is up and **`indexer/.env`** **`DATABASE_URL`** matches compose (**`postgres://cl8y_legal:cl8y_legal@127.0.0.1:5432/dex_indexer`** by default (override via repo-root `.env` / `scripts/lib/postgres-dev.env`)). If the host Postgres only has **`postgres:postgres`**, run **`./scripts/setup-postgres-dev-databases.sh`** to bootstrap **`cl8y_legal`** (GitLab [**#245**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/245); [`skills/AGENTS_LOCAL_POSTGRES_DEV.md`](../../skills/AGENTS_LOCAL_POSTGRES_DEV.md)).
- **LocalTerra not ready** — `docker compose logs localterra`; on port conflicts set **`QA_SHARED_HOST=1`** or free host ports.
- **Host `curl` to `127.0.0.1:26657` / `:1317` hangs** — Chain may still be healthy in-container. **`make qa-verify-deploy`** and **`make wait-localterra`** use [`scripts/lib/localterra-host-curl.sh`](../lib/localterra-host-curl.sh) (`docker exec` fallback). Run **`make test-localterra-host-curl`** when compose is up. Frontend/browser still need working published ports or Docker daemon **`userland-proxy: false`** — see [`docs/local-development.md`](../../docs/local-development.md) troubleshooting.
- **Stale wasm on disk** — `make build-optimized` then re-run deploy ( **`make deploy-local`** ).
- **Stale deployed contracts (reused Docker volumes)** — GitLab [**#203**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203). **`make start-qa`** runs **`make qa-verify-deploy`** after deploy; it queries the deployed pair for **`is_paused`** and **`expired_limit_refund`** and compares **`.qa-deploy-stamp`** (`git_sha`) to **`HEAD`**. Failure exits non-zero with volume-reset instructions.

  | Symptom | Likely cause | Fix |
  | ------- | ------------ | --- |
  | `unknown variant` on `is_paused` / `expired_limit_refund` | On-chain pair wasm older than repo tree | **`make reset-qa`** (or below) |
  | `qa-verify-deploy`: stamp `git_sha` ≠ `HEAD` | Pulled new commits without redeploying | `make deploy-local` or full `make start-qa` |
  | Live QA walks pass UI but contract queries fail | Laptop **`.env.local`** from an old server deploy | Re-`scp` **`frontend-dapp/.env.local`** after server deploy |

  **Reset chain + Postgres state** (when reuse is unsafe — after contract schema changes or failed verification):

  ```bash
  make reset-qa
  # or manually:
  make stop-qa
  docker compose down -v
  docker volume rm cl8y-dex-terraclassic_localterra-data cl8y-dex-terraclassic_postgres-data
  make start-qa
  ```

  Agent playbook: [`skills/AGENTS_QA_DEPLOY_VERIFY.md`](../../skills/AGENTS_QA_DEPLOY_VERIFY.md). Fresh-volumes toggle: [`skills/AGENTS_QA_FRESH_VOLUMES.md`](../../skills/AGENTS_QA_FRESH_VOLUMES.md) ([#202](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/202)).

  **When volume reuse is safe:** infra-only changes, frontend/indexer-only work, or no pair query/schema changes since last verified deploy on the same **`git_sha`** stamp.
