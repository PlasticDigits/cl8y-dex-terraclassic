# Runbook: pool-only (v2) DEX launch

Ordered checklist for **pool-only** swaps: direct pair and router paths with **`hybrid` unset** (no on-chain limit-book leg). For hybrid-specific launch, see [`docs/reviews/20260409T030009Z/REVIEW.md`](../reviews/20260409T030009Z/REVIEW.md) §11.

**Related docs:** [`docs/deployment-guide.md`](../deployment-guide.md), [`docs/security-model.md`](../security-model.md), [`docs/architecture.md`](../architecture.md), fee tiers [`docs/reference/fee-discount-tiers.md`](../reference/fee-discount-tiers.md), QA sign-off [`QA_TEMPLATE.md`](../../QA_TEMPLATE.md) § SIGN-OFF, agent playbook [`skills/AGENTS_LAUNCH_GO_NO_GO.md`](../../skills/AGENTS_LAUNCH_GO_NO_GO.md), governance emergency rehearsal [`governance-emergency-rehearsal.md`](./governance-emergency-rehearsal.md) (**SEC-B09**, [#397](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/397)). **Full executable matrix:** [GitLab **#337**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/337) (**LR-00** launch-readiness gate).

**Mandatory gate:** [**Phase 5 — Go / no-go**](#phase-5--go--no-go-decision-required-before-production-mainnet) is a **required sign-off** before any **production mainnet** deploy. Complete Phases 0–4 on staging/testnet first; do **not** begin mainnet Phase 1 until Phase 5 records an explicit **GO** (or **GO with accepted risk**) on the launch tracking issue ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)).

---

## Phase 0 — Preconditions

- [ ] Governance and treasury addresses are **multisigs or DAO** (not EOAs); see [Security model § Governance](../security-model.md).
- [ ] **Wasm policy:** production code uploaded from **workspace-optimizer** artifacts (`make build-optimized`; reference spec [`.github/workflows/contracts-wasm-optimizer.yml`](../../.github/workflows/contracts-wasm-optimizer.yml)), not from dev `cargo` wasm alone — [docs/testing.md § CI](../testing.md#ci).
- [ ] **Hook policy:** either **no hooks** on pairs, or only **audited** hook contracts with bounded gas (hook revert fails the whole swap — [Security model § Hook safety](../security-model.md)). Follow [hook registration runbook](./hook-registration.md).
- [ ] **Code ID whitelist** on the factory lists only intended CW20 code IDs for pair assets. **No fee-on-transfer templates** — [CW20 whitelist policy](./cw20-whitelist-policy.md); run [`scripts/verify-cw20-code-ids.sh`](../../scripts/verify-cw20-code-ids.sh) for GDEX/TerraPort IDs before whitelist.

---

## Phase 1 — Deploy contracts

Follow [`docs/deployment-guide.md`](../deployment-guide.md): optimized wasm → store → instantiate factory (governance, treasury, fees, whitelist) → router → fee-discount → tiers → **trusted router** → `set_discount_registry_all` (or per-pair) → create pairs.

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
- [ ] **Discount registry** set on all pairs that should participate (`GetDiscountRegistry` per pair or factory-driven policy).

```bash
terrad query wasm contract-state smart <fee_discount> '{"get_tiers":{}}' --node <lcd>
terrad query wasm contract-state smart <fee_discount> '{"is_trusted_router":{"router":"<router_addr>"}}' --node <lcd>
```

- [ ] **Pause:** understand factory/pair pause implications for swaps and limit cancels ([`docs/limit-orders.md`](../limit-orders.md), security model).

---

## Phase 3 — Post-deploy verification (pool-only)

- [ ] **Read-only / light tx checks:** [`scripts/smoke-pool-swap.sh`](../../scripts/smoke-pool-swap.sh) — LCD pool query and optional **`hybrid_simulation`** with pool-only params (`book_input: 0`).
- [ ] **Wrap-mapper pause (SEC-B06):** [`scripts/smoke-wrap-mapper-pause.sh`](../../scripts/smoke-wrap-mapper-pause.sh) or `make smoke-wrap-mapper-pause` — on-chain wrap/unwrap rejection under `set_paused`, restored after unpause ([#396](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/396); UI CTA: SEC-A02 / [#389](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/389)).
- [ ] **Single-hop swap** on staging with small size; confirm treasury fee and balances.
- [ ] **Multi-hop** via router (still pool-only per hop) if used in production.
- [ ] Optional: run repo E2E against staging (`frontend-dapp` Playwright) if your process includes UI gates.

**Pool-only invariant:** swap and simulation messages must **not** set hybrid / book-leg fields for this v2 launch path.

---

## Phase 4 — Off-chain stack (if applicable)

- [ ] **Indexer:** `DATABASE_URL`, migrations, `FACTORY_ADDRESS`, LCD URLs, `CORS_ORIGINS`, optional `ROUTER_ADDRESS` per [`indexer/src/config.rs`](../../indexer/src/config.rs).
- [ ] **Indexer URL (frontend):** production `VITE_INDEXER_URL` must use **`https://`** only — no mixed-content `http:` to the quote API. See [Security model § Off-chain trust boundaries](../security-model.md#off-chain-trust-boundaries-frontend) ([#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378)).
- [ ] **Frontend:** `VITE_*` addresses per [`docs/frontend.md`](../frontend.md); `VITE_WC_PROJECT_ID` set before `npm run build` (production guard); no `VITE_DEV_MNEMONIC` in release env.
- [ ] **Frontend CSP:** production builds inject env-scoped `connect-src` via [`viteCsp.ts`](../../frontend-dapp/viteCsp.ts) (no blanket `https:`); `render.yaml` omits a static CSP header so policy stays build-time aware.

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
| **Admin controls** | Governance/treasury are EOAs; factory admin paths reachable by non-governance; hook allowlist missing for registered hooks; trusted-router or discount-registry miswired | [Security model § Governance](../security-model.md), [hook registration](./hook-registration.md), Phase 0–2 above |
| **Value-flow invariants** | Failing contract audit invariants (P1–P7 pool path, L1–L10 limit/hybrid); fee-on-transfer or non-standard CW20 on whitelist; treasury/fee accounting mismatch on staging smoke | [`docs/contracts-security-audit.md`](../contracts-security-audit.md), [`scripts/smoke-pool-swap.sh`](../../scripts/smoke-pool-swap.sh), [`cw20-whitelist-policy.md`](./cw20-whitelist-policy.md) |
| **Deploy / runbook** | Missing or unexecuted launch phases; optimizer wasm policy violated; `make check-fee-discount-tier-docs` or launch go/no-go doc check failing; no rollback/incident owner | This runbook, [deployment guide](../deployment-guide.md), [`make verify-issue-391`](../../Makefile) |
| **User visibility of risk** | Users cannot see **pause**, **blacklist**, or **rate-limit / indexer outage** risk in the dApp or docs; mixed-content indexer URL; missing WalletConnect or CSP deploy guards | [Security model § Off-chain trust boundaries](../security-model.md#off-chain-trust-boundaries-frontend), [frontend.md § Paused pair](../frontend.md), indexer rate limits in [`indexer-invariants.md`](../indexer-invariants.md) |

### PAUSE — delay launch

Choose **PAUSE** when there is **no** open P0 blocker, but **any** of the following is true:

- A **pre-launch** item from Phases 0–4 or [GitLab **#337**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/337) remains open **without** a linked risk-acceptance comment on the launch issue.
- Production **governance multisig** has not rehearsed emergency admin txs (**pause, blacklist, unpause, unblacklist**) on staging/testnet — see [**SEC-B09** governance emergency rehearsal](./governance-emergency-rehearsal.md) ([#397](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/397)); evidence must be linked from the launch tracking issue. Automated LocalTerra dry-run: `make rehearse-governance-emergency` / `make verify-issue-397`.
- Production **governance multisig** has not rehearsed other deploy/admin txs (fee update, hook registration) on staging.
- **Incident / rollback** runbook gap: no on-call owner, or [`docs/templates/incident-dex-indexer.md`](../templates/incident-dex-indexer.md) not adapted for this network.
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

**Automated doc invariant:** `make verify-issue-391` (or `make check-launch-go-no-go-docs`) must pass before treating this gate as satisfied in CI or agent workflows. **SEC-B09 multisig rehearsal:** `make verify-issue-397` (or `make check-governance-emergency-rehearsal-docs` for docs only).

---

## Rollback / incident

- CosmWasm upgrades/migrations are **out of band** for this runbook; document admin keys and wasm migration policy separately.
- For live incidents: pause via factory if your governance policy allows; communicate hook/pause behavior per security model.
