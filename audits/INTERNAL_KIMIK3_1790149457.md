# Internal Security Audit — cl8y-dex-terraclassic

| | |
|---|---|
| **Audit ID** | `INTERNAL_KIMIK3_1790149457` |
| **Date** | 2026-09-23 |
| **Commit** | `f3dce2d4d760763b194b1b0eb321f955b0501431` (`main`, 2026-09-23) + working-tree #1324 migrate scripts (untracked) |
| **Type** | Internal, read-only, defense-in-depth review |
| **Primary ask** | Full-stack security audit: test coverage, common DeFi / smart-contract / database attacks, e2e, happy+bad path, missing security features, access control, privileges, Rust server, contract design, tokenomic/economic attacks, oracle manipulation — then **exploit chaining** (§14) across in-repo, prior-audit, and out-of-repo findings |
| **Scope** | CosmWasm contracts (factory, pair, router, fee-discount, faucet, community-tax token/launcher/AutoLP, hooks), Rust indexer (Axum 0.8 + sqlx 0.8.6 + Postgres 16), React/Vite dApp, 63 Playwright e2e specs, ops scripts, both CI systems (GitLab CI + Woodpecker), Docker, GitLab/Forgejo project settings, out-of-repo dependencies |
| **Method** | Codebase sweep for new surfaces → web research (2026 CosmWasm/DeFi/OWASP/Rust-API/npm supply-chain) → 5 parallel domain auditors (pair+dex-common; factory/router/tax; indexer/DB; frontend; ops/CI) with independent spot re-verification of every High/Medium by the lead → live `glab` + `fj` issue/setting checks → **executed test suites** (contracts 849 pass; indexer lib 436 pass + 1 flaky fail; frontend 2825 pass + 7 env-contaminated fails) → exploit chaining (§14) |

**Prior audits treated as still-active unless re-verified:** `INTERNAL_KIMIK3_1785897304`, `INTERNAL_KIMIK3_1786830980` (#514), `INTERNAL_KIMIK3_1787230030` (F6), `INTERNAL_KIMIK3_1787468843` (community tax), `INTERNAL_GROK46_1787908099` (API4 + full sweep, 2026-08-29).

**Live issue trackers (2026-09-23):** GitLab `PlasticDigits/cl8y-dex-terraclassic` — `only_allow_merge_if_pipeline_succeeds=false`, `forking_access_level=enabled`, `visibility=public`, no approval rule (**unchanged** since GROK46). Forgejo `code/cl8y-dex-terraclassic` open security-relevant issues: **#1225** (min-price ask head-clog, `high-risk security`), **#1229** (launcher spoof via unratified `InstantiateMsg.launcher`, `high-risk security`), **#1322** (live pair oracle u128 saturation — code fixed, **pairs not yet migrated**), **#1324** (ops: pair migrate to cw2 1.18.0 — script in this working tree), **#1232** (migrate must backfill `DISCOUNT_REGISTRY`/`ORACLE_STATE`), **#697** (wrap/window/CMM admin LCD attest close-out, `priority/high`), **#526** (EOA / 2-of-3 queue), **#687** (DeFiLlama null throw), **#690** (foreign-DEX hop, not shipped), **#718** (greedy default), **#1216** (on/off-chain divergence monitor), **#1208** (wrapped-backing ratio / mint cap).

---

## 0. Extra surfaces found in this sweep (new or changed since 2026-08-29)

223 commits landed since `INTERNAL_GROK46_1787908099`. The sweep re-crawled the tree for surfaces that decide custody, quote integrity, availability, or privilege — beyond the factory/pair/router set already covered:

| Surface | Why it matters | Status in this audit |
|---------|----------------|----------------------|
| `GET /api/v1/evidence/daily` (#1205) | New public export: 6-table UNION, keyset cursor, actor "redaction" | Reviewed → **LEAK-01**, **RES-01** |
| `GET /api/v1/protocol/top-pairs` (#1263) | New public ranking; reads `pair_volume_30d` + `pair_liquidity_usd` | Reviewed — well-guarded (strict 400s, 60s cache, overflow caps) |
| TWAP `Uint256` widening (#1224/#1322, !1323) | Consensus-path arithmetic on live pairs; migrate pending | Reviewed → code correct; **live pairs still vulnerable** (Chain M) |
| Pair `migrate` backfill set | Determines whether #1324 migrate bricks old pairs | **#1232 confirmed open** (Chain M) |
| `scripts/upgrade-1324-pair-twap.sh` (untracked, in-progress) | Live columbus-5 2-of-3 migrate of every factory pair | Reviewed — strong guards; one accidental #1232 guard (§6 OPS note) |
| `greedy.rs` book-first swap (#708/#709/#710, new since GROK46) | New default-adjacent execution path; #718 would make it the default | Reviewed → positive; interacts with **#1225** (Chain O) |
| Lunc Dash WalletConnect `payload` query (#1308/#1310) | New deep-link parsing | Reviewed — scheme allowlist intact |
| `docker-compose.override.yml` `network_mode: host` | Auto-loaded by `docker compose up`; Postgres on host interfaces | **DB-01 new** |
| `scripts/deploy-dex-mainnet-soft-launch.sh` DRY_RUN | Documents "no broadcast" — stores/executes anyway | **OPS-04 new, confirmed** |
| Hourly buyback scripts (#1318 `mint-swap-custc-ust1.sh`) | Live mainnet treasury movers using indexer USD marks | **OPS-05**; no chain-id assert (Chain K/P) |
| `candle_usd_leg_1315` up/down migrations | Schema + CHECK on candle USD provenance | Reviewed — low risk |
| Trader lifetime heal (#1277/#1292/#1303) | SET-from-aggregate heal, sqlx 0.8 `&mut **tx` | Reviewed — idempotent, correct |
| Woodpecker required check | Forgejo branch protection requires only `ci/woodpecker/pr/woodpecker` | **SEC-12: gitleaks-only gate** |
| CODEOWNERS removal (#1309) | Catch-all code-owner review gate removed; runbook stale | **SEC-11** |

Web-research additions checked against this code (2026 landscape):

- **CosmWasm `Uint256::pow`/`neg` wrapping (CWA, fixed cosmwasm-std ≥1.5.4)** — repo pins `cosmwasm-std 1.5.11`: **not affected**. The new TWAP `Uint256` code uses `checked_mul`/`checked_add` and was re-verified line-by-line (§3.1).
- **Oracle signer/reporting-layer compromise (Ostium $23.75M 2026-07, Bonzo $9M zero-signature, Full Sail/Switchboard $546k 2026-08)** — the 2026 shift is *trusted-path* failure, not AMM math. Mapped to: indexer CEX/Venus pollers (**ORB-01**, new f64 edge **ORB-05**), UST1 window / wrap-mapper admin keys (#526/#697, out-of-repo), rebalance-script peg gates (Chain K/P).
- **npm supply-chain (Sept-2025 chalk/18-package 2B-download hijack; Ledger Connect Kit $700k)** — mapped to frontend caret ranges + patch-package (**F-02**), CI caches (**SEC-06**), unpinned installer/optimizer images (**OPS-02**, **SEC-07/08**).
- **sqlx RUSTSEC-2024-0363 (binary-protocol smuggling, fixed 0.8.1)** — locked `sqlx 0.8.6`: **clear**. `tokio-postgres` not in tree (GHSA-3GJW-F78C-VVPW N/A).
- **PostgreSQL CVE-2026-14666 (RLS plan-cache)** — RLS not used; compose pinned to `postgres:16-alpine` digest (16.15 line): **not affected**, but no least-privilege role (**AUTH-02**).
- **WalletConnect session hijack (OAK-T4.006)** — deep-link scheme allowlist verified; no first-party Verify API wiring (wallet-side defense) (**F-08**).
- **Unbounded map iteration / reply-id confusion / migrate auth / addr_validate (Trail of Bits patterns)** — re-verified across all crates (§3, §4).

---

## 1. Executive summary

The settlement core remains **defense-in-depth** under the documented trust model (honest 2-of-3 governance, whitelisted non-FoT CW20 templates, factory-provenance ingest). No new Critical unauthorized fund-loss path was found in-repo on current `main`. The prior-audit API4 trio (RE-01/02/03) and FE-01 are **closed in code and verified in-tree**.

**The center of gravity has moved to liveness and trusted-path integrity:**

1. **A live economic pair is sitting at ~99.96% of `u128::MAX` oracle cumulative (#1322).** The in-repo fix (Uint256, !1323) is merged but **no columbus-5 pair has been migrated** (#1324 open; script in this working tree). The migrate itself carries a second brick risk for any pair whose storage predates `DISCOUNT_REGISTRY`/`ORACLE_STATE` (**#1232**, confirmed open in code). This is the single most time-sensitive item: **Chain M**.
2. **Two open `high-risk security` issues are confirmed live in current code:** #1229 (community-tax launcher spoof — confused deputy on `EnableFeature` invoices) and #1225 (in-band min-price asks permanently clog `match_asks` — book-match DoS that #718's greedy-default would make the *default* taker experience).
3. **The merge supply chain got weaker, not stronger:** GitLab still merges with red CI; the Forgejo required check is **gitleaks-only**; CODEOWNERS catch-all was removed (#1309); CI caches are not branch-scoped; the CosmWasm optimizer and both Dockerfiles float on tags. Combined with caret-ranged npm deps and `unsafe-inline` CSP, Chain B remains the highest-latency-impact path.
4. **The indexer's trusted inputs (CEX/Venus/LCD) still lack 2026-standard hardening:** no outlier rejection, no staleness TTL, no chain-id binding, and a concrete f64 edge (`"1e999"` → `inf` → average collapses to **0**) lets a single bad source zero a reference price (ORB-05). Settlement is unaffected; everything USD-displayed and every ops peg-gate is not.
5. **Test suites were executed, not just inventoried:** contracts 849/849 pass; indexer lib has **1 flaky parallel-isolation failure on `main`** (TST-04); frontend has **7 failures caused by `.env.local` bleeding into vitest** (TST-05) — both train operators to ignore red suites, which matters because merge gates don't enforce green ones.

| Sev | Count (new or re-verified this audit) | Notes |
|-----|----------------------------------------|-------|
| Critical | 0 | No silent drain under the trust model |
| High | 5 | #1229 launcher spoof; #1225 book clog; OPS-04 DRY_RUN mainnet footgun; #526/#697 EOA leftovers (ops); FoT-at-listing policy (Chain H) |
| Medium | 18 | #1322×#1232×#1324 migrate chain; P-03 hook/spread gap; AutoLP provide slippage; ORB-01/05 oracle; TRU-01 LCD chain-id; TRU-03 F6 fail-open; TRU-04 seed-qa; LEAK-01 evidence redaction; AUTH-02 PG superuser; DB-01 host-network override; SEC-06/07/08/11/12 supply chain; I-01 reorg; INT-02 parser zero-coercion; C-01 hooks; C-04 T9 self-cross |
| Low | 14 | P-04..P-08, faucet migrate, RES-01/02, F-04..F-07, SEC-02/04/13, L-03, X-02 |
| Info / positive | many | §12 |

**Verdict:** Do not treat "no Critical" as launch-complete for **liveness**. Before anything else: run the #1324 migrate (it is the fix for a live brick), but **fix #1232 first or rely on — and document — the script's `oracle_info` precheck as the accidental guard**; then schedule #1229 and #1225. On-chain settlement remains stronger than the HTTP, oracle, and ops perimeter.

---

## 2. Test coverage — measured, not inferred

Suites were **executed** on this commit (toolchain: cargo 1.98.0, node 24.21.0):

| Suite | Result | Notes |
|-------|--------|-------|
| `smartcontracts` workspace (`cargo test --workspace`) | **849 passed, 0 failed** (27 binaries) | Includes cw-multi-test integration, proptest, adversarial-token, F6 pin, blacklist fail-closed, audit-PoC inversions, TWAP Uint256, greedy, reprice FIFO |
| `indexer` lib (`cargo test --lib`) | **436 passed, 1 failed** | **TST-04**: `pair_discovery::tests::discover_new_pair_negative_caches_non_pair_contract` fails in the parallel run, passes in isolation |
| `indexer` integration | Not run here (needs Postgres); CI has `test-indexer-integration` | 60+ integration files inventoried incl. `security.rs`, `api_evidence_daily.rs`, `indexer_trader_rolling_numeric.rs` |
| Frontend (`vitest run`) | **2825 passed, 7 failed** (338 files) | **TST-05**: all 7 trace to `frontend-dapp/.env.local` (LocalTerra deploy) overriding `VITE_CL8Y_TOKEN_ADDRESS` etc. inside unit tests |
| Playwright e2e | 63 specs inventoried, **not run**, **not in either CI** | TST-02 still active |

### TST-04 — Low–Medium: flaky parallel test-isolation failure on `main`

`indexer/src/indexer/pair_discovery.rs:47` keeps the negative-cache in a `&'static Mutex<HashMap<String, Instant>>` shared by every test in the process; multiple tests use the same `ATTACKER_PAIR = "terra1attackerpair"` constant. Under the default parallel harness, one test's insert lands between another's `reject_cache_clear()` and its first assertion → `discover_new_pair_negative_caches_non_pair_contract` fails (reproduced: fails in full run, passes solo). The production pattern is fine (fail-open re-query); the **test** is not hermetic.

**Why it matters beyond hygiene:** the merge gates do not require green tests (§6), so a red suite on `main` can persist; each ignored failure makes the next real regression invisible. **Fix:** key the cache by test-unique pair addresses, or `serial_test` the negative-cache tests.

### TST-05 — Low: frontend unit tests are not hermetic against `.env.local`

`vitest` loads `frontend-dapp/.env.local`; a LocalTerra deploy rewrites `VITE_CL8Y_TOKEN_ADDRESS` (and siblings), and 7 tests that assert mainnet defaults fail (`tokenRegistry`, `tradeQueryResolve`, `swapQueryParams`, `SwapPage.queryParams` ×3, `TradePage` H4). CI is green because CI has no `.env.local`. Local red suites train "tests cry wolf" behavior. **Fix:** vitest env-mode isolation (`envPrefix`/mode override in `vitest.config`) or pin test env explicitly.

### Happy vs bad path (qualitative)

| Layer | Happy path | Bad / adversarial | Gap notes |
|-------|------------|-------------------|-----------|
| Contracts | Extensive multitest + proptest + invariant suites | `security_tests`, `adversarial_token`, audit-PoC inversions, dust-floor belief (#1230), reprice FIFO (#1227), greedy tax/pause/blacklist (#710) | Multi-block TWAP manipulation sim still missing (C-02); hook-count gas test missing (C-01); **#1225 head-clog test missing** (open issue asks for it) |
| Indexer | Route solve, pairs, GT, evidence, top-pairs, hub, protocol | `tests/security.rs` (SQLi, CORS, 400/413/429/502), evidence 400s, heal gates | Oracle outlier/staleness tests absent (ORB-01/05); LCD chain-id test absent (TRU-01); seed-qa prod-refusal absent (TRU-04); evidence/top-pairs not on LCD-heavy 429 tests (RES-01) |
| Frontend unit | Pages, fees, routes, decimals, query params | Logo XSS, OG host, build guards, WC scheme allowlist | Poisoned-route e2e absent (F-03); CSP header e2e absent |
| Playwright | swap/pool/limits/hybrid/tax/clickwrap/outage (63 specs) | Indexer-outage banners; blacklist CTA | **Not in CI** (TST-02); no poisoned `router_operations` spec; no frozen-pair execute-denial e2e |
| Fuzz | proptest in contracts + indexer utils | Parser ASCII stress | `cargo-fuzz` absent (TST-07) |

---

## 3. Smart contracts

### 3.1 Re-verified strong (spot-checked by lead, not just agent-reported)

- **TWAP Uint256 (#1224/#1322):** `price_times_dt` is a full `Uint256` product (`oracle.rs:105–110`); `compute_twap_price` rejects `end < start` and zero elapsed; legacy u128 JSON zero-extends (tested); migrate does **not** rewrite OBSERVATIONS. `cosmwasm-std 1.5.11` is patched against the 2026 Uint256 `pow`/`neg` wrapping advisory. Correct.
- **Swap arithmetic:** pool-favorable `ceil_div_u256`, `new_k >= k` monotonicity, commission floor to user dust. First-depositor `MINIMUM_LIQUIDITY` burn; donations don't inflate LP shares; flash provide→swap→withdraw unprofitable (tests exist).
- **L9 max_spread/belief:** unified `max_spread::check_max_spread`; zero and dust-floor belief rejected (#1230); hybrid floors (#307/#273).
- **Limit book:** price-time priority, reprice joins equal-price tail keeping `order_id` (#1227), escrow owner-gated, sweep excludes escrow, scan/maker/park caps, `book_start_hint` wrong-side fallback.
- **Guards fail-closed on execute:** blacklist (`BlacklistGuardUnavailable`), F6 code-id (`AssetCodeIdGuardUnavailable`). Pause blocks claim/cancel (L6, documented).
- **Router:** CW20-Receive-only entry, `MAX_HOPS=4`, per-hop balance-delta accounting, `SwapInProgress` mutex, deadline, final `minimum_receive`, `trader` propagation for community tax (#607 option-2), blacklist fail-closed.
- **Factory:** create-pair fee + 1/block + whitelist + decimals cap; `UpdateConfig` governance-only; existing pairs cannot be re-pointed via config; migrate `ensure_from_older_version` + backfills.
- **Community tax:** sell extra-debit hard-fails above balance; SendFrom allowance = `TaxPreview.debit` (#1228); tax caps ≤2500 bps; invoice exact-amount; adopt allowlist + legacy fail-closed; launcher `payer == manager` post-#606 (C-1 closed in crates); SKU dedupe; AutoLP spread floor + factory-listed gate (#610).
- **Reply hygiene:** unknown reply ids error in pair/router/factory/AutoLP; no `ibc_*` entry points; native-token pairs rejected.

### 3.2 New / confirmed-active contract findings

#### SC-01 — High (confirmed open #1229): unratified `InstantiateMsg.launcher` → launcher spoof / invoice siphon

`community-tax-token/src/contract.rs` `instantiate` copies `msg.launcher` into `CONFIG` after `addr_validate` only — `info.sender` is never required to equal it (verified line 96–118). `GetLauncherOrigin` returns the self-reported address. Post-#606, the official launcher's `enable_feature` trusts `GetLauncherOrigin.launcher == env.contract.address` as proof of origin, then forwards the manager-paid 50 UST1 invoice into that token, whose `cmm_treasury` is also caller-chosen at instantiate.

**Impact:** anyone who can instantiate the whitelisted tax code-id (permissionless instantiate on columbus-5) sets `launcher = <official launcher>`, `manager = attacker`, `cmm_treasury = attacker`. The official launcher then accepts `EnableFeature` for the rogue instance and the invoice routes to the attacker, not the protocol CMM; `AutoV2Lp` binds an AutoLP sister gated only by the same self-reported field. If catalog attestation ever treats origin-alone as sufficient, the rogue token also looks official in the dApp/indexer. **Not** a pair-LP drain; it is protocol invoice integrity + launcher-only privilege abuse on live wasm.

**Fix direction:** ratify at instantiate (`info.sender == msg.launcher` when `launcher` is set) or keep a launcher-side map of tokens created in `REPLY_TOKEN`; catalog must require code-id + CMM admin + launcher-tx + origin (never origin alone).

#### SC-02 — High (confirmed open #1225): in-band min-price asks skip forever and clog `match_asks`

`dex-common/src/limit_placement.rs:37,87` accepts `human == MIN_LIMIT_PRICE` (`Decimal::raw(1_000_000_000)`); `orderbook.rs:1609,1826,2005,2149` skip `cost.is_zero()` fills with no park/evict. For equal-decimal 6-dec assets, any fill below ~1e9 raw pays the maker 0 → the order is skipped **forever**, sits at the ask **head**, has no `expires_at`, and default `CleanLimitBook` thresholds are 0 so keepers cannot park it. ~500 such rows (`MAX_SCAN_STEPS`) consume the entire taker walk with zero fills.

**Impact:** permissionless book-match DoS / under-fill; maker escrow stays locked (not stolen); takers spill to pool or revert. Indexer `db_orderbook_sim` inherits the empty book leg. **Amplifier:** #718 (open) makes greedy book-first the *default* on pair `Swap` + router `TerraSwap` — every TerraSwap-style bot would walk the clogged head by default (Chain O).

**Fix direction (per issue constraints):** placement-time economic-fillability reject (`remaining × price` must yield `cost ≥ 1`), or park in-band zero-cost rows instead of `continue`; keep #470's no-free-fill guard; mirror in `simulate_match_asks` / indexer sim (L8 parity).

#### SC-03 — Medium (new): `max_spread` / `belief_price` evaluated **before** hook deductions

`pair/src/contract.rs`: `assert_max_spread` (1315) runs on `total_return`/`book_return_net`/`return_amount`; hook fee deductions are collected after (1332–1344) and only `min_return` is checked against post-hook `net_return`. With a tax/burn hook registered, a trade can pass the user's spread tolerance yet deliver less ask than that tolerance implies, unless the user also set `min_return`. The frontend's default protection is slippage→`max_spread`; quotes simulated pre-hook would also over-promise (revert) or under-deliver depending on which side of the hook the sim lands.

**Impact:** slippage-completeness gap on hook-bearing pairs (community-tax hooks are the live case). Bounded by hook bps (≤2500) and by `HookFeeExceedsReturn`. **Fix:** evaluate spread against post-hook `net_return`, or document + have the dApp always set `min_return` on hook pairs (it effectively does via route-solve floors — verify parity).

#### SC-04 — Medium (new): AutoLP provide leg has `slippage_tolerance: None`

`community-tax-autolp/src/contract.rs:296` — after the swap reply, `ProvideLiquidity` is built with `slippage_tolerance: None` and `deadline: None`. The swap leg is clamped (≤200 bps, #610 floor), but the LP deposit accepts whatever post-swap ratio the pool has; a front-run of the whole skim tx skews the ratio and the provide absorbs it. Same-tx atomicity prevents cross-block intra-skim sandwich; the exposure is whole-tx front-run. **Fix:** carry a min-LP or ratio band derived from the pre-skim reserves.

#### SC-05 — Medium (still active, C-01): `UpdateHooks` has no count cap

`pair/src/contract.rs:2087–2106` (verified): factory-only, arbitrary length. Every swap does O(n) `GetConfig` + AfterSwap work; enough succeeding-but-heavy hooks push swaps past block gas → pair halt without pause. Governance blast-radius item feeding Chain D.

#### SC-06 — Medium (still active, C-04/F-05/F-06): T9 self-cross + 0-bps placement

Placement uses `limit_discount_bps`; tier-9 placement can be 0 bps; self-crossing undercuts pool fee; 300s pair discount cache (P-06/D-02) briefly amplifies tier drops. Treasury revenue / book-quality degrade; no theft.

#### SC-07 — Low (new): `Observe` panics when `seconds_ago > block_time`

`pair/src/contract.rs:425` computes `block_time - seconds_ago` with workspace `overflow-checks = true` → query panic on oversized `seconds_ago`. Query-only DoS for bad clients; use `checked_sub` → `ContractError::Oracle`.

#### SC-08 — Low (new): extreme-ratio samples drop `dt` from the cumulative

`oracle_update` skips the sample when `Decimal::checked_from_ratio` fails (#465/#1231) — that interval never accrues, biasing multi-block extreme windows down vs wall-clock. Correct vs bricking; oracle consumers must know (docs say ≥30m + TVL floor).

#### SC-09 — Low (new): faucet `migrate` lacks `ensure_from_older_version`

`faucet/src/contract.rs` migrate only `set_contract_version` — downgrade/replay possible for the wasm admin (chain-gated). Also: faucet-as-minter + per-wallet cooldown has **no Sybil resistance** (A-01) — fine for test gems, catastrophic if ever minter of an economic token.

#### SC-10 — Info: pause freezes all exits (P-01, L6 documented)

LP withdraw, cancel, claim all pause-gated; admin paths stay live. Hard lock-in if governance is compromised/stuck — feeds Chain D. C-05 (F6 maximal freeze gates exits) unchanged.

#### SC-11 — Info: dead commission-remainder branch (P-10); `hook_settlement` `.expect` on fixed serialization (P-11); `book_start_hint` mid-book hurts only the taker (P-09); expired-park budget exhaustion leaves dead rows until `CleanLimitBook` (P-08); permissionless clean + high dust floor = forced park, escrow still owner-bound (P-07).

---

## 4. Indexer / database / API

### 4.1 New since last audit — reviewed

- **`GET /api/v1/evidence/daily` (#1205):** QueryBuilder with `push_bind` for every user-derived value (cursor tuple, LIMIT, day window); surface allowlist; `limit` clamped 1–1000; future-day rejected; static SQL fragments only; a unit test even asserts the SQL never scans `pair_reserves`. **Two residuals:**
  - **LEAK-01 (Medium):** "redacted" means `actor_hash` = truncated SHA-256 of the bech32 address — **unsalted, deterministic, dictionary-reversible** over the public address space, and `tx_hash`/`pair_address`/amounts/`order_id` are exposed, so the export is fully linkable to on-chain activity. Fine if the product goal is "not plaintext in casual view"; misleading if anyone treats it as privacy. Decide policy and document; at minimum stop calling it anonymous.
  - **RES-01 (Low–Medium):** the 6-table UNION sits on the **global** 60 RPS governor, not LCD-heavy; a busy day is a heavy bounded query. Add a statement timeout and/or move to a heavier tier.
- **`GET /api/v1/protocol/top-pairs` (#1263):** strict 400s on `from/to/sort/ticker/limit≠5/window≠30d`, single-slot 60s cache, overflow caps, reads rollup tables only. Clean.
- **Candle USD leg (#1315):** schema + CHECK migration; bounded repair migration (positive prices, `POWER(10,20)` cap). Low risk.
- **Trader heal (#1277/#1292/#1303):** mismatch-gated **SET from SUM/COUNT** — idempotent, no double-count; sqlx 0.8 `&mut **tx` correct.

### 4.2 Confirmed-active indexer findings

#### INT-02 — Medium (PARSER-01/02 still active): parser coerces bad amounts to zero

`indexer/src/indexer/parser.rs:850–851` (`offer_amount`/`return_amount` `.parse().unwrap_or_default()`), same pattern for liquidity amounts (~1615–1621). Malformed wasm attributes ingest as **0-value** swaps/LP events instead of failing the event — distorts volume, candles, heals, evidence exports. **Fix:** fail the event (or the block) loudly.

#### ORB-01 — Medium (I-02 still active): CEX oracle = plain average, no outlier rejection, no staleness TTL

`indexer/src/indexer/oracle.rs:252–278`: KuCoin/MEXC/CoinGecko simple average; CoinGecko only every other tick; last price retained forever on total outage. One bad print skews the average (with 2 sources, 50% of the attacker's deviation lands).

#### ORB-05 — Medium (new, concrete edge): non-finite / negative source prices are accepted; `inf` collapses the average to **0**

`fetch_kucoin`/`fetch_mexc` `parse::<f64>()` accepts `"1e999"` → `Ok(inf)` and `"-5"` → `Ok(-5.0)`; no finiteness or positivity check anywhere before `ok_prices.push`. `avg` over `[p, inf]` is `inf`; `f64_to_bd(inf)` → `0` (existing test `f64_to_bd_non_finite_defaults_to_zero` proves the coercion). **A single compromised/glitching source zeroes the USTC/LUNC/FDUSD reference price.** Settlement is unaffected (on-chain), but `volume_usd`, hub marks, P&L, protocol TVL, evidence `fee_usd`, DeFiLlama/CG/CMC adapters, and the **ops rebalance peg-gates** all consume these handles (Chains K/P). **Fix:** reject non-finite and `<= 0` at parse; add median/max-deviation discard; add staleness TTL with `stale:true` surfacing.

#### ORB-02/03/04 — Medium/Low/Info (unchanged): Venus BSC `eth_call` advisory with no cross-venue disagreement gate; hub USD = largest-liquidity factory marks (capital-influenced, TVL-floored, stale-skipped); USDT contract-pinned advisory $1 (symbol spoof stays unpriced; depeg displays $1).

#### TRU-01 — Medium (new): LCD client has no chain-id binding

`indexer/src/lcd/mod.rs` — rustls defaults (no `danger_accept_invalid_certs`), timeouts, failover/cooldown, deterministic-reject classification, path-redacted logs. But nothing pins `chain_id` (e.g. via `/cosmos/base/tendermint/v1beta1/node_info`) at startup or periodically: a misconfigured/ compromised `LCD_URLS` pointing at a fork indexes the wrong chain until hash divergence halts. Prod requires operator-controlled LCDs, which reduces but does not remove this (ops typo). **Fix:** assert expected `chain_id` at startup + on endpoint recovery. Note: the same missing check was found in the hourly buyback scripts (OPS-05).

#### TRU-03 — Medium (RE-05 still active): F6 freeze cache fails open for routing

`asset_code_id_freeze.rs`: lock poison / LCD failure → treated as not frozen; never-seen pairs tradable. Execute on-chain stays fail-closed; the hole is `/route/solve` quoting frozen hops during probe outages (users sign reverting txs — gas loss).

#### TRU-04 — Medium (new): `seed-qa` CLI has no run-mode guard

Anyone who can run the binary with a prod `DATABASE_URL` injects synthetic `SEEDQA_` swaps. No `RUN_MODE=dev`/`ALLOW_SEED_QA` refusal. Ops-hardening, not remote.

#### I-01 — Medium (still active): reorg = halt + webhook, no rollback; rollups not height-keyed

`block_indexer.rs` hash-guard halts the process; recovery is operator script. Candles/volumes/leaderboards/hub can stay wrong until replay. Correct fail-closed serving posture; integrity recovery is manual.

#### I-05 — Low (still active): already-indexed pairs not re-verified against factory; negative cache 1h (fail-open re-query on poison — acceptable).

### 4.3 Database

- **AUTH-02 — Medium:** the indexer connects as the Postgres **superuser** (`POSTGRES_USER` from the official image); `docker/postgres-init/` only creates a test DB; no GRANT/ROLE/RLS anywhere in `indexer/migrations/`. A compromised indexer process (or anyone with the DSN) has full DB including DDL. **Fix:** separate migrator role vs restricted runtime role (no CREATE/DROP/TRUNCATE on runtime).
- **DB-01 — Medium (new): `docker-compose.override.yml` sets `network_mode: host` for postgres + localterra and is auto-loaded by `docker compose up`.** The postgres image listens on `*`; on a public-IP host (Cloud Agent VMs run this stack) that exposes Postgres **5432 with the committed default `cl8y_legal`/`cl8y_legal`** to the network, and LocalTerra RPC/LCD likewise. Data is public-chain-derived, but a writable superuser DSN = drop/poison the indexed view the dApp serves (Chain Q). **Fix:** delete the override when the userland-proxy quirk is fixed, or scope it to a loopback-only `docker-compose` profile; never ship default creds with host networking.
- **Migrations:** no GRANT/RLS; repair migrations reviewed are bounded; `candle_usd_leg` has matching up/down; `NUMERIC(38,0)`/`NUMERIC(78,18)` widenings (#1277/#676) prevent 18-dec SUM overflow. `postgres:16-alpine` digest-pinned (16.15 line — CVE-2026-14666 RLS fix present; RLS unused anyway).
- **SQLi:** none found. Allowlisted ORDER BY/intervals, `escape_like_pattern`, QueryBuilder binds, sanitized 500/502 (`tests/security.rs`).

### 4.4 API perimeter

- Dual governors (global 60 RPS / LCD-heavy 10 RPS), `PeerIpKeyExtractor` (XFF ignored — anti-spoof verified by tests), prod zero-clamp, non-loopback dual-zero refuse (#458), IPv4-only default (#282), 30s timeout, POST body cap 128 KiB, GT span+row caps (#694), compression. **Positive.**
- RES-02 (Low): peer-IP keying collapses behind NAT/proxies — fairness, not spoofing.
- LEAK-02 (Low): Swagger/OpenAPI public — surface enumeration aid.
- No authenticated endpoints at all (AUTH-01, Info): the entire API is public read; "auth" is rate limits.

---

## 5. Frontend / wallet / signing

- **F-01 — Medium (still active):** CSP `script-src 'self' 'unsafe-inline'` via **meta tag**; no nonce/SRI for `/bootstrap/*.js`. Note: meta-delivered `frame-ancestors` is **ignored by browsers** — clickjacking protection actually comes from nginx `X-Frame-Options DENY` (verified in `docker/frontend/nginx.conf`), which is fine for the dApp but the indexer API responses carry no security headers (Low, API context).
- **F-02 — Medium (supply chain):** all npm deps caret-ranged; mitigated by `package-lock.json` + `npm ci` in the Docker build + patch-package with recorded SHA for the cosmes patch. No git/CDN runtime deps. Residual: lockfile is only as strong as the merge gate that protects it (Chain B).
- **F-03 — Medium (documented residual):** a compromised indexer can steer among *valid* routes within user floors; cannot invent endpoints (token_in/out equality checked), cannot skip floors (LCD sim + `min_return`/`max_spread` wallet-side). No poisoned-route e2e.
- **F-04/F-05 — Low/Low–Medium:** risk modal and expert mode are localStorage gates (expected UX; on-chain `max_spread` is the real bound).
- **F-06 — Low:** Legal clickwrap + risk modal skip under `VITE_PLAYWRIGHT_E2E` — prod must leave unset (build guards cover mnemonic/DEV_MODE, not this).
- **F-07 — Low:** dev `server.allowedHosts: true` (DNS-rebinding surface on mis-exposed Vite; prod is static).
- **F-08 — Info:** WalletConnect Verify API not wired first-party (wallet-side defense); Lunc Dash `payload` deep-link keeps the scheme allowlist (#1308 verified).
- **F-09..F-14 — Info/positive:** no `dangerouslySetInnerHTML` in product code (tests assert); logo/OG/product-link allowlists; query-param sanitization (#711/#713/#680); amount/decimals honesty (#1255/#1257); build guards reject prod `VITE_DEV_MODE` (#695 verified at `vite.config.ts:46`) and non-dev mnemonic; no sourcemaps in prod; tokenlist bundled with CI uniqueness check; fonts self-hosted; no iframes.
- **TST-05** (§2): unit tests not hermetic vs `.env.local`.

---

## 6. Access control, privileges, ops, CI

- **AC-01 — High (conditional, unchanged):** single-step governance rotation; no timelock/2-step. Production governance = DEX 2-of-3 `terra1zlmv2…`. Feeds Chain D.
- **AC-02 — High (ops, open #526 + #697):** wrap-mapper wasm admin, treasury wasm admin, wrapper minters, vFDUSD wasm admin, CMM/window/wrap accepts — still on `cl8y2_admin` or queued. Out-of-repo keys can move unwrap fees / mint wrappers / move the window oracle; this repo **displays and routes** those prints (Chain K).
- **OPS-04 — High (new, confirmed):** `scripts/deploy-dex-mainnet-soft-launch.sh` documents `DRY_RUN=1` as "no broadcast" (line 12), but `broadcast_and_wait` (64) and `store_code` (81) never check `DRY_RUN`; `store_code` is called for cw20_mintable/cw20_base/factory/pair/router/fee-discount (184–226). `DRY_RUN=1` **stores wasm on columbus-5 and executes instantiate-adjacent steps** — real fees, real code-ids, contrary to the banner. Only `instantiate_no_funds` (102) and pair creation (355) honor it. **Fix:** gate `broadcast_and_wait`/`store_code`/`execute_msg` on `DRY_RUN` like the #1324 script does (which has correct DRY_RUN/PROBE_ONLY handling).
- **OPS-01/02/03 — High (fleet, still active):** GCH golden image: `NOPASSWD:ALL` sudo, Cursor `approvalMode: unrestricted`, `curl|bash` rustup/Cursor/nvm installers without checksums, glab `.deb` without SHA256, docker.sock `chmod 666` fallback, `/etc/gch/job.env` tokens in agent shells. Chain I.
- **SEC-06 — Medium (new):** GitLab CI caches use fixed keys (`cargo-audit-smartcontracts`, `test-contracts`, etc.) shared across branches/MRs — a malicious MR can poison the cargo registry cache that later jobs (including default-branch) consume. Scope keys by `$CI_COMMIT_REF_SLUG` or make MR caches read-only.
- **SEC-07/08 — Medium (new):** `cosmwasm/workspace-optimizer:0.16.1` tag-only (mainnet wasm supply chain) and Dockerfile base images tag-only (`rust:1.96-bookworm`, `node:24-bookworm-slim`, `nginx:1.27-alpine`, `debian:bookworm-slim`) — while compose services *are* digest-pinned. Pin digests.
- **SEC-11 — Medium (new):** catch-all CODEOWNERS removed (#1309); `docs/runbooks/forgejo-pr-merge.md` still references `.* @code/maintainers`. Review-ownership gate is gone on both forges.
- **SEC-12 — Medium (new):** Forgejo branch protection requires only `ci/woodpecker/pr/woodpecker`, which runs **gitleaks alone**. Functional tests are not a required context on the SoT forge; GitLab runs them but doesn't gate merges (`only_allow_merge_if_pipeline_succeeds=false`, re-verified 2026-09-23). **Net: no forge requires a green test suite to merge.**
- **SEC-01/02/03/04/05 — Medium/Low:** committed LocalTerra test mnemonic (expected; never fund on mainnet); gitleaks BIP39 rule scoped to frontend `src/` only; default Postgres creds (see DB-01); `setup-postgres-dev-databases.sh` echoes full `DATABASE_URL` (password) into logs; job.env tokens (with OPS-01).
- **OPS-05 — High-by-design (new detail):** hourly buybacks (`mint-swap-burn-ust1-clunc.sh`, `mint-swap-custc-ust1.sh`) are live mainnet treasury movers with economic bounds (50 UST1/h, $200 cUSTC/h, stop bands, 5% slip, YES gates) but **no LCD chain-id assert** and mint-then-swap partial-tick state (minted inventory can sit on the admin key until the next tick; state file tracks burns only). They consume indexer USD marks → Chain K/P.
- **SEC-10/INFO-10 — Medium/Low:** ops scripts default to columbus-5 keyring; soft-launch verifies deployer address but has no interactive YES; `make reset` has no confirm (local).
- **Positives:** indexer image non-root; compose digest pins + loopback binds (base file); gitleaks on both forges (Woodpecker checksum-pinned binary); cargo-audit + npm-audit in GitLab CI; indexer log-secret lint; `.env`/`.msig-tx` gitignored; bots are localterra-only with chain-id asserts; SECURITY.md disclosure present; #1324 migrate script is a model of ops guards (chain-id, store-key, per-pair admin check, GetPairCount cross-check, OBSERVATIONS-unchanged assert, DRY_RUN/PROBE_ONLY).

---

## 7. Tokenomics / economic / oracle (summary)

| Vector | Assessment |
|--------|------------|
| Spot-price flash loan vs pair | Settlement is CP-AMM + book; no external oracle in settlement. Thin-pool TWAP consumers at risk (C-02; docs mandate ≥30m + TVL floor). |
| Pair TWAP | Uint256 fixed in code; **live pair saturated until #1324 migrate** (#1322); extreme-ratio dt drop (SC-08); `Observe` panic edge (SC-07). |
| Hub USD | Largest-liquidity factory marks, $100 TVL floor, stale skip; capital-influenced by design (ORB-03); freeze → stale hub (E389-08). |
| External USD | ORB-01/05: no outlier/TTL; single-source zero-injection edge. |
| Fee tiers | Live balance re-read on query; pair 300s cache residual (D-02); T9 self-cross (SC-06); no leaderboard anti-Sybil (POS-02). |
| Community tax | ≤2500 bps caps; extra-debit sells hard-fail; launch guards Sybil-bypassable by design (T-03); honeypot levers are the product (disclose). |
| Faucet | Test-only posture; no Sybil resistance (SC-09). |
| Buybacks/rebalance | Bounded, YES-gated, but consume indexer USD + no chain-id assert (OPS-05). |
| UST1 window / wrap mapper | **Out of repo**; admin keys are #526/#697. |

---

## 8. Prior-audit status rollup (2026-09-23)

| Status | Items |
|--------|-------|
| **Fixed since GROK46 (verified in code/tests)** | TWAP Uint256 (#1224/#1322 in crates); RE-01/RE-02/RE-03 leftovers closed (#698: GT row cap, progress + blacklist on LCD-heavy, list caps); FE-01 prod `VITE_DEV_MODE` reject (#695); #1303/#1292 heal correctness; #1263/#1205/#1315 landed with guards |
| **Partially mitigated** | F6 freeze (indexer excludes known-frozen; TRU-03 fail-open window + C-05 exit freeze remain); AMM-01 (post-listing migrate blocked; listing-time FoT open — C-08) |
| **Still active (re-verified)** | C-01 hooks cap; C-02 TWAP manipulation sim gap; C-03/C-04 discount cache + T9 self-cross; I-01 reorg; INT-02 (PARSER-01/02); ORB-01 (I-02); TRU-03 (RE-05); AC-01; AC-02 (#526/#697); INF-05/06/13/14/15/16/28 (now + SEC-06/11/12); TST-01/02/03/07; POS-02; LOB-01/03; #687; H-5; D11 |
| **New this audit** | SC-03, SC-04, SC-07, SC-08, SC-09; ORB-05; TRU-01; TRU-04; LEAK-01; RES-01; AUTH-02; DB-01; OPS-04; OPS-05 details; SEC-06/07/08/11/12; TST-04; TST-05; F-07 |
| **Open issues confirmed live in code** | #1229 (SC-01), #1225 (SC-02), #1232 (migrate backfill), #1322 (live until #1324) |
| **Worsened** | Supply-chain perimeter (CODEOWNERS removed; Woodpecker gitleaks-only gate; caches unscoped) |

---

## 9. Missing security features (checklist)

| Feature | Status |
|---------|--------|
| Third-party contract audit | **Not done** |
| On-chain governance timelock / 2-step transfer | Missing (AC-01) |
| Hook count cap | Missing (SC-05) |
| Launcher-origin ratification | Missing (SC-01 / #1229) |
| Min-price economic-fillability placement reject | Missing (SC-02 / #1225) |
| Migrate backfill for `DISCOUNT_REGISTRY`/`ORACLE_STATE` | Missing (#1232) — **blocks safe #1324** |
| Oracle outlier rejection + staleness TTL + finiteness checks | Missing (ORB-01/05) |
| LCD chain-id pinning (indexer + ops scripts) | Missing (TRU-01 / OPS-05) |
| Postgres least-privilege roles | Missing (AUTH-02) |
| seed-qa prod guard | Missing (TRU-04) |
| Transactional ingest + rollup rebuild on reorg | Missing (I-01) |
| Parser fail-closed on bad amounts | Missing (INT-02) |
| CSP nonce/SRI; HTTP CSP at nginx (not meta) | Missing (F-01) |
| Indexer security headers | Missing (Low) |
| Playwright e2e in CI; poisoned-route + frozen-pair specs | Missing (TST-02) |
| Branch-scoped CI caches; digest-pinned optimizer/Dockerfiles | Missing (SEC-06/07/08) |
| Required functional-test merge gate (either forge) | Missing (SEC-12 / INF-28) |
| Hermetic frontend test env | Missing (TST-05) |
| `cargo-fuzz` on parser | Missing (TST-07) |
| Evidence export privacy policy (LEAK-01) | Undecided |
| Public bug bounty | Deferred (TVL ladder) |

---

## 10. Recommendations (priority)

**P0 — liveness / this week:**
1. **Fix #1232 then run #1324** (or formally rely on the script's `oracle_info` precheck as the guard and document the partial-migrate recovery runbook). A live economic pair is at the saturation edge; the longer it runs, the more likely swaps brick with LP locked.
2. **Fix OPS-04** (DRY_RUN must gate store/execute) before anyone "dry-runs" the soft-launch script against mainnet.
3. **Triage #1229 and #1225** (both `high-risk security`, both confirmed live): ratify launcher origin; add placement-time fillability reject or park-on-zero-cost.

**P1 — integrity:**
4. ORB-05/ORB-01: reject non-finite/≤0 source prices; median or deviation discard; staleness TTL with `stale:true` surfaced to dApp + used by ops scripts.
5. TRU-01: chain-id assert at indexer startup + in buyback/rebalance scripts before broadcast.
6. INT-02: fail-closed parser amounts. I-01: transactional ingest / rollup rebuild runbook.
7. SC-03: spread check vs post-hook net (or mandate min_return on hook pairs in dApp + sim parity).

**P2 — perimeter:**
8. SEC-12 + INF-28: make a functional test context required on Forgejo **or** flip GitLab `only_allow_merge_if_pipeline_succeeds`; re-scope CI caches (SEC-06); digest-pin optimizer + Dockerfiles (SEC-07/08); fix the stale merge runbook (SEC-11).
9. AUTH-02 least-privilege DB roles; DB-01 remove/scope the host-network override; SEC-04 stop echoing DSNs.
10. OPS-01/02/03 fleet hardening (no NOPASSWD:ALL, no unrestricted approval, no chmod 666, checksum every installer).
11. TST-04/TST-05 hermetic tests; TST-02 e2e into CI (even scheduled); SC-07 `checked_sub`; SC-04 AutoLP provide floor; LEAK-01 policy decision.

---

## 11. What this audit did not do

- No live columbus-5 exploit transactions; no third-party wasm decompile (the #589 harness exists for that).
- No review of wrap-mapper / UST1-window / CMM / treasury source (out of repo; their **admin key state** is tracked via #526/#697).
- No publication of #650 third-party findings (forbidden by that issue); treated only as an external-actor node in §14.
- Postgres/LocalTerra integration tests not executed locally (inventoried + CI-verified instead); contract + indexer-lib + frontend unit suites **were** executed.
- Not a substitute for a formal third-party audit.

---

## 12. Positive controls (short)

- Fail-closed blacklist + F6 **execute** guards; fail-closed blacklist on query error.
- Parameterized/allowlisted SQL everywhere audited; sanitized 500/502; startup secret lint; log path redaction.
- Dual rate governors with prod clamps; PeerIp keying (XFF ignored); IPv4-default-off; body caps; 30s timeout; GT caps.
- Route graph/hybrid/book walk caps; maker-fill clamp; discount cache fails to full fee.
- 849 contract tests incl. adversarial + PoC inversions; community-tax e2e-tx (extra-debit/net/1:1) is stronger than most DEX UIs.
- Build-time mnemonic/DEV_MODE/WC-id guards; no product `dangerouslySetInnerHTML`; logo/OG/link allowlists; wallet-authoritative LCD sim before sign.
- Compose digest pins + loopback binds; indexer non-root image; gitleaks both forges; cargo-audit/npm-audit; #1324 migrate script guard depth.
- No IBC entry points; native pairs rejected; factory provenance at ingest + negative cache; empty-factory config rejected in all modes.

---

## 13. Finding index (this audit)

| ID | Sev | Area | One-line |
|----|-----|------|----------|
| SC-01 (#1229) | High | tax token/launcher | Unratified `InstantiateMsg.launcher` → EnableFeature invoice siphon / confused deputy |
| SC-02 (#1225) | High | pair book | In-band min-price asks skip forever at head → taker-walk DoS |
| OPS-04 | High | ops script | Soft-launch `DRY_RUN=1` still stores/executes on mainnet |
| AC-02 (#526/#697) | High | ops | Wrap/window/CMM/minter admin leftovers on EOA |
| C-08 | High | policy | FoT-at-listing passes F6; only #589 harness gates |
| SC-03 | Medium | pair | max_spread/belief evaluated pre-hook; min_return post-hook |
| SC-04 | Medium | AutoLP | Provide leg `slippage_tolerance: None` |
| SC-05 (C-01) | Medium | pair | Unbounded hook list |
| SC-06 (C-04) | Medium | fees | T9 self-cross + 0-bps placement |
| #1232 | Medium | pair migrate | No `DISCOUNT_REGISTRY`/`ORACLE_STATE` backfill → brick on migrate |
| #1322 | Medium | pair oracle | Live u128 saturation until #1324 migrates |
| ORB-01 | Medium | indexer oracle | Plain average, no outlier/TTL |
| ORB-05 | Medium | indexer oracle | `inf`/negative source prices accepted; average collapses to 0 |
| TRU-01 | Medium | indexer LCD | No chain-id binding |
| TRU-03 (RE-05) | Medium | indexer F6 | Freeze cache fail-open for routing |
| TRU-04 | Medium | indexer CLI | seed-qa has no prod guard |
| LEAK-01 | Medium | indexer API | Evidence "redaction" is reversible actor hash + full linkage |
| AUTH-02 | Medium | DB | Indexer connects as Postgres superuser |
| DB-01 | Medium | DB/ops | Auto-loaded host-network override + default PG creds |
| INT-02 | Medium | indexer parser | Bad amounts coerce to 0 |
| I-01 | Medium | indexer ingest | Reorg halt without rollback; rollups not rebuilt |
| SEC-06 | Medium | CI | Fixed cache keys across branches |
| SEC-07/08 | Medium | supply chain | Tag-only optimizer + Dockerfile bases |
| SEC-11 | Medium | process | CODEOWNERS removed; stale runbook |
| SEC-12 | Medium | CI | Forgejo required check is gitleaks-only |
| RES-01 | Low–Medium | indexer API | Evidence UNION on global governor; no statement timeout |
| SC-07 | Low | pair oracle | `Observe` panic when `seconds_ago > block_time` |
| SC-08 | Low | pair oracle | Extreme-ratio samples drop dt |
| SC-09 | Low | faucet | migrate without `ensure_from_older_version`; no Sybil resistance |
| TST-04 | Low–Medium | indexer tests | Flaky shared-static negative-cache test red on main |
| TST-05 | Low | frontend tests | `.env.local` contaminates vitest (7 local failures) |
| F-01 | Medium | frontend | CSP `unsafe-inline`, meta-delivered, no SRI |
| F-02 | Medium | frontend | Caret-ranged deps (lockfile mitigates) |
| F-03 | Medium | frontend | Indexer route-steering within floors (documented residual) |
| F-04..F-08 | Low/Info | frontend | localStorage gates, e2e skip flag, dev allowedHosts, WC Verify |
| OPS-01/02/03 | High | fleet | GCH sudo/unrestricted/curl-bash/docker.sock 666 |
| OPS-05 | High(design) | ops | Hourly mainnet buybacks; no chain-id assert; partial-tick state |
| SEC-01..05 | Med/Low | ops | Test mnemonic, gitleaks scope, PG defaults, DSN echo, job.env |
| P-01..P-12 | Low/Info | pair | Pause exit freeze, hint, park budget, dead branch, etc. (§3.2 SC-10/11) |

---

## 14. Exploit-chaining investigation

**Method:** every High/Medium finding in §§2–9, every still-active finding from the four prior internals + GROK46, and every **open** security-adjacent issue on both forges (#1225, #1229, #1232, #1322, #1324, #526, #697, #687, #690, #718, #1216, #1208, #650-confidential, #617/#618) was treated as a graph node. Edges exist where one node's preconditions enable/amplify another, or where an **out-of-repo** dependency (Terra Classic reorgs/halts, public LCD, KuCoin/MEXC/CoinGecko, BSC RPC/Venus, wrap-mapper/UST1-window/CMM contracts and their keys, GitLab/Forgejo/Coolify/npm/Docker Hub/GHCR, the Cloud Agent fleet, WalletConnect relay/wallets, third-party bots, the single-operator bus factor) feeds an in-repo weakness. "Cost" is attacker outlay; "gain" is extractable value, liveness harm, or integrity harm. No reproduction steps or payloads are included.

### 14.1 What changed since GROK46 §14

| Prior chain | 2026-09-23 update |
|-------------|-------------------|
| A (freeze → unfreeze arb) | **Mutated into Chain A′**: the saturated-oracle pair (#1322) is *already* bricked; the #1324 migrate creates the same stale-price first-swap arb without any freeze event. |
| B (supply chain → prod UI) | **Weaker perimeter:** CODEOWNERS gone (#1309), Forgejo required check is gitleaks-only (SEC-12), CI caches branch-shared (SEC-06). FE-01 node closed; the rest stand. |
| C (oracle cascade) | **Sharpened into Chain P:** a concrete single-source zero-injection edge (ORB-05) replaces the abstract "no outlier check." |
| D (governance 2-of-3) | Unchanged; new levers noted (pair_code_id rotation during #1324 window, uncapped hooks SC-05). |
| E (listing-spam) | Unchanged (blunted by #585 exclusion; ops pain remains). |
| F (reorg analytics) | Unchanged (I-01). |
| G2 (API4 exhaustion) | **Downgraded:** RE-01/02/03 closed; residual RES-01 + RE-04 only. |
| H (whitelisted FoT) | Unchanged (policy High). |
| I (agent fleet) | Re-verified nodes (OPS-01/02/03, SEC-05); added prompt-injection-via-issue-tracker vector. |
| K (#526 wrap/window keys) | **Still open** (#697 is the close-out); added OPS-05 (buyback scripts consume the same marks, no chain-id assert). |
| L (tax wasm skew) | Added SC-01 (#1229) node. |
| J (external composability) | Unchanged; #690/#617/#618 still unshipped — keep it that way until provenance + cost caps exist. |

New chains this pass: **M** (oracle saturation → migrate brick), **N** (launcher spoof → invoice siphon), **O** (book clog × greedy default), **P** (oracle zero-injection cascade), **Q** (QA Postgres exposure → view poisoning), **R** (red-suite normalization meta-chain).

---

### Chain M — Live oracle saturation → forced migrate → migrate-brick risk (NEW — highest urgency)

**Nodes:** #1322 (live pair `price_a_cumulative` at ~99.96% of `u128::MAX`; `observe()` already reverts; the same checked add runs on the swap/provide/withdraw write path) → **LP lock-in when the accumulator saturates** → #1324 migrate is the only remedy (script in this working tree) → #1232 (migrate does **not** backfill `DISCOUNT_REGISTRY`/`ORACLE_STATE`; hard `.load()` on swap/simulate/place paths) → partial-batch halt state → E389-13-style first-swap arb after unbrick.

1. On the affected economic pair, once wall-clock accrual pushes the cumulative past `u128::MAX`, **every reserve-mutating execute reverts** — swap, provide, *and withdraw*. LP funds are locked, not stolen; the pair is bricked until governance migrates it. (Nine other pairs are far from the ceiling; the decimal-asymmetry pairs accrue fastest.)
2. The #1324 script migrates every factory pair with `{}`. For any pair whose storage predates the `DISCOUNT_REGISTRY`/`ORACLE_STATE` items, migrate **succeeds** (cw2 bump) and the pair then bricks on first execute — the script's per-pair `oracle_info` precheck *accidentally* guards missing `ORACLE_STATE` (it halts the batch), but **nothing prechecks `DISCOUNT_REGISTRY`**; a pair missing only that item migrates green and bricks on the next swap/place.
3. A mid-batch halt leaves mixed cw2 1.17/1.18 pairs. Indexer/dApp tolerate that (code-id-agnostic queries), but operations now owes a second 2-of-3 ceremony under time pressure — exactly when mistakes happen.
4. Post-migrate, the first swap on the previously-bricked pair trades reserves that have been frozen while CEX prices moved — a deterministic arb for whoever sequences that tx (MEV bots can watch for the 2-of-3 migrate tx; the script's own suggested "small reserve move" is that tx).

**Cost:** zero to wait; the arb is gas-only. **Gain:** liveness (pair brick, LP lock) + integrity (arb at unbrick) + ops-risk (partial migrate). **Severity: High (liveness, time-sensitive) — the only chain with a *wall-clock* trigger.** **Breaks:** fix #1232 in code before #1324 (backfill both items), or add a `GetDiscountRegistry` precheck per pair to the script; document partial-migrate recovery; pause-through-migrate (`SetPairPaused`) to kill the unbrick arb; private/ordered first tx.

---

### Chain N — Launcher spoof → invoice siphon → official-looking honeypot (NEW)

**Nodes:** SC-01 / #1229 (unratified `InstantiateMsg.launcher`; verified: `addr_validate` only, no `info.sender` check) → post-#606 launcher `enable_feature` trusts `GetLauncherOrigin == self` → token `invoice::assert_invoice_payer` accepts `payer == config.launcher` → attacker-chosen `cmm_treasury` at instantiate → `AutoV2Lp` binds AutoLP gated by the same self-reported field → indexer catalog (#594) + dApp Create/Manage Token UI confer legitimacy → columbus-5 permissionless instantiate (out-of-repo).

1. Attacker instantiates the whitelisted tax code-id with `launcher = <official launcher>`, `manager = attacker`, `cmm_treasury = attacker`. Cost: instantiate gas only.
2. The **official** launcher now accepts `EnableFeature` forwards for the rogue token; the 50 UST1 invoice lands in the token and transfers to the **attacker's** treasury, not the protocol CMM. Protocol invoice integrity is broken without touching any pair.
3. If any consumer (catalog, UI, ops runbook) treats launcher-origin as sufficient attestation, the rogue token presents as official; the attacker can then run a within-caps honeypot (≤2500 bps sell tax, launch guards) under apparent official provenance.

**Contained by:** catalog attestation today requires code-id + CMM admin + launcher-tx + origin (#626) — **origin alone must never become sufficient** (that is #1229's explicit warning). **Severity: High (auth/integrity on live wasm).** **Breaks:** ratify `info.sender == launcher` at instantiate (or launcher-side created-token map); keep the 4-part catalog attestation; dApp warning for unattested tokens.

---

### Chain O — Book head-clog × greedy-by-default × sim parity (NEW)

**Nodes:** SC-02 / #1225 (immortal zero-cost in-band asks at the ask head; ~500 rows burn `MAX_SCAN_STEPS`) → #718 (open: greedy book-first becomes the **omitted-params default** on pair `Swap` + router `TerraSwap`) → indexer `db_orderbook_sim` L8 parity requirement → RE-04-class client polling amplifies the visible failure.

1. Attacker places dust-floor-priced asks (accepted by placement today) until the ask head is a skip-forever prefix. Cost: dust escrow per row, refundable on cancel — near-zero ongoing cost.
2. Today, greedy is opt-in; after #718, **every** TerraSwap-style `Send+{"swap":{}}` bot walks the clogged head by default → under-fills, pool spillover at worse prices, or slippage reverts. Integrators read this as "CL8Y is broken."
3. If the indexer sim and the pair execute diverge on the skip rule (sim skips, execute clogs, or vice versa), quotes advertise liquidity that never fills — the L8 parity requirement in #1225 is load-bearing.

**Severity: Medium now; High if #718 ships before #1225.** **Breaks:** #1225 placement/eviction fix **before** #718; sim parity test; keep #707's "pair.swap is pool-only" guidance until then.

---

### Chain P — Single-source oracle zero-injection → USD integrity cascade (NEW)

**Nodes:** ORB-05 (`parse::<f64>()` accepts `"1e999"`→`inf`, `"-5"`→negative; no finiteness/positivity check; `f64_to_bd(inf)`→`0`) + ORB-01 (plain average; 2-source ticks; no staleness TTL) → `volume_usd`, hub marks, P&L, protocol TVL, evidence `fee_usd` → DeFiLlama/CG/CMC adapters (+ #687 throw-on-null = listing-feed availability) → ops peg gates in rebalance/buyback scripts (OPS-05) → dApp display → user decisions.

1. One compromised or glitching source (KuCoin/MEXC/CoinGecko — out-of-repo; or DNS/BGP to their API) returns a huge/negative print. The average collapses to **0** (or drags negative-ward) for USTC/LUNC/FDUSD.
2. Every USD-denominated surface the DEX publishes corrupts at once; adapters export the corruption to listing venues (public-integrity damage; #687 turns nulls into adapter crashes).
3. Ops scripts that gate treasury movement on indexer USD (UST1 $1 peg gate, buyback stop bands) mis-size or mis-time mainnet transactions; #1216 (divergence monitor) is still open, so nothing alarms.

**Cost:** one CEX API compromise/glitch (2026 precedent: Ostium/Bonzo/Switchboard — the trusted reporting path *is* the attack surface). **Gain:** no direct settlement theft (on-chain settlement never reads these); integrity + ops-financial harm. **Severity: Medium (High for ops treasury if a peg gate consumes a zeroed mark).** **Breaks:** finiteness/`>0` parse checks; median or max-deviation discard; staleness TTL with `stale:true` surfaced to dApp **and** to the ops scripts; ship #1216.

---

### Chain Q — QA/dev Postgres exposure → indexed-view poisoning (NEW)

**Nodes:** DB-01 (`docker-compose.override.yml` host-networking, auto-loaded; postgres image listens on `*`; default `cl8y_legal`/`cl8y_legal`) + AUTH-02 (superuser role; no least-privilege) + SEC-04 (setup script echoes full DSN into logs/history) + TRU-04 (`seed-qa` unguarded) + I-05 (indexed rows not re-verified).

1. On any public-IP host that runs the stack with the override present (Cloud Agent QA VMs are the documented case), Postgres 5432 answers the network with committed default credentials.
2. Attacker (or anyone who read a leaked DSN from logs) gains **superuser** Postgres: `UPDATE` reserves/swaps/candles, inject `SEEDQA_`-style rows, or `DROP`/`TRUNCATE`.
3. The dApp, adapters, and ops scripts then serve attacker-curated analytics. Execute remains wallet-simulated on-chain (F-03 bounds settlement harm), so the damage is **decision-level integrity + availability**, not direct settlement theft.

**Severity: Medium (environment-dependent; Low where loopback-only).** **Breaks:** delete/scope the override; non-default creds via env (already supported); least-privilege runtime role; `seed-qa` run-mode guard; stop echoing DSNs.

---

### Chain B — Supply chain → production artifacts/UI (updated; perimeter re-verified weaker)

**Nodes:** SEC-12 (Forgejo required check = gitleaks only) + INF-28 (GitLab merges with red CI; re-verified 2026-09-23) + SEC-11 (CODEOWNERS catch-all removed; stale runbook) + SEC-06 (fixed CI cache keys shared across branches/MRs → cargo-registry poisoning from any MR) + SEC-07/08 (tag-only optimizer + Dockerfile bases) + F-02 (caret npm ranges; lockfile is the only pin) + F-01 (`unsafe-inline` CSP) + CI artifacts (wasm + indexer binary, 7-day expiry; QA can `QA_FETCH_CI_ARTIFACTS=1`) + TST-04/05 (red suites normalized → real regressions blend in) + Coolify/npm/Docker Hub/GHCR (out-of-repo).

1. A malicious MR (public forks enabled) or a compromised Maintainer lands code or poisons a shared CI cache; no forge requires a green functional suite to merge.
2. Poisoned artifacts flow three ways: wasm artifacts → store/migrate scripts; indexer binary → Coolify/QA deploys; npm cache → frontend bundle.
3. A compromised indexer steers routes within user floors (F-03) and corrupts every USD mark (Chain P); a compromised bundle with `unsafe-inline` CSP runs injected scripts directly.

**Severity: High (latent; needs one foothold).** **Breaks:** required functional-test gate on one forge minimum; branch-scoped/read-only MR caches; digest pins; nonce/SRI CSP; keep artifacts off production unless reproducibly rebuilt.

---

### Chain I — Agent fleet → repo → Chain B (updated)

**Nodes:** OPS-01 (`NOPASSWD:ALL` + Cursor `approvalMode: unrestricted`) + OPS-03 (docker.sock `chmod 666`) + SEC-05 (`/etc/gch/job.env` tokens in agent shells) + OPS-02 (curl|bash installers) → GitLab/Cursor credentials → INF-28/SEC-12 → Chain B.

**Sharpened vector:** this repo's own issue tracker is agent-readable input. A crafted issue body or comment is a prompt-injection delivery mechanism into a root-equivalent agent session; the agent has `glab`/`fj` tokens and push access. 2026's pattern (Drift pre-signed admin, Kelp poisoned RPC) is "infrastructure and human trust," and the agent fleet *is* that layer here.

**Severity: High (fleet-latent).** **Breaks:** scoped sudo, restricted approval, 0600 job.env, drop chmod-666, checksum installers, minimum token scopes, treat issue/comment text as untrusted input in agent playbooks.

---

### Chain K — #526/#697 leftover keys × wrap/window/display (still open; updated)

**Nodes:** AC-02 (wrap-mapper wasm admin, treasury wasm admin, wrapper minters, vFDUSD wasm admin on `cl8y2_admin` or unaccepted 2-of-3; #697 is the LCD-attest close-out) → out-of-repo contracts this repo **displays and routes** (wrap/unwrap fees ingested per #613; window mint/redeem fees per #614; hub wrap identity per #570) → ORB-01/05 (operators cannot distinguish "feed lie" from "window lie") → OPS-05 (buybacks size off the same marks, no chain-id assert) → dApp wrap/UST1 UX.

1. A leftover EOA (or a never-accepted 2-of-3 path) changes unwrap fees, mints wrapper supply, or moves the UST1 window oracle.
2. Users wrap/unwrap/mint/redeem at attacker-influenced rates through the official UI; indexer fee/USD surfaces ratify the prints; rebalance ops can be steered.
3. #1208 (backing ratio / mint cap, open) is the user-facing mitigation for wrapper-supply risk and is not yet implemented.

**Severity: High (ops; keys out of this crate).** **Breaks:** finish #526/#697 accepts + LCD attestation; treat wrap/window admin as factory-governance blast radius; stale-oracle halt on `/ust1`; chain-id asserts in ops scripts.

---

### Chain D — Governance 2-of-3 compromise (updated levers)

**Nodes:** out-of-repo signer compromise → AC-01 (instant `UpdateConfig`/rotation; no timelock) → SC-05 (uncapped reverting/gas-heavy hooks) → SUP-19 (malicious discount registry) → E389-03 (de-whitelist 10184 ≈ protocol halt) → **#1324-window lever:** `UpdateConfig { pair_code_id }` + wasm-admin migrate powers mean a compromised gov can store and migrate pairs to attacker wasm (full custody of pair reserves) — this is inside the documented trust model but is the maximal version of it → E389-13 (unfreeze/refresh timing MEV).

**Cannot:** pin attacker-chosen *token* templates (refresh refuses unlisted live ids) or Sweep escrow/reserves directly. **Severity: High (conditional on 2-of-3).** **Breaks:** timelock + 2-step transfer; hook cap; document migrate powers; keep the #1324 ceremony scripted (as done) so an ad-hoc gov tx stands out.

---

### Chain A′ — Unbrick/freeze first-swap arb (mutated from Chain A)

**Nodes:** #1322 live brick → #1324 migrate → stale reserves vs CEX → first-swap arb (E389-13) — plus the original F6 path: C-05 (exit freeze) + TRU-03 (fail-open probe window) + E389-08 (hub marks freeze while CEX moves).

Same economic shape as prior Chain A, but the trigger is now a **scheduled, publicly-announced migrate** rather than an adversarial freeze: the arb window and even its approximate block are knowable in advance. **Severity: Medium.** **Breaks:** pause-through-migrate; operator-executed first reserve move in the same ceremony (the script already hints at this — formalize it); document refresh/migrate as MEV-sensitive.

---

### Chain L — Community-tax live-wasm skew (updated)

**Nodes:** `1787468843` C-2/T592-13 (option-2 fixed in current crates; live 11611 migrate state) + #596 always-on hybrid (≥2-hop always router) + SC-01 (#1229 adds invoice-siphon on top) + AutoLP SC-04 (provide-leg slippage).

If production tax tokens remain pre-option-2, official multi-hop and permissionless 1-op router sends skip buy/sell tax while pair-direct pays it — two economic regimes — and #1229 lets a rogue instance additionally capture launcher invoices. Revenue/expectation harm, not reserve desync. **Severity: Medium (ops/migrate), Low on `main` source.** **Breaks:** store+migrate 11611/11619 path; fix #1229; do not market tax-on until LCD code-id matches the option-2 build.

---

### Chain G2 — API4 exhaustion (downgraded)

RE-01/02/03 closed (row caps; LCD-heavy placement; list caps — verified in code). Residual: RES-01 (evidence UNION on the global governor; no statement timeout), RE-04 (browser refetch/progress polling as friendly load), tokens `limit=500`. **Severity: Low–Medium.** **Breaks:** statement timeout; consider heavier tier for evidence; client-side budget.

---

### Chain R — Red-suite normalization (meta-chain, NEW)

**Nodes:** TST-04 (flaky indexer test red on `main`) + TST-05 (7 frontend tests red under any LocalTerra-deployed checkout) + SEC-12/INF-28 (no forge requires green) → a future *real* security regression (e.g., an injection guard or build-guard test failing) is indistinguishable from the standing noise and merges anyway.

**Severity: Medium (process).** **Breaks:** fix both suites' hermeticity; require a green functional context somewhere; treat any new red as blocking.

---

### Chain J — External composability (unchanged, latent)

#650 (confidential third-party bot/LP contracts — **no findings published here**), #690 (foreign-DEX hop, unshipped), #617/#618/#546 (V3 grid, unshipped). Any external keeper/hop inherits: #1225 head-clog, L5/L17 hint grief, L6 pause-frozen refunds, C-05 exit freeze, RES-01/RE-04 load, Chain K pricing. **Severity: Medium (latent).** **Breaks:** provenance + cost caps before #690; keep #650 private; integrator docs already exist.

---

### Dead-ends re-verified (do not chain)

- **Freeze → theft:** escrow/reserves isolated; Sweep excludes them; same-tx migrate+swap impossible (Cosmos atomicity + live F6 query).
- **F6 → discount theft:** frozen pair does not trade; cache orthogonal.
- **SQLi → dump:** allowlists + binds; no HTTP write API.
- **Evidence export → fake pair:** UNION joins only indexed (provenance-checked) pairs.
- **Oracle zero-injection → settlement loss:** settlement never reads CEX/Venus handles; harm is display/ops-only (Chain P).
- **Router custody:** hop balance-delta + `SwapInProgress` + atomic revert; no stranded-intermediate path found.
- **IBC-hooks reentrancy:** no `ibc_*` entry points in any crate.
- **#650 details:** chained only as "an external actor exists," per that issue's constraint.

### 14.2 Chain priority summary (2026-09-23)

| Chain | Severity | Trigger class | Primary breaks |
|-------|----------|---------------|----------------|
| **M** (oracle saturation → migrate brick) | **High, wall-clock** | Time | #1232 fix → #1324 migrate; pause-through-migrate |
| **N** (launcher spoof → invoice siphon) | High | Attacker, anytime | Ratify launcher origin (#1229) |
| **B** (supply chain → prod) | High (latent) | Foothold | Merge gate + cache scoping + pins + CSP |
| **I** (agent fleet → repo) | High (latent) | Prompt injection / session compromise | Fleet hardening (OPS-01/02/03, SEC-05) |
| **K** (wrap/window keys × display) | High (ops) | Key misuse | Finish #526/#697; oracle halt; chain-id asserts |
| **D** (governance 2-of-3) | High (conditional) | 2 keys | Timelock / 2-step; hook cap |
| **H** (FoT at listing) | High (policy) | Governance lists bad template | #589 harness as launch blocker |
| **O** (book clog × greedy default) | Medium→High | Attacker dust + #718 shipping | #1225 before #718; sim parity |
| **P** (oracle zero-injection cascade) | Medium (High for ops treasury) | One CEX source | ORB-05/ORB-01 fixes; #1216 monitor |
| **Q** (QA Postgres exposure) | Medium | Public-IP QA host | Remove host-net override; least privilege |
| **A′** (unbrick arb) | Medium | Scheduled migrate | Pause-through-migrate; ordered first tx |
| **L** (tax wasm skew) | Medium ops / Low on main | Migrate lag | 11611 migrate; #1229 |
| **E** (listing spam) | Medium | Attacker fees | Freeze probe health; batch skip |
| **F** (reorg analytics) | Medium | Chain reorg | Transactional ingest; rebuild |
| **R** (red-suite normalization) | Medium (process) | Standing red suites | TST-04/05 fixes; required green gate |
| **G2** (API4) | Low–Medium | Scrapers | Statement timeout; heavier tier |
| **J** (external composability) | Medium (latent) | Third-party shipping | Provenance; keep #650 private |

---

*End of audit `INTERNAL_KIMIK3_1790149457`. Initial audit §§0–13; exploit-chaining §14 appended same day.*
