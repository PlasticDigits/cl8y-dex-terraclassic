# Internal Security Audit — MR 389 / Issue #582 (listed CW20 `code_id` pin + factory whitelist re-check)

| | |
|---|---|
| **Audit ID** | `INTERNAL_KIMIK3_1787230030` |
| **Date** | 2026-08-20 (epoch 1787230030) |
| **Target** | [MR 389](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/389) — `fix/582-cw20-code-id-pin` → `main` @ `1a1a6b21` (closes #582; gates #581 SpaceUSD/8266 listing policy) |
| **Auditor** | kimi-k3 lead review + 5 parallel composer-2.5 audit subagents (contracts, tests/adversarial, indexer/DB, frontend/E2E/ops, economic/oracle) |
| **Scope** | All 30 changed files **plus** a full-codebase sweep: CosmWasm contracts (pair, factory, router, hooks ×3, fee-discount, faucet), Rust indexer (axum API + Postgres + LCD ingestion + external oracle feeds), React/Vite frontend, E2E/Playwright, ops/upgrade scripts, infra/CI. Prior-audit findings (INTERNAL_KIMIK3_1785897304, INTERNAL_KIMIK3_1786830980) re-verified for still-active status. |
| **Method** | Full-diff review, surrounding-code reads, gate-coverage inventory, cross-version serde analysis, upgrade-ordering analysis, targeted test execution, web research of 2026 CosmWasm/DeFi/backend attack checklists (Trail of Bits CosmWasm patterns, OWASP SC Top-10 2026, OWASP node/API security handbooks). Read-only: no files modified by the audit. |

---

## 0. Executive summary

MR 389 implements invariant **F6** — pair instantiate pins each listed asset's live `ContractInfo.code_id`, and every fund-moving pair write path (swap, provide, withdraw, limit place/fill, cancel, claim) now aborts unless the live id equals the pin **and** the factory `IsCodeIdWhitelisted` check still passes. The on-chain implementation is **correct and conservative**: access control is right (governance → factory → factory-only pair refresh), fail-closed on query errors, same-tx migrate+swap exploits are impossible (live query per write + tx atomicity), cw2 gates are correct, the new tests pass (448/448 lib suite; clippy clean), and the docs/runbooks match the code. **The post-listing `MsgMigrateContract` → FoT/rebase trading hole (SEC-I H05) is closed.**

**No Critical or direct fund-loss vulnerability was found in the MR.** The residual risk concentrates in four areas:

1. **Liveness-by-design (the freeze is total).** The guard gates *exit* paths too, so a third-party token issuer with wasm admin (the actor F6 defends against — any permissionless 6036 listing; 8266 SpaceUSD if listed) can freeze **all** user value in a pair (LP underlying, maker escrow, parked claims) at will, until governance whitelists + refreshes. Accounting analysis shows exits could safely stay open under FoT (recipient bears outbound tax; escrow/reserves stay consistent) — the maximal freeze is a conservative choice that is currently **undocumented as a tradeoff** and **untested** for cancel/claim/withdraw.
2. **Ops/ordering hazards.** Pair 1.15.0 before factory 1.9.0 → every gated write freezes (fail-closed but total); batch refresh is all-or-nothing and the incident-scenario pair blocks it; `RemoveWhitelistedCodeId(10184)` is a near-protocol-wide halt (13 assets share the template). All are documented in playbooks but **nothing is script-enforced** — the same class as prior-audit F-01/F-02.
3. **The freeze is invisible off-chain.** Queries (`Simulation`/`HybridSimulation`) are ungated, the indexer has zero F6 awareness (route/solve keeps quoting frozen pairs; `is_active` hardcoded `true`; no `refresh_asset_code_ids` ingestion), and the frontend has no `GetAssetCodeIds` probe and no humanization for the four new error variants — users discover freezes as raw reverted transactions after signing. `verify-issue-582.sh` is contracts+docs greps only and would green-light this state.
4. **Prior-audit findings remain largely active** (see §8 rollup) and several **chain** with the new freeze mechanics — see §9 exploit-chaining analysis (stale-price unfreeze arb, supply-chain → prod dev-mode, oracle-poisoning cascade, governance-key amplification, listing-spam liveness attrition).

| Severity | Count | Notes |
|---|---|---|
| Critical | 0 | — |
| High | 8 | 3.1 freeze-griefing/extortion, 3.2 upgrade ordering, 3.3 10184 kill-switch blast radius, 3.4 whitelisted-FoT residual (policy), F389-01/02 frozen-pair UX blindness, F389-13/14 E2E+CI gaps, I389-01/02 indexer frozen-pair blindness, I389-09/10/11 ingestion/reorg (pre-existing, re-verified) |
| Medium | ~25 | liveness/economic/ops/display across all layers |
| Low | ~20 | hardening, documentation, edge cases |
| Info / positive | ~30 | verified-safe properties |

**Tests executed for this audit:** contract lib suite 448/448 PASS (delta vs main = exactly the 6 new pin tests), migration 4/4, adversarial 9/9, clippy (pair+factory) clean; indexer lib 279/1 (1 unrelated pre-existing failure); indexer integration + Playwright not run (Postgres/LocalTerra not provisioned for this read-only audit).

**Verdict:** **Approve the contract changes; block production rollout on ops controls** — an order-enforcing upgrade script (factory 1.9.0 → pair 1.15.0 → paginated refresh) with post-migrate smoke checks, a documented freeze/unfreeze incident runbook (incl. exit-path policy decision and unfreeze-arb mitigation), and at minimum a frontend error-humanization + indexer frozen-pair exclusion ticket before the #581/8266 listing relies on F6.

---

## 1. Scope map — areas identified by the codebase sweep

The MR diff (30 files, +1084/−36) touches factory + pair + dex-common + tests + docs. The sweep beyond the diff found these in-scope areas:

| Area | Why in scope |
|---|---|
| `pair/src/asset_code_id_guard.rs` (new) | Core control: pin snapshot, write-path assert, governance refresh |
| `pair/src/contract.rs` dispatcher | Gate placement on 6 execute arms + `execute_receive`; ungated paths (`UpdateLimitOrderPrice`, `CleanLimitBook`, `Sweep`, admin) need safety proof |
| `pair` migrate | Backfill of `ASSET_CODE_IDS` from live `ContractInfo`; cw2 gate; failure modes abort the migrate |
| `factory/src/contract.rs` | `IsCodeIdWhitelisted` query (new trust dependency for every pair write), `RefreshPairAssetCodeIds(Batch)` governance executes, batch pagination/atomicity |
| `dex-common` factory/pair msgs | Cross-version serde (`deny_unknown_fields`) between pair 1.15.0 and factory <1.9.0 / ≥1.10.0 |
| Pair subsystems | discount_cache, blacklist_guard (cumulative per-write query/gas stack), hooks, TWAP `OBSERVATIONS` (staleness during a freeze), orderbook (frozen book behavior), hybrid sim (sim/execute divergence during drift) |
| Indexer (Rust/axum/Postgres) | Frozen pairs remain quotable — `Simulation`/`HybridSimulation` queries are ungated so route/solve + quotes keep working while execution reverts; new events/attrs parsing; full re-audit of post-2026-08-05 surface (external oracle feeds, hub prices, protocol stats, route progress) |
| Frontend dApp | Frozen-pair UX (raw contract errors vs meaningful copy), route display, cancel/claim failure UX, prod guards, CSP, wallet signing paths |
| Ops/scripts | `verify-issue-582.sh` false-pass analysis, migrate ordering enforcement (factory 1.9.0 **before** pair 1.15.0), `verify-issue-536.sh` version bumps, runbook accuracy |
| Economic/oracle | Freeze as griefing/extortion vector, de-whitelist blast radius (13 assets share code 10184), unfreeze-arb, TWAP staleness consumers, flash-loan tier gaming, self-cross undercut, hub-price source manipulation |
| Infra/CI | CI gates for the new tests, lcov coverage of new module, E2E gating |

---

## 2. Lead-auditor verified findings (contracts, MR core)

*(to be merged with contract subagent findings)*

### L-01 — MEDIUM (liveness/design): exit paths (cancel / claim / LP withdraw) freeze on code-id drift

`gate_asset_code_ids` is applied to `CancelLimitOrder`/`CancelLimitOrders` (contract.rs:689,700), `ClaimExpiredLimitOrder(s)` (:711,722), and all `Cw20HookMsg` receives (:794) including `WithdrawLiquidity`. A listed token's wasm admin (the exact third-party-issuer actor F6 defends against — any permissionless 6036 listing, or 8266 SpaceUSD once listed) can `MsgMigrateContract` to any non-pinned code_id at any time and freeze **all** user value in the pair — LP underlying, resting-order escrow (`PENDING_ESCROW_*`), and parked expired claims — until governance both whitelists a replacement template **and** runs `RefreshPairAssetCodeIds`. Verified: no funds are stealable this way (fail-closed), and same-tx migrate+swap is impossible because the guard reads live state per write (tx atomicity reverts both messages).

Accounting analysis of the alternative: on cancel/claim/withdraw the pair sends tokens **out**; under an FoT token the transfer tax is borne by the recipient (user) while `PENDING_ESCROW`/`RESERVES` decrement by the gross amount — pair-side accounting stays consistent, remaining LPs unaffected. So gating exits is a conservative choice, not a solvency requirement. **Recommendation:** either (a) restrict the guard to value-inflow paths (swap-in, provide, placement) and leave exits open (documented: exiting users eat any FoT tax), or (b) keep the maximal freeze and document the liveness tradeoff + governance response SLA in `cw20-whitelist-policy.md`. Currently the tradeoff is implicit.

### L-02 — MEDIUM (ops): upgrade-ordering total freeze — pair 1.15.0 before factory 1.9.0

Pair write paths query factory `IsCodeIdWhitelisted` (new in factory 1.9.0). Against factory ≤1.8.0 the smart query fails (`unknown variant`) → `AssetCodeIdGuardUnavailable` → **every** gated write on **every** upgraded pair reverts until the factory migrates. Fail-closed (no fund loss) but a total liveness freeze if ops migrates pairs first. The MR documents the order (playbook step 5, test plan) but nothing enforces it on-chain or in scripts; there is no `verify-issue-582` on-chain rung. Mirrors prior-audit F-01 (the #514 fee-discount ordering hazard) — same class, opposite direction. **Recommendation:** upgrade script that (1) asserts factory cw2 ≥1.9.0 before broadcasting any pair 1.15.0 migrate, (2) post-migrate smoke: one `Simulation` + one `GetAssetCodeIds` per pair, (3) reconciles migrated count vs `GetPairCount` (prior F-02 pagination pattern).

### L-03 — LOW: batch refresh is all-or-nothing; skip procedure undocumented

`execute_refresh_pair_asset_code_ids_batch` sends one `WasmMsg::Execute` per pair in a **single tx** (factory/src/contract.rs:915-965). In the incident scenario the batch is meant for, the pair whose token migrated to an unlisted id **must** fail refresh (pair refuses unlisted live ids) — which reverts the entire batch, blocking all later-indexed pairs. Workaround exists (single `RefreshPairAssetCodeIds` for good pairs, then batch with `start_after` past the bad index) but is not documented in the playbook. Pagination verified correct: `calc_limit` clamps to `[1,30]` (pagination.rs:4-6 — no limit=0 stall), `start_idx = start_after+1`, `next_start_after = idx-1` — no overlap/skip (walked count=14/50 cases). `PAIR_INDEX.load` failure is silently skipped (index-gap tolerant).

### L-04 — LOW: frozen pairs stay quotable — sim/execute divergence by design

The guard gates **execute** paths only. Pair `Simulation`/`HybridSimulation`/book queries are ungated, and the indexer's `/route/solve` `maybe_simulate` uses the router's `simulate_swap_operations` (query path, indexer/src/api/route_solver.rs:495-530) — so a frozen pair continues to appear in quotes/routes and the user's **execute** reverts on-chain (lost gas, raw `AssetCodeIdDrift` error). Nothing in indexer or frontend references `GetAssetCodeIds`/`IsCodeIdWhitelisted`/`refresh_asset_code_ids` (verified by grep — zero hits). **Recommendation:** indexer watch for `AssetCodeId*` revert reasons (or periodic `GetAssetCodeIds` vs live `ContractInfo` diff) to flag/exclude frozen pairs from the route graph; frontend maps the three new error strings to user copy ("pair paused pending token upgrade").

### L-05 — LOW: `GetAssetCodeIds` errors on unmigrated pairs

`query_asset_code_ids` uses `ASSET_CODE_IDS.load` (pair/src/contract.rs:2738-2741) — pre-1.15.0 pairs return a query error, not `None`. No off-chain consumer exists yet; document the error shape before integrators adopt it (contrast the `GetDiscountRegistry` raw-key fallback pattern documented in contracts-terraclassic.md).

### L-06 — INFO (positive): gas, reentrancy, same-tx migrate, storage keys

- Gate adds 4 read-only queries per gated write (2× `ContractInfo` + 2× factory smart query), once per tx regardless of batch size — bounded, paid by the caller. No new submessage/reply flows → no reentrancy/reply-id surface.
- Same-transaction `MsgMigrateContract` + swap is impossible: guard reads live state; tx atomicity reverts both messages.
- `ASSET_CODE_IDS` storage key `"asset_code_ids"` has no historical collision in pair state (state.rs:45-47).
- cw2 `ensure_from_older_version` present on both contracts (factory:1446, pair:2840); factory needs no state change for the new query (`WHITELISTED_CODE_IDS` map pre-exists).
- `query_wasm_contract_info` chain support is not a new risk: factory `CreatePair` has used it since launch (factory/src/contract.rs:226-238) and the MR's LocalTerra verify passed; still worth a columbus-5 pre-flight check in the deploy runbook.
- `UpdateLimitOrderPrice` correctly ungated: verified it only relinks the price node (contract.rs:1417-1456) — no escrow/fund movement; matching is gated at swap time. `CleanLimitBook` ungated: parks rows (no transfers); claims themselves are gated — consistent.
- Refresh front-running is harmless: issuer re-migrating to another **unlisted** id makes the refresh revert (refuse-unlisted); to another **whitelisted** id pins a source-reviewed template.

---

## 3. Contract findings (MR core + interactions) — consolidated lead + contract subagent

Severity harmonized across lead review (L-*) and contract subagent (C389-*); duplicates merged.

| # | Finding | Severity | Sources |
|---|---------|----------|---------|
| 3.1 | Third-party wasm-admin migrate freezes whole pair incl. exits (griefing/extortion) | **High (liveness)** | C389-01, L-01, E389-04 |
| 3.2 | Upgrade ordering: pair 1.15.0 before factory 1.9.0 → total write freeze; not script-enforced | **High (ops)** | C389-04, L-02 |
| 3.3 | `RemoveWhitelistedCodeId(10184)` is a near-protocol-wide halt (13 assets share the template) | **High (ops)** | E389-03 |
| 3.4 | FoT behind a *still-whitelisted* template remains open (ops audit gap; F6 only blocks migrate-off-template) | **High (policy residual)** | E389-05, AMM-01 partial |
| 3.5 | Exit paths (cancel/claim/withdraw) gated — parked escrow can be stranded; `CleanLimitBook` still parks during freeze | Medium | C389-02/07, E389-01 |
| 3.6 | Sim/execute divergence: `Simulation`/`HybridSimulation`/route-solve quotes succeed on frozen pairs; executes revert | Medium | C389-03, L-04, T389-03 |
| 3.7 | First-swap-after-unfreeze arbitrage against stale reserves/TWAP | Medium | E389-02 |
| 3.8 | Batch refresh all-or-nothing (one unrefreshable pair reverts batch); skip-via-`start_after` undocumented | Medium (ops) | C389-05, L-03 |
| 3.9 | Pair migrate aborts if `ContractInfo` query fails → pair stuck on pre-F6 version | Medium (ops) | C389-09 |
| 3.10 | `UpdateLimitOrderPrice` ungated during freeze (no fund movement; UX/book-consistency oddity; docs say "limit" paths gated) | Low | C389-08, T389-01, E389-09 |
| 3.11 | Silent `PAIR_INDEX` skip in batch refresh | Low (ops) | C389-06 |
| 3.12 | `GetAssetCodeIds` hard-errors on unmigrated pairs (query shape for integrators) | Low | L-05 |
| 3.13 | Forward serde: `CodeIdWhitelistedResponse` **does** carry `deny_unknown_fields` (cw_serde verified in cosmwasm-schema-derive 1.5.11) — a future factory response-field addition fails closed on pair 1.15.0 | Low (forward-compat) | lead verification (corrects C389 sub-note) |
| 3.14 | Governance freeze/unfreeze timing MEV (insiders position for post-refresh arb) | Medium | E389-13 |
| 3.15 | Guard gas overhead (+4 queries/write); drift errors echo pin/live ids (public data) | Info | C389-11/12 |
| 3.16 | Columbus-5 `WasmQuery::ContractInfo` support — pre-existing usage in `CreatePair` + LocalTerra pass; still probe mainnet before rollout | Info (deploy gate) | C389-10, lead |

### Confirmed-safe properties (contract layer)

- Pin at instantiate (`contract.rs:546-547`) and migrate backfill (`:2863-2867`); dual check pin==live AND whitelisted (`asset_code_id_guard.rs:83-97`); refresh refuses unlisted live ids (`:106-113`); fail-closed on query errors (`:34,47`).
- Access control: factory refresh executes are `ensure_governance` (factory:898,923); pair `RefreshAssetCodeIds` is factory-only (`contract.rs:2065-2066`); pair→factory binding immutable; `assert_pair_in_registry` on single refresh; `addr_validate` on all new address inputs.
- Same-tx migrate+swap exploit **impossible** (live query per write; tx atomicity) — tested (`asset_code_id_pin_tests.rs:157-226`).
- No reentrancy/reply-id surface added (read-only queries, no new submessages); CEI unchanged.
- `Sweep` excludes escrow/reserves; ungated and factory-only — safe.
- cw2 `ensure_from_older_version` on both contracts; `calc_limit` clamps `[1,30]` (no limit-0 stall); batch pagination walk verified (no overlap/skip).
- Indexer parser does not consume the new attributes — no spoof/collision surface.
- Gate placement is pre-mutation in every dispatcher arm; direct `ExecuteMsg::Swap` remains a hard error (CW20-only design).

### Gate coverage matrix (pair execute)

| Entry point | Funds move? | Gated? | Assessment |
|---|---|---|---|
| Receive → Swap / PlaceLimitOrder(Batch/Ladder) / WithdrawLiquidity | yes | **yes** | correct |
| ProvideLiquidity | yes | **yes** | correct |
| CancelLimitOrder(s), ClaimExpiredLimitOrder(s) | yes (refunds) | **yes** | intentional freeze — see 3.1/3.5 |
| UpdateLimitOrderPrice | no (book relink only) | no | safe, undocumented (3.10) |
| CleanLimitBook | no (parks rows) | no | safe but asymmetric vs gated claims (3.5) |
| Sweep / admin paths / RefreshAssetCodeIds | config/factory-only | no | safe |
| All queries (Simulation, HybridSimulation, Pool, Observe…) | no | no | divergence risk (3.6) |

---

## 4. Test coverage & adversarial testing (test subagent, executed on the MR worktree)

### 4.1 Test execution evidence (run during this audit)

| Suite | Result | Notes |
|---|---|---|
| `cargo test -p cl8y-dex-tests --lib asset_code_id_pin` | **6/6 PASS** | the MR's new suite |
| `cargo test -p cl8y-dex-tests --lib migration_tests` | **4/4 PASS** | version constants updated correctly (prior F-14 hygiene issue not present here) |
| `cargo test -p cl8y-dex-tests --lib adversarial` | **9/9 PASS** | new migrate-able adversarial CW20 fixtures |
| Full `cargo test -p cl8y-dex-tests --lib` | **448/448 PASS** | main passes 442/442; delta = exactly the 6 new tests |
| `cargo clippy -p cl8y-dex-pair -p cl8y-dex-factory --all-targets` | **clean** | local-only (not CI-gated) |

### 4.2 MR test inventory

`asset_code_id_pin_tests.rs` covers: pin-at-create + `IsCodeIdWhitelisted` query shape; FoT-migrate → swap fails + refresh refuses FoT id; `RemoveWhitelistedCodeId` freezes swap **and** provide until restored; migrate-to-other-whitelisted-template frozen until governance refresh (incl. non-governance refresh → `Unauthorized`); pair-migrate backfill of missing pins; batch refresh happy path. `verify-issue-582.sh` = pin suite ×2 + docs/version grep presence checks (no LocalTerra rung, no full-suite/clippy).

### 4.3 Missing tests (consolidated T389-*)

| Priority | Gap |
|---|---|
| High | Cancel/claim/withdraw **during drift/de-whitelist** (the liveness finding 3.1/3.5 has no behavioral test); `HybridSimulation`-vs-execute divergence during drift; `AssetCodeIdGuardUnavailable` fail-closed path (factory query failure) — no analogue to the blacklist-unavailable test |
| High | Cross-version skew: pair 1.15.0 + factory 1.8.0 (documented ops order, untested); factory refresh → pair <1.15.0 |
| Medium | Batch refresh atomic failure (one bad pair reverts all); batch edge cases (empty range, `start_after` OOB, `limit: 0` clamp); token1-only drift (all tests drift token0); pair `RefreshAssetCodeIds` direct non-factory caller; refresh idempotence; migrate with failing `ContractInfo`; migrate from versions older than 1.14.0; `migration_tests` should assert `GetAssetCodeIds` post-upgrade |
| Medium | Gas benchmark of +4 queries/write; TWAP-staleness-during-freeze linkage test |
| Low | Rebase-token migrate fixture (only FoT tested); error-shape assertions for `GetAssetCodeIds` on unmigrated pairs |

### 4.4 Test infrastructure (prior TST findings re-verified on this branch)

| ID | Status |
|---|---|
| TST-01 lcov excludes orderbook.rs/faucet | **Active** — `lcov.info` gitignored; no CI check that `asset_code_id_guard.rs` is covered |
| TST-02 Playwright E2E not GitLab-CI-gated | **Active** — 49 specs, none for #582/frozen-pair UX; E2E deferred to #421 |
| TST-03 clippy not CI-gated | **Active** — passes locally |
| TST-04 router/hooks/faucet migrate tests | **Active** — still absent |
| TST-05/06/07 hook coverage, poller e2e, no cargo-fuzz | **Active** — no fuzz/proptest for the new guard |

---

## 5. Indexer (Rust server) + database findings (indexer subagent)

| # | Finding | Severity |
|---|---|---|
| I389-01 | Route graph ignores code-id-frozen pairs — `/route/solve` enumerates all DB pairs with no freeze check | **High** |
| I389-02 | No ingestion of `refresh_asset_code_ids` / pin state; `pairs.is_active` never updated; CMC `is_frozen` hardcoded `"0"` | **High** |
| I389-09 | Non-transactional block ingestion — crash after `swap_events` insert permanently skips derived updates (re-verified PARSER-04) | **High** |
| I389-10/11 | Reorg halts without automated rollback; trader/position rollups not height-keyed so recovery leaves them wrong (re-verified REORG-01/02) | **High** |
| I389-03 | `/route/solve/progress` on 60 RPS router invokes LCD `GetDiscount` per poll → 6× LCD amplification (re-verified API-02) | Medium |
| I389-04 | Compliance blacklist probe accepts unbounded comma-split address lists → LCD fanout (re-verified API-03) | Medium |
| I389-05 | CEX oracle parses `f64` prices with no positive/sanity bounds before averaging + DB write | Medium |
| I389-06 | Stale CEX/Venus prices retained indefinitely; no max-age before `volume_usd`/hub use (re-verified ORACLE-01) | Medium |
| I389-07 | Poisoned/stale CEX price cascades: hub cUSTC/LUNC peg → protocol TVL → trader P&L USD → frontend portfolio | Medium |
| I389-08 | Pool-only `GET /route/solve` without `amount_in` skips simulation → returns ops through frozen pairs that revert on-chain | Medium |
| I389-12 | `seed-qa` subcommand shipped in prod binary without `RUN_MODE` guard (re-verified SEC-01) | Medium |
| I389-13/14 | No Postgres TLS enforcement; migrations run as the runtime role (re-verified DB-02/03) | Medium |
| I389-15/16 | Invalid swap amounts coerced to zero (`unwrap_or_default`); no DB CHECK constraints on amounts (re-verified PARSER-01/02, MIG-02) | Medium |
| I389-17 | Reorg guard skipped when checkpoint hash empty (re-verified REORG-03) | Medium |
| I389-25 | DB pool: 10 connections, no statement/acquire timeouts (re-verified DOS-02) | Medium |
| I389-26 | Sybil-friendly leaderboard persists despite USD ranking (#553) (re-verified POS-02, partially mitigated) | Medium |
| I389-27 | Huge swap amounts inflate aggregates uncapped (re-verified AGG-02) | Medium |
| I389-18/19/20/21/22/23/24/28/29 | Hook idempotency DB-error swallow; unsigned reorg webhook; `BSC_RPC_URLS` unconstrained (env-only SSRF); misleading `is_active`/`is_frozen` API fields; Swagger always mounted; no HTTP security headers; `token_in/out` format validation; compression without decompressed-size cap; test harness echoes `DATABASE_URL` on panic | Low |

**Confirmed-safe (indexer):** SQLi controls (allowlisted sorts/intervals, `QueryBuilder` binds, LIKE escaping), error sanitization (DB→500 generic, LCD→502), CSV formula-injection neutralization, per-IP rate limiting (XFF ignored, prod clamps zeros), CORS allowlist, 128 KiB POST cap + 30 s timeout, bounded limit-book LCD budget (101/page), factory-provenance pair discovery, `_contract_address` event scoping, `DATABASE_URL` never logged (startup test), swap idempotency unique key, Venus `eth_call` read-only with pinned addresses, oracle/hub ticker allowlists, bounded progress registry (256/30 s). Poller false-reorg-on-dual-writer fixed since prior audit (`Resync`); CoinGecko UA (#579) fixed.

**Recommended indexer work for F6 (priority):** (1) schema + parser for pin state and `refresh_asset_code_ids`; (2) exclude frozen pairs from route graph; (3) real `is_active`/`swap_frozen` on pair APIs; (4) route-solve failure surfacing `frozen_pairs`; (5) ops metric/alert on frozen-pair count; (6) LocalTerra integration test asserting frozen-pair exclusion.

---

## 6. Frontend, E2E, and ops findings (frontend subagent)

| # | Finding | Severity |
|---|---|---|
| F389-01 | No humanization for `AssetCodeId*` errors — users see raw `Asset CW20 code_id drifted: token … pinned …, live …` after signing | **High (UX)** |
| F389-02 | No proactive drift detection — zero frontend use of `GetAssetCodeIds`/`IsCodeIdWhitelisted`; pause/blacklist have probes, F6 does not | **High (UX)** |
| F389-13 | No E2E for frozen-pair/F6 contract errors (49 Playwright specs, none reference #582) | **High (testing)** |
| F389-14 | Playwright not GitLab-MR-gated (GitHub Actions only; GitLab defers to Phase 2 #421) | **High (testing)** |
| F389-03 | Frontend accepts indexer routes with frozen hops (no filter; quote → sign → revert) | Medium |
| F389-04 | Limit cancel/claim buttons stay enabled on frozen pairs; raw mutation errors | Medium |
| F389-05 | `verify-issue-582.sh` false-pass surface — contracts+docs only; cannot catch missing frontend/indexer/ops wiring or wrong mainnet migrate order | Medium |
| F389-06 | No `upgrade-582` script — factory-first ordering + paginated refresh are doc-only | Medium |
| F389-07 | `rotate-fee-treasury.sh` pair enumeration `limit: 60` unpaginated (prior F-02 pattern persists) | Medium |
| F389-08 | `VITE_DEV_MODE` prod-build guard still missing (re-verified FE-01; mnemonic/WC guards exist, dev-mode does not) | Medium |
| F389-09 | Indexer URL defaults HTTP, no prod HTTPS build gate (re-verified FE-14) | Medium |
| F389-10 | CSP `unsafe-inline` + bootstrap `innerHTML` without SRI (re-verified FE-09) | Medium |
| F389-11 | Compromised-indexer route influence residual; route-mismatch warn-only despite "reject" comment (re-verified FE-04/05) | Medium |
| F389-12 | Pre-sign summary vs wallet decode gap (industry limitation; re-verified FE-06) | Medium |
| F389-15 | No malicious-indexer-payload E2E (outage specs exist; poisoned `route/solve` fixtures missing) | Medium |
| F389-16 | Lookalike CW20 symbols partially mitigated (#541 TokenIdentity on trade/pool/charts; swap pickers still symbol-first) | Medium |
| F389-17 | Simulated-wallet local signing gated only by runtime `DEV_MODE` (re-verified FE-15) | Medium |
| F389-18 | Docs/runbooks accurate for F6; ops-only enforcement is the gap | Info |

**Frozen-pair UX gap (4 questions):** (a) F6 errors → raw chain strings, no humanization; (b) no proactive `GetAssetCodeIds`/live-`ContractInfo` probe — user discovers at simulate/execute; (c) indexer routes through frozen pairs unfiltered; (d) cancel/claim panels only gate on pause/blacklist — frozen escrow shows enabled buttons that fail raw.

**Ops-script sweep:** `eval` on `.env.local` in 3 upgrade scripts (low, operator-controlled files); `curl | bash` in cloud toolchain (INF-13 active); `set -euo pipefail` widespread (good); **migrate ordering for #582 not encoded anywhere**; CODEOWNERS still misses `frontend-dapp/`, `indexer/`, `.gitlab-ci.yml` (INF-22); optimizer pinned by tag not digest (INF-24).

---

## 7. Economic / tokenomic / oracle findings (economic subagent)

| # | Finding | Severity |
|---|---|---|
| E389-01 | Freeze + ungated `CleanLimitBook` parks orders whose claims stay gated → parked escrow stranded | Medium |
| E389-02 | First-swap-after-unfreeze arb vs stale reserves/TWAP (quantified: ~5% × frozen TVL extractable, depth-limited) | Medium |
| E389-03 | `RemoveWhitelistedCodeId(10184)` ≈ protocol-wide halt (13 assets / ~12-14 pairs); milder single-pair options exist (BlacklistPair/SetPairPaused) | High (ops) |
| E389-04 | Issuer migrate → ransom/LP-hostage leverage over governance (SpaceUSD 8266 scenario) | Medium |
| E389-05 | FoT behind still-whitelisted template (listing-time audit failure) not covered by F6 | High (policy residual) |
| E389-07 | CEX oracle simple average, no outlier rejection / staleness TTL (ORACLE-01 active) | Medium |
| E389-08 | Hub USD marks freeze with pair reserves → stale advisory USD across /protocol, candles, P&L | Medium |
| E389-10 | Permissionless 6036 listing + self-migrate griefs external LPs (reputation/grief, no theft) | Medium |
| E389-11/12 | Self-cross undercut (F-05) and T9 zero-fee dust spam (F-06) unchanged | Medium/Low |
| E389-13 | Compromised-governance freeze/unfreeze timing MEV (new lever vs pre-389) | Medium |
| E389-06 | Atomic migrate+swap verified impossible (live query per write) | Info (positive) |

### Oracle dependency map (condensed)

| Source | Consumer | Manipulation impact | MR 389 effect |
|---|---|---|---|
| Pair TWAP (`Observe`) | external integrators, dApp charts (not UST1 window) | low-liquidity pools manipulable; `seconds_ago=0` extrapolation (AMM-05 active) | freeze → no new observations → stale TWAP |
| CEX avg (KuCoin/MEXC/CoinGecko; CMC) | hub cUSTC/LUNC, `volume_usd`, protocol overview | single-source compromise moves mean; stale retained on outage (ORACLE-01 active) | freeze decouples hub reserves from CEX |
| Venus vFDUSD `eth_call` | /protocol vFDUSD tab | malicious BSC RPC → wrong rate (response validation present) | none |
| Hub reserves (largest-TVL pool) | hub UST1/USTR, candle USD marks, portfolio P&L | wash-liquidity source capture is costly (TVL floor) | reserves freeze → stale hub USD |
| UST1-window `effective_swap` | /ust1 mint/redeem (out-of-repo) | assumed independent of pair freeze | verify ops never conflate window oracle with pair TWAP |
| Terra burn tax (chain) | wrap-mapper unwrap payout | user absorbs; ~2% all-in retune (#516) | none |

### Prior-finding status (economic domain)

AMM-01 **partially mitigated** (post-listing migrate blocked; listing-time FoT remains ops-gated); AMM-04/05/06, LOB-01, LOB-13, SUP-05 (theoretical — no in-repo CL8Y lending), F-05, F-06, POS-02, ORACLE-01 all **active**; LOB-03 **worsened/compounded** (two independent freeze layers: pause + F6).

---

## 8. Attack checklists (consolidated)

### 8.1 Common smart-contract attacks (vs MR diff)

| Vector | Assessment |
|---|---|
| Reentrancy (incl. cross-contract via new queries) | Safe — read-only queries pre-mutation; CosmWasm actor model; no new submessages/replies |
| Access control / privilege escalation | Sound — governance→factory→pair factory-only refresh; all new inputs `addr_validate`d; EOA cannot refresh |
| Integer overflow / unchecked math | N/A — u64 equality only in guard |
| Unauthorized migrate / cw2 | Sound — `ensure_from_older_version` both contracts; backfill tested; older-than-1.14.0 migrate untested (gap) |
| Storage collision (`asset_code_ids`) | Clear (static review) |
| DoS / gas | +4 queries/write (bounded, caller-paid); batch all-or-nothing (3.8); no benchmark test |
| Front-running | Refresh front-run harmless (refuse-unlisted / pin-whitelisted); unfreeze-arb is the real race (3.7) |
| Event spoofing → indexer | New attrs unused by parsers; no collision |
| FoT / rebase post-listing migrate | **Closed** (tested) |
| FoT at listing (whitelisted template) | **Open** — ops audit gate only (3.4) |
| Error-path coverage | Partial — `GuardUnavailable`/`Unpinned` under-tested (§4.3) |

### 8.2 Common DeFi / economic attacks

| Vector | Assessment |
|---|---|
| Flash-loan tier gaming (SUP-05) | Active, theoretical (no in-repo CL8Y lending; 300 s cache bounds) |
| Self-cross fee undercut (F-05) | Active (T9: 5 vs 9 bps) — undocumented design decision |
| Dust/spam (F-06, LOB-01) | Active — T9 zero-fee placement, no order-count cap |
| Oracle manipulation (pair TWAP) | Freeze → stale TWAP (no new observations); `seconds_ago=0` extrapolation (AMM-05) active |
| Oracle manipulation (indexer CEX feeds) | Medium — no outlier rejection/staleness TTL; cascade to hub/protocol/P&L (I389-05/06/07) |
| Hub-price source capture | Costly (largest-TVL floor) but reserves freeze → stale hub USD (E389-08) |
| Wash trading / Sybil leaderboards | Active (LOB-13, POS-02) |
| Unfreeze arbitrage | **New** — first-swap-after-refresh extracts stale-reserve value (3.7) |
| Governance timing MEV | **New lever** — freeze/unfreeze timing knowledge (3.14) |

### 8.3 Database / API / backend

Covered in §5 — SQLi well-controlled; main residuals: ingestion transactionality, reorg completeness, oracle bounds/staleness, throttle asymmetry, prod guards (TLS/roles/seed-qa), security headers/Swagger.

### 8.4 Frontend / wallet / E2E

Covered in §6 — main residuals: F6 UX blindness, prod build guards (dev-mode, HTTPS indexer), CSP, E2E gating, malicious-indexer fixtures.

### 8.5 Prior-finding status rollup (both earlier KIMIK3 audits)

| Status | Findings |
|---|---|
| **Fixed since** | Poller false-reorg dual-writer; CoinGecko UA (#579); Venus/hub DB CHECK constraints (partial MIG-02); FE-10 noopener (mostly) |
| **Partially mitigated** | AMM-01 (post-listing migrate blocked by F6; listing-time FoT open); POS-02 (USD ranking, no anti-Sybil); FE-11 (#541 identity on 3 pages, pickers symbol-first); INF-19 (verify gate; bypass residual) |
| **Still active** | SUP-01, SUP-05, SUP-07, SUP-08, SUP-19, SUP-23, LOB-01, LOB-02, LOB-03 (compounded), LOB-14, AMM-03/04/05/06/08/09, F-05, F-06, F-07, F-08, F-15, API-02/03/04/05/06, PARSER-01/02/04, REORG-01/02/03, DB-02/03, ORACLE-01, AGG-02, SEC-01, DOS-02, FE-01/04/05/06/09/14/15/16/17, INF-05/06/13/14/15/16/22/24/28, TST-01..07 |
| **Worsened/compounded by MR 389** | LOB-03 (second freeze layer on maker exits); indexer/analytics staleness surface (frozen pairs invisible → stale quotes/aggregates presented as live) |

---

## 9. Exploit-chaining investigation

Method: every High/Medium finding from this audit plus every **still-active** finding from INTERNAL_KIMIK3_1785897304 and INTERNAL_KIMIK3_1786830980 (§8.5) was treated as a graph node; edges were added where one finding's preconditions enable or amplify another, or where an **out-of-repo dependency** (Terra Classic chain layer, token issuers, CEX APIs, Venus/BSC RPC, wallet supply chain, GitLab/Coolify/agent infra, UST1-window oracle) feeds an in-repo weakness. Chains below are ordered by assessed severity. "Cost" is attacker outlay; "gain" is extractable value or harm.

### Chain A — Freeze → stale-price unfreeze arbitrage (in-repo; **new from MR 389 mechanics**)

**Nodes:** C389-01/E389-04 (issuer migrate freeze) or E389-03 (de-whitelist) → E389-08 (hub/TWAP staleness during freeze) → I389-01/02 + F389-01/02 (freeze invisible off-chain) → E389-02 (first-swap-after-unfreeze arb) → E389-13 (insider timing).

1. A pair freezes — maliciously (6036/8266 issuer migrates), or defensively (governance `RemoveWhitelistedCodeId` on a discovered template bug).
2. During the freeze: reserves/TWAP stop updating; external CEX prices keep moving; the indexer keeps quoting the frozen pair as healthy and the dApp shows no banner.
3. Governance refreshes pins (unfreeze). The first swap executes against stale reserves. On a $500k-TVL pair with a 5% external move, up to ~$25k is extractable (depth/slippage-limited) from LPs.
4. Who captures it: whoever sees the refresh tx first — mempool watchers, or multisig insiders who control refresh timing (E389-13). Users who attempted swaps during the freeze lost gas on reverts (F389-01..04).

**Mitigations today:** none specific — pause-before-refresh and private rebalance-at-unfreeze are not in the runbooks. **Recommended:** incident runbook step: `SetPairPaused` stays on through refresh; treasury/keeper performs a corrective arb or re-seed (e.g. via `rebalance-mint-ust1-lp.sh` pattern) before unpause; document that refresh timing is MEV-sensitive. **Severity: Medium.**

### Chain B — Supply-chain → production dev-mode → mass key exposure (in-repo + GitLab/Coolify/npm)

**Nodes:** INF-28 (merge gate off — re-verified 2026-08-20 via glab: `only_allow_merge_if_pipeline_succeeds=false`, no required approvals) + INF-06 (public repo, forks enabled) + INF-05 (CI caches not branch-scoped) → F389-08/FE-01 (`VITE_DEV_MODE` not build-guarded) + F389-17/FE-15 (local signing path) + F389-10/FE-09 (CSP `unsafe-inline`, no SRI) + FE-17 (legacy WalletConnect deps).

1. Attacker lands code on `main` via a compromised maintainer account or a merged-with-red-CI MR (gate is off), or poisons a shared CI cache / dependency.
2. Build-time guards catch a prod **mnemonic** but not `VITE_DEV_MODE=true`; a bundle with Simulated Wallet + local signing reaches Coolify/render production.
3. CSP `unsafe-inline` + no SRI means any injected bootstrap script executes; the dev-signing path or a poisoned WalletConnect/Keplr flow harvests or mis-signs user transactions.
4. FE-06 (opaque wallet decode) and FE-11 (symbol-first pickers) blunt the user's last line of defense.

**Why it chains:** each control individually is Medium; the chain converts a repo/CI compromise into user-key compromise without any on-chain exploit. **Severity: High (latent; requires a supply-chain foothold).** **Recommended:** enable "pipelines must succeed" + approval rules; add the FE-01/FE-14 build guards (one-line each); branch-scoped CI cache keys; nonce-based CSP.

### Chain C — Oracle/feed compromise → cascade to USD display + route steering (out-of-repo → in-repo)

**Nodes:** out-of-repo (KuCoin/MEXC/CoinGecko/CMC API compromise or outage; BSC RPC for Venus) → I389-05 (no price sanity bounds) + I389-06 (stale retention, ORACLE-01) → I389-07 (hub peg 1:1 to CEX → protocol TVL, trader P&L #560, candle USD marks #568) → F389-11/FE-04 (indexer-influenced routes within slippage) + F389-09/FE-14 (HTTP indexer default).

1. Single-source CEX compromise or a malformed response moves the simple average (no outlier rejection); on all-source outage the last price is served indefinitely.
2. Poisoned/stale prices propagate: `volume_usd`, `/overview`, hub cUSTC/LUNC, `/protocol` TVL, portfolio/trader USD P&L — the dApp renders attacker-influenced USD.
3. If the indexer itself is compromised (or MITM'd where FE-14's HTTP default is in play), route/solve responses can additionally steer users onto toxic-but-valid routes within their slippage (warn-only today, F389-11).
4. MR 389 interaction: during a code-id freeze, hub reserve marks freeze while CEX feeds keep moving (E389-08) — the divergence is presented as live data, and nothing flags it.

**Impact:** analytics/display integrity and decision harm; no direct on-chain theft (settlement is chain-side). **Severity: Medium.** **Recommended:** price bounds + ≥2-source deviation check + staleness TTL → NULL/`stale:true`; HTTPS build gate; block submit on route-reconciliation mismatch.

### Chain D — Governance multisig compromise (worst case; MR 389 adds levers, not theft)

**Nodes:** out-of-repo (2-of-3 multisig key compromise) → SUP-01 (single-step rotation — instant, no timelock) → pre-existing powers (SUP-19 registry repoint → 100% discount; SUP-07 reverting hook → pair halt; pause) → **new:** E389-03 (`RemoveWhitelistedCodeId(10184)` ≈ protocol-wide halt), E389-13 (freeze/unfreeze timing MEV).

1. Two-of-three keys compromised → attacker rotates governance instantly (SUP-01), locking out honest signers.
2. Attacker halts the DEX (de-whitelist 10184 and/or pause), repoints discount registries to a malicious contract returning `discount_bps: 10000` (fee evasion at scale), and times unfreezes to capture Chain-A arb.
3. **Limit:** the attacker cannot move pairs onto attacker-chosen token templates (refresh pins *live* ids and refuses unlisted ones; token code changes require the token's own wasm admin) and cannot directly withdraw user escrow/reserves (Sweep excludes both). The damage is liveness + fee evasion + MEV, not direct drainage.

**Severity: High (conditioned on multisig compromise).** **Recommended:** 2-step governance transfer (SUP-01 fix), timelock on `RemoveWhitelistedCodeId`/registry/hook changes, documented emergency runbook.

### Chain E — Listing-spam liveness attrition (in-repo; permissionless surface)

**Nodes:** SUP-02 (permissionless CreatePair, 100 LUNC fee) + E389-10 (6036 self-migrate grief) + 3.8/C389-05 (batch refresh all-or-nothing) + I389-01 (frozen pairs still quoted) + F389-01 (raw errors).

1. Attacker cheaply lists N tokens on 6036, seeds dust LP, then migrates each to FoT/unlisted wasm → N frozen pairs.
2. Each freeze needs governance attention (refresh can't fix unlisted ids; the pairs stay frozen by design). Batch recovery reverts on the first bad pair, forcing index-skip surgery per pair.
3. Indexer keeps routing users into the frozen pairs; every victim loses gas and trust. Sustained cost to attacker: ~100 LUNC + dust LP per pair; cost to protocol: continuous ops + UX damage.

**Severity: Medium.** **Recommended:** indexer frozen-pair exclusion (I389-01 fix) blunts most user harm; consider a listing bond or governance-approval gate for non-10184 templates; document batch-skip procedure.

### Chain F — Chain-layer events → corrupted analytics (out-of-repo: Terra Classic reorgs/halts)

**Nodes:** Terra Classic reorg/halt → I389-10/REORG-01 (halt, no auto-rollback; orphaned data served) + I389-09/PARSER-04 (non-transactional ingestion skips derived state) + I389-11/REORG-02 (rollups never rebuilt) → stale/wrong candles, volumes, leaderboards, USD marks for days; amplified by ORACLE-01 staleness and by MR-389 freezes (which the indexer cannot see at all).

**Severity: Medium (display integrity).** **Recommended:** transactional ingestion; rollup rebuild in reorg recovery; frozen-pair tracking (I389-02).

### Chain G — DoS amplification against indexer/LCD (in-repo)

**Nodes:** I389-03 (progress-poll 6× LCD amplification) + I389-04 (unbounded compliance lists) + I389-25 (no DB statement timeouts) + I389-28 (no decompressed-size cap). Combined, a single attacker IP can multiply load on public LCD endpoints and the indexer DB well beyond the nominal 60 RPS budget, degrading quotes for all users. Frontend indexer-outage handling (E2E-covered) is the mitigating control. **Severity: Low–Medium.**

### Chain H — Whitelisted-FoT at listing + false confidence (ops → in-repo)

**Nodes:** E389-05/3.4 (F6 does not cover FoT present at listing) + AMM-01 (reserve desync → insolvency/LP lock) + F389-05 (`verify-issue-582` green check misread as "FoT handled") + E389-03 (the incident response — de-whitelist — then halts the protocol template-wide).

1. Governance whitelists a subtly-FoT template (source-audit failure — the one step F6 cannot check).
2. Pairs pin the FoT id and trade; reserves desync per AMM-01 until outbound transfers revert → LP lock.
3. Incident response de-whitelists the template → every pair on it freezes (including exits, 3.5) — correct but maximally disruptive, and recovery needs token migrations the protocol doesn't control.

**Severity: High (policy residual) — the pre-whitelist source audit (`verify-cw20-code-ids.sh` + recorded evidence) is the only gate; treat it as a launch-blocking control.**

### Chain I — Agent/infra credential theft → repo → prod (out-of-repo: cloud-agent fleet)

**Nodes:** INF-14 (GCH golden image: passwordless sudo + unrestricted agent approval — re-verified `gch-cloud-setup.sh:46,97`) + INF-15 (docker.sock chmod 666 fallback — `cloud-agent-docker.sh:70`) + INF-16 (`/etc/gch/job.env` tokens into agent shells) → GitLab token theft → MR/merge access (INF-28 gate off) → Chain B.

Prompt injection against an agent VM (e.g. via malicious repo content or a poisoned web fetch) yields root-equivalent execution and the VM's GitLab/Cursor credentials; from there the path is Chain B's supply-chain entry. **Severity: High (fleet-dependent).** **Recommended:** scoped sudo, sandboxed/manual approval mode, 0600 root-only job.env, remove the chmod-666 fallback, minimum token scopes.

### What does **not** chain (verified dead-ends)

- **Freeze → theft:** no path. Escrow/reserves are isolated; Sweep excludes them; refresh cannot pin unlisted or attacker-chosen templates; same-tx migrate+swap is impossible (live query + atomicity).
- **F6 → discount/fee manipulation:** the guard is orthogonal to `discount_cache`/registry; a frozen pair simply doesn't trade.
- **F6 → oracle write manipulation:** freeze *stops* observation writes (staleness, Chain A) but cannot forge them (extreme-ratio skip AMM-06 remains the only write-suppression vector, unchanged).
- **F6 → indexer parser corruption:** new attributes are unconsumed; no collision with `wasm_attr_last` keys.
- **Governance freeze → direct fund withdrawal:** no message path moves user escrow/reserves to governance.

### Chain priority summary

| Chain | Severity | Primary fixes |
|---|---|---|
| B (supply chain → prod keys) | High (latent) | INF-28 gate + approvals; FE-01/FE-14 build guards; INF-05 cache scoping; FE-09 CSP |
| I (agent fleet → repo) | High (latent) | INF-14/15/16 hardening |
| D (governance compromise) | High (conditional) | SUP-01 2-step transfer; timelocks; runbook |
| H (whitelisted FoT) | High (policy) | pre-whitelist source audit as launch blocker |
| A (freeze/unfreeze arb) | Medium | pause-through-refresh + private rebalance runbook |
| E (listing-spam attrition) | Medium | I389-01 route exclusion; listing policy for non-10184 |
| C (oracle cascade) | Medium | I389-05/06 bounds+TTL; FE-14 HTTPS gate |
| F (reorg analytics) | Medium | PARSER-04/REORG-02 fixes |
| G (DoS amplification) | Low–Medium | I389-03/04 throttles/caps |

---

## 10. Recommendations (priority order)

**P0 — before the columbus-5 rollout of factory 1.9.0 / pair 1.15.0:**
1. Write `scripts/upgrade-582-code-id-pin.sh`: assert factory cw2 ≥1.9.0 before any pair migrate; paginated pair migrate reconciled against `GetPairCount`; paginated `RefreshPairAssetCodeIdsBatch` loop with documented bad-pair skip; post-migrate smoke (`GetAssetCodeIds` + one `Simulation` per pair) (3.2, 3.8, F389-06).
2. Probe columbus-5 `GET /cosmwasm/wasm/v1/contract/{addr}` support explicitly in the deploy runbook (3.16).
3. Decide and document the exit-path policy (keep maximal freeze vs open cancel/claim/withdraw) and the unfreeze procedure (pause-through-refresh + corrective-arb step) (3.1, 3.5, 3.7).
4. Re-affirm the pre-whitelist source-audit gate as the **only** FoT-at-listing control; do not let `verify-issue-582` green checks substitute for it (3.4, F389-05).

**P1 — before #581/8266 listing relies on F6:**
5. Frontend: humanize the four `AssetCodeId*` errors; proactive `GetAssetCodeIds` + live-`ContractInfo` probe disabling CTAs with clear copy (F389-01/02/04).
6. Indexer: ingest pin state + `refresh_asset_code_ids`; exclude frozen pairs from route graph; real `is_active`/`swap_frozen` on APIs (I389-01/02/08/21).
7. Tests: cancel/claim/withdraw-during-freeze, `GuardUnavailable` fail-closed, cross-version skew (pair 1.15 vs factory 1.8), batch atomic failure, sim-vs-execute divergence (§4.3 High rows); Playwright frozen-pair spec (F389-13).
8. Extend `verify-issue-582.sh` (or rename it contracts-only) to grep the frontend humanizer + indexer exclusion so it cannot false-pass (F389-05).

**P2 — hardening (chain-breakers):**
9. Enable GitLab "pipelines must succeed" + approval rules (INF-28); branch-scoped CI caches (INF-05); FE-01/FE-14 build guards; CSP nonce/SRI (FE-09).
10. 2-step governance transfer (SUP-01); timelock on `RemoveWhitelistedCodeId`/registry/hooks; document 10184-removal as protocol-halt with milder alternatives (BlacklistPair/SetPairPaused) (3.3).
11. Oracle: price bounds + deviation check + staleness TTL (I389-05/06); route-progress throttle fix (I389-03); compliance list caps (I389-04).
12. Indexer data layer: transactional ingestion (I389-09); reorg rollup rebuild (I389-11); amount rejection + CHECK constraints (I389-15/16); `seed-qa` prod guard (I389-12).
13. Agent fleet: scoped sudo, approval mode, job.env perms, drop chmod-666 fallback (INF-14/15/16).

---

## 11. Audit verdict

**Approve MR 389's contract code.** F6 is correctly and conservatively implemented, tested for the primary threat (448/448 lib tests, clippy clean), and closes the post-listing migrate-to-FoT hole with no new fund-loss vector. **Do not treat merge as production readiness:** the rollout needs the order-enforcing upgrade script and incident runbook (P0), and the #581/8266 listing should not rely on F6 until the frontend/indexer freeze-visibility gaps (P1) are closed — otherwise the first real freeze will be discovered by users as unexplained reverted transactions, and the unfreeze will hand a stale-price arbitrage to whoever is watching the mempool. The highest latent risks in the repo remain the supply-chain/governance chains (B, I, D, H), none of which this MR worsens materially — but its new freeze/unfreeze levers make the governance-key and runbook disciplines measurably more important.

---

*Audit performed read-only. Subagent findings were lead-verified where noted; items marked "(Needs verification)" warrant manual confirmation before remediation planning. Prior-audit statuses re-verified against the MR branch on 2026-08-20.*
