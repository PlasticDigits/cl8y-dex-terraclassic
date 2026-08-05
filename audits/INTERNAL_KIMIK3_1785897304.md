# Internal Security Audit — cl8y-dex-terraclassic

| | |
|---|---|
| **Audit ID** | `INTERNAL_KIMIK3_1785897304` |
| **Date** | 2026-08-05 |
| **Commit** | `ab5da35` (main) |
| **Type** | Internal, read-only, defense-in-depth review |
| **Scope** | CosmWasm smart contracts (pair, factory, router, hooks ×3, fee-discount, faucet), Rust indexer (axum API + Postgres data layer), React/Vite frontend dApp, infra/CI-CD/secrets, test coverage & testing practice |
| **Method** | 8 parallel domain audits (static analysis + test-inventory mapping). All findings verified against source with `file:line` citations. Items marked *(Needs verification)* could not be fully confirmed statically. |

---

## 1. Executive Summary

The codebase is **unusually mature for an internal-audit candidate**: extensive multi-contract `cw-multi-test` integration suite (~476 contract tests), a dedicated indexer security suite (36 async tests for 400/429/502/CORS/SQLi), ~1,303 frontend unit cases + 38 Playwright specs, gitleaks in CI, and documented invariants (`docs/contracts-security-audit.md`, `docs/indexer-invariants.md`).

**No Critical-severity issues were found.** The residual risk concentrates in:

1. **Operational / governance blast radius** — single-step governance rotation, unbounded hook lists, misconfigured burn hooks, malicious-registry repointing. All require governance compromise or error, but lack on-chain guardrails (2-step transfer, code-ID allowlists, count caps).
2. **Economic design assumptions** — fee-on-transfer CW20s desync pair reserves; CL8Y fee tiers are snapshot-balance based (flash-loan gameable if CL8Y liquidity exists); self-trading/wash volume permitted; Sybil leaderboards.
3. **Indexer data integrity** — non-transactional block ingestion can permanently skip derived aggregates; reorg recovery does not rebuild non-height-keyed trader/position rollups; parser coerces invalid amounts to zero instead of rejecting.
4. **Cloud-agent / infra hardening** — passwordless sudo + unrestricted agent approval on golden images, `curl | bash` bootstrap, docker socket `chmod 666` fallback, unscoped CI caches, fork-MR pipeline exposure (GitLab settings need verification).
5. **Test-coverage blind spots** — committed `lcov.info` omits `orderbook.rs` (4,090 LOC) and the entire faucet; Playwright E2E and clippy are not CI-gated; no `cargo-fuzz` targets; migrate tests missing for router/hooks/faucet.

### Severity totals

| Severity | Count | Domains |
|---|---|---|
| Critical | 0 | — |
| High | 8 | AMM×1, SUP×1, DB×3, FE×1, INF×1, TST×2 |
| Medium | 32 | AMM×1, LOB×3, SUP×5, API×2, DB×11, FE×6, INF×6, TST×7 (deduped) |
| Low | 29 | across all domains |
| Info / positive controls | ~55 | across all domains |

### High-severity quick list

| ID | Finding | Domain |
|---|---|---|
| AMM-01 | Fee-on-transfer / deflationary CW20 breaks reserve accounting → insolvency / LP lock | Pair AMM |
| SUP-07 | Governance can register reverting hook → all swaps on pair halt | Support contracts |
| PARSER-04 | Non-transactional block ingestion → partial failure permanently skips derived state | Indexer data |
| REORG-01 | Reorg handling halts but does not roll back orphaned data | Indexer data |
| REORG-02 | Trader/position rollups not height-keyed → reorg cleanup incomplete | Indexer data |
| FE-01 | `VITE_DEV_MODE=true` can reach production builds (Simulated Wallet UI) | Frontend |
| INF-14 | GCH golden image: passwordless sudo + unrestricted agent approval | Infra |
| TST-01 | LCOV excludes `orderbook.rs` + faucet → false coverage confidence | Testing |
| TST-02 | Playwright E2E not enforced in GitLab CI | Testing |

---

## 2. Pair AMM Core Findings

Scope: `pair/src/contract.rs`, `state.rs`, `orderbook.rs` (AMM side), `discount_cache.rs`, `blacklist_guard.rs`, `hybrid_reverse.rs`, `tx_swap_index.rs`, `dex_common` (max_spread, oracle, fee_discount, hook_settlement).

### [AMM-01] Fee-on-transfer / deflationary CW20 breaks reserve accounting — **High**
- **Location:** `contract.rs:793-794`, `contract.rs:1112-1181`, `contract.rs:1687-1689`
- Swaps and liquidity operations credit declared amounts to internal `RESERVES` without verifying actual CW20 balance delta. A fee-on-transfer token leaves `RESERVES` > balance; swaps continue against inflated reserves until outbound transfers revert → insolvency / LP fund lock for that leg.
- **Fix:** balance-delta checks on receive/transferFrom, or factory-level restriction to audited non-deflationary CW20 code IDs.
- **Test:** partial (`adversarial_token::fee_on_transfer_creates_reserve_imbalance` documents imbalance after provide; swap-drain / withdraw-brick NOT TESTED).

### [AMM-02] Post-swap hooks can grief all swaps (DoS) — **Medium**
- **Location:** `contract.rs:1227-1247`, `1274-1277`; comment `1925-1928`
- A reverting registered hook rolls back the whole swap. Governance error/compromise → pair halt. See also SUP-07/SUP-08.
- **Fix:** audited-hook allowlist, optional `reply_on_error` tolerance, hook count cap.
- **Test:** covered (`swap_fails_atomically_when_allowlisted_hook_reverts`).

### [AMM-03] No maximum on registered hook count (gas griefing) — **Low**
- **Location:** `contract.rs:1929-1948`, `hook_settlement.rs:104-146`
- Unbounded `Vec<Addr>`; each swap queries every hook's `GetConfig`. Dozens of hooks → block gas limit. **Fix:** cap `hooks.len()` (≤5–10). **Test:** NOT TESTED.

### [AMM-04] Fee-discount cache serves stale tier up to 300s — **Low** (accepted tradeoff)
- **Location:** `discount_cache.rs:1-12,40-42,124-127`; `pair.rs:52-58`
- Tier cached for `DISCOUNT_CACHE_TTL_SECONDS`; sell-after-cache keeps discount ≤5 min. Bounded fee leakage. **Test:** covered (TTL boundary, sim/execute parity).

### [AMM-05] `Observe { seconds_ago: 0 }` extrapolates from current reserves — **Low**
- **Location:** `contract.rs:396-424`, `2678-2692`; `oracle.rs:15-36`
- Recorded observations correctly sample pre-trade reserves, but `seconds_ago == 0` uses live `RESERVES` — can mislead naive TWAP consumers reading within the same block. **Fix:** document; integrators use `seconds_ago ≥ 1`. **Test:** partial.

### [AMM-06] Extreme reserve ratios skip oracle writes (TWAP staleness) — **Low**
- **Location:** `contract.rs:327-338`, tests `2844-2852`
- `checked_from_ratio` failure silently skips observation (prevents bricking, #465). Attacker pushing extreme ratio stalls TWAP updates. **Fix:** emit event on skipped observation. **Test:** covered.

### [AMM-07] Direct token donations create sweepable excess, not LP theft — **Low**
- **Location:** `contract.rs:2034-2094`; `state.rs:45`
- Donations don't touch `RESERVES`; factory-only `execute_sweep` recovers `balance - reserves - PENDING_ESCROW`. **Test:** covered.

### [AMM-08] `HookFeeExceedsReturn` guard present but untested end-to-end — **Low**
- **Location:** `contract.rs:1205-1215`, `hook_settlement.rs:69-73`, `error.rs:63-67`
- Overlapping high-bps hooks block swaps rather than over-drain (safe but brittle). **Test:** NOT TESTED (unit only).

### [AMM-09] Hybrid forward sim can diverge from execute when expired orders are parked — **Low**
- **Location:** `orderbook.rs:199-209` vs `1485-1494`; `contract.rs:2393-2420` vs `1056-1109`
- Sim skips expired orders read-only; execute parks ≤15 (`MAX_EXPIRED_PARKS_PER_SWAP`), potentially matching more liquidity than sim shows. **Fix:** document; always set `min_return`. **Test:** partial.

### Positive controls verified (Info)
- **AMM-10** `trader` discount field: registry is the trust boundary; pair doc overstates local enforcement (`pair.rs:374-377`).
- **AMM-11** Reentrancy/CEI: state committed before outbound messages; oracle update → match → reserves → slippage → transfers → hooks. CosmWasm rollback model holds.
- **AMM-12** First-deposit inflation mitigated: `isqrt(a×b)` mint + 1000 LP permanent burn + min-deposit rejection (Uniswap V2 pattern). Tested.
- **AMM-13** Slippage controls: default `max_spread` 1%, `belief_price`/`min_return`, hybrid book legs require slippage floor without belief (#334), deadlines on swap/provide. Sandwich tests exist.
- **AMM-14** Indexer event attributes are contract-emitted, not user-spoofable (`tx_swap_index.rs:37-52`).
- **AMM-15** Pause blocks swaps/provide/withdraw; factory-only `SetPaused`; admin paths remain.
- **AMM-16** Blacklist guard fails closed on factory query errors.
- **AMM-17** `migrate` has no in-contract auth — relies on chain-level wasm admin; ensure admin is governance multisig.

**Additional areas discovered (AMM):** factory token whitelist policy for fee-on-transfer tokens; unbounded `seconds_ago` vec in `Observe` (query DoS); `MAX_HYBRID_REVERSE_SIM_CALLS = 32` may return suboptimal quotes (query-only).

---

## 3. Limit Order Book Findings

Scope: `limit_placement.rs`, `limit_batch_withdraw.rs`, `limit_book_clean.rs`, `orderbook.rs`, limit paths in `contract.rs`/`state.rs`/`msg.rs`, incl. #504 (ExpiredLimitParkReason) and #505 (OrderStatus).

### [LOB-01] Unbounded open orders per pair (storage/gas griefing) — **Medium**
- **Location:** `limit_placement.rs:86-104`, `orderbook.rs:308-321`, `state.rs:70`
- No cap on resting orders per pair/owner; min size is `amount > 0`; maker fee can floor to 0. Attacker spams 1-unit orders → storage growth, longer match/clean walks (bounded per-tx by `MAX_SCAN_STEPS=500` but repeated).
- **Fix:** governance-configurable `max_open_orders_per_pair` and/or `min_remaining_notional` at placement; non-refundable placement deposit for sub-threshold notionals. **Test:** NOT TESTED.

### [LOB-02] Expired-order prefix can block hybrid fills within a single swap — **Medium**
- **Location:** `orderbook.rs:1485-1494`, `pair.rs:44`, `contract.rs:1056-1108`
- ≤15 expired orders parked per walk; the rest skipped (not removed); scan cap 500. Attacker lets many short-TTL orders expire → victim's hybrid swap degrades to pool-heavy execution.
- **Fix:** auto-park all expired nodes when cap hit (gas-benchmarked); document keeper obligation. **Test:** covered (park-cap / scan-cap tests).

### [LOB-03] Emergency pause freezes all maker withdrawal paths — **Medium** (liveness)
- **Location:** `contract.rs:620-622`, `667-705`, `1434-1436`, `2012-2016`
- Pause gates placement, cancel, claim-expired, clean. Funds stay safe in `PENDING_ESCROW_*` (excluded from `Sweep`) but stranded during pause.
- **Fix:** consider allowing cancel/claim during pause (placement still blocked). **Test:** covered.

### [LOB-04] Governance force-clean can park valid sub-threshold orders — **Low**
- **Location:** `limit_book_clean.rs:74-76,161-184`, `contract.rs:1360-1374`
- Permissionless `CleanLimitBook` trigger parks orders below factory-set dust thresholds (funds owner-claimable, not stolen). Misconfiguration inconvenience + claims bloat. **Fix:** sane defaults, monitor `force_expired_count`, timelock on threshold raises. **Test:** covered.

### [LOB-05] Batch/ladder placement partially atomic (skip-not-revert) — **Low**
- **Location:** `limit_placement.rs:220-234`, `260-277`
- `LimitInsertStepsExceeded` rungs are skipped + refunded; others commit; order-ID gaps burned. **Fix:** document; optional `strict_batch` flag. **Test:** covered.

### [LOB-14] `EXPIRED_LIMIT_CLAIMS` grows unboundedly until claimed — **Low**
- **Location:** `state.rs:95`, `orderbook.rs:1337-1357`
- Every park adds a row; no TTL on claims; pause blocks claims. **Fix:** permissionless sweep-to-owner after long delay, or batch-claim incentives. **Test:** partial.

### Positive controls verified (Info)
- **LOB-06** Escrow single-lock on placement verified sound (fee → treasury; batched `PENDING_ESCROW` increment; skipped rungs refunded).
- **LOB-07** Double-cancel / double-claim / cross-path replay prevented (load-then-remove; owner gates). *Explicit double-claim integration test missing.*
- **LOB-08** Claim requires only parked-row existence + owner — safe by construction.
- **LOB-09** Price validation: `MIN_LIMIT_PRICE` 1e-9 / `MAX_LIMIT_PRICE` 1e27; legacy out-of-band orders skipped at match (#467).
- **LOB-10** FIFO (price, then order_id); bad `book_start_hint` falls back to head — cannot skip better orders.
- **LOB-11** Fill rounding reduces size while `cost > remaining`; zero-cost fills skipped; dust <10 auto-parked.
- **LOB-12** Hybrid book leg fills at maker limit price — no price-theft vector for routers.
- **LOB-13** Self-trading permitted — wash volume at cost of fees; consider `owner != trader` guard if undesirable. NOT TESTED.
- **LOB-15** #504 reasons set at all park call sites; **gap:** `OrderStatus` (#505) doesn't surface `reason` — integrators must use `ExpiredLimitRefund` query.
- **LOB-16** #505 `OrderStatus` semantics sound; `Unknown` ≠ proof of fill (documented).
- **LOB-17** Config updates factory-only; `clamp_max_batch_rungs` [1,100]. *`UpdateLimitOrderConfig` unauthorized-path integration test missing.*
- **LOB-18** Clean/cancel/batch bounds enforced (dedupe, caps, resume cursor #274).

**Additional areas discovered (LOB):** `Sweep` correctly excludes limit escrow; blacklisted makers parked with `Blacklisted` reason; no fee on cancel/claim; ladder math uses checked Decimal ops; fill events contract-emitted with `_contract_address` (indexer must verify); `ORDER_NEXT_ID` u64 overflow → `InvariantViolation` (theoretical).

---

## 4. Support Contracts Findings (factory, router, hooks ×3, fee-discount, faucet)

*Note: there is no standalone hook-registry contract; hooks are per-purpose contracts (`burn-hook`, `tax-hook`, `lp-burn-hook`) registered per-pair by factory governance.*

### [SUP-01] Single-step governance/admin transfer everywhere — **Medium**
- **Location:** `factory/src/contract.rs:770-803`, `fee-discount/src/contract.rs:406-427`, `faucet/src/contract.rs:181-212`, hook admins (e.g. tax `130-141`)
- Immediate rotation, no pending/accept step, no recovery from typos. Fat-fingered `UpdateConfig { governance }` bricks gated ops. Also: factory `UpdateConfig` does **not** rotate LP-token admins (`800-802`).
- **Fix:** 2-step ownership transfer; multisig + rehearsal runbooks. **Test:** partial (rotation-without-LP-fanout test; bricking NOT TESTED).

### [SUP-05] Fee-discount tier = snapshot CL8Y balance (flash-loan gameable) — **Medium**
- **Location:** `fee-discount/src/contract.rs:230-245` (register), `518-537` (swap-time query)
- No time-weighted holding / anti-flash guard. If CL8Y is flash-borrowable: borrow → register → discounted swap via trusted router → repay, all in one tx.
- **Fix:** document as accepted risk, or holding period / snapshot block / staking receipt. **Test:** flash-loan path NOT TESTED.

### [SUP-07] Governance can assign arbitrary hook contracts; reverting hook bricks swaps — **High** (config risk)
- **Location:** `factory/src/contract.rs:394-418`; pair `1925-1948`, `1227-1247`
- `SetPairHooks` validates address strings only — no code-ID/interface check. Reverting hook halts all swaps on the pair.
- **Fix:** hook code-ID allowlist; `reply_on_error` for non-critical hooks; testnet staging runbook. **Test:** covered (atomic revert).

### [SUP-08] Hook failure semantics differ by type (burn fails closed, others fail open) — **Medium**
- **Location:** `burn-hook/src/contract.rs:129-133` (Err → parent swap fails), `lp-burn-hook:195-199` (skip), `tax-hook:106-108,210-221` (skip/swallow)
- Misconfigured burn hook (wrong `burn_token`, no settlement slice) reverts **all** swaps on the pair.
- **Fix:** align burn-hook to fail-open, or factory-side config probes in `SetPairHooks`. **Test:** burn-hook bricking NOT TESTED.

### [SUP-19] Discount registry swappable by governance; pairs trust query result — **Medium** (operational)
- **Location:** `factory/src/contract.rs:421-447`; pair `1982-2000`
- Per-pair `discount_registry` set to any valid bech32 — no code-ID allowlist. Compromised governance → registry returning `discount_bps: 10000`.
- **Fix:** registry code-ID allowlist; multisig + timelock on `SetDiscountRegistry*`. **Test:** malicious registry NOT TESTED.

### [SUP-23] Unbounded `SetPairHooks` list → swap gas DoS — **Medium**
- **Location:** `factory/src/contract.rs:400`; pair hook loop `1228-1247`; `hook_settlement.rs:104-145`
- Same root as AMM-03, governance-reachable. **Fix:** on-chain max hooks (3–5). **Test:** NOT TESTED.

### [SUP-04] Faucet migrate lacks version guard — **Low**
- **Location:** `faucet/src/contract.rs:261-263` — `set_contract_version` only, no `ensure_from_older_version` (contrast factory `1149-1155`). **Test:** NOT TESTED.

### [SUP-09] Tax/burn `UpdateAllowedPairs` doesn't validate pair contracts — **Low**
- **Location:** `burn-hook:191-214`, `tax-hook:166-189` vs `lp-burn-hook:273-276` (validates via `Pair {}` query)
- Attacker contract added as allowed caller can drain pre-funded hook treasury with inflated amounts (not pair reserves). **Fix:** reuse lp-burn validation; keep hook treasuries minimal. **Test:** NOT TESTED for burn/tax.

### [SUP-15] Router deadline not forwarded to pair hops — **Low**
- **Location:** `router/src/contract.rs:84-95,119,277-279,454-455` — entry-only deadline; hops use `deadline: None`. **Fix:** forward monotonic deadline. **Test:** router deadline covered.

### [SUP-20] Faucet: per-sender cooldown only; weak Sybil resistance — **Low**
- **Location:** `faucet/src/contract.rs:88-117` — unlimited addresses × cooldown = drain of mintable supply if uncapped. **Fix:** mint caps, pause, keep faucet tokens off factory whitelist (already policy, SUP-21). **Test:** unit covered.

### [SUP-24] `GetTiers` unbounded range query — **Low**
- **Location:** `fee-discount/src/contract.rs:545-554` — full scan (bounded by u8 tier count). **Fix:** paginate or cap tiers.

### Positive controls verified (Info)
- **SUP-02** Permissionless `CreatePair` gated: CW20-only, distinct tokens, code-ID whitelist, decimals cap, creation fee, one-per-block, duplicate check. Tested.
- **SUP-03** Migrates use `ensure_from_older_version` except faucet; auth is chain-level wasm admin.
- **SUP-06** Discount capped ≤10000 bps; cannot go negative. Tested.
- **SUP-10** Users cannot attach hooks; factory governance only.
- **SUP-11** Router multihop atomic (`reply_on_success` + tx rollback); `minimum_receive` on final hop; per-hop `min_return` for book legs. Tested.
- **SUP-12** Router `to` arbitrary recipient — intended; phishing surface for UIs (see FE-06).
- **SUP-13** Router balance-delta accounting ignores pre-existing dust (donation-resistant). Tested.
- **SUP-14** `MAX_HOPS = 4`; cycles not forbidden but fail on asset mismatch. Benign.
- **SUP-16** `SWAP_STATE` reentrancy guard blocks concurrent router swaps. Tested.
- **SUP-17** Trusted-router fee attribution; untrusted router falls back to sender. Tested.
- **SUP-18** `pair_code_id`/`lp_token_code_id` immutable post-instantiate — limits code-ID compromise blast radius.
- **SUP-21** Faucet excluded from factory whitelist; soft-launch only.
- **SUP-22** Faucet holds no native funds; drain bounded by CW20 minter caps.
- **SUP-25** Factory pair-admin ops require registry membership (O(1) check). Tested (SEC-I03).
- **SUP-26** Router blacklist guard fails closed. Tested.
- **SUP-27** Indexer swap events scoped by runtime `_contract_address`; pair discovery validates factory listing.
- **SUP-28** Malicious-hook reentrancy into pair/router not constrained — hook code-ID allowlist recommended. NOT TESTED.

---

## 5. Indexer API Findings (axum HTTP layer)

### [API-02] Route-solve progress polls bypass LCD-heavy throttle — **Medium**
- **Location:** `api/mod.rs:393-428,493-496`, `route_solve_progress.rs:209-251`
- `/route/solve` on `lcd_heavy_router` (10 RPS/IP); `/route/solve/progress` on main router (60 RPS) yet can invoke LCD `get_discount` per poll (`route_solve_progress.rs:240` → `route_solver.rs:630-646`) → 6× LCD amplification beyond budget.
- **Fix:** move progress to heavy router, or cache `discount_bps` per subject with short TTL. **Test:** NOT TESTED.

### [API-03] Compliance blacklist-check accepts unbounded address lists — **Medium**
- **Location:** `api/compliance.rs:49-86` — comma-split `tokens`/`pairs` with no count/length cap → large LCD smart-query payloads.
- **Fix:** cap ≤20 per dimension, validate `terra1` shape, cap query-string size. **Test:** NOT TESTED.

### [API-04] Swagger UI / OpenAPI always mounted — **Low**
- **Location:** `api/mod.rs:507` — no `RUN_MODE` gate; production recon surface. **Fix:** gate in prod or ACL. **Test:** availability only.

### [API-05] No HTTP security headers — **Low**
- **Location:** `api/mod.rs:376-519` — missing `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`, CSP. **Fix:** `SetResponseHeaderLayer`. **Test:** NOT TESTED.

### [API-06] `token_in`/`token_out` lack address-format validation — **Low**
- **Location:** `route_solver.rs:386-397,144-150` — arbitrary-length strings reach graph snapshot load before 400 (not SQLi; in-memory lookup). **Fix:** reuse `validate_optional_terra_address`. **Test:** partial.

### Positive controls verified (Info)
- **API-01** SQLi well-controlled: allowlisted enums for sort/order/interval/side, `QueryBuilder` binds, `escape_like_pattern`. Tested.
- **API-07** Route hybrid cache keyed on all quote-relevant inputs incl. `discount_bps` tier; amount bucketing (1M raw units) documented nuance. Tested (tier isolation).
- **API-08** Route graph only traverses factory-provenanced pairs; empty `FACTORY_ADDRESS` rejected at startup.
- **API-09** Read-only public API by design; no authN/Z surface; `seed-qa` is CLI-only (but see SEC-01).
- **API-10** Error sanitization: DB → generic 500, LCD → 502; startup logs exclude secrets. Tested.
- **API-11** CSV formula injection mitigated (`= + - @` prefixed with `'`); filename slug alphanumeric only. Tested.
- **API-12** Rate limiting per-IP (XFF ignored), global 60 / heavy 10 RPS, prod clamps zero limits, 30s timeout, 128 KiB POST cap. Caveats: `ALLOW_ZERO_RATE_LIMITS=1` on public bind removes governors; no decompressed-size cap on compression. Tested.
- **API-13** Oracle endpoint serves indexer-stored CEX feeds; no per-response freshness SLA — clients treat as indicative.
- **API-14** Compliance endpoint is a public blacklist probe — intended; rate-limit separately if abused.
- **API-15** HTTP smuggling depends on reverse proxy config *(Needs verification)*.

**Additional areas discovered (API):** `max_maker_fills` hard cap 100; deep limit-book LCD budget capped at 101 queries/page; progress registry bounded 256 entries/30s TTL; `TraceLayer` on all routes — keep sensitive query params out of INFO logs; `hybrid_by_hop` leg fields not range-validated before router sim (400 on invalid).

---

## 6. Indexer Data Layer Findings (DB, parser, reorg, oracle, migrations)

### [PARSER-04] Block ingestion not transactional — partial failure permanently skips derived state — **High**
- **Location:** `parser.rs:258-284,387-469`, `block_indexer.rs:127-140`
- `process_swap`: `trade_exists` check → insert `swap_events` → separate candle/trader/position updates, no wrapping transaction. Crash after insert → retry sees `trade_exists` and returns early, **skipping** derived updates forever.
- **Fix:** wrap per-block ingestion in one DB transaction; or reconcile derived tables on retry. **Test:** NOT TESTED (row-level dedup only).

### [REORG-01] Reorg handling halts but never rolls back — **High**
- **Location:** `reorg_alert.rs:38-51`, `poller.rs:140-190`, `block_indexer.rs:50-86`
- Hash mismatch → alert + stop. Orphaned-fork swaps/candles/volume persist until operator runs `indexer-reorg-recover.sh --cleanup-derived`; APIs serve inflated history meanwhile.
- **Fix:** automated rollback to `recovery_fork_height` or `orphaned` flagging; mandatory cleanup in runbook. **Test:** detection covered.

### [REORG-02] Trader/position rollups not height-keyed — cleanup incomplete — **High**
- **Location:** `scripts/indexer-reorg-recover.sh:176-178`, `migrations/20260310000003_add_pnl_tracking.sql:8-19`
- Recovery deletes height-keyed tables but `traders`/`trader_positions` rollups stay wrong even after cleanup.
- **Fix:** rebuild rollups from `swap_events` post-cleanup, or store height provenance on derived updates. **Test:** NOT TESTED.

### [DB-02] No application-enforced TLS for Postgres — **Medium**
- **Location:** `main.rs:111-114` — raw `DATABASE_URL`; no `sslmode=require` enforcement in `RUN_MODE=prod`. **Test:** NOT TESTED.

### [DB-03] Migrations run as application role (not least-privilege) — **Medium**
- **Location:** `main.rs:116`, `indexer/.env.example:3` — same role for DDL migrations and runtime DML. **Fix:** split migration role; run migrations in deploy job. **Test:** NOT TESTED.

### [PARSER-01] Invalid swap amounts silently become zero — **Medium**
- **Location:** `parser.rs:577-578` — `.parse().unwrap_or_default()` on untrusted event amounts. Crafted/buggy contract attrs → zero-amount swaps indexed, distorting volume/PnL.
- **Fix:** reject on parse failure or `<= 0`. **Test:** NOT TESTED for swap amounts.

### [PARSER-02] No negative-amount validation or DB CHECK constraints — **Medium**
- **Location:** `parser.rs:577-578`, `migrations/20260310000001_initial_schema.sql:45-46` — `BigDecimal` accepts negatives; `NUMERIC(38,0)` without `CHECK (>= 0)`.
- **Fix:** parse-time rejection + CHECK constraints. **Test:** NOT TESTED.

### [REORG-03] Reorg guard skipped when checkpoint hash empty — **Medium**
- **Location:** `block_indexer.rs:67-72` — post-recovery window with no hash → no reorg detection at tip. **Fix:** require hash backfill from LCD before resuming. **Test:** covered.

### [ORACLE-01] USTC/USD from external CEX APIs; stale price retained on outage — **Medium**
- **Location:** `indexer/oracle.rs:61-79`, `db/queries/oracle.rs:18-22` — KuCoin+MEXC(+CG) average; all-source failure keeps last price with no max-age check before `compute_volume_usd`.
- **Fix:** staleness TTL; NULL `volume_usd` when stale; widen sources. **Test:** partial.

### [AGG-02] Huge swap amounts irreversibly inflate aggregates — **Medium**
- **Location:** `candle_builder.rs:63-64` — no sanity cap beyond `NUMERIC(38,0)`. Whale/contract-bug amounts distort 24h rollups until manual rebuild. **Test:** NOT TESTED.

### [POS-02] Trader leaderboard is Sybil-friendly — **Medium**
- **Location:** `trader_tracker.rs:28-41`, `position_tracker.rs:26-111` — raw `offer_amount` volume (not USD), no PoP; wash volume across wallets dominates leaderboards cheaply. **Fix:** USD-denominated volume + min-liquidity filters; document limitation. **Test:** NOT TESTED.

### [MIG-02] Missing CHECK constraints on economic invariants — **Medium**
- **Location:** `migrations/20260310000001_initial_schema.sql:35-51` — add `offer_amount > 0` etc. **Test:** NOT TESTED.

### [SEC-01] `seed-qa` subcommand shipped in production binary without guard — **Medium**
- **Location:** `main.rs:23-24,40-88`, `seed_qa.rs:30-101` — anyone with shell + `DATABASE_URL` in prod can poison charts. **Fix:** `cfg(feature = "qa")` or refuse when `RUN_MODE=prod`. **Test:** NOT TESTED.

### [DOS-02] DB pool: 10 connections, no statement/acquire timeouts — **Medium**
- **Location:** `main.rs:111-113` — slow queries stall ingestion; multiple instances compete. **Fix:** `statement_timeout`, pool sizing, read replica for heavy API. **Test:** NOT TESTED.

### Lower-severity data-layer items
- **DB-04** (Low) `.env.example` dev credentials `cl8y_legal:cl8y_legal` — document never-prod.
- **DB-05** (Low) Test harness panic echoes `DATABASE_URL` (`tests/common/mod.rs:75-81`) — redact password.
- **PARSER-03** (Low) Cached DB pairs skip factory re-verification — direct DB inserts bypass provenance; optional periodic re-validation.
- **MIG-03** (Low) Manual down-migrations; orientation migration runs `TRUNCATE candles` — backup before prod migrations.
- **MIG-04** (Low) Concurrent-process migration race — sqlx advisory lock expected *(Needs verification)*; run migrations once in deploy.

### Positive controls verified (Info)
- **DB-01** `DATABASE_URL` from env, never logged (lint-enforced, SEC-F13).
- **SQL-01/02** Dynamic SQL identifiers from allowlists; LIKE patterns escaped.
- **PARSER-05** Pseudo-fuzz: 12k random amount strings + 800 attr lists, no panic.
- **ORACLE-02** USD leg is off-chain CEX data — dust trades can't manipulate it (but can manipulate pair candles).
- **AGG-01** Zero/negative prices skip candle writes; merge-candle proptest.
- **AGG-03** 5-min volume refresh recomputes from `swap_events` (partial self-healing).
- **POS-01** Oversell clamps `net_position_quote` to zero. Tested.
- **MIG-01** Strong swap idempotency: unique `(tx_hash, pair_id, swap_index)` + `ON CONFLICT DO NOTHING`. Tested.
- **SEC-02** No hardcoded prod credentials; prod requires custom `LCD_URLS`.
- **DOS-01** Block tx pagination bounded (100×50 pages); incomplete fetch fails closed without cursor advance.
- **DOS-03** LCD client: 8s timeout + endpoint cooldown on 429/502.
- **DOS-04** Pair reject cache bounded (1h TTL, 10k entries).
- **DOS-05** `hybrid_limits.rs` is a cap helper, not a cache.

**Additional areas discovered (DB):** hook-event idempotency uses `unwrap_or(false)` on DB errors (`parser.rs:337-346`) — transient failure silently skips inserts; `indexer_failed_blocks.error_message` stores full error display — ensure wrapped errors never carry secrets; `get_or_discover_pair` soft-fails on LCD outage (availability vs integrity tradeoff); reorg recovery runbook is operator-dependent.

---

## 7. Frontend Findings (React/Vite dApp)

### [FE-01] Production builds can bundle `VITE_DEV_MODE=true` (Simulated Wallet UI) — **High**
- **Location:** `src/utils/constants.ts:76`, `src/components/wallet/WalletModal.tsx:72-87`, `vite.config.ts:22-41`, `.env.example:57`
- `DEV_MODE` is a runtime env check with **no** production build guard (unlike `VITE_DEV_MNEMONIC`, which is guarded). `.env.example` defaults `VITE_DEV_MODE=true`. If it reaches a prod bundle, Simulated Wallet renders; combined with the staging mnemonic escape hatch (`VITE_ALLOW_DEV_MNEMONIC=local-only`), the shared dev mnemonic could sign real mainnet txs.
- **Fix:** build guard mirroring the mnemonic check; default `false` in `.env.example`; CI failure on prod `VITE_DEV_MODE=true`. **Test:** NOT TESTED for prod-build rejection.

### [FE-04] Compromised indexer can influence swap routes within slippage bounds — **Medium**
- **Location:** `src/utils/cw20RouteSolveQuote.ts:50-64`, `src/services/indexer/routeOperations.ts:39-63`, `src/pages/SwapPage.tsx:748-764,955-957`
- API `router_operations` flow into submit after token-match + wallet simulation + spread preflight. Router address comes from env (not API); `to` not set from API. Residual: hostile-but-valid multihop within user slippage (toxic route), not address substitution.
- **Fix:** HTTPS-only indexer; hard-block submit when pair-address resolution fails for multihop; optional indexer response signing. **Test:** adversarial-indexer E2E NOT TESTED.

### [FE-06] Pre-sign summary vs wallet decode gap — **Medium** (industry-standard limitation)
- **Location:** `src/components/swap/SwapPreSubmitSummary.tsx:73-125`, `terraWalletSignTxRaw.ts:20-22,179-189`
- dApp shows rich summary; wallet popup shows CW20 `send` + base64 inner msg. Attacker-controlled CW20 `symbol` can make summary look benign while user signs different contracts.
- **Fix:** emphasize contract addresses (done for swaps); show router address on multihop; encourage full-address verification. **Test:** covered for summary rendering.

### [FE-09] CSP allows `'unsafe-inline'` scripts; bootstrap scripts lack SRI — **Medium**
- **Location:** `index.html:18-24`, `viteCsp.ts:47-59`, `public/bootstrap/theme.js`, `render.yaml:8-10`
- Any XSS or compromised origin asset executes despite other directives. **Fix:** remove inline scripts, SRI for bootstrap assets, nonce-based CSP. **Test:** NOT TESTED.

### [FE-11] Phishing via lookalike CW20 symbols on factory-listed pairs — **Medium**
- **Location:** `SwapPage.tsx:189-196`, `tokenDisplay.ts:75-84`, `tokenRegistry.ts:12-77`
- Token list derives from factory pairs (permissionless); symbols cached in localStorage; static registry doesn't gate unknown pairs.
- **Fix:** show contract address beside symbol for unregistered tokens; "verified" badge. **Test:** NOT TESTED for lookalike UX.

### [FE-14] Indexer URL defaults to HTTP localhost; no HTTPS build enforcement — **Medium** (deploy misconfig)
- **Location:** `src/services/indexer/client.ts:31` — prod must set `VITE_INDEXER_URL` to HTTPS; nothing fails the build otherwise. Amount math itself is safe (BigInt/string raw amounts; `parseFloat` only in display). **Fix:** prod build fails if indexer URL not `https:` (with explicit local-only escape). **Test:** NOT TESTED.

### [FE-15] Simulated wallet + local signing path (dev mnemonic) — **Medium** (misconfiguration amplifier of FE-01)
- **Location:** `devWallet.ts:16-33`, `terraWalletSignTxRaw.ts:109-110,166-168`, `useWallet.ts:51-60,157-172` — local private-key signing exists for dev wallet; gated by `DEV_MODE` only. **Fix:** tie to FE-01 guard. **Test:** unit covered.

### Lower-severity frontend items
- **FE-05** (Low) Route display/indexer mismatch warns ("Route adjusted") but doesn't block submit; comment says "reject before signing" — code drift (`swapRouteDisplay.ts:24-25`).
- **FE-10** (Low) One `target="_blank"` link with `rel="noreferrer"` only (`PoolPage.tsx:1203-1205`) — add `noopener`.
- **FE-16** (Low) Empty-query token browse renders full factory set uncapped (`tokenSearchQuery.ts:75-77`) — virtualize.
- **FE-17** (Low) Supply chain: `@walletconnect/legacy-client`, `postinstall` mutates git hooks, `window.keplr` trust model; periodic `npm audit` recommended (CI audit job exists per TST inventory — verify schedule).

### Positive controls verified (Info)
- **FE-02** Mnemonic bundling guarded; prod sourcemaps disabled. Tested.
- **FE-03** Only `.env.example` tracked; local `.env*` gitignored.
- **FE-07** Slippage default 5%, presets, deadline clamp 30–3600s, expert mode gates >30% expected slippage with typed confirmation. Tested.
- **FE-08** XSS well-controlled: no `dangerouslySetInnerHTML`, logo host allowlist mirrored in CSP `img-src`. Tested.
- **FE-12** Chain-ID validation on connect (columbus-5/rebel-2/localterra). Partial E2E.
- **FE-13** localStorage holds non-sensitive prefs only; defensive parsing. Tested.

**Additional areas discovered (FE):** optional deploy-address verification (`VITE_VERIFY_DEPLOY_ADDRESSES`); trading blacklist/pause gates block submit; error copy avoids leaking env URLs; limit-order pre-submit summary lacks pair contract address row (swap has richer transparency); `swapRouteDisplay.ts` comment/code drift.

---

## 8. Infrastructure / CI-CD / Secrets Findings

### [INF-14] GCH golden image: passwordless sudo + unrestricted agent approval — **High** (agent infra)
- **Location:** `gch-cloud-setup.sh:46-47,92-102,207-218` — `NOPASSWD:ALL` sudo; Cursor `approvalMode: unrestricted`; finalize runs `agent --force --trust`. Prompt injection on an agent VM → root-equivalent execution without human approval.
- **Fix:** restrict sudo to required commands; manual/sandboxed approval mode. **Test:** NOT TESTED.

### [INF-05] CI Cargo/npm caches not branch-scoped — **Medium**
- **Location:** `.gitlab-ci.yml:76-80,97-101,148-152,252-256` — cache keys omit `CI_COMMIT_REF_SLUG`; shared runners could serve poisoned caches across branches. **Fix:** per-branch keys or `cache:key:files` on lockfiles. **Test:** NOT TESTED.

### [INF-06] Fork MR pipelines may execute untrusted code — **Medium** *(partially verified via `glab`)*
- **Location:** `.gitlab-ci.yml:36-37,49-50,67-68,109-110,141-142,240-241` — `merge_request_event` jobs without fork guards. Verified via API: project is **public** with forking **enabled**, so external fork MRs are possible. Still to verify in GitLab UI: "pipelines must run in parent project", Protected+Masked variables, `CI_JOB_TOKEN` scope.

### [INF-28] MRs can merge with failing pipelines — **Medium** *(verified via `glab`)*
- **Location:** GitLab project setting `only_allow_merge_if_pipeline_succeeds = false` (API-verified 2026-08-05)
- All CI security gates (gitleaks, cargo-audit, npm-audit, contract/indexer/frontend tests) are advisory — a failing pipeline does not block merge. Positive: `main` is a protected branch (push/merge restricted to Maintainers, force-push disabled).
- **Fix:** enable "Pipelines must succeed" (and consider "all discussions resolved" + approval rules) in project merge-request settings.

### [INF-13] `curl | bash` bootstrap in cloud toolchain scripts — **Medium**
- **Location:** `scripts/lib/cloud-agent-toolchain.sh:39,145`; `gch-cloud-setup.sh:70,83,143-145` — nvm/rustup/Cursor CLI piped installers; fetches from GitLab raw when not vendored. **Fix:** vendor + checksum-pin all bootstrap scripts.

### [INF-15] Docker socket `chmod 666` fallback — **Medium**
- **Location:** `scripts/lib/cloud-agent-docker.sh:70` — any local process → docker control → host root. **Fix:** fix group membership instead; remove fallback.

### [INF-16] GCH job credentials from `/etc/gch/job.env` with undefined permissions — **Medium**
- **Location:** `gch-cloud-init.sh:6-7`, `gch-cloud-setup.sh:118-125`, `gch-cloud-init-runner.sh:67-73` — `GITLAB_TOKEN`/`CURSOR_API_KEY`/`JOB_RUNTIME_TOKEN` into agent shells. **Fix:** `chmod 600` root:agent; minimum token scopes.

### [INF-19] `.qa-deploy-stamp` skip-deploy can leave stale chain state — **Medium** (mitigated)
- **Location:** `scripts/lib/deploy-up-to-date.sh:7-34`, `scripts/qa/start-qa.sh:96-99`, `scripts/qa/verify-deploy.sh:135-164` — mitigated by mandatory `qa-verify-deploy`; risk only if verification bypassed (`QA_SKIP_*`). **Fix:** never skip verify; fresh volumes after contract changes.

### Lower-severity infra items
- **INF-01** (Info) LocalTerra `TEST_MNEMONIC` committed (`docker/init-chain.sh:21-22`) — standard dev pattern; never fund on mainnet.
- **INF-02** (Low) Dev DB credentials in tracked files (`scripts/lib/postgres-dev.env`) — mitigated by `127.0.0.1` binding.
- **INF-03** (Low) `.gitleaks.toml:23-39` allowlists all `terra1…` strings and test paths — review scope periodically; run full-history gitleaks on release tags.
- **INF-07** (Low) GitHub reference workflow uses floating `stable` Rust (marked not executed); GitLab images pinned.
- **INF-11** (Low) Compose `POSTGRES_PASSWORD` defaults to `cl8y_legal`.
- **INF-21** (Low) Git hooks locally bypassable; CI gitleaks is the real gate; consider server-side pre-receive.
- **INF-22** (Low) CODEOWNERS misses `.gitlab-ci.yml`, `indexer/`, `frontend-dapp/`, `docker/`, `.gitleaks.toml`.
- **INF-24** (Low) Optimizer pinned by tag not digest (`smartcontracts/scripts/optimize.sh:7`); no `rust-toolchain.toml`.

### Positive controls verified (Info)
- **INF-04** No committed production tokens/keys (repo-wide scan + history spot-check).
- **INF-08** CI Postgres creds ephemeral per job.
- **INF-09** QA artifact publish limited to default branch / manual web trigger; `CI_JOB_TOKEN` only.
- **INF-10** Compose binds to `127.0.0.1`; images digest-pinned; no privileged containers / docker.sock mounts.
- **INF-12** Indexer container non-root (uid 10001); `API_BIND=0.0.0.0` appropriate behind ingress.
- **INF-17** Mainnet deploy keys off-repo (host `terrad` keyring); passphrase via stdin env; `addresses.env` gitignored.
- **INF-18** Deploy trace contains only public addresses/tx hashes.
- **INF-20** `render.yaml` has no inline secrets.
- **INF-23** SECURITY.md + disclosure template present; `main` branch protection verified via `glab` (Maintainers-only push/merge, no force-push) but `only_allow_merge_if_pipeline_succeeds` is off (see INF-28).
- **INF-25/26/27** Logs gitignored and sampled-clean; QA_PASS files secret-free; idle-wrap script handles no secrets.

**Secret-scan summary:** `git ls-files` + ripgrep for `mnemonic|private key|glpat-|ghp_|sk-|postgres://user:pass@` across tracked tree → no production secrets. Expected hits only (LocalTerra test mnemonic, docs, fixtures, dev templates). 21 MB `.indexer-dev.log` (gitignored) sampled clean.

---

## 9. Test Coverage & Testing Practice Findings

### Coverage summary

| Component | Inventory | Measurement | CI |
|---|---|---|---|
| Smart contracts | 476 `#[test]` (418 integration + 58 in-crate); proptest; no cargo-fuzz | `lcov.info`: **85.2%** but only 10 files (see TST-01) | `test-contracts` on MR; **no clippy/coverage gate** |
| Pair | 89 limit-order tests; security/fuzz/invariant suites | `contract.rs` 88.5%; **`orderbook.rs` (4,090 LOC) absent from LCOV** | — |
| Factory | factory/blacklist/migration tests | 93.4% | — |
| Router | router/hop/multihop tests | 87.7% (gaps: `reply_swap_hop`) | — |
| Fee-discount | tier/registry tests | 87.0% (`migrate` 11 uncovered lines) | — |
| Hooks | coverage + integration + mock-failing-hook | tax **63.1%**, lp-burn **67.9%**, burn **74.2%** | — |
| Faucet | 8 tests | **absent from LCOV** | — |
| Indexer | 426 tests (193 lib + 233 integration, 37 binaries); parser pseudo-fuzz; candle proptest | no LCOV in repo | `test-indexer-lib` + `test-indexer-integration` (serialized) |
| Frontend | 211 Vitest files / ~1,303 cases; 38 Playwright specs / ~168 tests | coverage configured, not enforced | `test-frontend` + build; **no E2E/coverage in CI** |
| Supply chain | gitleaks, cargo-audit, npm-audit, log-secret lint | — | present in GitLab CI |

### [TST-01] LCOV excludes most pair logic and entire faucet — **High**
- `smartcontracts/lcov.info` covers only `contracts/*/src/contract.rs` + 3 dex-common files. Missing: `orderbook.rs`, `limit_placement.rs`, `limit_book_clean.rs`, `hybrid_reverse.rs`, `discount_cache.rs`, all of `faucet/`. The 85.2% figure overstates readiness of limit-book security logic.
- **Fix:** regenerate with full module linkage; CI check that `orderbook.rs`/`faucet` appear in LCOV; per-module thresholds.

### [TST-02] Playwright E2E not enforced in GitLab CI — **High**
- 38 specs (~168 tests: swap, pool, limits, ladder, hybrid multihop, wrap, blacklist mocks, indexer outage) run locally only; docs confirm Phase-2 E2E pending (#421).
- **Fix:** scheduled or MR-gated DinD LocalTerra job mirroring `make test-e2e-tx` + indexer-outage project; keep tx project at 1 worker (#201).

### [TST-03] Contract clippy `-D warnings` / fmt not in GitLab CI — **Medium**
- `make lint-contracts` and indexer clippy are local-only despite docs mapping them to CI. **Fix:** path-filtered lint jobs.

### [TST-04] Router/hook/faucet migrate paths lack integration tests — **Medium**
- `migration_tests.rs` covers factory/pair/fee-discount only (#405). **Fix:** downgrade→migrate snapshots for router + 3 hooks + faucet.

### [TST-05] Hook contracts have lowest line coverage — **Medium**
- Uncovered: `execute_update_allowed_pairs`, `execute_after_swap`, `reply`, `migrate`. **Fix:** unauthorized-caller tests per hook msg; reply-failure rollback integration.

### [TST-06] Indexer poller loop and >100-tx blocks untested end-to-end — **Medium**
- `poller.rs` reserved for full e2e (`indexer_ingestion_hardening.rs:42`). **Fix:** mocked-LCD test with 150+ tx block; poller smoke over N heights.

### [TST-07] No cargo-fuzz / structured fuzz targets — **Medium**
- Proptest is substantial (k-invariant, LP conservation, sandwich net-negative, escrow DLL, wrap treasury) but no byte-level fuzzing of parser/LCD JSON/wasm attrs. **Fix:** `cargo-fuzz` targets for `parser::parse_*`, `cg_ticker_segments`, limit-price validation; nightly CI.

### [TST-09] Frontend lacks E2E for malicious indexer/LCD payloads — **Medium**
- Indexer-outage E2E exists; no poisoned `route/solve` responses (negative amounts, hop mismatch, huge arrays) or wallet-adversarial flows beyond mocks. **Fix:** MSW-based malicious-fixture specs.

### [TST-11] Coverage and charts suites not CI-gated — **Medium**
- `make coverage-contracts`, chart vitest/integration local-only. **Fix:** path-filtered jobs + LCOV upload.

### Lower-severity testing items
- **TST-08** (Low) Reentrancy tests assert CosmWasm model, not adversarial callbacks — add nested-call CW20 if such tokens are ever supported.
- **TST-10** (Low) No frontend E2E for chain halt/reorg mid-tx (indexer layer covered).
- **TST-12** (Low) Faucet invisible to coverage tooling.
- **TST-13** (Low) Oracle execute paths low-hit in LCOV (instrumentation gap).
- **TST-14** (Low) `best_execution` API lacks dedicated integration file.
- **TST-15** (Info) Some weak assertions (`toBeTruthy`) in UI tests; prefer value assertions on quotes/escrows/hop counts.
- **TST-16** (Info) `make test` excludes indexer integration — document scope or extend.

### Missing security tests checklist (attack class → status)

| Attack class | Contracts | Indexer | Frontend |
|---|---|---|---|
| Reentrancy | Partial (model + hook-revert; no malicious nested call) | N/A | N/A |
| Access control / unauthorized | **Covered** (extensive) | **Covered** (`security.rs`) | **Covered** (SEC-A02/B05/E01) |
| Overflow / underflow | **Covered** (checked math, fuzz boundaries) | Partial | Partial |
| Rounding / fee bounds | **Covered** (proptest k/LP, fee math) | Partial | Partial |
| Oracle manipulation | **Covered** (same-block TWAP) | Partial | Mocked |
| Sandwich / slippage | **Covered** (prop + integration) | Covered (route slippage) | E2E alignment |
| Dust / griefing | **Covered** (placement, clean, scan caps) | Partial | Ladder UI |
| Double-claim / double-spend | **Covered** (cancel-after-park, batch owner checks) | Partial | E2E claim-all |
| Expiry edge cases | **Covered** (park caps, TTL vs force) | Covered (parked lifecycle) | E2E claim harness |
| Pause enforcement | **Covered** | N/A | Covered |
| Blacklist enforcement | **Covered** (10 tests) | LCD-mock 502 | Covered |
| Fee bounds / discount registry | **Covered** | Covered | Covered |
| Migrate / upgrade safety | **Partial — router/hooks/faucet missing** | Partial (no down-migration tests) | N/A |
| Indexer outage | N/A | Covered (429/502) | Covered (E2E project) |
| Malicious API data | N/A | Parser stress + ticker matrix | **Missing E2E** |
| Wallet adversarial | `adversarial_token.rs` | N/A | **Missing** (mocks only) |
| Chain halt / reorg | N/A | Covered (halt + webhook) | **Missing** |
| Flash-loan tier gaming | **Missing** (SUP-05) | N/A | N/A |
| Hook count gas DoS | **Missing** (AMM-03/SUP-23) | N/A | N/A |
| Burn-hook brick | **Missing** (SUP-08) | N/A | N/A |
| Storage spam (LOB-01) | **Missing** | N/A | N/A |
| Non-transactional ingestion (PARSER-04) | N/A | **Missing** | N/A |
| Reorg rollup rebuild (REORG-02) | N/A | **Missing** | N/A |
| Negative/huge amounts (PARSER-01/02, AGG-02) | N/A | **Missing** | N/A |
| Prod-build dev-mode guard (FE-01) | N/A | N/A | **Missing** |

---

## 10. Consolidated Additional Areas Discovered (beyond the original audit brief)

These emerged from the codebase sweep and merit their own future analysis:

1. **Keeper/operator dependence** — expired-order parking, book cleaning, and reorg recovery all assume external keepers/operators; no economic incentive design reviewed.
2. **Cross-contract version skew** — pairs trust factory-set registry/hooks addresses without code-ID pinning; a contract-wide code-ID allowlist strategy is absent.
3. **Query-layer DoS** — unbounded `Observe { seconds_ago }` vectors, `GetTiers` full scan, empty-query token browse rendering (chain/query-layer, not just HTTP).
4. **Wasm admin key management** — all migrates rely on chain-level admin; deployment runbook should mandate multisig + document rotation (currently implicit).
5. **Comment/code drift as a security signal** — `swapRouteDisplay.ts` ("reject" vs warn-only), `pair.rs` (trader validation claim vs registry enforcement) — drift can mislead integrators into unsafe assumptions.
6. **Cloud-agent fleet security** (INF-13..16) — golden-image hardening is part of the project's security perimeter since agents hold GitLab/Cursor credentials.
7. **Leaderboard/analytics integrity** (POS-02, AGG-02) — economic incentives to poison public stats exist even without fund theft.
8. **Hook treasury hygiene** (SUP-09) — pre-funded hook balances are a separate theft surface from pair reserves.
9. **Sim/execute divergence documentation** (AMM-09, LOB-02) — integrators relying on simulation without `min_return` are systematically at risk; should be a bold integration-guide warning.
10. **GitLab project settings** (INF-06/INF-28) — partially audited via `glab`: `main` protected (Maintainers, no force-push), but pipeline-success merge gate is **off** and fork-MR variable protection remains to verify in the UI.

---

## 11. Prioritized Recommendations

**P0 — before next mainnet-relevant deploy:**
1. Decide fee-on-transfer policy: factory code-ID restriction or balance-delta accounting (AMM-01).
2. Add prod-build guards: `VITE_DEV_MODE` rejection + HTTPS indexer enforcement (FE-01, FE-14).
3. Make block ingestion transactional (PARSER-04); rebuild rollups in reorg recovery (REORG-02).
4. Reject invalid/negative/huge amounts at parser; add DB CHECK constraints (PARSER-01/02, MIG-02).
5. Harden GCH golden image: scoped sudo, manual approval mode, remove docker-socket `chmod 666` (INF-14, INF-15).

**P1 — near-term hardening:**
6. On-chain caps: hook count (AMM-03/SUP-23), open orders / min notional (LOB-01), compliance list length (API-03).
7. 2-step governance transfer + registry/hook code-ID allowlists (SUP-01, SUP-07, SUP-19).
8. Align burn-hook failure semantics or add factory config probes (SUP-08).
9. Move route-solve progress to LCD-heavy throttle or cache discount lookups (API-02).
10. Fix LCOV instrumentation; CI-gate E2E + clippy; add migrate tests for router/hooks/faucet (TST-01..04).

**P2 — defense in depth:**
11. `cargo-fuzz` targets for parser/LCD/wasm-attr decoding (TST-07).
12. Security headers on indexer API; prod-gate Swagger (API-04/05).
13. CSP without `unsafe-inline`; SRI for bootstrap scripts (FE-09).
14. Flash-loan-resistant fee tiers or documented acceptance (SUP-05).
15. Branch-scoped CI caches; enable "pipelines must succeed" for merges (INF-28); verify fork-MR pipeline/variable protection settings (INF-05/06).
16. Oracle staleness TTL for `volume_usd` (ORACLE-01); statement timeouts + pool sizing (DOS-02).

---

*Audit performed read-only. No files were modified during analysis. Sub-findings marked "(Needs verification)" warrant manual confirmation before remediation planning.*
