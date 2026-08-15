# Internal Security Audit — MR 340 / Issue #514 (limit-order placement discount shift)

- **Audit ID:** INTERNAL_KIMIK3_1786830980
- **Date:** 2026-08-16 (epoch 1786830980)
- **Target:** [MR 340](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/340) — `feat/514-limit-order-tier-fees` → `main` @ `35c2a7cc` (closes #514)
- **Auditor:** kimi-k3 lead review + 3 parallel composer-2.5 audit subagents (indexer/DB, frontend/scripts, test coverage/attacks)
- **Scope:** all 29 changed files **plus** downstream consumers found by codebase sweep: indexer event parsers + Postgres schema, factory `Pairs` pagination, cw2/`cw_serde` cross-version behavior, pair discount cache, router, oracle/TWAP surface, frontend fee-display consumers, ops/drift scripts.
- **Method:** full-diff review, surrounding-code reads, macro source verification (`cosmwasm-schema-derive 1.5.11`), targeted test execution (contract unit/integration/migration suites, frontend vitest, drift guard, upgrade-script dry run). No files modified by this audit.

---

## 0. Executive summary

The MR adds optional `limit_discount_bps: Option<u16>` to fee-discount tiers and applies it **only** to maker placement (`floor(effective/2)`); swap and book-take legs stay on `discount_bps`. The core on-chain logic is **sound**: placement has a single entry point, the taker leg is untouched, fail-closed semantics on registry error are preserved, access control and pause gating are unchanged, and the arithmetic is exact (all nine placement targets at 180 bps verified in tests).

**No critical vulnerabilities. No direct fund-loss vector.** The material risks are **operational** (upgrade sequencing can silently strip discounts chain-wide) and **economic** (the maker subsidy opens a self-cross fee undercut and removes the tier-9 anti-spam fee floor), plus **test-coverage gaps** on exactly the paths this MR adds (migration backfill, T9 book-take, placement-time deregister).

| # | Finding | Severity |
|---|---------|----------|
| F-01 | Upgrade script migrates fee-discount **before** pairs → `deny_unknown_fields` parse failure → full-fee window on unmigrated pairs | **High (ops)** |
| F-02 | Pair enumeration unpaginated; `limit: 60` clamped to factory `MAX_LIMIT = 30` → pairs 31+ silently never migrate | **High (ops, if >30 pairs)** |
| F-03 | Migration backfill maps by discount **value**, not tier ID; no pre-migrate state verification | Medium |
| F-04 | Backfill-from-legacy path, T9 book-take, placement deregister parity and more are **untested** (10 gaps) | Medium |
| F-05 | Self-crossing the book is now strictly cheaper than a pool swap at every registered tier (T9: 5 vs 9 bps) | Medium (tokenomic) |
| F-06 | 0-bps tier-9 placement removes the dust-spam fee floor; 300 s cache window amplifies | Low |
| F-07 | Indexer merged-event attr collision can stamp placement limit-fee onto swap rows' `effective_fee_bps` | Low (pre-existing, amplified) |
| F-08 | `UpdateTier` can never clear `limit_discount_bps` back to `None`; ladder not enforced (any limit ≤ 10000) | Low |
| F-09 | Frontend can understate maker fee during a partial-upgrade window (UI 0 % vs chain full fee) | Low (display) |
| F-10 | Frontend discount cache `staleTime: 60 s` lags tier changes; expected-receive optimistic while loading | Low (display) |
| F-11 | Drift guard only spot-checks one Rust shift arm (`9_500 => 10_000`); fixtures omit `limit_discount_bps` | Low (tooling) |
| F-12 | No automated post-migrate verification in the upgrade script (manual echo only) | Low (ops) |
| F-13 | `eval` on `.env.local`; `update_tier` emits no limit attribute; `CONTRACT_VERSION` decoupled from Cargo version | Low |
| F-14 | **Pre-existing on main:** `migration_tests` factory constant `1.6.0` vs shipped `1.7.0` → suite red (432/433) | Low (hygiene) |
| F-15 | Indexer `total_fees_paid` never counts maker placement fees (analytics gap, more visible at T9 = 0) | Info |

**Tests executed for this audit:** contract lib suite 432/433 (only failure is F-14, pre-existing), dex-common fee_discount 3/3, `#514` integration 1/1, fee_discount suite 27/27, limit_order suite 90/90, migration 3/4 (F-14), frontend vitest 4/4, docs drift guard PASS, upgrade script `bash -n` + DRY_RUN PASS.

---

## 1. Scope map — areas identified by the codebase sweep

| Area | Why in scope |
|---|---|
| `fee-discount` contract (`contract.rs`, `msg.rs`, `state.rs`) | New field, validation, migration backfill, access control |
| `pair` contract (`contract.rs` 1.11→1.12, `discount_cache.rs`, `limit_placement.rs`) | Swap/limit fee-leg split, 300 s discount cache, event attribute semantics |
| `dex-common::fee_discount` | Shift table, fee math helpers, shared serde shapes |
| Cross-version serde (`cw_serde` = `deny_unknown_fields`, verified in cosmwasm-schema-derive 1.5.11 vendored source) | Old pair vs new registry parse failure; new pair vs old registry tolerated |
| Factory `Pairs` query (`calc_limit` → `MAX_LIMIT = 30`, `start_after` = asset-infos key) | Upgrade script pair enumeration |
| Indexer (Rust) + Postgres | `effective_fee_bps` attr semantics change, `GetDiscount` parsing, SQL safety, analytics |
| Frontend (`limitOrderFeeSummary.ts`, `useLimitOrderMakerFeeRates.ts`, `TiersPage.tsx`, `limitOrderExpectedReceive.ts`, consumers) | Display correctness, stale cache, fallback semantics |
| Ops scripts (`upgrade-514-limit-discount.sh`, `deploy-dex-local.sh`, `mainnet-soft-launch-defaults.sh`, `check_fee_discount_tier_docs.py`) | Upgrade order/pagination/auth, drift coverage, `eval` |
| Oracle / TWAP (`OBSERVATIONS`) | Reviewed — no interaction (§6) |
| Tokenomics | Maker subsidy, self-cross undercut, dust griefing, treasury revenue shift |
| Router contract | Reviewed — no `DiscountResponse` use; `trader` forwarding unchanged |

---

## 2. Confirmed-safe properties (verified by code reads + tests)

- **Single placement path.** Only `Cw20HookMsg::PlaceLimitOrderBatch` / `PlaceLimitOrderLadder` exist; ladder expands into the batch. `limit_placement.rs:120-127` selects the limit leg; `contract.rs:1039-1104` (swap + book fills) still uses the swap leg via `effective_fee_bps_with_deregister_msgs`. Taker half on fills = taker's swap `discount_bps` — unchanged.
- **Fail-closed preserved (I10/P5).** Registry query error → `(fee_bps, None)` → full fee on both legs (`discount_cache.rs:151`); swaps never revert on registry failure.
- **Deregister parity.** The rewritten `effective_fee_bps_with_discount_msgs` reproduces the swap path's `DeregisterWallet` + cache-invalidation behavior exactly (`contract.rs:155-166` mirrors `:90-102`); `needs_deregister` placements pay full fee.
- **Cache correctness.** Cache stores swap `effective_fee_bps` + the full `DiscountResponse`; the limit leg is re-derived from the cached response on every hit, so no stale/live leg mixing. Pre-upgrade cache entries deserialize (missing `Option` field → `None` → swap fallback). `needs_deregister` entries are never cached/served (`discount_cache.rs:65-67`, `:94-95`).
- **Zero-fee batch safety.** At `total_maker_fee == 0` the treasury CW20 transfer message is skipped entirely (`limit_placement.rs:280-289`) — no zero-amount transfer. `LimitOrderMakerFeeExceedsAmount` guard intact.
- **Access control unchanged.** `AddTier`/`UpdateTier` governance-only; both fields validated `<= 10000` (`fee-discount/contract.rs:141-144`, `:190-197`). EOA-only self-register, governance-tier lock, community deregister balance check — all untouched. Placement is pause-gated via `ExecuteMsg::Receive` → `assert_not_paused` (`contract.rs:626-627`).
- **Math exact.** `u32` intermediates (no u16 overflow); ladder verified: 81/72/58/45/36/22/13/4/**0** bps placement at 180 bps base (dex-common `placement_targets_at_180_bps_pair`).
- **Frontend formula parity.** `resolveLimitDiscountBps` fallback/clamp matches on-chain `resolve_limit_discount_bps`; all `useLimitOrderMakerFeeRates` consumers are limit-order contexts (Trade ticket limit tab, `/limits`, pre-submit summary, expected-receive); swap/pool pages use the swap discount through a different hook.
- **Indexer parsing robust.** No typed `DiscountResponse` (loose `serde_json::Value` reads of `discount_bps`) → the new field cannot break indexing. Placement events are not parsed for fees at all (`parser.rs:924-963` reads id/side/owner/price/expiry only). Dynamic SQL uses whitelists + bound parameters; injection test suite exists (`indexer/tests/security.rs`). No DB schema change required.
- **Route solver unaffected.** `/route/solve` simulates taker legs using swap `discount_bps` (chain query with `tier_discount_bps` fallback); correct post-MR.
- **Router unaffected.** Forwards `trader` strings only; never deserializes `DiscountResponse`.

---

## 3. Findings

### F-01 — HIGH (ops): upgrade migrates fee-discount before pairs → chain-wide full-fee window

`cw_serde` emits `#[serde(deny_unknown_fields)]` on structs (verified in the vendored `cosmwasm-schema-derive-1.5.11/src/cw_serde.rs`). After the 1.1.0 registry migrate, `DiscountResponse` always includes `limit_discount_bps` (`fee-discount/contract.rs:119-126`). A pre-1.12.0 pair's `dex_common::fee_discount::DiscountResponse` lacks the field → smart-query deserialization **fails** → `lookup_effective_fee_bps_cached` returns `(fee_bps, None)` (`discount_cache.rs:136-152`): swaps **and** placements on that pair execute at full pair fee, and deregister side-effects stop. The script's step order is [3] fee-discount, [4] pairs (`upgrade-514-limit-discount.sh:129-147`), so the window covers every pair until its migrate lands — indefinitely for any pair that fails mid-loop. Swaps don't revert (fail-closed), but discounted users are silently **overcharged** (e.g. T9 swap: 180 bps instead of 9) and the indexer/dApp give no on-chain signal beyond missing discount attributes. The repo already documents this exact hazard pattern for wrap-mapper (W13, `dex-common/src/wrap_mapper.rs:66-71`).

**Fix:** migrate **pairs first** (1.12.0 tolerates the old registry: missing `Option` field → `None` → old behavior), then fee-discount. Zero-failure window that way.

### F-02 — HIGH (ops, if >30 pairs): pair enumeration is unpaginated and over-limit

Factory `query_pairs` clamps `limit` via `calc_limit` to `MAX_LIMIT = 30` (`dex-common/src/pagination.rs:1-6`, `factory/src/contract.rs:1016-1041`); `PairsResponse` has no `has_more`. The script queries `{"pairs":{"start_after":null,"limit":60}}` once and migrates the returned page (`upgrade-514-limit-discount.sh:138-147`). With > 30 pairs, the tail is silently skipped and — per F-01 — left charging full fees against the migrated registry. `start_after` is an asset-info pair, not an index, so the loop must feed the last page's `asset_infos`. (Same pattern exists in `rotate-fee-treasury.sh:180-188`.) Fix: page with `limit: 30` until a short page, and assert migrated count == `GetPairCount`.

### F-03 — MEDIUM: migration backfill keys on discount value, not tier ID

`backfill_standard_limit_discounts` gates on tier **ID** (`is_standard_ladder_tier`: 0, 1-9, 255) but computes the value from the tier's current `discount_bps` (`fee-discount/contract.rs:614-629`). If governance ever ran `UpdateTier` moving a standard tier off its canonical value (e.g. tier 3 → `discount_bps: 9500`), migration grants that tier `limit_discount_bps = 10000` — free placement for a mid tier. The upgrade script has **no pre-migrate `GetTiers` assertion**. Recommended: backfill from a tier-ID → limit-value table (the canonical ladder), or assert canonical `discount_bps` on-chain before broadcasting the migrate. Related (accepted but undocumented in tests): the ladder is not enforced — governance can set any `limit_discount_bps ≤ 10000`, including below the swap discount; a one-line doc note in `AGENTS_FEE_DISCOUNT_TIERS.md` would do.

### F-04 — MEDIUM: the paths this MR adds are largely untested (gap matrix)

Verified against the test tree (see §4 for the full inventory). Notably absent:

| Gap | Risk if regressed |
|---|---|
| (a) T9 **book-take** pays taker-half of **9 bps** swap effective, not 0 — the 514 integration test uses a *pool-only* hybrid swap; the only book-fill discount test uses tier 1 | Fill path silently adopting the limit discount would zero T9 taker fees — the exact thing the MR promises not to do |
| (b) `limit_discount_bps: None` custom tier → placement == swap discount (integration) | Silent behavior change for custom tiers |
| (c) Backfill from **legacy** storage (field `None`) — migration tests build tiers via the *new* `AddTier` with the field already `Some(..)`, so the backfill loop is a no-op in every run; `limit_discount_backfilled` attr never asserted; custom tier 42 skip untested | The migration's only state mutation ships untested |
| (d) Placement with `needs_deregister` (balance dropped): full maker fee + `DeregisterWallet` on the placement path | Stale-discount leak on placements |
| (e) Cache entry written pre-migrate (old serialized response) served post-migrate; TTL-window placement after balance drop | Cache/TTL regression |
| (f) `limit_discount_bps: 10001` rejected on both `AddTier` and `UpdateTier` (only `discount_bps: 10001` is tested) | Validation bypass |
| (g) `LimitOrderMakerFeeExceedsAmount` on the limit-effective path | Edge revert |
| (h) `UpdateTier` cannot clear the field to `None` (code-only; intended?) | Governance surprise |
| (i) Mixed `Some`/`None` registry via `GetTiers`/`GetDiscount` | Query-shape regression |
| (j) T9 batch: `total_maker_fee == 0` → **no** treasury transfer message (code guard exists; untested) | Zero-amount transfer revert risk if refactored |

Fuzz/proptest (`fee_math_property_tests`) covers only the swap-leg `effective_fee_bps`; limit placement math is not fuzzed.

### F-05 — MEDIUM (tokenomic): self-cross undercuts the pool fee at every registered tier

Maker half is charged at placement; taker half = `eff − floor(eff/2)` of the **taker's** swap discount. Pre-MR the halves summed to the full effective fee (book ≈ pool, neutral). Post-MR the maker leg uses the deeper limit discount, so the sum is strictly below the pool fee for registered tiers (180 bps base): T8 self-cross = 4 + 14 = **18** vs 27 pool; T9 = 0 + 5 = **5** vs 9 pool. Any tier holder (or two cooperating wallets) can execute below the "unchanged" swap fee by resting then taking — on a thin book, interference risk is low. This erodes the MR's "swap/take unchanged" framing and shifts treasury revenue. If intentional (maker subsidy), document it as an explicit design decision in `fee-discount-tiers.md`; if not, floor `maker + taker` at the swap effective fee per tier.

### F-06 — LOW: 0-bps tier-9 placement removes the dust-spam fee floor

Placement requires only non-zero amount, valid price, future expiry (`limit_placement.rs:317-334`). At `limit_discount_bps = 10000`, a T9 wallet places arbitrary `amount = 1` dust orders at gas-only cost. Mitigations exist — bounded match walks (`max_makers`), expiry + `clean_limit_book`, dust parks (#504), blacklisting (#468) — and 7,500 CL8Y is a real bond, but the per-order fee deterrent is gone for the top tier; book bloat raises taker gas (bounded) and indexer/storage load. Amplifier: the accepted 300 s cache window (I9) now yields **free** placement for up to one window after a balance drop (previously 4 bps). Consider a minimum order size if spam is observed.

### F-07 — LOW: indexer merged-event attribute collision (pre-existing, amplified)

Swap parsing reads `wasm_attr_last(attrs, "effective_fee_bps")` over the **whole** wasm event (`indexer/src/indexer/parser.rs:31-36`, `:605-606`). If a tx merges swap + placement attrs with placement **after** the swap action, the swap row stores the placement's limit-effective value (T9: 0) instead of the swap's (9). Pre-MR the two were equal, so this was latent; now it's observable in `/pairs/{addr}/trades` and CSV exports. Informational field only — `commission_amount`, volume, and PnL math are unaffected. Fix: scope the attr to the swap segment (the placement/fill parsers already segment).

### F-08 — LOW: `UpdateTier` cannot clear `limit_discount_bps`; ladder unconstrained

`execute_update_tier` only ever sets `Some(bps)` (`fee-discount/contract.rs:194-197`); reverting a tier to "follow the swap discount" requires remove+add. Combined with no ordering constraint between `discount_bps` and `limit_discount_bps`, tier management is all-or-nothing. Governance-footgun class; document or add a clear semantic (e.g. sentinel) if desired.

### F-09 — LOW (display): frontend understates maker fee during a partial-upgrade window

The dApp queries the registry directly and applies #514 math. If the registry is migrated while a pair is not (the F-01 window), that pair fails closed to full fee while the UI shows e.g. T9 = 0 % maker. Users are overcharged relative to the display (never undercharged). Resolves when F-01's ordering is fixed; otherwise the UI has no pair-code-version awareness.

### F-10 — LOW (display): 60 s react-query stale time after register/deregister; optimistic receive while loading

`useLimitOrderMakerFeeRates` uses `staleTime: 60_000` on fee config and discount; tier register/deregister invalidates only the registration key (`TiersPage.tsx:185`), so maker-fee copy can lag up to ~60 s. Separately, `limitOrderExpectedReceiveHuman` treats `effectiveFeeBps: null` as **0 maker fee** (`limitOrderExpectedReceive.ts:35-36`), briefly inflating expected receive while queries load. Display-only; execution uses chain math.

### F-11 — LOW (tooling): drift guard spot-checks the Rust shift table

`check_fee_discount_tier_docs.py` validates docs/deploy scripts against its own `_LIMIT_SHIFT`, but the Rust side is checked by a single substring (`9_500 => 10_000`). A divergence on any other arm (e.g. `8_500 => 9_000`) would pass. `tier_fixtures.rs` rows omit `limit_discount_bps` entirely. Fix: parse all nine arms from `dex-common/src/fee_discount.rs` and add the field to fixtures.

### F-12 — LOW (ops): no automated post-migrate verification

The script ends with a manual "Confirm GetTiers…" echo. No `GetTier(9)` assertion, no pair-count reconciliation, no T9 placement smoke. Given F-01/F-02/F-03, the script should self-verify before printing success.

### F-13 — LOW: minor hardening nits

- `eval "$(grep … | sed …)"` on `frontend-dapp/.env.local` (`UPGRADE514_LOCAL=1` path, `upgrade-514-limit-discount.sh:31-34`): only three `VITE_*` keys survive the filter and the file is operator-controlled, but `$(…)` in a value would execute. Same pattern in `upgrade-518-lp-symbol.sh:42-44`.
- `update_tier` emits only `tier_id` — off-chain monitors must re-query to learn the new limit value (add_tier emits it).
- Fee-discount `CONTRACT_VERSION` is now hardcoded `"1.1.0"` while the workspace package version stays `1.0.0` (`smartcontracts/Cargo.toml:17`) — was `env!("CARGO_PKG_VERSION")`; intentional but a release-process hazard.
- `InvalidDiscountBps` message ("exceeds maximum of 10000") doesn't say whether `discount_bps` or `limit_discount_bps` failed.
- `wasm migrate` requires the signing key (`TERRAD_HOST_KEY`, default `cl8ydeploy`) to be the **admin** of the registry and every pair; the script doesn't pre-check admin — failures surface per-tx.

### F-14 — LOW (hygiene, pre-existing on main): stale factory version constant

`migration_tests.rs:25` asserts factory `1.6.0`; factory shipped `1.7.0` in `ca94d12` (main). Full lib suite on the MR tree: **432 passed / 1 failed** — the failure is this constant, **not** MR 340 (whose own version constants were updated correctly). `make verify-issue-514` doesn't run that test, but the red suite on main can mask future migration regressions.

### F-15 — INFO: indexer analytics never count maker placement fees

`total_fees_paid` accumulates swap `spread + commission` only (`position_tracker.rs:37-38`); placements/fills carry no maker-fee ingestion (schema has no columns for it). Pre-existing gap; with T9 placement at 0 the omission is more visible to analytics consumers. Also informational: `place_limit_order*` `effective_fee_bps` now carries the **limit** effective while the same key on swap events carries the swap effective (documented in I13) — integrators keying on the attribute name without scoping by `action` will misread one of the two.

---

## 4. Test coverage inventory

| Location | Test | #514 behavior |
|---|---|---|
| `dex-common/src/fee_discount.rs:130-166` | 3 unit tests | Shift table incl. T9 9500→10000; 180 bps targets; resolve fallback |
| `tests/src/limit_order_tests.rs:6834+` | `limit_placement_shifted_discount_swap_fee_unchanged_514` | T9 place 0 (`remaining == escrow`), **pool** swap eff 9, unregistered place 90 |
| `tests/src/limit_order_tests.rs:584-720` | `hybrid_book_fill_uses_taker_discounted_effective_fee_bps` | Book-take uses swap discount — but tier 1, not T9 |
| `tests/src/lib.rs:2963-2994` | `test_query_tiers` | GetTiers shows limit column (T1=1000, T9=10000) |
| `tests/src/lib.rs` (fee_discount suites, 27 tests) | add/update/remove/register/deregister/trusted-router | Regression-safe with new optional field (`None` cases) |
| `tests/src/migration_tests.rs:630-661` | `fee_discount_migration_preserves_*` | Version bump + state preservation; **backfill not exercised** (tiers pre-populated) |
| `frontend limitOrderFeeSummary.test.ts` | 4 vitest cases | `resolveLimitDiscountBps` preference/null/undefined/clamp; T9 place 0 |
| `scripts/qa/verify-issue-514.sh` | orchestration + retest | unit + integration + docs + frontend + upgrade DRY_RUN |
| `scripts/check_fee_discount_tier_docs.py` | drift guard | 11 rows aligned across 4 sources (limit column enforced) |

**E2E:** no live-chain rung (LocalTerra store→migrate→`GetTiers`→T9-place-0 assertion) — matches repo convention for prior upgrades (#518), but recommended here given F-01..F-03. The MR test plan's on-chain items are manual checkboxes.

---

## 5. Test execution evidence (run during this audit)

| Suite | Result |
|---|---|
| `python3 scripts/check_fee_discount_tier_docs.py` | PASS — 11 tiers aligned |
| `cargo test -p dex-common fee_discount` | PASS 3/3 |
| `cargo test -p cl8y-dex-fee-discount` | PASS |
| `cargo test -p cl8y-dex-tests --lib limit_placement_shifted_discount_swap_fee_unchanged_514` | PASS 1/1 |
| `cargo test -p cl8y-dex-tests --lib fee_discount` | PASS 27/27 |
| `cargo test -p cl8y-dex-tests --lib limit_order` | PASS 90/90 |
| `cargo test -p cl8y-dex-tests --lib migration` | 3/4 — fail is F-14 (pre-existing) |
| Full `cargo test -p cl8y-dex-tests --lib` | 432/433 (F-14) |
| `vitest run limitOrderFeeSummary.test.ts` | PASS 4/4 |
| `upgrade-514-limit-discount.sh` `bash -n` + DRY_RUN (skip store/migrate) | PASS (structure only) |

---

## 6. Attack checklists

### 6.1 Common smart-contract attacks

| Vector | Assessment |
|---|---|
| Reentrancy | N/A (CosmWasm actor model); no new submessage flows — treasury transfer pattern unchanged |
| Integer overflow/underflow | Safe: `u32` intermediates, `checked_*` in the placement loop, `saturating_sub` in fee math; `effective_fee_bps ≤ fee_bps ≤ 10000` ⇒ `maker ≤ 5000 < 10000`, so `maker_fee ≥ amount` only via the explicit guard |
| Access control / privilege escalation | Unchanged; governance-gated field with ≤ 10000 validation on add **and** update; EOA-only register; governance-tier lock; community deregister balance check intact |
| Migration/state corruption | cw2 version gate correct; backfill idempotent (skips `is_some()`); tier-map iteration is governance-bounded — but see F-03/F-04(c) |
| Cross-version msg compatibility | New pair ↔ old registry OK; old pair ↔ new registry **fails closed** (F-01); old pair cache entries deserialize post-migrate |
| DoS / gas | Bounded loops everywhere in the diff; F-06 covers book bloat; F-02 covers the ops-side truncation |
| Front-running / MEV | No ordering assumptions changed; placement and match paths untouched apart from the fee leg |
| Event/attribute spoofing | Indexer scopes by `_contract_address` (forge-resistant, #285); F-07 is the adjacent residual |

### 6.2 Common DeFi / economic attacks

| Vector | Assessment |
|---|---|
| Fee evasion (self-cross) | **New channel** — F-05 |
| Dust/griefing | Fee floor removed at T9 — F-06 (bounded) |
| Treasury revenue | Intentional maker-half loss at shifted tiers; taker half unchanged; quantified in F-05 table |
| Cache-window abuse (I9) | Accepted 300 s snapshot; amplification at 0 bps noted in F-06 |
| Sandwich / slippage | Unaffected (`belief_price`/`max_spread`/`min_return`/deadline untouched) |
| LP/share inflation | Untouched (P3 gates intact) |

### 6.3 Oracle manipulation

The MR touches no oracle code. The pair TWAP (`OBSERVATIONS`) accumulates only on swap/liquidity events; placement does not observe, and fees are taken from output, not reserves. Second-order effect: cheaper placement deepens the book, shifting hybrid volume toward maker-priced fills, which if anything reduces pool price impact. **No new oracle-manipulation surface.**

### 6.4 Database / indexer (Rust server)

| Vector | Assessment |
|---|---|
| SQL injection | Clean: whitelisted sort/interval columns, bound parameters; security test suite exists (`indexer/tests/security.rs`) |
| Data leaks via API | No endpoint exposes `limit_discount_bps`/tier ladder; error responses omit DB internals |
| Parsing breakage from new field | None: indexer reads `GetDiscount` via `serde_json::Value`; placement events not parsed for fees |
| Accounting drift | F-07 (merged-event attr collision), F-15 (maker fees uncounted) — both informational/analytics, not fund-affecting |

---

## 7. Recommendations (priority order)

1. **Block mainnet run until fixed:** reorder `upgrade-514-limit-discount.sh` — migrate **pairs first**, then fee-discount (F-01); add the `limit: 30` pagination loop with `start_after = last asset_infos` and reconcile against `GetPairCount` (F-02).
2. Add pre-migrate `GetTiers` assertion of canonical `discount_bps` values, and post-migrate assertions (tier 9 `limit_discount_bps == 10000`; simulated T9 placement maker fee 0) (F-03, F-12).
3. Add the F-04 tests — highest value first: (a) T9 book-take fee, (c) legacy-`None` backfill incl. custom-tier skip, (d) placement `needs_deregister` parity, (f) `limit_discount_bps: 10001` rejection, (j) zero-fee batch has no treasury message. Extend `fee_math_property_tests` to the placement leg.
4. Decide and document the self-cross economics (F-05); if unintended, enforce `maker + taker ≥ swap effective` per tier.
5. Scope indexer swap `effective_fee_bps` parsing to the swap segment (F-07).
6. Harden the drift guard to diff **all** shift arms against the Rust source; add `limit_discount_bps` to `tier_fixtures.rs` (F-11).
7. Fix the stale `FACTORY_VERSION` constant on main (F-14).
8. Nits: emit `limit_discount_bps` on `update_tier`; name the field in `InvalidDiscountBps`; replace the `.env.local` `eval` with `read`-based parsing; consider a `clear_limit_discount_bps` affordance (F-08); document the indexer route-solve = swap-leg-only caveat for integrators (F-15); add a LocalTerra live-migration rung to `verify-issue-514`.

---

## 8. Audit verdict

**Approve with required changes to the upgrade script (F-01, F-02) before any mainnet execution, and strongly recommended test additions (F-04) plus a documented decision on self-cross economics (F-05).** The contract code itself is correct, conservative (fail-closed), and backward compatible in the safe direction (new pair vs old registry). Everything user-facing in the frontend and indexer degrades toward over-charging or stale display rather than under-charging, and no fund-loss or privilege-escalation vector was identified.
