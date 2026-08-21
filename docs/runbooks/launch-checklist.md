# Runbook: pool-only (v2) DEX launch

Ordered checklist for **pool-only** swaps: direct pair and router paths with **`hybrid` unset** (no on-chain limit-book leg). For hybrid-specific launch, see [`docs/reviews/20260409T030009Z/REVIEW.md`](../reviews/20260409T030009Z/REVIEW.md) §11.

**Related docs:** [`docs/deployment-guide.md`](../deployment-guide.md), [`docs/security-model.md`](../security-model.md), [`docs/architecture.md`](../architecture.md), fee tiers [`docs/reference/fee-discount-tiers.md`](../reference/fee-discount-tiers.md), QA sign-off [`QA_TEMPLATE.md`](../../QA_TEMPLATE.md) § SIGN-OFF, agent playbook [`skills/AGENTS_LAUNCH_GO_NO_GO.md`](../../skills/AGENTS_LAUNCH_GO_NO_GO.md), test evidence gate [`skills/AGENTS_TEST_EVIDENCE_GATE.md`](../../skills/AGENTS_TEST_EVIDENCE_GATE.md) (**SEC-H08**, [#444](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/444)), governance emergency rehearsal [`governance-emergency-rehearsal.md`](./governance-emergency-rehearsal.md) (**SEC-B09**, [#397](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/397)), key custody [`key-custody.md`](./key-custody.md) (**SEC-B10**, [#398](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/398)). **Full executable matrix:** [GitLab **#337**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/337) (**LR-00** launch-readiness gate).

**Mandatory gate:** [**Phase 5 — Go / no-go**](#phase-5--go--no-go-decision-required-before-production-mainnet) is a **required sign-off** before any **production mainnet** deploy. Complete Phases 0–4 on staging/testnet first; do **not** begin mainnet Phase 1 until Phase 5 records an explicit **GO** (or **GO with accepted risk**) on the launch tracking issue ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)).

---

## Phase 0 — Preconditions

- [ ] Governance and treasury addresses are **multisigs or DAO** (not EOAs); **custody, signer roster, threshold, backup signer, and key-rotation policy** documented per [key custody runbook](./key-custody.md) (**SEC-B10**, [#398](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/398)). See [Security model § Governance](../security-model.md). Regression: `make verify-issue-398`.
- [ ] **Wasm policy:** production code uploaded from **workspace-optimizer** artifacts (`make build-optimized`; reference spec [`.github/workflows/contracts-wasm-optimizer.yml`](../../.github/workflows/contracts-wasm-optimizer.yml)), not from dev `cargo` wasm alone — [docs/testing.md § CI](../testing.md#ci).
- [ ] **Hook policy:** either **no hooks** on pairs, or only **audited** hook contracts with bounded gas (hook revert fails the whole swap — [Security model § Hook safety](../security-model.md)). Follow [hook registration runbook](./hook-registration.md).
- [ ] **Code ID whitelist** on the factory lists only intended CW20 code IDs for pair assets. **No fee-on-transfer templates** — [CW20 whitelist policy](./cw20-whitelist-policy.md); run [`scripts/verify-cw20-code-ids.sh`](../../scripts/verify-cw20-code-ids.sh) for GDEX/TerraPort IDs before whitelist. **Post-listing pin (F6 / [#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582) / [#584](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/584)):** factory **1.9.0** + pair **1.15.0** + `config.pair_code_id` **RAN 2026-08-21** on columbus-5 via [`scripts/upgrade-582-code-id-pin.sh`](../../scripts/upgrade-582-code-id-pin.sh). **8266 / #581 remains BLOCK.** Merge is not a listing go.
- [ ] **IBC-hooks chain exposure (SEC-D02):** record Terra Classic **chain binary / SDK version** and whether the **IBC-hooks** module is active on the target network; record that **app contracts do not expose IBC receive/ack/timeout entry points** (or document exposure + mitigation if that changes). **Re-run this gate after any chain upgrade or when adding new contract modules.** See [Security model § IBC hooks](../security-model.md#ibc-hooks-chain-dependency-sec-d02) and agent playbook [`skills/AGENTS_IBC_HOOKS_DEPLOY.md`](../../skills/AGENTS_IBC_HOOKS_DEPLOY.md) ([#407](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/407)).

  **Record on the launch / deploy tracking issue (paste command output + date UTC):**

  ```bash
  # Chain version at deploy time (replace <lcd> with production LCD URL)
  terrad version --long --node <lcd>
  terrad query params subspaces --node <lcd> | grep -i ibchooks || echo "IBC-hooks params subspace: not listed"
  # Optional when the node exposes module version queries:
  terrad query upgrade module_versions --node <lcd> 2>/dev/null | grep -i ibchooks || true

  # Static contract posture — must pass before deploy sign-off
  make verify-no-ibc-hooks-in-contracts
  # Or full #407 acceptance (docs + static grep):
  make verify-issue-407
  ```

  **Operator attestation (required text in the deploy record):** *"CL8Y DEX app contracts do not implement `ibc_receive`, `ibc_ack`, or `ibc_timeout` CosmWasm entry points; verified by `make verify-no-ibc-hooks-in-contracts` at commit `<git-sha>` on `<date-utc>`."* If a future release adds IBC callbacks, update this statement, document threat model + mitigations, and obtain security sign-off before mainnet upload.

- [ ] **Test evidence gate (SEC-H08):** before **production mainnet** deploy, record passing test output on the launch / release tracking issue ([#444](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/444)). Operators deploying from a **local build** must paste or link full command output (or a log artifact) for each suite below at the **same git SHA** as the wasm being uploaded. Operators deploying **CI-built artifacts** satisfy this gate automatically when the GitLab pipeline for that commit shows green `test-contracts`, `test-indexer-integration`, and `test-frontend` jobs — link the pipeline URL instead of re-pasting. LocalTerra swap smoke output is also required (see [Phase 3](#phase-3--post-deploy-verification-pool-only)).

  **Record on the launch / deploy tracking issue (paste command output + commit SHA + date UTC, or CI pipeline link):**

  ```bash
  git rev-parse HEAD   # must match deployed build SHA

  make test-contracts
  make test-indexer-integration   # requires Postgres — make setup-indexer-postgres or full stack
  make test-frontend

  # LocalTerra swap smoke — run after deploy (Phase 3); paste here or in deploy trace
  make smoke-pool-swap            # or ./scripts/smoke-pool-swap.sh with PAIR_ADDR / TERRA_LCD_URL
  ```

  **CI equivalent:** link the GitLab pipeline for commit `<git-sha>` with passing `test-contracts`, `test-indexer-integration`, and `test-frontend` jobs (see [docs/testing.md § CI](../testing.md#ci)).

  Agent playbook: [`skills/AGENTS_TEST_EVIDENCE_GATE.md`](../../skills/AGENTS_TEST_EVIDENCE_GATE.md). Deploy trace template: [`docs/templates/deploy-trace.md`](../templates/deploy-trace.md) § Test results. Regression: `make verify-issue-444`.

- [ ] **Indexer FACTORY_ADDRESS (SEC-I02):** confirm `FACTORY_ADDRESS` in `indexer/.env` is **non-empty** (whitespace-only is rejected) before starting the indexer — applies in **every** `RUN_MODE`, not only production ([#451](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/451)). Startup fails with `ConfigError::EmptyFactoryAddress` when missing. Post-deploy QA also asserts the env var via **`make qa-verify-deploy`**. Agent playbook: [`skills/AGENTS_FACTORY_ADDRESS_GUARD.md`](../../skills/AGENTS_FACTORY_ADDRESS_GUARD.md). Regression: `make verify-issue-451`.

---

## Phase 1 — Deploy contracts

Follow [`docs/deployment-guide.md`](../deployment-guide.md): optimized wasm → store → instantiate factory (governance, treasury, fees, whitelist) → router → fee-discount → tiers → **trusted router** → `set_discount_registry_all` (or per-pair) → create pairs.

**Soft launch shortcut (non-economic CW20 only):** [`mainnet-soft-launch.md`](./mainnet-soft-launch.md) / `make deploy-mainnet-soft-launch` (single script + Coolify Dockerfiles).

### Deploy trace (audit record) — required before leaving Phase 1

Record these fields on the **launch tracking issue** ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)) using [`docs/templates/deploy-trace.md`](../templates/deploy-trace.md). Without this trace, operators cannot reliably determine which code was deployed or what chain version it ran against after a missed security patch (SEC-D12, [GitLab #410](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/410)).

- [ ] **Git SHA** of the deployed build: `git rev-parse HEAD` — paste output into the launch tracking issue
- [ ] **Terra Classic chain version** at deploy time: `terrad version` or `terrad status --node <rpc> | jq -r .node_info.version` — paste output
- [ ] **Chain ID / network** (e.g. `columbus-5`, `rebel-2`, LocalTerra `localterra`)
- [ ] **Contract code IDs** for factory, pair, router, fee-discount (and hook contracts if deployed)
- [ ] **`wasm-checksums.txt`** artifact hashes from `smartcontracts/artifacts/wasm-checksums.txt`
- [ ] **Post-deploy verification command output** (at minimum: factory `get_config` query and [`scripts/smoke-pool-swap.sh`](../../scripts/smoke-pool-swap.sh) — paste or link log)

Agent playbook: [`skills/AGENTS_DEPLOY_TRACE.md`](../../skills/AGENTS_DEPLOY_TRACE.md). Regression: `make verify-issue-410`.

**Verify (replace placeholders, chain id, node, fees):**

```bash
terrad query wasm contract-state smart <factory> '{"get_config":{}}' --node <lcd>
terrad query wasm contract-state smart <router> '{"config":{}}' --node <lcd>
```

---

## Phase 2 — Governance-sensitive settings

- [ ] **Treasury** on factory matches intended fee recipient (`get_config`).
- [ ] **Fee-discount tiers** match [`docs/reference/fee-discount-tiers.md`](../reference/fee-discount-tiers.md) (or your approved variant); `make check-fee-discount-tier-docs` passes ([GitLab #198](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/198)).
- [ ] **Router** registered as **trusted** on fee-discount (`IsTrustedRouter` = true) before relying on `trader` forwarding for discounts.
- [ ] **Discount registry** set on all pairs that should participate (`GetDiscountRegistry` per pair). Factory `config.discount_registry` must be set so **new** `CreatePair`s inherit it ([#536](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/536), **F5**). Existing unwired pairs are a separate sweep ([#535](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/535)).

```bash
terrad query wasm contract-state smart <fee_discount> '{"get_tiers":{}}' --node <lcd>
terrad query wasm contract-state smart <fee_discount> '{"is_trusted_router":{"router":"<router_addr>"}}' --node <lcd>
```

- [ ] **Pause:** understand factory/pair pause implications for swaps and limit cancels ([`docs/limit-orders.md`](../limit-orders.md), security model).

---

## Phase 3 — Post-deploy verification (pool-only)

- [ ] **Config assertions (SEC-H03):** run [`scripts/qa/verify-deploy-config.sh`](../../scripts/qa/verify-deploy-config.sh) (`make qa-verify-deploy-config`) and **paste the full output** on the release / launch tracking issue. The script queries factory governance, treasury, default fee, whitelisted CW20 code IDs, fee-discount tiers, trusted router status, pair hooks, and blacklist clean-wallet state; exits non-zero if any required field is empty or fails assertion ([#441](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/441)). Agent playbook: [`skills/AGENTS_DEPLOY_CONFIG_VERIFY.md`](../../skills/AGENTS_DEPLOY_CONFIG_VERIFY.md); regression: `make verify-issue-441`.
- [ ] **Read-only / light tx checks:** [`scripts/smoke-pool-swap.sh`](../../scripts/smoke-pool-swap.sh) — LCD pool query and optional **`hybrid_simulation`** with pool-only params (`book_input: 0`).
- [ ] **Wrap-mapper pause (SEC-B06):** [`scripts/smoke-wrap-mapper-pause.sh`](../../scripts/smoke-wrap-mapper-pause.sh) or `make smoke-wrap-mapper-pause` — on-chain wrap/unwrap rejection under `set_paused`, restored after unpause ([#396](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/396); UI CTA: SEC-A02 / [#389](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/389)).
- [ ] **Single-hop swap** on staging with small size; confirm treasury fee and balances.
- [ ] **Multi-hop** via router (still pool-only per hop) if used in production.
- [ ] Optional: run repo E2E against staging (`frontend-dapp` Playwright) if your process includes UI gates.

**Pool-only invariant:** swap and simulation messages must **not** set hybrid / book-leg fields for this v2 launch path.

---

## Phase 4 — Off-chain stack (if applicable)

- [ ] **Env/chain address cross-check (SEC-H04):** run [`scripts/qa/verify-env-addresses.sh`](../../scripts/qa/verify-env-addresses.sh) (`make qa-verify-env-addresses`) and **paste the full output** on the release / launch tracking issue before go/no-go sign-off. The script compares `FACTORY_ADDRESS`, `ROUTER_ADDRESS`, and `FEE_DISCOUNT_ADDRESS` in `indexer/.env` against `VITE_*` counterparts in the frontend env, then queries on-chain factory/router/fee-discount `config` to assert router `factory` matches env and fee-discount `governance` is set; exits non-zero on any mismatch ([#442](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/442)). Also runs inside **`make qa-verify-deploy`** after schema/stamp checks. Agent playbook: [`skills/AGENTS_DEPLOY_ENV_ADDRESSES_VERIFY.md`](../../skills/AGENTS_DEPLOY_ENV_ADDRESSES_VERIFY.md); regression: `make verify-issue-442`.
- [ ] **Indexer:** `DATABASE_URL`, migrations, `FACTORY_ADDRESS`, LCD URLs, `CORS_ORIGINS`, optional `ROUTER_ADDRESS` per [`indexer/src/config.rs`](../../indexer/src/config.rs).
- [ ] **Indexer URL (frontend):** production `VITE_INDEXER_URL` must use **`https://`** only — no mixed-content `http:` to the quote API. See [Security model § Off-chain trust boundaries](../security-model.md#off-chain-trust-boundaries-frontend) ([#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378)).
- [ ] **Frontend:** `VITE_*` addresses per [`docs/frontend.md`](../frontend.md); `VITE_WC_PROJECT_ID` set before `npm run build` (production guard); no `VITE_DEV_MNEMONIC` in release env.
- [ ] **Frontend CSP:** production builds inject env-scoped `connect-src` via [`viteCsp.ts`](../../frontend-dapp/viteCsp.ts) (no blanket `https:`); `render.yaml` omits a static CSP header so policy stays build-time aware.
- [ ] **Extension fee guard scope (SEC-E08):** confirm the post-sign fee/gas guard is **LocalTerra-only** by design and that **Keplr on mainnet does not require** this check ([#429](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/429)). Run `make verify-issue-429` (doc invariants + unit test that `columbus-5` skips the guard). Record manual wallet QA on the launch tracking issue per [`extension-fee-guard-wallet-qa.md`](./extension-fee-guard-wallet-qa.md) — Keplr swap on `columbus-5` (or staging) with correct fees, and Keplr/simulated wallet on LocalTerra without false `Transaction fee mismatch`. See [Security model § Extension fee guard](../security-model.md#extension-wallet-fee-guard-sec-e08) and [`skills/AGENTS_EXTENSION_FEE_GUARD.md`](../../skills/AGENTS_EXTENSION_FEE_GUARD.md).
- [ ] **Launch monitoring (SEC-G01):** operator has the [`launch-monitoring.md`](./launch-monitoring.md) signals wired — indexer lag, contract/index errors, API 429/5xx, large swaps, large LP withdrawals, blacklist hits, pause-state changes, and the existing reorg halt. Run `make check-launch-monitoring-docs` ([#434](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/434)).

---

## Phase 5 — Go / no-go decision (required before production mainnet)

Consolidates launch criteria that were previously scattered across review artifacts ([`RELEASE_READINESS_MATRIX.md`](../reviews/20260409T030009Z/RELEASE_READINESS_MATRIX.md), [`REVIEW.md`](../reviews/20260409T030009Z/REVIEW.md)) and QA checklists. **One explicit decision** is required before production mainnet deploy.

### Decision outcomes

| Decision | When to choose | Action |
|----------|----------------|--------|
| **BLOCK** | Any **P0** blocker is open (see below) | Do **not** deploy to production mainnet. File or reopen issues; re-run Phases 0–4 on staging after fixes. |
| **PAUSE** | No P0 blockers, but launch is not ready to proceed on schedule | Delay mainnet deploy. Document why, target resume date, and owners for each open pre-launch item. |
| **GO** | All P0 items closed; all pre-launch items closed | Proceed to mainnet Phase 1. |
| **GO with accepted risk** | All P0 items closed; some pre-launch items remain open with **documented** risk acceptance | Proceed only after named sign-off (below) records each accepted residual risk on the launch issue. |

### BLOCK — do not launch (any P0 open)

Choose **BLOCK** when **any** of these P0 categories has an open, unmitigated finding:

| P0 category | Examples (not exhaustive) | Primary references |
|-------------|---------------------------|-------------------|
| **Admin controls** | Governance/treasury are EOAs; factory admin paths reachable by non-governance; hook allowlist missing for registered hooks; trusted-router or discount-registry miswired; no custody signer roster / rotation policy | [Security model § Governance](../security-model.md), [key custody](./key-custody.md) (**SEC-B10**), [hook registration](./hook-registration.md), Phase 0–2 above |
| **Value-flow invariants** | Failing contract audit invariants (P1–P7 pool path, L1–L10 limit/hybrid); fee-on-transfer or non-standard CW20 on whitelist; treasury/fee accounting mismatch on staging smoke | [`docs/contracts-security-audit.md`](../contracts-security-audit.md), [`scripts/smoke-pool-swap.sh`](../../scripts/smoke-pool-swap.sh), [`cw20-whitelist-policy.md`](./cw20-whitelist-policy.md) |
| **Deploy / runbook** | Missing or unexecuted launch phases; optimizer wasm policy violated; `make check-fee-discount-tier-docs` or launch go/no-go doc check failing; **SEC-D02** IBC-hooks chain version / contract IBC entry-point record missing or stale after chain upgrade; **SEC-H08** test evidence (contracts, indexer integration, frontend, pool smoke) not pasted or linked on the release issue; no rollback/incident owner | This runbook, [deployment guide](../deployment-guide.md), [`make verify-issue-391`](../../Makefile), [`make verify-issue-407`](../../Makefile), [`make verify-issue-444`](../../Makefile) |
| **User visibility of risk** | Users cannot see **pause**, **blacklist**, or **rate-limit / indexer outage** risk in the dApp or docs; mixed-content indexer URL; missing WalletConnect or CSP deploy guards | [Security model § Off-chain trust boundaries](../security-model.md#off-chain-trust-boundaries-frontend), [frontend.md § Paused pair](../frontend.md), indexer rate limits in [`indexer-invariants.md`](../indexer-invariants.md) |

### PAUSE — delay launch

Choose **PAUSE** when there is **no** open P0 blocker, but **any** of the following is true:

- A **pre-launch** item from Phases 0–4 or [GitLab **#337**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/337) remains open **without** a linked risk-acceptance comment on the launch issue.
- Production **governance multisig** has not rehearsed emergency admin txs (**pause, blacklist, unpause, unblacklist**) on staging/testnet — see [**SEC-B09** governance emergency rehearsal](./governance-emergency-rehearsal.md) ([#397](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/397)); evidence must be linked from the launch tracking issue. Automated LocalTerra dry-run: `make rehearse-governance-emergency` / `make verify-issue-397`.
- Production **governance multisig** has not rehearsed a **key rotation** (wasm contract-admin + `governance` pointer) on staging/testnet — see [**SEC-D10** governance key rotation](./governance-key-rotation.md) ([#408](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/408)); evidence must be linked from the launch tracking issue. Automated LocalTerra dry-run: `make rehearse-governance-key-rotation` / `make verify-issue-408`.
- Production **governance multisig** has not rehearsed other deploy/admin txs (fee update, hook registration) on staging.
- **Incident / rollback** runbook gap: no on-call owner, or [`docs/templates/incident-dex-indexer.md`](../templates/incident-dex-indexer.md) not adapted for this network (including [anomaly signals](./anomaly-signals.md) thresholds for bootstrap TVL — SEC-G02, [#435](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/435)).
- External dependency blocker (indexer DB migration, DNS/TLS, WalletConnect project) is unresolved.

Record the pause reason and planned resume date on the launch tracking issue.

### GO with accepted risk — residual risks documented

Choose **GO with accepted risk** only when:

1. **All P0 categories above are closed** (no BLOCK triggers).
2. Every remaining pre-launch gap is either **closed** or **explicitly risk-accepted** in a named comment on the launch issue (link the GitLab issue or checklist row).
3. **Residual risks** are listed in the sign-off comment (severity, owner, review date).

**GO** (no qualifier) requires item 2 with **zero** open pre-launch gaps.

### Mandatory sign-off gate (final step)

**Do not begin mainnet Phase 1 until this step is complete.**

1. Complete Phases **0–4** on staging/testnet and the [#337](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/337) matrix as applicable.
2. Record the go/no-go decision using the role table from [`QA_TEMPLATE.md` § SIGN-OFF](../../QA_TEMPLATE.md#sign-off):

| Role | Name | Date | Signature |
|------|------|------|-----------|
| QA Tester | | | |
| Dev Lead | | | |
| Product Owner | | | |

3. **Post the sign-off on the launch tracking issue** (create one if needed). The issue comment must include:
   - **Decision:** `BLOCK`, `PAUSE`, `GO`, or `GO with accepted risk`
   - **Date** (UTC)
   - **Open residual risks** (or `none`)
   - **Risk acceptance statement** (required for `GO with accepted risk`; for `GO`, state that no residual pre-launch gaps remain)
   - Links to any risk-accepted GitLab issues

4. For **BLOCK** or **PAUSE**, stop here — do not deploy to production mainnet.

5. For **GO** or **GO with accepted risk**, attach or link the completed QA summary table from [`QA_TEMPLATE.md` § Summary](../../QA_TEMPLATE.md#summary) when your process requires it, then proceed to mainnet Phase 1.

**Automated doc invariant:** `make verify-issue-391` (or `make check-launch-go-no-go-docs`) must pass before treating this gate as satisfied in CI or agent workflows. **SEC-H08 test evidence:** confirm Phase 0 test output (or CI pipeline link) is on the launch issue before **GO** — `make verify-issue-444` (docs only). **SEC-B09 multisig rehearsal:** `make verify-issue-397` (or `make check-governance-emergency-rehearsal-docs` for docs only). **SEC-B10 key custody:** `make verify-issue-398` (or `make check-key-custody-docs` for docs only). **SEC-D10 key rotation:** `make verify-issue-408` (or `make check-governance-key-rotation-docs` for docs only).

### LP ticker digits ([#518](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/518))

**BLOCK** economic-pool creation (UST1/cUSTC, UST1/USTR, cLUNC/UST1, any CL8Y pair) until the #518 upgrade has run: new pair wasm **and** factory `lp_token_code_id` on digit-allowing `cw20-mintable` (`./scripts/upgrade-518-lp-symbol.sh`). Classic LP CW20 still rejects `UST1-CUST-LP`. Regression: `make verify-issue-518`. Playbook: [`skills/AGENTS_LP_SYMBOL_DIGITS.md`](../../skills/AGENTS_LP_SYMBOL_DIGITS.md). Invariant **F3**.

### Listed CW20 `code_id` pin ([#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582) / [#584](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/584))

**RAN 2026-08-21** on columbus-5: factory **11602** / cw2 **1.9.0**, every listed pair **11601** / cw2 **1.15.0**, `config.pair_code_id` **11601**, smoke 14/14 ([#584](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/584) close comment). Future F6 wasm upgrades still use [`scripts/upgrade-582-code-id-pin.sh`](../../scripts/upgrade-582-code-id-pin.sh) (factory first, `UpdateConfig { pair_code_id }`, LCD retries). **[#581](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/581) / 8266 SpaceUSD remains BLOCK** until source review (or admin cleared / wrap-to-10184) — do not `AddWhitelistedCodeId 8266` because F6 is live. Regression: `make verify-issue-582` / `make verify-issue-584`. Playbook: [`skills/AGENTS_CW20_CODE_ID_PIN.md`](../../skills/AGENTS_CW20_CODE_ID_PIN.md). Incident / unfreeze: [`cw20-code-id-ops.md`](./cw20-code-id-ops.md). Invariant **F6**.

### UST1 / wrap Phase 5 extras ([#503](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/503))

Before marketing UST1 withdraw capacity or native wrap on `dex.cl8y.com`, complete the go/no-go extras in [`ust1-wrap-production-ops.md`](./ust1-wrap-production-ops.md) (oracle freshness, treasury vFDUSD capacity, wrap unpaused, silence-alert + pause-drill evidence on #503). Probe:

```bash
make verify-issue-503
UST1_OPS_STRICT_PAUSE=1 UST1_OPS_STRICT_STALE=1 UST1_OPS_STRICT_INVENTORY=1 ./scripts/check-ust1-wrap-ops-health.sh
```

Agent playbook: [`skills/AGENTS_UST1_WRAP_PRODUCTION_OPS.md`](../../skills/AGENTS_UST1_WRAP_PRODUCTION_OPS.md). Registry / Coolify pack: [`deployments/mainnet-ust1-wrap/`](../../deployments/mainnet-ust1-wrap/).

---

## Rollback / incident

- **Rollback vs forward-fix (SEC-H09):** classify the incident surface (frontend, indexer, contract, chain dependency) and follow the decision tree in [rollback-decision.md](./rollback-decision.md) — decision criteria, rollback commands, limitations, and recovery verification for each type ([#445](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/445)). Agent playbook: [`skills/AGENTS_ROLLBACK_DECISION.md`](../../skills/AGENTS_ROLLBACK_DECISION.md).
- CosmWasm upgrades/migrations are **out of band** for this runbook's deploy phases; admin keys and wasm migration policy live in [wasm admin migration](./wasm-admin-migration.md). **Rollback limitations** (reversible vs irrecoverable migration, indexer DB down.sql, partial fleet recovery): [§ Rollback and limitations](./wasm-admin-migration.md#rollback-and-limitations-sec-h05) (**SEC-H05**, [#443](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/443)); contract decision criteria in [rollback-decision.md § Contract](./rollback-decision.md#3-contract-incident).
- For **active on-chain loss**: pause or blacklist via [emergency-commands.md](./emergency-commands.md) while executing the off-chain rollback path; communicate per [security model](../security-model.md) and [incident template](../templates/incident-dex-indexer.md).

**Doc invariants:** `make check-rollback-decision-docs` or `make verify-issue-445` (SEC-H09); `make verify-issue-443` (SEC-H05).
