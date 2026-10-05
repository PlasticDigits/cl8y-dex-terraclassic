# Internal Security Audit — cl8y-dex-terraclassic

| | |
|---|---|
| **Audit ID** | `INTERNAL_KIMIK3_1791113518` |
| **Date** | 2026-10-04 |
| **Commit** | `748579ab` (`main`, 2026-10-04) |
| **Type** | Internal, read-only, defense-in-depth review |
| **Primary ask** | Full-stack security audit (test coverage, DeFi/smart-contract/DB attacks, e2e, happy+bad path, missing security features, access control, privileges, Rust server, contract design, tokenomic/economic attacks, oracle manipulation) with **focus on recently merged PRs whose smart-contract code is queued for columbus-5 migration**, then **exploit chaining** (§14) across in-repo, prior-audit, and out-of-repo findings |
| **Scope** | 134 commits since prior audit `f3dce2d4`, emphasizing: **#1381/#1383** route-local tax funding (router + community-tax token + dex-common), **#1376/#1378** bounded exact-output/reverse pair math (pair cw2 1.19.0), **#1380/#1382** liquidity-quality (indexer), **USDT-Bridge-Debt** (`8f9dd98e`, two brand-new contracts already live on mainnet), plus re-verification of every still-active prior finding |
| **Method** | Codebase sweep for new surfaces → web research (2026 Q3–Q4 incidents) → 5 parallel domain auditors (bridge-debt; #1381 route tax; #1376 pair reverse math; indexer new surfaces; prior-finding re-verification) with lead-level independent review of the router/token tax path, pair reverse math, hub-USD selection, and bridge-debt → **executed test suites** (contracts 884/884; indexer lib 469+1 known-flaky) → live LCD verification of the bridge-debt deployment → exploit chaining (§14) |

**Prior audits treated as still-active unless re-verified:** `INTERNAL_KIMIK3_1785897304`, `INTERNAL_KIMIK3_1786830980` (#514), `INTERNAL_KIMIK3_1787230030` (F6), `INTERNAL_KIMIK3_1787468843` (community tax), `INTERNAL_GROK46_1787908099` (API4 + full sweep), `INTERNAL_KIMIK3_1790149457` (2026-09-23 full stack).

**Pending columbus-5 migrations this audit targets:** [#1384](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1384) (router + community-tax token route-funded v1), [#1386](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1386) (pair 1.19.0 exact-output rollout), [#1385](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1385) (liquidity-quality indexer rollout). The #1324 pair TWAP 1.18.0 rollout is **completed and closed** (`docs/runbooks/pair-twap-uint256-columbus5.md`: "The completed 1.18.0 TWAP rollout is historical evidence and remains closed") — prior-audit Chain M's wall-clock trigger is defused.

---

## 0. Extra surfaces found in this sweep (new or changed since 2026-09-23)

| Surface | Why it matters | Status |
|---------|----------------|--------|
| `bridge-debt` + `bridge-debt-token` contracts (`8f9dd98e`) | **Brand-new contracts, already live on mainnet** (10,000 USDT issued against CMM debt; UST1/USDT LP seeded to treasury); never previously audited | Reviewed + **LCD-verified live** → §3.3 (BD-1…BD-10) |
| Router route-local tax funding (#1381) | New value-flow: taxes funded from per-hop gross budgets; new token↔router authentication (`RouteTaxContext`); new `RouteTransfer` execute | Lead + domain reviewed → §3.1 (RT-1…RT-9) |
| Pair exact-output / reverse math (#1376) | Consensus-path reverse math on every pair; migrates via #1386 | Lead + domain reviewed → §3.2 (EO-1…EO-7) |
| Indexer `route_exact_output.rs` + `route_exact_output_reference.rs` (1,139 new lines) | New public solver endpoints on the LCD-heavy governor | Reviewed → §4 (well-bounded; IX-1…IX-8) |
| Indexer `trading_tax_discovery.rs` (251 new lines) | New tax-capability attestation; fails closed | Reviewed → §4 (IX-2, IX-3) |
| Indexer `protocol_tvl.rs` / `liquidity_snapshots.rs` / `protocol_liquidity.rs` (#1380) | Reserve-coverage classification; false-collapse prevention | Reviewed → §4 (sound; ORB-05 blast radius reduced for TVL only) |
| `book_snapshot.rs` bounded snapshots | Per-pair response byte caps (65,536 B), 1,000 orders/side, concurrency limits | Positive hardening (IX-4, IX-5 notes) |
| `lcd/mod.rs` (+206 lines) | Bounded bodies, cache namespaces, code-hash query — **but no chain-id binding (TRU-01 still active)** | §4 |
| Frontend route-tax funding (`routeTaxFunding.ts`, `swapRoutePreflight.ts`, gas) | Quote parity with contract math; fail-closed discovery | Lead-reviewed → §5 |
| Hourly buyback retry + Hexxagon RPC switch (#1375) | Live mainnet treasury movers changed RPC trust anchor; the two scripts now use **divergent** RPCs | §6 (OPS-05 updated) |
| `scripts/upgrade-1324-pair-twap.sh` | The executed 1.18.0 migration; reused for 1.19.0 (#1386) | §6 — **#1232 blind spot persists for 1.19.0** |

**Web-research additions mapped to this code (2026 Q3–Q4 landscape):**

- **Nostra $3.5M (2026-09-17, Starknet):** attacker seeded a thin one-sided pool, wash-traded it, and hijacked **GeckoTerminal's pool-selection logic** so the rigged pool became the price reference. Maps directly to this repo's own GeckoTerminal adapter (`/gt/`) and hub-USD **largest-liquidity pool selection** → **ORB-06** (new), Chain S.
- **Moonwell/MAMO $8.7M (2026-08-27, Base):** thin-token spot price pumped ~40× and accepted as collateral value; "not a single line of code changed." Maps to any external lending/credit consumer of this DEX's spot or hub marks, and to the UST1 window (out-of-repo) reading manipulated marks → Chain S/K.
- **Spiral SpiralHookV2 $26.8K (2026-09-13):** raw spot-price collateral valuation; same-block `tx.origin` guard bypassed with 6 EOAs. Maps to per-wallet launch-guard cooldowns (Sybil-bypassable by design, T-03) and the new cyclic-route finding RT-1 → §7.
- **CWA-2025-006 (wasmd IBC channel-open error handling):** chain-level; no contract in this repo exposes `ibc_*` entry points (re-verified) → not exploitable through this code.
- **CWA-2024-002 (Uint256 pow/neg wrapping):** repo pins `cosmwasm-std 1.5.11` → not affected (re-verified; TWAP Uint256 path uses checked ops).

---

## 1. Executive summary

The settlement core remains **defense-in-depth** under the documented trust model, and the three migration-bound contract changes are in good shape:

- **#1381 route-local tax funding is sound.** Exact (not approximate) integer round-trips in both directions; sell-hop authentication binds trader/pair/offer/economic/gross against the router's live state; buy tax is charged exactly once per boundary; settlement transfers are exact-match authorized; both migration orders fail closed (liveness-only, never double-charge or waiver). No Critical/High. The consequential items are a **deployment constraint** (RT-5: `config.router` is immutable — #1384 must migrate the router **in place**) and a cyclic-route quote/execution divergence (RT-1).
- **#1376 pair reverse math is a strict improvement** — O(1) closed-form pool inversion replaces a 32-call-capped search; execute/forward/reverse now share one Uint256 commission implementation (widening a u128 overflow-revert edge). Approved for the #1386 1.19.0 rollout from a contract-math standpoint; all findings Low/Info.
- **Bridge-debt contracts are small, conservative, and match their trust model** — and this audit **independently verified the live deployment via LCD** (code hashes 11708/11709 match committed `checksums.txt`; owner/admin = 2-of-3; `outstanding == total_supply` at 10,000e18). No Critical/High; BD-1 (code-id rotation bricks repay) and BD-3 (missing `ensure_from_older_version`) are cheap pre-second-market fixes.

**The risk picture is otherwise unchanged in shape and slightly improved at the edges:** OPS-04 (DRY_RUN mainnet footgun) is **fixed**; #1324 is **executed** (Chain M defused); the #1380 TVL quality model eliminates false-zero TVL and partially contains ORB-05 for the TVL path. But **25 of 26 re-verified prior findings remain active**, including both `high-risk security` issues (#1229 launcher spoof, #1225 book head-clog — now formally recorded as violated invariant L26, with #718 explicitly gated on it), the oracle trio (ORB-01/05, TRU-01 — the LCD rewrite added bounds but **not** chain-id binding), and the entire merge-gate perimeter (SEC-06/11/12).

| Sev | Count (new this audit) | Count (re-verified active) | Notes |
|-----|------------------------|----------------------------|-------|
| Critical | 0 | 0 | No silent drain under the trust model |
| High | 0 | 5 | #1229, #1225, AC-02 (#526/#697), C-08 (policy), supply-chain/fleet latent (Chains B/I) |
| Medium | 4 (RT-5, ORB-06, BD-1, IX-3-borderline) | ~20 | See §13 |
| Low | 12 | ~14 | See §13 |
| Info / positive | many | many | §12 |

**Verdict:** the migration-bound code is ready modulo its runbook constraints (in-place router migration, `GetDiscountRegistry` precheck for 1.19.0, hash-based — not cw2-based — version verification). The standing backlog (#1229, #1225, oracle hardening, merge gates) is where the next real incident would come from.

---

## 2. Test coverage — measured

Suites **executed** on HEAD `748579ab` (this audit):

| Suite | Result | Notes |
|-------|--------|-------|
| `smartcontracts` workspace (`cargo test --workspace`) | **884 passed, 0 failed** | Includes 758-line `route_tax_1381_tests.rs`, 8 `issue1376_*` unit tests, greedy, reprice FIFO, adversarial token, F6 pin, migration 1.18→1.19 |
| `indexer` lib (`cargo test --lib`) | **469 passed, 1 failed** | **TST-04 still active**: `pair_discovery::tests::discover_new_pair_negative_caches_non_pair_contract` fails in the parallel run (shared `&'static Mutex` negative cache, `pair_discovery.rs:47-50`), passes solo |
| Frontend (`vitest`) | Not re-executed; merge receipt records 268 focused tax + 172 supplemental passing | TST-05 (`.env.local` contamination) still active (`vitest.config.ts:11-15`, no env isolation) |
| Playwright e2e | 63+ specs incl. new `swap-receive-1376-tx.spec.ts` (209 lines); **still not in either CI** (TST-02) | |
| Merge-time evidence | `docs/qa/merge-2026-10-04.md`: contracts 884, indexer lib 470, route/exact-output/funding Postgres 55, liquidity 43, browser 17/17 | Recorded at merge of #1379/#1382/#1383 |

**Test gaps identified this audit (highest priority first):**

1. **#1381 suite** (`route_tax_1381_tests.rs`, 6 tests): no same-token-twice routes (RT-1), no `unwrap_output` tests (RT-2), no `max_wallet` boundary tests (RT-8), no `to ≠ sender` binding tests, no zero/tiny or `u128::MAX` gross end-to-end, no old-hook entry rejection with a real v1 token, no AfterSwap-hook mid-route interaction (RT-3), no `SendFrom`-funded entry.
2. **#1376 suite:** no contract-level book-branch budget-exhaustion test (TG-1), no book-branch offer/offer−1 forward-validation test (TG-2), no seed-fallback doubling test (TG-3, the EO-1 corner), no `belief_price` reverse-query test (TG-4), no discount-registry exact-minimality test (TG-5), no AfterSwap-fee sim/execute consistency test (TG-6).
3. **#1232 not covered:** the 1.18→1.19 migration test downgrades cw2 on a full-storage pair; it does **not** exercise missing-item backfill.
4. Standing: multi-block TWAP manipulation sim (C-02), hook-count gas test (C-01), #1225 head-clog test (the open issue asks for it), poisoned-route e2e (F-03), oracle outlier/staleness tests (ORB-01/05), LCD chain-id test (TRU-01), `cargo-fuzz` (TST-07).

---

## 3. Smart contracts

### 3.1 #1381 route-local tax funding (router ↔ token authentication)

Files: `router/src/route_tax.rs` (new), `router/src/contract.rs`, `packages/dex-common/src/route_tax.rs` (new), `community-tax-token/src/tax.rs`. Domain auditor + lead line-level review; invariants RT1381-1…10 checked one by one.

**Verified strong (lead + auditor, independently confirmed):**

1. **`sell_from_gross` exactness** (`dex-common/src/route_tax.rs:67-90`): maximal `S` with `S + floor(S·bps/10000) ≤ G`, subtraction-only comparisons (no overflow at `u128::MAX`), property test includes maximality assertion across bps/gross extremes.
2. **Router-side preview validation** (`router/src/route_tax.rs:70-102`): version, `gross == amount`, bounds, and per-kind accounting identities re-checked — a malicious/buggy token cannot make the router over-credit.
3. **Fail-closed capability discovery** (`route_tax.rs:10-58`): unknown funding version → error; legacy tax tokens without v1 capability → error (never silently wallet-funded); only exact cw2 names `crates.io:cw20-base`/`crates.io:cw20-mintable`/`cw20-mintable` pass as ordinary.
4. **Sell-hop authentication** (`tax.rs:493-510`): exact 5-tuple check against the router's **live** `RouteTaxContext`; supplying `Swap.trader` without router state is worthless (RT1381-5 holds). Manager exemption binds to the authenticated funder, never to `from`/`to` roles (`tax.rs:521-528`).
5. **Buy boundary** (`tax.rs:512-519`): only `from` = listed pair ∧ `to` = official router ∧ live context binds `pair`/`ask_token`. One charge per boundary; final delivery/refunds use exact-match-authorized `RouteTransfer` (`tax.rs:754-797`; multiset equality — duplicate entries cannot over-authorize because each message is backed by a real entry and `sub_balance` bounds the total).
6. **Owned funds (RT1381-4):** per-hop balance-delta accounting; pre-existing router dust excluded; integer residue refunded to the route payer; unfilled hybrid book input rolls into the pool leg (`pair/src/contract.rs:1325-1327`) — no offer-token refunds to the router.
7. **`reverse_sell`/`reverse_buy` exactness** (`route_tax.rs:156-210`): minimal-sufficiency proofs hold (per-unit tax delta ∈ {0,1}); no 1-unit under-funding revert is possible; quoting/execution cannot diverge at floor boundaries.
8. **`funded_hybrid` conservation** (`route_tax.rs:114-130`): `pool + book == economic` always after rescale; the pair's declared-split validation cannot be violated by it.
9. **Both migration orders fail closed:** token-first → sell hop hard-errors (`RouteTaxContext`); router-first → legacy capability probes reject. No order produces a silent waiver or double charge.
10. **Atomicity (RT1381-9):** any failure reverts the whole tx; `SWAP_STATE` removed only on settlement success; Cosmos rollback covers error paths.
11. **Quote parity:** frontend and indexer both use the router's own tax-inclusive simulation as authoritative; the indexer hard-fails (503) route-local quotes when simulation is unavailable (`route_solver.rs:1146-1152`); indexer fallback math is formula-identical (`community_tax_rank.rs:371-389`, `route_exact_output.rs:51-59`).

**New findings:**

- **RT-1 — Low–Medium: launch-guard cooldown self-block on token re-acquisition (A→B→A, A→B→C→B).** `already_checked` is computed only for router **sell** hops (`tax.rs:493-510`); the buy branch never sets it, and `record_trade_blocks` stamps the trader at **every** Buy/Sell boundary (`tax.rs:360-370`). Any route touching the same launch-guarded token twice reverts with `Cooldown` for `cooldown_blocks ≥ 1` — while router sim, indexer solver, and frontend (none model launch guards) all quote successfully. Atomic revert, no loss; quote/execution divergence on exactly the launch-phase tokens the feature targets. **Fix:** router rejects routes where a capable token appears twice (cheapest), or token treats same-height route-attributed stamps as pass; add A→B→A regression tests.
- **RT-2 — Low (latent): `unwrap_output` + taxed output token double-charges and under-delivers vs `minimum_receive`.** Router entry checks only that `WRAP_MAPPER` is set (`contract.rs:270-275`); final settlement uses a raw CW20 `Send` to the mapper (`contract.rs:529-537`), which `classify_trade` treats as **Buy** (`tax.rs:177`; the mapper is never protocol-exempt — `contract.rs:124-131` registers only self/factory/router/autolp), while `minimum_receive` is asserted against the pre-tax figure (`contract.rs:490-507`). Today revert-only (mapper rejects foreign tokens; wrapped assets are never taxed). **Fix:** reject `unwrap_output` at entry when the output token's capability shows `buy_bps > 0`.
- **RT-3 — Low (governance-gated): AfterSwap-hook interference during a live route context.** Buy recognition binds any `ctx.pair == from ∧ ctx.ask_token == self` transfer to `ctx.trader` without amount/initiator check; a governance-registered hook that causes an extra pair→router ask-token transfer mid-route forces Buy attribution (cooldown ≥ 1 → route revert; cooldown 0 → donation inflates `hop_output`). Hooks are factory-governance-only. **Fix:** document the restriction; optionally bind recognition to an expected-output window.
- **RT-4 — Low: protocol-exempt/self trader — preview succeeds, execution reverts** (`RouterTraderRequired`). `route_preview` applies only `is_manager_exempt` (`tax.rs:689-728`); `hop_trader_addr` returns `None` for protocol-exempt traders (`tax.rs:124-126`). Unfulfillable quotes for automation using protocol identities. **Fix:** `route_preview` rejects/flags protocol-exempt traders.
- **RT-5 — Medium (deployment constraint): `config.router` is immutable — #1384 must migrate the router in place.** Router set at instantiate (`contract.rs:86-131`) or one-time adopt (`adopt.rs:231-263`); **no `SetRouter` execute exists** (`msg.rs:162-221`). A new-address router deployment permanently bricks routed trading of every existing taxed token (fail-closed; pair-direct still works). **Fix:** pin in-place migration in the #1384 runbook; consider a governance-gated `SetRouter` in a future token code migration.
- **RT-6 — Info: cw2 allowlist is a breaking liveness change for ordinary tokens.** Honest CW20s whose cw2 name is not exactly one of the three allowlisted names become unroutable (fail-closed per RT1381-7). The pre-migration token inventory must enumerate cw2 names of all listed tokens.
- **RT-7 — Info: `route_preview` is a public exemption probe** — but `GetExemptions` already paginates the full set publicly; no new leak; no panics on adversarial input.
- **RT-8 — Info: `route_transfer` has no Receive hook for contract recipients** (same semantics as legacy `Transfer`; self-inflicted at worst); **max_wallet still applies to refunds/final delivery** — a trader within ~1 unit of max_wallet on a residue token reverts the whole route at settlement (atomic, narrow, semantically correct).
- **RT-9 — Info: frontend legacy-path asymmetry during the migration window** — revert-only, window-scoped; divergence direction is safe (tx reverts rather than over-promises).

### 3.2 #1376 pair exact-output / reverse math (cw2 1.18.0 → 1.19.0)

Files: `pair/src/contract.rs`, `pair/src/hybrid_reverse.rs`. Domain auditor + lead algebra re-derivation.

**Verified strong:**

- Pool inverse is the exact integer minimum: `gross = ⌊(target−1)·10000/(10000−fee)⌋+1`, `offer = ⌈k/(out−gross)⌉ − in`; minimality/sufficiency test-anchored at offer and offer−1 (`hybrid_reverse.rs:287-298`, `contract.rs:3939-3947`). All products Uint256; `ceil_div_u256` cannot overflow.
- `bounded_hybrid_reverse_search` maintains `(hi, sufficient)` consistency on every update; budget and infeasible errors are explicit; no approximate result is ever returned as success.
- Execute/forward/reverse share one Uint256 `pool_commission` (`hybrid_reverse.rs:60-68`; call sites `contract.rs:1277-1278, 2667-2668`); the 1-token remainder invariant is preserved (and mathematically unreachable — harmless defense-in-depth). The rewrite also **extends the execute domain**: pool commission no longer reverts for `gross > u128::MAX/fee_bps`.
- Monotonicity assumption of the bisection is sound (book return, pool input, pool return each non-decreasing in total offer).
- Sim→execute protection is structural: exact-output quotes require `sizing_mode='pool_only_exact_output'` + `fidelity_check='passed'` + fresh wallet forward sim (`cw20RouteSolveQuote.ts:190-196, 241-244`); the frontend submits the exact quoted offer with slippage-floored `minReceive`. No path executes with less protection than quoted.
- Migration: no storage/schema change; `migrate` is `ensure_from_older_version` + missing-key backfills only; 1.18.0/1.19.0 coexist safely; the indexer exact-output path never calls the pair reverse query (`route_exact_output.rs:429`).

**Findings (all Low/Info):**

- **EO-1 — Low:** seed-fallback doubling can exhaust the 32-call budget on feasible targets in an adversarial corner (template pool leg ≲ 10⁻¹⁴ with large reserves; `hybrid_reverse.rs:191-214`). Fail-closed, parity with old code. **Fix:** fall back to `min(input_reserve, max_offer)` instead of `Uint128::one()`.
- **EO-2 — Low (pre-existing):** reverse (and forward) sim ignores AfterSwap hook fee deductions that execute enforces against `min_return` (`contract.rs:1337-1355` vs `2500-2692`). Revert-only; no listed pair uses fee hooks today. **Fix:** document; optionally model hook fees in sim.
- **EO-3 — Info:** book-leg commission still uses Uint128 products (`orderbook.rs:2024-2026, 2166-2168`) — consistent sim/execute, unreachable at realistic sizes.
- **EO-4 — Info:** router `reverse_greedy_offer` is the heaviest query path in the repo (~129 cross-contract sims per hop); `reverse_sell` gross-up is the approximate form (exact for v1 route-local funding; potentially off-by-one for deduct-from-gross tax tokens — backstopped by fidelity checks + `min_return`).
- **EO-5 — Info:** query-node load bounded and strictly reduced by this commit.
- **EO-6 — Info:** fee-tier drift between quote and execute reverts safely (pre-existing).
- **EO-7 — Info:** `commission_remainder` invariant unreachable (keep as defense-in-depth).

**Overall:** approve for the #1386 1.19.0 rollout from a contract-math standpoint.

### 3.3 USDT-Bridge-Debt (new contracts, **live on columbus-5**)

`bridge-debt` (issuer) + `bridge-debt-token` (receipt), commit `8f9dd98e`. Domain auditor read both crates in full and **independently verified the live deployment via LCD** (2026-10-04):

| Item | Verified live |
|---|---|
| Issuer `terra12c6h…qk5x6rq`, code 11709 | owner = admin = 2-of-3 `terra1zlmv…hep7` |
| Receipt `terra1qrxch…h42xm7`, code 11708 | admin = 2-of-3; minter = issuer; debtor = CMM `terra16j5u…3lr2`; `minters=[]` |
| On-chain `data_hash` 11708/11709 | **exactly match committed `artifacts/checksums.txt`** |
| Market | `outstanding` 10000e18 == receipt `total_supply` 10000e18 |
| USDT `terra1z0xe…v8mv4` | supply 12,068.85 (**82.9% debt-issued**); primary minter = 2-of-3; extra minters = {issuer, Terra bridge} |
| Pair `terra1qfa8…trknvf` | factory-registered; ~10.4k UST1 / ~9.6k USDT; LP to treasury (1000 min-liquidity lock) |

**Findings:**

- **BD-1 — Medium: `SetDebtCodeId` rotation bricks existing markets' Issue *and* Repay.** `contract.rs:78-94` lets the owner overwrite `debt_code_id` repeatedly; `inspect_debt_token` (`361-370`) re-checks each market's receipt against the *current* value on every Issue/Repay. Rotating the id (e.g. to register a second asset) makes the live USDT market fail `UnexpectedDebtCode` — debt cannot be repaid until the id is restored or the old receipt is migrated first. Liveness failure on a live debt facility via owner *mistake*, not just malice. **Fix:** set-once, or store `debt_code_id` per `Market` at registration.
- **BD-2 — Low: single-step `UpdateOwner`** (`contract.rs:283-296`) — a typo'd address permanently bricks all owner-gated functions. **Fix:** two-step propose/accept.
- **BD-3 — Low: `migrate` lacks `ensure_from_older_version` in both contracts** (`bridge-debt/src/contract.rs:348-352`, `bridge-debt-token/src/contract.rs:222-226`) — every other contract in this repo uses it. Admin is the 2-of-3 (LCD-verified), so this needs an admin/tooling mistake. **Fix:** add it.
- **BD-4 — Low: core invariants are admin-revocable.** Non-transferability and the code-id pin hold only at the current code ids; the 2-of-3 migrate admin could migrate the receipt to transferable code and brick Repay — beyond the documented trust model. Mitigating: the multisig is *already* primary USDT minter, so the issuer adds public accounting, not new mint power. **Fix:** document, or move admin to timelocked governance once no upgrade is planned.
- **BD-5 — Low: `Recover` dead-ends unregistered tokens** (`contract.rs:264` loads the market first). **Fix:** drop the market lookup or document.
- **BD-6 — Info:** registration does not verify the issuer can mint the underlying (Issue reverts until the multisig `AddMinter`s it; deploy script ordered correctly). **Fix:** check `Minter` at registration.
- **BD-7 — Info:** underlying is not code-id-pinned (decimals only); dormant because only the owner registers and the live underlying is factory-whitelisted bridge USDT (code 10184).
- **BD-8 — Info:** instantiate owner validation is format-only; live owner verified = 2-of-3.
- **BD-9 — Info: advisory $1 pin vs IOU backing is disclosed but worth restating.** Indexer pins USDT at advisory $1 by contract identity (`indexer/src/config.rs:24-27`, `pair_price_usd.rs:111-131`); 82.9% of live supply is CMM-IOU-backed. The pin is display-only, symbol-spoofs don't inherit it, the route solver has no USDT special case, and **USDT is not a hub leg** (`hub_usd.rs:66`) — so the new ~$20k UST1/USDT pool **cannot** hijack the UST1 hub mark (Nostra-pattern check: clean). Chain-registry listing honestly discloses the local issuance. Residual: third-party consumers of indexer `price_usd`/`volume_usd` see IOU-backed volume priced at $1.
- **BD-10 — Info:** build reproducibility — on-chain `data_hash` equals committed checksums and source is drift-free (deployed bytecode == audited source); recommend a #589-style decomp pass as belt-and-suspenders.

**Verified strong:** every state-changing execute except `Receive` is owner-gated; `reject_funds` everywhere; registration gauntlet (code-id pin, sole-minter, `Debtor` probe, empty `Minters`, zero supply, decimals equality, dup maps); `total_supply == outstanding` re-checked on every Issue/Repay; receipt non-transferable with mint-only-to-debtor and burn-only-from-debtor; Repay needs **no allowance** (minter-gated `BurnFrom`; no allowance surface exists — a lead hypothesis refuted); permissionless Repay is strictly beneficial to the protocol; no outsider extraction/grief/brick vector found.

### 3.4 Prior contract findings — re-verified (agent E + lead)

| Finding | Status | Evidence |
|---------|--------|----------|
| SC-01 / #1229 launcher spoof | **STILL ACTIVE** | `community-tax-token/src/contract.rs:96-118` — `msg.launcher` stored with `addr_validate` only |
| SC-02 / #1225 book head-clog | **STILL ACTIVE** | `limit_placement.rs:37,87`; `orderbook.rs:1609,1826,2005,2149`; now formally invariant **L26 (OPEN/violated)** in `docs/contracts-security-audit.md:96`; #718 explicitly gated on it (`docs/limit-orders.md:267`) |
| SC-03 max_spread pre-hook | **STILL ACTIVE** | `pair/src/contract.rs:1316-1354` — ordering unchanged by #1376 |
| SC-04 AutoLP provide `slippage_tolerance: None` | **STILL ACTIVE** | `community-tax-autolp/src/contract.rs:296-298` |
| SC-05 / C-01 UpdateHooks no count cap | **STILL ACTIVE** | `pair/src/contract.rs:2094-2109` |
| SC-06 / C-04 T9 self-cross + 0-bps placement | **STILL ACTIVE** | `pair/src/discount_cache.rs:27-32` |
| SC-07 Observe panic `seconds_ago > block_time` | **STILL ACTIVE** | `pair/src/contract.rs:426` (plain subtraction; #1231 fixed only the ratio path) |
| SC-09 faucet migrate w/o `ensure_from_older_version` | **STILL ACTIVE** | `faucet/src/contract.rs:261-264` — **same gap now also in bridge-debt (BD-3)** |
| #1232 migrate backfill | **STILL ACTIVE — residual risk reassessed** | `pair/src/contract.rs:2992-3034` backfills 5 items, not `DISCOUNT_REGISTRY`/`ORACLE_STATE`. **Low in practice for #1386:** all 21 live pairs were born after both items existed (instantiate writes them, `:603,:613`), and the completed #1324 batch proves `ORACLE_STATE` present everywhere. **The blind spot persists:** nothing prechecks `DISCOUNT_REGISTRY`; a pair missing only that item migrates green and bricks on next swap. Cheap fix: add a `{"get_discount_registry":{}}` probe per pair to the 1.19.0 rollout command |
| Greedy book-first (#718) | **STILL OPT-IN (not worsened)** | `dex-common/src/pair.rs:183` `(None,None) → PoolOnly` |
| AC-01 single-step governance | **STILL ACTIVE** | `factory/src/contract.rs:1032-1037` |
| C-08 FoT-at-listing policy | **STILL ACTIVE (policy)** | `factory/src/contract.rs:337-346` — admission is governance discretion; only the off-chain #589 harness gates |

**New incidental (agent E):**

- **OPS-06 (Low–Medium, new): router/tax-token cw2 gives no upgrade signal for #1384.** Router and community-tax-token both report cw2 `1.2.0` from `CARGO_PKG_VERSION`; `ensure_from_older_version` only rejects strictly-newer stored versions, so a pre-#1381 and post-#1381 router are **indistinguishable by cw2**. Migration verification must use **code hash / data hash**, not cw2. The #1386 runbook covers pair only.
- **OPS-07 (Low, new): divergent buyback RPC trust anchors.** `mint-swap-custc-ust1.sh:35` now defaults to `rpc.terra-classic.hexxagon.io` while `mint-swap-burn-ust1-clunc.sh:33` still uses `publicnode.com`; neither asserts node chain-id (OPS-05). #1375 added genuine economic hardening (quote validation, 0.25% tolerance, randomized amounts, retries) — but the two hourly treasury movers now trust different single RPCs silently.
- **SEC-11 worsened cosmetically:** the runbook's CODEOWNERS link (`docs/runbooks/forgejo-pr-merge.md:12`) is now a *broken* link.

---

## 4. Indexer / database / API

### 4.1 New surfaces (domain auditor, full read)

**Verified strong:**

1. **Exact-output resource bounds enforced on all paths:** strict `validate_request` *before* the semaphore (`route_exact_output.rs:30-47`); `MAX_CONCURRENT_REQUESTS=4` via `try_acquire`→429; permit held across the 20s `TIME_BUDGET`; `MAX_GRAPH_PAIRS=8192`; `PATH_WORK_BUDGET=100_000` with the `truncated` flag surfaced.
2. **Rate-limit tiering correct:** exact-output rides `GET /route/solve` on the **LCD-heavy 10 RPS** governor plus global 60; `trading_tax_discovery` exposes no HTTP endpoint.
3. **Tax discovery fail-closed end-to-end:** any error/timeout → `Unavailable` → `unresolved` → 422; attestation = live code id + optional data hash + live `route_tax_capability` with `funding_version==1`, router equality, bps ≤10000; legacy `get_config` observation never attests execution.
4. **`path_eligible` fail-closed per hop** (freeze + pause + blacklist must be explicitly false) — **stricter than the offer-in solver, which has no blacklist check at all** (asymmetry worth noting).
5. **No executable response cache:** every candidate validated by fresh router sim + fidelity drift + staleness re-checks + ≥99% symmetric fair-rate gate; reference unavailable → 503.
6. **#1380 TVL fix is sound:** Option-based TVL (zero only from fresh validated both-zero reserves); non-positive oracle quotes omitted, never $0 (`protocol_tvl.rs:120-127` — **this partially contains ORB-05 for the TVL path**); coverage identity hash; single-tx publish with advisory lock; comparability-gated Δ%. No f64 in production rollup math.
7. **Snapshot pipeline hardened:** advisory lock + `lock_timeout 5s` + `statement_timeout 15s` (`liquidity_snapshots.rs:112-124` — the precedent RES-01 should copy); generation monotonicity reject.
8. **No SQLi in new code:** QueryBuilder/`push_bind` or constant-identifier `format!` from fixed allowlists; API caches keyed on validated params only.
9. **Error hygiene:** static client messages; sanitized 500/502 with regression tests; bounded LCD streaming bodies; path-redacted logs.

**New findings:**

- **IX-1 — Low: `effective_fee_bps_from_discount_bps` underflows on untrusted `discount_bps > 10_000`** (`db_orderbook_sim.rs:44-46`; `resolve_discount_bps` at `route_solver.rs:834-838` parses with `as_u64().unwrap_or(0) as u16`, admitting 10001..=65535 from a misconfigured/migrated registry → debug panic / release wrap → arbitrary fee in offer-in hybrid quote math). Pre-existing function, newly load-bearing; the new exact-output path rejects `discount > 10_000` and is backstopped by fresh sim. **Fix:** validate at `resolve_discount_bps`; `checked_sub`.
- **IX-2 — Low: tax-discovery cache has no event-driven invalidation across token migration** (`trading_tax_discovery.rs:69-75,172-185`; 5s TTL). A downgrade leaves a stale `Community(funding=Some(1))` for ≤5s; an upgrade leaves stale `Unsupported` ≤5s. Bounded and backstopped (offer-in cache TTL clamped to same 5s; exact-output revalidates every candidate with fresh sim; `min_return` protects execution). **Fix:** key the cache on `(addr, code_id)` — discovery already reads the live code id, making any migrate an instant key change.
- **IX-3 — Low: `MAX_DISCOVERY_ASSETS=64` blanket fail-closed can wedge dense graphs** (`community_tax_rank.rs:494-497`): >64 unique assets across candidates → **all** unresolved → every candidate 422. Reachable without an attacker; worst-case discovery wall time (~48s) exceeds the 20s exact-output budget (backstopped → 504). Availability only; tax is never waived. **Fix:** prioritize `token_in`/`token_out`; resolve partially.
- **IX-4 — Low: panic on short `assets` vec from a hostile pair contract** (`book_snapshot.rs:216-217` indexes `[0]`/`[1]` on untrusted LCD JSON). Contained by the `JoinSet` (panic → `JoinError` → pair counted failed), but logged generically. **Fix:** `.get(0)`/`.get(1)` with typed error.
- **IX-5 — Info: book-depth cap failure is loud internally, invisible to API consumers** (`book_snapshot.rs:351-353`): >1000 orders/side → whole pair snapshot errors before write; prior snapshot retained and goes stale → pair drops out of routing with only log/census surfacing. Integrity preserved (never silent truncation). **Fix:** failure-reason taxonomy + metric.
- **IX-6 — Info:** discovery cache key relies on `HashSet` `Debug` order stability (fail-safe: cache misses only).
- **IX-7 — Info:** reference vs production objective divergence is by design — do not treat `route_exact_output_reference.rs` as an independent oracle (it can itself pick a thin path); execution stays offer-in with fresh sim + `min_return`.
- **IX-8 — Info:** invariant-dependent unwraps/indexing in exact-output (`:403,412-413,352,391,400`) — safe today, fragile under refactor.

### 4.2 Prior indexer findings — re-verified (agent D)

| ID | Status | Evidence / notes |
|---|---|---|
| **ORB-05** | **STILL ACTIVE** | `oracle.rs:364,384` `parse::<f64>()` with no finiteness/positivity check; `f64_to_bd` coerces inf→0 (`:311-314`). `"1e999"` from one source zeroes the average. **Partial mitigation:** `protocol_tvl.rs:120-127` filters `p > 0` — TVL degrades to `missing_quote` instead of printing false $0. Volume/hub/P&L/evidence/adapter consumers still eat the zeroed average |
| **ORB-01** | **STILL ACTIVE** | Plain average (`oracle.rs:252`); no outlier rejection; last price retained forever on outage; no staleness TTL |
| **TRU-01** | **STILL ACTIVE** | The +206 `lcd/mod.rs` lines added bounded bodies, cache namespaces, `get_code_data_hash`, cooldown — but **zero** `chain_id`/`node_info` references anywhere in `indexer/src` |
| **TRU-03** | **STILL ACTIVE (documented deliberate)** | `asset_code_id_freeze.rs:62-66,95-97`; fail-open for routing is stated intentional (`:13-15`). New exact-output re-checks freeze per hop through the same fail-open cache but adds fail-closed pause + blacklist — strictly stricter than offer-in |
| **TRU-04** | **STILL ACTIVE** | `main.rs:23-25,47-95` — `seed-qa` has no run-mode guard |
| **INT-02** | **STILL ACTIVE** | `parser.rs:850-851,1610-1622` — bad amounts coerce to 0. Contrast: new `book_snapshot.rs:256-264` parser is strict |
| **LEAK-01** | **STILL ACTIVE** | `evidence.rs:25-32` — unsalted deterministic truncated SHA-256 + full linkage |
| **AUTH-02** | **STILL ACTIVE** | No `CREATE ROLE`/`GRANT` anywhere in `indexer/`; indexer connects as superuser |
| **RES-01** | **STILL ACTIVE** | `/api/v1/evidence/daily` on the global governor (`api/mod.rs:707`); no statement timeout — **in-repo precedent now exists** (`liquidity_snapshots.rs:115`) |

**Net: 9/9 prior indexer findings remain active.** ORB-05 gained a meaningful blast-radius reduction in the TVL path only.

### 4.3 Lead review — hub-USD selection vs the Nostra pattern

`indexer/src/indexer/hub_usd.rs`:

- cUSTC and LUNC marks come from CEX oracles (wrap 1:1), **not** DEX pools — not pool-hijackable. USDT is not a hub leg (BD-9).
- UST1 and USTR marks **are** pool-derived (`resolve_token_vs_priced_hub`, lines 290-347): candidates are factory pairs vs an already-priced hub leg; selection is **largest TVL** (`pick_best`, 246-251) with a **$100 TVL floor** and staleness skip.
- **ORB-06 (Medium, new): largest-liquidity hub selection is capital-bound but cheap relative to pool depth.** Because `pair_tvl` values the token leg at its *own implied* price, candidate TVL = 2× the real priced-leg value (CPAMM identity) — so hijacking the UST1 mark requires real cUSTC capital ≈ half the incumbent pool's TVL (current UST1/cUSTC LP ≈ $5k per ops runbooks → ≈ $2.5k, refundable afterward). A successful hijack skews `volume_usd`, P&L, protocol TVL, evidence exports, DeFiLlama/CG/CMC adapter outputs, and the **ops rebalance/buyback peg gates** that consume hub marks. USTR resolution can use UST1 as its priced leg (lines 400-415), so a UST1 hijack **cascades into USTR**. This is exactly the Nostra/GeckoTerminal pool-selection failure mode; the saving grace is that on-chain settlement never reads these marks. **Fix:** deviation band vs previous mark and/or external reference; median across top-N candidates; minimum *priced-leg* depth independent of implied price; ship the #1216 divergence monitor.
- The `hub_usd(other)?` early-return inside the candidate loop (line 324) is **currently unreachable-None** because the `is_priced_hub` predicate and the `hub_usd` closure agree (verified lines 390-415); flagged as fragile coupling only (Info).

---

## 5. Frontend / wallet / signing (lead review of #1381/#1376 client code)

- `routeTaxFunding.ts`: preview responses are shape- and identity-validated (`funding_version === 1`, `gross === amount`, `≤2^128−1`, per-kind accounting identities) and **all** uncertainty throws `ROUTE_TAX_FUNDING_UNAVAILABLE` — fail-closed; no auto-resend. Legacy detection treats transport/malformed results as uncertainty, never as proof of zero tax. A zero-rate tax token passes legacy discovery (economically correct); a *dishonest* token misreporting zero rates causes mid-route shortfall → atomic revert (liveness only).
- Router `funding_version` is queried live ("never cache across migrations") — correct given #1384's pending rollout.
- `assertRouteTaxFunding` bounds work: ≤4 operations, ≤5 assets, 3 sequential queries per asset — no unbounded LCD fan-out from the browser.
- Exact-output submission uses the exact quoted offer + slippage-floored `minReceive` + fresh wallet forward sim (§3.2).
- Prior frontend findings re-verified: **F-01** CSP `unsafe-inline` meta, no nonce/SRI — **still active** (`index.html:24-25`; nginx has `X-Frame-Options DENY` but no CSP header). **F-02/F-03** unchanged. **TST-05** still active.

---

## 6. Access control, privileges, ops, CI

- **OPS-04 — FIXED (lead-verified):** `terrad_host_tx()` short-circuits on `DRY_RUN=1` (`scripts/lib/terrad-host.sh:194-199`); the soft-launch script routes all store/execute through it. The prior-audit fix direction was implemented at the lib layer.
- **OPS-05 — still active, trust anchor changed:** buyback scripts gained economic hardening (#1375: retries, quote validation, 0.25% tolerance, randomized amounts) but still no chain-id assert, and the two scripts now default to **different** RPCs (OPS-07).
- **OPS-06 (new, §3.4):** cw2 is not a reliable version signal for the #1384 router/token migration — use code hashes.
- **SEC-06 — still active:** `.gitlab-ci.yml:77,98,149,172,253` fixed cache keys, no `$CI_COMMIT_REF_SLUG`; MR pipelines enabled → MR→cache-poisoning path is real.
- **SEC-11 — still active (cosmetically worse):** CODEOWNERS absent; runbook link now broken.
- **SEC-12 — still active:** `.woodpecker.yaml:13-26` is gitleaks-only; header admits functional jobs stay local. Gitleaks binary is checksum-pinned (positive).
- **SEC-07/08 — still active:** tag-only optimizer + Dockerfile bases.
- **DB-01 — still active:** `docker-compose.override.yml:4-7` host networking, auto-loaded.
- **AC-01 / AC-02 (#526/#697) — still active** (unchanged; see prior audit).
- **#1386 1.19.0 rollout note:** the rollout reuses the #1324 script (`upgrade-1324-pair-twap.sh`); add a per-pair `GetDiscountRegistry` probe before migrate (§3.4 #1232 residual).

---

## 7. Tokenomics / economic / oracle (summary)

| Vector | Assessment |
|--------|------------|
| Spot-price flash loan vs pair | Settlement is CP-AMM + book; no external oracle in settlement. Thin-pool TWAP consumers at risk (C-02; docs mandate ≥30m + TVL floor). Spiral/Moonwell reinforce: **spot price ≠ liquidation depth** — any external credit consumer of DEX marks inherits this |
| Pair TWAP | Uint256 fix **deployed** (1.18.0 rollout complete); extreme-ratio dt drop (SC-08) and `Observe` panic edge (SC-07) remain |
| Hub USD | **ORB-06 (new):** UST1/USTR marks pool-hijackable with ≈half-incumbent-TVL capital; cascades UST1→USTR; feeds ops peg gates (Chain S). cUSTC/LUNC/USDT not pool-derived |
| External USD | ORB-01/05 still active; TVL path partially contained (#1380) |
| USDT-Bridge-Debt | Credit-issued (CMM IOU) — 82.9% of supply; advisory $1 pin is identity-gated and disclosed (BD-9); trust concentration = 2-of-3 (BD-4, Chain U) |
| Community tax | Route-local v1 sound (§3.1); cyclic-route divergence (RT-1); launch guards Sybil-bypassable by design (T-03; Spiral pattern noted) |
| Fee tiers | T9 self-cross (SC-06) unchanged; IX-1 discount-bps parse edge (Low) |
| Buybacks/rebalance | Bounded + hardened (#1375) but consume indexer USD + divergent RPC anchors + no chain-id assert (OPS-05/07) |
| UST1 window / wrap mapper | **Out of repo**; admin keys are #526/#697 |

---

## 8. Prior-audit status rollup (2026-10-04)

| Status | Items |
|--------|-------|
| **Fixed since 2026-09-23 (verified)** | **OPS-04** (DRY_RUN gates at lib layer); **#1322/#1324** (1.18.0 TWAP rollout executed on columbus-5 — Chain M defused); **#1380** (false-zero TVL eliminated; quality model + coverage identity); partial: ORB-05 contained for the TVL path |
| **Still active (re-verified this audit)** | SC-01/#1229, SC-02/#1225 (L26), SC-03, SC-04, SC-05, SC-06, SC-07, SC-09, #1232 (residual: DISCOUNT_REGISTRY precheck), AC-01, AC-02, C-08, ORB-01, ORB-05, TRU-01, TRU-03, TRU-04, INT-02, LEAK-01, AUTH-02, RES-01, DB-01, I-01, SEC-06, SEC-07/08, SEC-11, SEC-12, TST-04, TST-05, F-01, F-02, F-03, OPS-05, #718 gating (unchanged) |
| **New this audit** | RT-1…RT-9 (#1381), EO-1…EO-7 (#1376), BD-1…BD-10 (bridge-debt), IX-1…IX-8 (indexer), ORB-06 (hub selection), OPS-06 (cw2 blindness), OPS-07 (divergent RPC anchors) |
| **Worsened** | None materially (SEC-11's runbook link is now broken — cosmetic) |

---

## 9. Missing security features (checklist)

| Feature | Status |
|---------|--------|
| Third-party contract audit | **Not done** (bridge-debt now live without one — internal + LCD-verified only) |
| On-chain governance timelock / 2-step transfer | Missing (AC-01; BD-2 same pattern in bridge-debt) |
| Launcher-origin ratification | Missing (SC-01 / #1229) |
| Min-price economic-fillability placement reject | Missing (SC-02 / #1225 / L26) |
| `DISCOUNT_REGISTRY` precheck in pair rollout script | Missing (#1232 residual; one-line probe) |
| `SetRouter` recovery path on tax tokens | Missing (RT-5 — in-place migration is the only option) |
| `ensure_from_older_version` on bridge-debt ×2 + faucet | Missing (BD-3, SC-09) |
| Per-market debt code-id pin (or set-once) | Missing (BD-1) |
| Oracle outlier rejection + staleness TTL + finiteness checks | Missing (ORB-01/05) |
| Hub-mark deviation band / median-of-N / priced-leg depth floor | Missing (ORB-06) |
| LCD chain-id pinning (indexer + ops scripts) | Missing (TRU-01 / OPS-05/07) |
| Hash-based (not cw2) migration verification for #1384 | Missing (OPS-06) |
| Cyclic-route guard or token-side same-height cooldown pass | Missing (RT-1) |
| `unwrap_output` × taxed-output entry guard | Missing (RT-2) |
| Postgres least-privilege roles | Missing (AUTH-02) |
| seed-qa prod guard | Missing (TRU-04) |
| Parser fail-closed on bad amounts | Missing (INT-02) |
| Evidence statement timeout (copy `liquidity_snapshots` precedent) | Missing (RES-01) |
| CSP nonce/SRI; HTTP CSP at nginx | Missing (F-01) |
| Playwright e2e in CI; poisoned-route + frozen-pair specs | Missing (TST-02) |
| Branch-scoped CI caches; digest-pinned optimizer/Dockerfiles | Missing (SEC-06/07/08) |
| Required functional-test merge gate (either forge) | Missing (SEC-12) |
| Hermetic frontend test env; hermetic indexer negative-cache test | Missing (TST-05/TST-04) |
| #1381 test gaps 1–13; #1376 test gaps TG-1…TG-6 | Missing (§2) |
| Divergence monitor (#1216) | **Open** — load-bearing for ORB-05/06 |
| Public bug bounty | Deferred (TVL ladder) |

---

## 10. Recommendations (priority)

**P0 — before the pending migrations (#1384 / #1386):**
1. **Pin in-place router migration in the #1384 runbook** (RT-5) and verify by **code hash, not cw2** (OPS-06). Inventory cw2 names of all listed tokens first (RT-6).
2. **Add a per-pair `GetDiscountRegistry` probe to the 1.19.0 rollout command** (#1232 residual blind spot).
3. **Decide the cyclic-route policy** (RT-1): cheapest is a router-side reject of routes touching a capable token twice; add the A→B→A regression test either way.
4. **Bridge-debt: fix BD-1 (per-market code-id pin) and BD-3 (`ensure_from_older_version`) before any second market is registered** — both are live-deployment liveness items triggered by owner *mistake*.

**P1 — standing security backlog (unchanged urgency):**
5. Triage **#1229** (ratify launcher origin) and **#1225** (placement fillability reject / park-on-zero-cost) — both confirmed live; #718 stays gated on #1225.
6. **ORB-05/ORB-01:** reject non-finite/≤0 source prices at parse; median or deviation discard; staleness TTL surfaced to dApp **and** ops scripts. **ORB-06:** deviation band / median-of-N for hub marks. Ship **#1216**.
7. **TRU-01:** chain-id assert at indexer startup + in buyback/rebalance scripts (OPS-05/07: unify the RPC trust anchors).
8. **INT-02** fail-closed parser amounts; **RES-01** copy the `statement_timeout` precedent.

**P2 — perimeter and hygiene:**
9. SEC-12/INF-28: require a green functional suite on one forge minimum; branch-scoped/read-only MR caches (SEC-06); digest pins (SEC-07/08); fix the broken CODEOWNERS runbook reference (SEC-11).
10. AUTH-02 least-privilege DB roles; DB-01 scope the host-network override; TRU-04 seed-qa guard.
11. TST-04/TST-05 hermetic tests; TST-02 e2e into CI; #1381 test gaps 1–4 and #1376 TG-1…TG-3.
12. RT-2 entry guard; RT-3 documentation; RT-4 preview/execute parity; EO-1 seed fallback; IX-1 discount-bps validation; IX-2 code-id-keyed discovery cache.

---

## 11. What this audit did not do

- No live columbus-5 exploit transactions; no third-party wasm decompile of the deployed bridge-debt code ids (recommended as belt-and-suspenders — BD-10; on-chain hashes match committed checksums and source is drift-free).
- No review of wrap-mapper / UST1-window / CMM / treasury source (out of repo; admin key state tracked via #526/#697).
- Playwright e2e not executed (inventoried; still absent from CI); Postgres integration suites not executed locally (merge-receipt evidence cited).
- No publication of #650 third-party findings (forbidden by that issue); treated only as an external-actor node in §14.
- Not a substitute for a formal third-party audit.

---

## 12. Positive controls (short)

- **#1381:** exact integer round-trips both directions; live-state route authentication; fail-closed capability discovery and both migration orders; exact-match settlement authorization; owned-funds balance deltas; formula-identical indexer/frontend parity with 503-on-no-sim.
- **#1376:** closed-form pool inversion with minimality proofs test-anchored at offer±1; shared Uint256 commission; explicit budget/infeasible errors; structural sim→execute protection (fidelity + fresh sim + min_receive).
- **Bridge-debt:** registration gauntlet (code-id pin, sole-minter, zero-supply, decimals, dup maps); `total_supply == outstanding` re-checked every Issue/Repay; non-transferable receipt; owner-gated everything except permissionless (protocol-beneficial) Repay; **live deployment LCD-verified against committed checksums**.
- **Indexer new surfaces:** enforced resource bounds on all paths (semaphore + time budget + graph caps + work budget); LCD-heavy tiering; fail-closed tax discovery and per-hop eligibility; Option-based TVL with coverage identity; statement-timeout precedent; no SQLi; sanitized errors.
- **Standing:** fail-closed blacklist + F6 execute guards; dual rate governors with prod clamps; route/book walk caps; 884 contract tests incl. adversarial + PoC inversions; build-time mnemonic/DEV_MODE guards; compose digest pins + loopback binds (base file); gitleaks both forges (checksum-pinned); cargo-audit/npm-audit; no IBC entry points; factory provenance at ingest.
- **Ops:** buyback economic hardening (#1375); DRY_RUN now actually dry (OPS-04 fixed); the #1324 executed migration is the model ceremony.

---

## 13. Finding index (this audit)

| ID | Sev | Area | One-line |
|----|-----|------|----------|
| RT-5 | Medium | tax token/router | `config.router` immutable — #1384 must migrate router **in place** |
| ORB-06 | Medium | indexer hub USD | Largest-liquidity UST1/USTR mark hijackable with ≈half-incumbent-TVL capital; cascades to USTR + ops peg gates (Nostra pattern) |
| BD-1 | Medium | bridge-debt | `SetDebtCodeId` rotation bricks live markets' Issue/Repay (owner-mistake liveness) |
| RT-1 | Low–Medium | tax token | Cyclic route (A→B→A) reverts on cooldown; quotes succeed — launch-phase tokens |
| OPS-06 | Low–Medium | ops | cw2 cannot distinguish pre/post-#1381 router/token — verify migration by hash |
| IX-3 | Low | indexer | `MAX_DISCOVERY_ASSETS=64` blanket fail-closed can wedge dense graphs |
| RT-2 | Low (latent) | router | `unwrap_output` + taxed output: double buy-tax, pre-tax `minimum_receive`; today revert-only |
| RT-3 | Low | pair hooks | Governance AfterSwap hook can force Buy attribution / route revert mid-context |
| RT-4 | Low | tax token | Protocol-exempt trader: preview OK, execution reverts |
| EO-1 | Low | pair reverse | Seed-fallback doubling can exhaust 32-call budget on feasible targets (fail-closed) |
| EO-2 | Low (pre-exist) | pair | Sim ignores AfterSwap hook fees that execute enforces against `min_return` |
| IX-1 | Low | indexer | `discount_bps > 10_000` underflows `effective_fee_bps_from_discount_bps` (offer-in path) |
| IX-2 | Low | indexer | Tax-discovery cache not code-id-keyed (≤5s stale verdicts across migration) |
| IX-4 | Low | indexer | Panic on short `assets` vec from hostile pair (contained by JoinSet) |
| BD-2 | Low | bridge-debt | Single-step `UpdateOwner` |
| BD-3 | Low | bridge-debt | `migrate` lacks `ensure_from_older_version` (both contracts; same gap as SC-09) |
| BD-4 | Low | bridge-debt | Core invariants admin-revocable (2-of-3 migrate admin) — document or timelock |
| BD-5 | Low | bridge-debt | `Recover` dead-ends unregistered tokens |
| OPS-07 | Low | ops | Buyback scripts divergent RPC trust anchors, no chain-id assert |
| RT-6…RT-9, EO-3…EO-7, IX-5…IX-8, BD-6…BD-10 | Info | various | See §§3–4 |
| **Re-verified still-active (from prior audits)** | | | SC-01/#1229 (High), SC-02/#1225 (High), AC-02 (High ops), C-08 (High policy), SC-03/04/05/06, #1232-residual, ORB-01/05, TRU-01/03/04, INT-02, LEAK-01, AUTH-02, DB-01, I-01, RES-01, SEC-06/07/08/11/12, TST-04/05, F-01/02/03, OPS-05, AC-01, SC-07/09 |
| **Fixed since last audit** | | | **OPS-04**, **#1322/#1324** (executed), **#1380** (TVL false-zero) |

---

## 14. Exploit-chaining investigation

**Method:** every active finding (new + re-verified), every open security-adjacent issue (#1225, #1229, #1232-residual, #1384, #1385, #1386, #526, #697, #718, #1216, #1208, #650-confidential), and every out-of-repo dependency (Terra Classic reorgs/halts, public LCD/RPC providers, KuCoin/MEXC/CoinGecko, BSC RPC/Venus, wrap-mapper/UST1-window/CMM contracts and keys, GitLab/Forgejo/Coolify/npm/Docker Hub, the Cloud Agent fleet, WalletConnect, third-party bots, single-operator bus factor) was treated as a graph node. Edges exist where one node's preconditions enable/amplify another. No reproduction steps or payloads included.

### 14.1 What changed since 2026-09-23 (`INTERNAL_KIMIK3_1790149457` §14)

| Prior chain | 2026-10-04 update |
|-------------|-------------------|
| **M** (oracle saturation → migrate brick) | **DEFUSED:** #1324 executed; all 21 live pairs on 1.18.0. Residual sliver: #1232's `DISCOUNT_REGISTRY` blind spot carries into the 1.19.0 rollout (#1386) — one-line probe fixes it |
| **A′** (unbrick first-swap arb) | **Closed with M.** The 1.19.0 rollout does not change pricing math (query-side + overflow widening), so no new scheduled-migrate arb window |
| **N** (launcher spoof → invoice siphon) | **Still open** (#1229 re-confirmed in code); now interacts with the #1384 token-migration window (Chain N′) |
| **O** (book clog × greedy default) | **Unchanged, formally gated:** #1225 is now invariant L26 (OPEN/violated); docs state #718 "remains gated on this fix" — the amplifier is explicitly disarmed until the fix lands |
| **P** (oracle zero-injection cascade) | **Partially contained:** #1380 filters non-positive quotes out of TVL (degrades to `missing_quote`, never false $0). Volume/hub/P&L/evidence/adapters/ops-gates still exposed |
| **B / I** (supply chain / agent fleet) | **Unchanged perimeter** (SEC-06/11/12 re-verified); OPS-04 node removed (fixed) |
| **K** (wrap/window keys × display) | Still open (#526/#697); unchanged |
| **D** (governance 2-of-3) | Unchanged; new lever noted: bridge-debt admin powers (BD-4 → Chain U) |
| **Q** (QA Postgres exposure) | Unchanged (DB-01, AUTH-02, TRU-04 all re-verified active) |
| **R** (red-suite normalization) | **Continues:** TST-04 reproduced red on HEAD this audit; TST-05 unchanged |
| **G2 / E / F / J / L / H** | Unchanged (see prior audit §14) |

New chains this pass: **S** (hub-mark pool hijack — Nostra pattern), **T** (#1384 migration window), **U** (bridge-debt trust concentration), **V** (cyclic-route quote/execution divergence), **N′** (launcher spoof × migration-window attestation).

---

### Chain S — Hub-mark pool hijack → USD integrity + ops treasury steering (NEW — the Nostra pattern, in-repo)

**Nodes:** ORB-06 (largest-liquidity UST1/USTR selection; TVL = 2× priced-leg; $100 floor; ≈$2.5k capital vs current pools) → UST1→USTR cascade (`hub_usd.rs:400-415`) → `volume_usd`, hub marks, P&L, protocol TVL, evidence `fee_usd` → DeFiLlama/CG/CMC/GeckoTerminal adapters (the repo *operates* the same kind of feed Nostra's attacker gamed — third parties selecting CL8Y pools as reference inherit this) → ops peg gates in rebalance/buyback scripts (OPS-05/07: no chain-id assert, divergent RPC anchors) → #1216 (divergence monitor) still open → dApp display → user decisions.

1. Attacker posts a UST1/cUSTC pool with ≈$2.5k+ real cUSTC and a skewed ratio; the implied UST1 mark wins largest-TVL selection. Capital is LP-refundable after the mark moves.
2. Every USD surface corrupts; USTR follows via the UST1 leg. Adapters export the corruption to listing venues (public-integrity damage; #687 turns nulls into adapter crashes).
3. Ops scripts that gate treasury movement on indexer USD mis-size or mis-time mainnet transactions; with no divergence monitor, nothing alarms. An attacker can, e.g., dump the UST1 mark to trigger or suppress protocol buys and trade against them.

**Cost:** ≈half the incumbent pool TVL (currently low-thousands USD), recoverable. **Gain:** no direct settlement theft (on-chain settlement never reads these marks); integrity + ops-financial harm, plus reputational damage via listing venues. **Severity: Medium (High for ops treasury if a peg gate consumes a hijacked mark).** **Breaks:** deviation band vs previous mark/external reference; median across top-N candidates; priced-leg depth floor independent of implied price; ship #1216; chain-id asserts in ops scripts.

---

### Chain T — #1384 migration window: immutable router binding × cw2 blindness × cache staleness (NEW)

**Nodes:** RT-5 (`config.router` immutable; no `SetRouter`) + OPS-06 (router/token cw2 1.2.0 pre- and post-#1381 are indistinguishable; `ensure_from_older_version` accepts same-version migrates silently) + IX-2 (5s discovery-cache staleness across migrates) + RT-6 (cw2 allowlist can de-route honest tokens if the pre-migration inventory is incomplete) + runbook ordering (indexer+frontend must roll out together; legacy tax routes stay explicitly unavailable) + #1384 (open).

1. If the router is deployed at a **new address** instead of migrated in place, every existing taxed token permanently fails the router binding → routed trading bricked (fail-closed; pair-direct survives). No on-chain remedy short of a token code migration.
2. If verification relies on cw2 version, a stale pre-#1381 router passes a "1.2.0" check — the rollout could be declared complete while route funding is silently legacy (discovery fails closed, so the failure mode is routes unavailable, not tax evasion).
3. During the cutover, ≤5s stale discovery verdicts can briefly block funded routes (upgrade) or admit legacy classification (downgrade) — bounded, backstopped by fresh sims and `min_return`.
4. An incomplete cw2-name inventory silently de-routes previously routable honest tokens (availability; users see "route unavailable").

**Severity: Medium (ops/liveness; no fund-loss path found — every order fails closed).** **Breaks:** in-place router migration pinned in the runbook; hash-based verification (`get_code_data_hash` now exists in the LCD client); code-id-keyed discovery cache (IX-2 fix); pre-migration cw2 inventory (RT-6); keep the runbook's "migrate together" step.

---

### Chain U — Bridge-debt trust concentration × advisory $1 display (NEW)

**Nodes:** BD-4 (receipt non-transferability and code-id pin are admin-revocable; admin = 2-of-3) + BD-2 (single-step `UpdateOwner`) + BD-1 (code-id rotation bricks repay even by mistake) + out-of-repo CMM key compromise (#526-class) + BD-9 (indexer/frontend advisory $1 pin by identity; 82.9% of supply is CMM-IOU-backed) + third-party consumers of indexer `price_usd`/`volume_usd`.

1. Owner mistake: `SetDebtCodeId` rotation (to add a second market) bricks Issue/Repay on the live USDT market until reverted — a live debt facility halts on a single-fat-finger (BD-1).
2. Owner compromise (2-of-3): migrate the receipt to transferable code + rotate the code id → CMM IOUs become movable/sellable, Repay bricks; the DEX keeps displaying the pool and the $1 advisory pin. Containment: the multisig is already primary USDT minter, so this adds **accounting-integrity** damage, not new mint power.
3. Third-party integrators reading indexer USD see IOU-backed volume priced at $1 — honest today (disclosed), but any depeg event displays $1 until the pin is revisited (#1208-class backing-ratio surfacing is the user-facing mitigation and is open).

**Severity: Medium (conditional on 2-of-3 or owner mistake; liveness + display integrity, not reserve theft).** **Breaks:** BD-1 per-market pin; BD-3 version guard; two-step owner; document admin powers in the runbook; consider timelocked admin once stable; #1208 backing-ratio surface.

---

### Chain V — Cyclic-route quote/execution divergence × launch-phase tokens (NEW)

**Nodes:** RT-1 (A→B→A / A→B→C→B revert on cooldown-stamped re-acquisition; solvers don't model launch guards) + RT-4 (protocol-exempt traders: preview OK, execute reverts) + RE-04-class client retry behavior + T-03 (launch guards are the launch-phase anti-sniping feature — exactly when volume and bots peak).

1. On a launch-guarded token with `cooldown_blocks ≥ 1`, any solver- or hand-crafted cyclic route quotes successfully everywhere (router sim, indexer, frontend) and reverts atomically at the second touch — gas loss only, but indistinguishable from "DEX is broken" during the highest-visibility window for a new token.
2. Bots retrying (RE-04) amplify wasted gas and LCD load; the solver has no signal to deprioritize cyclic candidates because the failure is token-stateful, not quote-structural.

**Severity: Low–Medium (liveness/UX; no fund loss).** **Breaks:** router-side reject of duplicate capable tokens (also fixes the quote); or token-side same-height route-attributed cooldown pass; solver cyclic-route filter; regression tests.

---

### Chain N′ — Launcher spoof × #1384 migration-window attestation (updated Chain N)

**Nodes:** SC-01/#1229 (unratified `InstantiateMsg.launcher` — re-confirmed at `contract.rs:96-118`) → post-#606 launcher `enable_feature` trusts `GetLauncherOrigin == self` → attacker-chosen `cmm_treasury` → **new edge:** the #1384 migration re-attests catalogs, re-pins code ids, and refreshes pair asset code-ids (runbook steps 3–4) — a rogue token instantiated *before* the window with a spoofed launcher origin could be swept into the refreshed attestation set if any step treats origin as sufficient → indexer catalog (#594) + dApp confer legitimacy → within-caps honeypot under apparent official provenance.

**Contained by:** catalog attestation requires code-id + CMM admin + launcher-tx + origin (#626) — **origin alone must never become sufficient**, and the migration runbook's inventory step must re-check it (that is #1229's explicit warning, now with a concrete window where it could be forgotten). **Severity: High (unchanged; window adds a procedural amplifier).** **Breaks:** ratify `info.sender == launcher` at instantiate (the actual fix); keep 4-part attestation through #1384; dApp warning for unattested tokens.

---

### Carried-over chains (status)

- **Chain B (supply chain → prod):** unchanged perimeter; OPS-04 node removed. **High (latent).**
- **Chain I (agent fleet → repo → B):** unchanged. **High (latent).** Prompt-injection-via-issue-tracker remains the sharpest vector.
- **Chain K (#526/#697 × wrap/window/display):** unchanged; ORB-06 adds a second display-side manipulation input. **High (ops).**
- **Chain D (governance 2-of-3):** unchanged; BD-4 adds bridge-debt admin to the blast-radius inventory. **High (conditional).**
- **Chain O (#1225 × #718):** amplifier formally disarmed (gated); #1225 itself still live. **Medium now; High if #718 ships first.**
- **Chain P (oracle zero-injection):** TVL branch contained; rest unchanged. **Medium.**
- **Chain Q (QA Postgres exposure):** unchanged. **Medium (environment-dependent).**
- **Chain R (red-suite normalization):** TST-04 reproduced on HEAD this audit — the standing red suite persists. **Medium (process).**
- **Chains E, F, G2, H, J, L:** unchanged from prior audit (Medium/Low–Medium; see `INTERNAL_KIMIK3_1790149457` §14).

### Dead-ends re-verified (do not chain)

- **#1381 tax waiver / double-charge:** both migration orders fail closed; context binding is exact; settlement transfers are multiset-authorized; no waiver path found (§3.1).
- **Router custody / dust theft:** per-hop balance deltas exclude pre-existing dust; refunds are context-bound; no stranded-intermediate path.
- **Bridge-debt outsider extraction:** permissionless surface is Repay-only (strictly protocol-beneficial); registration gauntlet + consistency re-checks verified in code **and live on LCD**.
- **USDT/UST1 pool → UST1 hub hijack:** USDT is not a hub leg (`hub_usd.rs:66`); the pool cannot enter UST1 mark selection (BD-9).
- **Exact-output → execute mismatch:** fidelity gate + fresh router sim + user `min_return`; reference-implementation divergence is not executable (IX-7).
- **Oracle zero-injection → settlement loss:** settlement never reads CEX/Venus/hub handles; harm is display/ops-only (Chains P/S).
- **Freeze → theft / F6 → discount theft / SQLi → dump / IBC-hooks reentrancy:** unchanged from prior audits.
- **#650 details:** chained only as "an external actor exists," per that issue's constraint.

### 14.2 Chain priority summary (2026-10-04)

| Chain | Severity | Trigger class | Primary breaks |
|-------|----------|---------------|----------------|
| **N/N′** (launcher spoof × migration window) | High | Attacker, anytime; procedural amplifier during #1384 | Ratify launcher origin (#1229); keep 4-part attestation |
| **B** (supply chain → prod) | High (latent) | Foothold | Merge gate + cache scoping + pins + CSP |
| **I** (agent fleet → repo) | High (latent) | Prompt injection / session compromise | Fleet hardening |
| **K** (wrap/window keys × display) | High (ops) | Key misuse | Finish #526/#697; oracle halt; chain-id asserts |
| **D** (governance 2-of-3) | High (conditional) | 2 keys | Timelock / 2-step; hook cap |
| **H** (FoT at listing) | High (policy) | Governance lists bad template | #589 harness as launch blocker |
| **O** (book clog × greedy default) | Medium (High if #718 first) | Attacker dust + #718 shipping | #1225 before #718 (gated); sim parity |
| **S** (hub-mark hijack) | Medium (High for ops treasury) | ≈$2.5k capital | ORB-06 fixes; #1216 monitor |
| **P** (oracle zero-injection) | Medium | One CEX source | ORB-05/01 fixes; #1216 |
| **T** (#1384 migration window) | Medium (ops/liveness) | Migration ceremony | In-place router migration; hash verification; IX-2; RT-6 inventory |
| **U** (bridge-debt trust concentration) | Medium (conditional) | Owner mistake / 2-of-3 | BD-1/BD-3 fixes; two-step owner; document |
| **Q** (QA Postgres exposure) | Medium | Public-IP QA host | Remove host-net override; least privilege |
| **V** (cyclic-route divergence) | Low–Medium | Solver/crafted route on launch token | RT-1 router-side reject; tests |
| **R** (red-suite normalization) | Medium (process) | Standing red suites | TST-04/05 fixes; required green gate |
| **E / F / G2 / J / L** | Medium / Low–Medium | various | See prior audit |

---

*End of audit `INTERNAL_KIMIK3_1791113518`. Initial audit §§0–13; exploit-chaining §14 appended same day.*
