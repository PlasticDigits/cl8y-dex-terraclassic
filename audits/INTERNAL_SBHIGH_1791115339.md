# Internal Security Audit (HIGH focus) — cl8y-dex-terraclassic

| | |
|---|---|
| **Audit ID** | `INTERNAL_SBHIGH_1791115339` |
| **Date** | 2026-10-04 (UTC) |
| **Commit** | `748579ab` (`main`) |
| **Type** | Internal, read-only, defense-in-depth review (no code changes) |
| **Primary ask** | Full-stack security audit (test coverage, DeFi / smart-contract / DB attack testing, e2e, happy + bad paths, missing security features, access control, privileges, database, Rust server, contract design, tokenomics/economic attacks, oracle manipulation) **focused on recently merged PRs whose smart-contract code is queued for columbus-5 migration**, followed by an **exploit-chaining** investigation over in-repo, prior-audit and out-of-repo findings |
| **Scope (contracts, migration targets)** | **#1383 / `ef873dcc` "fix(swap): fund token taxes from gross route budgets"** (router 1.2.0, community-tax-token 1.2.0, `dex-common::route_tax` — issues #1381/#1384), **#1378 / `045812ab` "fix(swap): size receive targets through bounded route solver"** (pair 1.19.0 — issues #1376/#1386), plus the supporting indexer/frontend surfaces introduced by those PRs, plus re-verification of still-active prior findings |
| **Method** | Codebase surface sweep (new/changed files since the previous audit) → targeted source review of the router, community-tax-token, pair reverse math, factory, bridge-debt, hooks, indexer API/DB layers → executed test suites → exploit-chaining analysis (§14) |
| **Test runs executed** | `smartcontracts: cargo test --workspace` → **884 passed / 0 failed**; `indexer: cargo test --lib` → **469 passed / 1 FAILED, reproduced flaky 2-of-3 runs** |
| **Prior audits treated as still-active unless re-verified** | `INTERNAL_KIMIK3_1785897304`, `INTERNAL_KIMIK3_1786830980`, `INTERNAL_KIMIK3_1787230030`, `INTERNAL_KIMIK3_1787468843`, `INTERNAL_GROK46_1787908099`, `INTERNAL_KIMIK3_1790149457`, `INTERNAL_KIMIK3_1791113518` |

**Honesty statement.** This is an internal engineering review, not a third-party audit. Every finding
below is backed by a file/line reference or a reproduced test run. Items marked *Unverified* require a
signed-transaction or live-chain check that was not performed here. No PASS evidence is asserted for
checks that were not actually executed; where a prior report claims a green suite and this run
disagrees, the disagreement is recorded as a finding.

---

## 0. Additional surfaces identified in this sweep

| Surface | Why it matters | Status |
|---|---|---|
| `smartcontracts/packages/dex-common/src/route_tax.rs` (new, 127 lines) | Shared amount math (`sell_from_gross` / `buy_from_gross`) now on the consensus path for **every** taxed router hop | Reviewed → §3.1 |
| `smartcontracts/contracts/router/src/route_tax.rs` (new, 272 lines) | Token capability discovery, preview validation, funded hybrid partition, reverse helpers, `RouteTransfer` emission | Reviewed → §3.1 |
| Router `swap_state.tax_context` / `refunds` (new fields) | New router→token authentication surface consumed by the token | Reviewed → §3.1 |
| `community-tax-token`: `RouteTaxCapability` / `RouteTaxPreview` / `RouteTransfer` + `acquired_tokens` cooldown suppression | New execute/query surface on a **live, already-deployed** token contract | Reviewed → §3.1 |
| `pair/src/hybrid_reverse.rs` (`min_pool_input_for_net_output`, `bounded_hybrid_reverse_search`, `pool_commission`) | Exact-output pool inversion on the pair; also now used by **execute** fee math (u128 overflow fix) | Reviewed → §3.2 |
| `indexer/src/api/route_exact_output.rs` (672 lines) + `route_exact_output_reference.rs` (397) | New public solver endpoints; fresh LCD validation required | Reviewed → §4 |
| `indexer/src/api/trading_tax_discovery.rs` (251 lines) | New tax-capability attestation surface (code-id/data-hash pinned) | Reviewed → §4 |
| `indexer/src/db/queries/protocol_liquidity.rs` (dynamic SQL, #1380/#1382) | `format!`-built SQL — the only such site in the repo; verified constant-only | Reviewed → §4.3 |
| `indexer/src/indexer/pair_discovery.rs` negative cache | Process-global 1 h / 10 000-entry reject cache; **flaky test** + pollution surface | Reviewed → §4.1 |
| `bridge-debt` / `bridge-debt-token` (`8f9dd98e`, live) | Two brand-new contracts, already used for a 10,000-unit issuance | Reviewed → §3.4 |
| Router `ReverseSimulateSwapOperations` → `reverse_greedy_offer` | Unbounded nested simulation loop in a public, unauthenticated, browser-reachable query | Reviewed → §3.1 (SBHIGH-03) |

**Web-research additions applied to this code (2026 landscape, carried forward and re-verified):**

- **Nostra ($3.5M, 2026-09-17)** — attacker seeded a thin pool, wash-traded it, and hijacked
  *third-party pool-selection logic* so the rigged pool became the price reference. Maps to this
  repo's own GeckoTerminal adapter (`/gt/`), `indexer/src/indexer/hub_usd.rs` largest-liquidity-pool
  selection, and every downstream consumer of either. → §7.3, Chain S.
- **Moonwell/MAMO ($8.7M, 2026-08-27)** — thin-token spot accepted as collateral; **no code change
  required**. Maps to any external lending/credit consumer of this DEX's spot or hub marks and to
  the out-of-repo UST1 window reading hub marks. → Chain S/K.
- **Spiral SpiralHookV2 ($26.8K, 2026-09-13)** — same-block guard keyed on `tx.origin`, bypassed with
  6 EOAs. Maps to the per-wallet launch-guard cooldown in `community-tax-token`, which is
  Sybil-bypassable by construction and additionally suppressible (SBHIGH-05). → §3.1, Chain C.
- **CWA-2024-002 (Uint256 `pow`/negation wrapping)** — repo pins `cosmwasm-std 1.5.11`; re-verified
  not affected. The new TWAP/pool math uses `checked_*` and explicit `Uint256` intermediates.

---

## 1. Executive summary

The recently merged, migration-queued contract work is **structurally sound and, in several places,
materially better than the code it replaces**: taxes are now funded from the transaction's own swap
budget instead of the trader's wallet; execute authenticates the router through live on-chain state
rather than a caller-supplied hook field; pool inverse math is exact integer inversion with 256-bit
intermediates; and the indexer fails closed on any token whose tax capability it cannot attest.

The **most serious issues found are not in the swap math — they are lifecycle and resource
governance issues that the columbus-5 migration will expose:**

1. **SBHIGH-01 (High, migration-blocking):** `CONFIG.router` in `community-tax-token` is
   **write-once** (instantiate / foreign-cw2 adopt only). The same-crate `migrate` path cannot change
   it and `adopt` is explicitly rejected for same-crate tokens. Therefore, if the columbus-5 rollout
   deploys the route-tax v1 router at a **new contract address**, every already-adopted
   community-tax token permanently rejects v1 routing (`preview` → *"token does not trust this
   route-local router"*) with **no on-chain recovery path**. The runbook
   (`docs/runbooks/route-local-tax-1381.md` §"Coordinated rollout" step 3) does not state the
   invariant that the router must be migrated **in place at the existing address**.
2. **SBHIGH-03 (Medium-High):** the router's public reverse query performs up to ~257 nested
   `HybridSimulation` queries **per hop** (≈1000 per request over 4 hops), each of which walks the
   capped limit book. It is unauthenticated, unrated at the contract level, and directly reachable
   from a browser (and from any LCD client). The pair's own reverse path caps at 32; the router path
   does not. This is a cheap, capital-free public DoS with a direct MEV payoff (Chain B).
3. **SBHIGH-02 (Medium):** `RouteTransfer` settlement allowances are matched but **never consumed**,
   so an authorized (and, after SBHIGH-01, an *old, still-pinned*) router can replay a settlement
   until its token balance is drained.
4. **SBHIGH-11 (Medium, measured):** the indexer unit suite is **nondeterministic** — reproduced
   2 failures in 3 runs — because two tests share a process-global negative cache and the same
   fixture address. Both `test-indexer-lib` and `test-indexer-integration` run it, so merge-gate
   status and every "N indexer tests pass" receipt are unreliable.
5. **SBHIGH-09 (Low-Medium):** the declared-book reverse path still dies at the 32-simulation cap for
   large targets, i.e. exact-output sizing is unavailable precisely for the deepest liquidity —
   pushing users toward unbounded-input swaps, which are the sandwichable shape (Chain C).

No fund-draining, arbitrary-mint, or auth-bypass defect was found in the migrated consensus path.
The conservation algebra of route-local funding v1 was re-derived and holds, including at the
`Uint128` boundary; the router re-previews at execute time, so a mid-flight tax-rate change causes a
**revert**, not an under-fill (§3.1.6 — verified negative).

---

## 2. Test coverage — measured this pass

| Suite | Command | Result | Notes |
|---|---|---|---|
| Smart-contract workspace | `cd smartcontracts && cargo test --workspace` | **884 passed / 0 failed / 0 ignored** | Includes `route_tax_1381_tests.rs` (758 lines), `reverse_issue1376_tests` (real pair query, both orientations), `adversarial_token.rs`, `cw20_mutants.rs`, `asset_code_id_pin_tests.rs`, `limit_order_tests.rs`, `blacklist_tests.rs`, `migration_tests.rs` |
| Indexer unit | `cd indexer && cargo test --lib` | **469 passed / 1 FAILED** (`indexer::pair_discovery::tests::discover_new_pair_negative_caches_unlisted_pair`) | Flaky, see SBHIGH-11; reproduced 2/3 runs |
| Indexer Postgres integration | `cargo test --tests -j1 -- --test-threads=1` | **Not run** (no Postgres in this environment) | Prior receipts claim 55 route/exact-output/funding + 43 liquidity tests |
| Frontend unit | `npm run test:run` | **Not run** this pass | 357 test files present; prior receipt claims 268 focused tax + 2,825 suite lineage |
| Playwright e2e | `frontend-dapp/e2e/*.spec.ts` | **Not run**; 70 specs exist | `swap-receive-1376-tx.spec.ts`, `protocol-liquidity-quality.spec.ts` are new |
| E2E in CI | `.gitlab-ci.yml` | **Absent** | Only tracked under #421; no merge gate |

### 2.1 Coverage gaps identified (not covered by any test in the repo)

| Gap | Risk | Related finding |
|---|---|---|
| No test asserts the **router's economic split** on a repeated-token route (`A→B→A`, same pair twice) | Cooldown suppression + balance-delta accounting on a self-pair route are unexercised | SBHIGH-05 |
| No test asserts `RouteTransfer` **replay** behaviour (two identical settlements in one ctx) | Allowance matching is `any()`-based with no consumption | SBHIGH-02 |
| No test bounds the **sim-call count of `reverse_greedy_offer`** | Unbounded nested query loop is untested | SBHIGH-03 |
| No test asserts `CONFIG.router` immutability / post-migration router-trust behaviour | The migration-blocking trap is undocumented in tests | SBHIGH-01 |
| No property/fuzz test that the token's `do_transfer` debit **equals** `sell_from_gross(gross, bps).tax` across a bps × amount grid | Parity between the shared math and the live classifier is asserted only by example | §3.1.2 |
| No test asserts reverse-quote **proportionality assumption** (single probe at 10 000 extrapolated linearly) | Silent reverse misquotes for any non-linear tax | SBHIGH-06 |
| No `cargo test --lib -- --test-threads=1` guard, and no CI retry/quarantine for the flaky test | Random red pipelines train reviewers to ignore the gate | SBHIGH-11 |
| No Playwright job in CI | 70 e2e specs (incl. the new on-chain receive-sizing spec) are advisory only | TST-02 (re-verified) |

---

## 3. Smart-contract findings

### 3.1 Route-local tax funding v1 — #1381/#1383/#1384 (router 1.2.0 + community-tax-token 1.2.0)

#### 3.1.1 Verified-correct properties (recorded so re-reviews do not re-litigate them)

- **Conservation.** For sell gross `G`, `sell_from_gross` returns the maximal `S` with
  `S + floor(S·bps/10⁴) ≤ G`, so `economic + tax ≤ gross` and the integer residue
  `G − economic − tax` is refunded. The live token debits the router `amount + floor(amount·bps/10⁴)`
  where `amount = economic` (`tax.rs:540-548`) — exactly the previewed value. The router holds `G`
  because the user `Send`s the gross budget, so the debit is always fundable; shortfall is a distinct
  `RouteFundingShortfall` error, not a silent under-payment. **Checked at `u128::MAX` in
  `route_tax.rs` unit tests (subtraction-based binary search, no overflow).**
- **Tax charged exactly once per economic event.** Buy tax is levied only at the authenticated
  `pair → router` boundary (`tax.rs:513-520`, requires `ctx.pair == from && ctx.ask_token == self`);
  final delivery and residue refunds use `RouteTransfer` with `TaxKind::Honest`, so no second buy tax
  and no incidental transfer tax. A token bought on hop *i* and sold on hop *i+1* is taxed once on
  each side, as documented.
- **Execute authentication is state-based, not caller-supplied.** `require_hop_trader` /
  `route_context` re-query the router's live `swap_state` and require
  `ctx.trader == hop trader ∧ ctx.pair == to ∧ ctx.offer_token == self ∧ ctx.economic == amount ∧
  amount + tax ≤ ctx.gross`. Supplying `Swap.trader` on a direct pair send establishes nothing.
  Manager/protocol exemptions on the router or pair **cannot** waive another trader's tax
  (`tax.rs:522-536`).
- **Sink redirection is blocked.** Both `route_preview` and the live transfer path reject any
  non-burn sink crediting the router or a listed pair, closing a direct "route tax → attacker wallet
  via AutoLP/sink" route (`tax.rs:718-726`, `tax.rs:551-560`).
- **Fail-closed capability discovery.** `capability()` errors on a version mismatch, and for any
  token that answers a known legacy fee API (`get_config` / `tax_map` / `tax_info`) it refuses to fall
  back to a wallet-funded `Send`; only genuine `cw20-base` / `cw20-mintable` identities (cw2
  `contract_info`) are treated as tax-free (`route_tax.rs:10-59`).
- **Reentrancy.** `SWAP_STATE` is written before the first external send and removed only in the
  settlement reply, so a recipient contract re-entering the router hits `SwapInProgress`.
- **Router storage compatibility.** `SwapState` gained fields without `serde(default)`, but the item
  is never persisted across blocks (created and cleared inside one transaction), so in-place
  migration is storage-safe.

#### SBHIGH-01 — `CONFIG.router` is write-once: a new router address permanently bricks v1 routing for every already-adopted token · **High · migration-blocking · confirmed**

*Evidence.*
- `contract.rs:124-131` — `PROTOCOL_EXEMPT` / `CONFIG.router` written **only** in `instantiate`.
- `contract.rs:728-750` — `migrate()`: same-crate ⇒ `ensure_from_older_version` only; foreign cw2 ⇒
  `adopt`, and `AdoptNotForSameCrate` is returned if `adopt.is_some()` for a same-crate contract.
- `msg.rs` `ExecuteMsg` has **no** `UpdateRouter` / `SetRouter` variant (full enum reviewed:
  `RouteTransfer, Transfer, Burn, Send, IncreaseAllowance, DecreaseAllowance, TransferFrom, SendFrom,
  BurnFrom, Mint, Receive, RegisterListedPair, BindAutolp`).
- `adopt.rs:230-256` — the only other writer, reachable **only** from a foreign cw2 source.

*Failure mode.* `preview()` compares `capability.router` with the calling router
(`route_tax.rs:63-70`) and errors on mismatch. The token is *not* on the old router's allowlist, so
`is_router_sell_hop` is false and the transfer would take the wallet-funded `Sell` debit path
(`classify_trade`, `tax.rs:161-170`) against a router balance that only holds the economic amount —
also fatal, but the `preview` error fires first. Net effect: **every taxed route reverts at hop 0**.

*Why it is not self-healing.* For an already-adopted token the only mutation paths are the paid
invoice (`UpdateSettings`: bps, treasury, sinks, exemptions, autolp, guards, minter — **no router**)
and governance `wasm migrate` with `adopt: null` (version bump only). There is no code path that
rewrites `CONFIG.router` for a same-crate token. Recovery would require migrating the token to a
*foreign* cw2 (rejected by `AdoptNotForSameCrate`) or a bespoke new contract.

*Recommendation (must be satisfied before #1384 executes).*
1. Treat **"the official router keeps its existing contract address"** as a rollout invariant and
   state it explicitly in `docs/runbooks/route-local-tax-1381.md` step 3 and in the #1381 QA
   checklist. Migrate the router **in place** (`cw2 1.1.0 → 1.2.0` at the same address) — the router
   needs no storage migration, so this is safe (§3.1.1).
2. Add a **read-only preflight** to `scripts/qa/probe-route-tax-1381.py`: for every token in the
   migration inventory, assert `route_tax_capability.router == <the router address being rolled out>`
   and abort the rollout on any mismatch. Today the probe would only discover this on the first live
   user trade.
3. Consider adding an owner/governance-gated `UpdateRouter` (with immediate
   `PROTOCOL_EXEMPT` re-stamping and a clear-old-exempt step) so a future router rotation is
   possible at all. If added, it must be invoice- or timelock-gated to avoid becoming a
   trade-blocking griefing vector (it can brick every taxed route).

#### SBHIGH-02 — `RouteTransfer` settlements are not consumed: replayable by any authorized router · **Medium · confirmed**

`tax.rs:766-773` matches the requested `(token, recipient, amount)` against
`ctx.transfers` with `.any(...)` and **writes no state**. The handler then debits `info.sender`
(the router) and credits the recipient. Within one settlement the router issues one `RouteTransfer`
per refund plus one for the final output, and the router is the only permitted caller — so under
current code the reachable loss is bounded by the router's own token balance. That balance is not
necessarily zero: users can send tokens directly to the router address, residue is held between hops,
and future non-reverting partial-failure handling would leave balances behind.

This matters specifically because of SBHIGH-01: after a router migration, the **old** router address
stays authorized in every token forever (it can never be un-stamped), and it remains a
`WasmMsg::Execute` target for governance. A buggy or maliciously-migrated old router can call
`RouteTransfer` in a loop and drain whatever balance it holds, with no on-chain evidence beyond
repeated `route_transfer` attributes.

*Recommendation.* Bind the allowance to a single-use marker (e.g. store a consumed
`(block_height, sequence, token, recipient, amount)` key, or clear `transfers` after the matching
call by writing a modified `RouteTaxContext` back through a router-side callback — note the token
cannot write the router's state, so prefer an idempotency key such as
`ctx.gross + hop index`), and/or add a per-settlement non-repaying nonce derived from the router's
`swap_state` identity. Failing that, document the residual risk and add a monitoring alert on
repeated identical `route_transfer` attributes from a single block.

#### SBHIGH-03 — Unbounded nested simulation in the public router reverse query (query-amplification DoS) · **Medium-High · confirmed**

`route_tax.rs:206-257` `reverse_greedy_offer`:
- exponential search: `for _ in 0..128 { … hi = hi*2 … }` → up to **128** `HybridSimulation` queries,
- plus one confirmation query, plus bisection `while hi-lo>1` → up to **128** more.

Each `HybridSimulation` executes the full greedy book walk (bounded by `max_maker_fills`, but each
maker order is a storage read plus price/size math) against the live pair. `MAX_HOPS = 4`, so a
single `reverse_simulate_swap_operations` query with `greedy` on every hop can trigger **~1,000**
nested smart queries. There is no call budget, no memoisation, and no concurrency guard; the pair's
own reverse path demonstrates the intended control (`MAX_HYBRID_REVERSE_SIM_CALLS = 32`,
`hybrid_reverse.rs:33`).

Reachability: `QueryMsg::ReverseSimulateSwapOperations` is unauthenticated and called directly from
the browser (`frontend-dapp/src/services/terraclassic/router.ts:118`) as well as from any LCD/REST
client. On Terra Classic, LCD smart queries are node-CPU work that is not metered per caller, so this
is a cheap public DoS against the RPC/LCD that the dApp's own quoting depends on.

*Recommendation.* Cap total `HybridSimulation` invocations per `reverse_greedy_offer` call (e.g. 32,
matching the pair), return an explicit *"greedy reverse not available — use pair-direct or exact-output
sizing"* error rather than looping, and short-circuit the common case by seeding the bracket from the
pool math (`seed_upper_offer_from_pool_math`) before any search. Consider an indexer-side
rate limit specifically on reverse-solve calls.

#### SBHIGH-04 — Capability discovery multiplies wasm queries on the execute path · **Low-Medium · confirmed**

`capability()` costs **1** query for a v1 tax token but **up to 5** for an ordinary CW20
(`RouteTaxCapability` → 3 legacy probes → raw `contract_info`). It is invoked:
- in the legacy `ExecuteSwapOperations` gate, **once per asset per hop with no dedup**
  (`router/contract.rs:150-175`) → up to 8 × 5 ≈ **40 queries** for a 4-hop taxed route before any
  swap work begins;
- again inside every `preview()`, again inside `reverse_buy`/`reverse_sell`, and again per
  `transfer()` emission (final output + each refund token).

There is no in-contract cache (CosmWasm has no cross-tx memory), so this is a permanent per-tx cost.
It is not a vulnerability by itself, but it consumes the tx gas budget that the new reverse-sizing
and hybrid guards also draw on, and columbus-5's fee schedule governs wasm query cost. A 4-hop
taxed route that fit before #1383 may no longer fit after it.

*Recommendation.* Deduplicate the legacy-gate capability lookups per address (a small local
`BTreeSet` over `(offer, ask)` per operation list), short-circuit the legacy probes with a single
`contract_info` cw2 identity check *first* (ordinary `cw20-base`/`cw20-mintable` ⇒ no tax APIs by
construction), and measure the gas delta of the worst-case 4-hop taxed route in
`docs/qa/issue-1381/localterra-evidence.json` as an explicit acceptance criterion.

#### SBHIGH-05 — Repeated-token routes suppress the per-wallet launch-guard cooldown · **Low-Medium · confirmed**

`tax.rs:477-500` sets `already_checked = ctx.acquired_tokens.contains(self_addr)` for an authenticated
router sell hop, and `tax.rs:551-560` then passes `None` instead of `Some(trader)` into
`apply_launch_guards`, which **skips `check_cooldown(trader)`**. The router populates
`acquired_tokens` with the ask token of every completed hop (`router/contract.rs:~455-465`) but
**never validates it** — the list is write-only on the router side; only the token reads it.

A route whose offer token was already acquired earlier in the same route (`A→B→A`, which resolves to
the *same* pair twice because the factory key is unordered) therefore sells token B back to the pair
in the same transaction that bought it, with the trader's cooldown check skipped. `trading_enabled`
and `max_wallet` are still enforced, so this is a launch-guard weakening, not a fund-loss path —
but it is exactly the anti-snipe property `cooldown_blocks` exists to provide, and it is Sybil-shaped
in the same way as the Spiral `tx.origin` incident (§0).

*Recommendation.* Either (a) have the router reject any hop whose `ask_token ==` a previously
acquired token (making `A→B→A` unrepresentable and `acquired_tokens` self-justifying), or (b) keep
the cooldown check unconditional for the authenticated trader and reserve `already_checked` for
`max_wallet`/trading-enabled only. Add the missing regression test (§2.1).

#### SBHIGH-06 — Reverse helpers assume strictly proportional tax from a single probe · **Low-Medium · confirmed (design gap)**

`reverse_buy` derives the rate from **one** preview at `amount = 10 000` and extrapolates:
`G = (net−1)·10⁴/(10⁴−tax(10⁴)) + 1`. `reverse_sell` derives it from **one** preview at
`amount = 10 000 + sell_bps` and extrapolates `economic + economic·tax/10⁴`.

The canonical `community-tax-token` is exactly proportional, so this is correct *today*. But the
protocol is specified as a general token↔router contract (`RouteTaxCapability` / `RouteTaxPreview`
are the only attested surface, and `capability()` deliberately admits any contract that answers
them), and no cap field or proportionality assertion exists. A token with a **minimum tax**, a
**tiered/holding-period tax**, an exemption that the router's `trader` argument does not reproduce,
or a `tax_map` keyed on the destination would make `reverse_*` silently wrong — under-quoting the
required gross (→ execute revert, UX/DoS) or over-quoting (→ user overpays). The indexer side
(`route_exact_output::inverse_net`, `sell_economic`) has the same assumption, so client and chain
would agree with each other and both be wrong.

*Recommendation.* State proportionality as a **v1 protocol invariant** in
`docs/runbooks/route-local-tax-1381.md` and enforce it cheaply: extend
`RouteTaxCapabilityResponse` with a `tax_model: "proportional_bps"` discriminator, and/or validate
`preview(G) == { economic: S, tax: S·bps/10⁴ }` at two magnitudes before trusting the extrapolation.

#### SBHIGH-07 — `funded_hybrid` re-partitions the book/pool split with a floor that favours the pool · **Low · confirmed**

`route_tax.rs:112-125`: `book_input = economic·book/gross` (floor), `pool_input = economic − book`.
The declared ratio is preserved to within 1 raw unit, with the remainder always going to the **pool**
(i.e. slightly worse execution than quoted). The subsequent
`validate_hybrid_declared_split_for_no_belief(funded.economic, …)` — the guard that enforces a
"material pool leg" when `belief_price` is absent — is therefore evaluated against a split the user
never saw. Magnitude is dust (≤1 unit per hop), direction is unfavourable, and the frontend mirrors
the same arithmetic (`swapRoutePreflight.ts::fundedHybridForQuote`), so quote and execute agree.

*Recommendation.* Keep the behaviour, but add an explicit test asserting the parity between
`funded_hybrid` and the frontend mirror, and record the ≤1-unit unfavourable rounding in the #1381
QA checklist so it is not rediscovered as a "quote mismatch" bug report.

#### SBHIGH-08 — Residue refunds re-enter `apply_launch_guards` and can revert the whole route at `max_wallet` · **Low · confirmed**

`execute_route_transfer` calls `apply_launch_guards(..., &TaxKind::Honest, amount, to_new_balance)`
(`tax.rs:774-788`). The `match kind` arm only gates the cooldown, so the **`max_wallet` check still
runs for `Honest`**. A trader sitting exactly at `max_wallet` who triggers an integer residue refund
gets `MaxWallet`, which reverts the entire route — including the swap they were trying to make. The
comment in the source ("max-wallet protection remains enforced") shows the intent; the side effect
on refunds is almost certainly unintended.

*Recommendation.* Exempt refund legs explicitly — the router's `RouteTransfer` list already
distinguishes refunds from final delivery, so either add a `is_refund` flag to the context transfer
entry or skip `max_wallet` when `kind == Honest` and the credit equals an owned-residue refund.

#### 3.1.2 Re-verified negative: a mid-flight tax-rate change cannot under-fill a trade

Managers can raise `buy_bps`/`sell_bps` (up to the instantiate-time caps, `MAX_TAX_BPS = 2500`)
through a paid UST1 invoice at any time. Because the router **re-runs `preview()` at execute time**
rather than trusting the client's quoted amounts, a raise between quote and execute reduces the
economic input, which reduces the output, which fails the user's `minimum_receive` check (the router
converts the user's net floor to a gross floor per hop via `reverse_buy`, and the pair enforces
`min_return`). Outcome: a **revert**, never a silent worse fill. This is the correct design and should
be preserved through the migration.

The residual, non-exploitable user-facing risk is *tax-honeypot economics*: the caps are chosen by the
token creator, not the protocol, so a 25% buy/sell tax is technically permitted, and a post-listing
raise is a legitimate manager action. → §7.1.

### 3.2 Pair exact-output / reverse math — #1376/#1378/#1386 (pair 1.19.0)

**Verified improvements.**
- `min_pool_input_for_net_output` is an exact integer inversion of the constant product with the
  output-side fee applied in the same direction as execute (`gross = (target−1)·10⁴/(10⁴−fee) + 1`),
  and the minimality property is property-tested over a fee × target grid
  (`issue1376_inverse_fee_rounding_small_domain`, `…_high_decimals_and_more_than_half_reserve`).
- All reserve products moved to `Uint256` intermediates with explicit `narrow_u128`, fixing a real
  `u128` overflow in both `execute_swap` and `simulate_hybrid_swap_with_fee`
  (`4ad54cd3…` wasm; overflow at ~20 whole 18-dec tokens per side).
- Fee rounding is now **shared** by execute, forward simulation and the pool inverse
  (`hybrid_reverse::pool_commission`), removing a class of quote/execute divergence. The
  `commission rounding exceeds 1 token` invariant check was widened to `Uint256` to match.
- The pool-only reverse path no longer burns a second full-range bisection: it uses the exact
  minimum plus **one** forward validation call, which is what fixed the live 32-call cap breach
  documented in `issue1376_replays_old_32_vs_35_call_boundary`.
- No storage rewrite; `cw2` guard rejects same-version migration (`1.18.0` → `1.19.0`).

#### SBHIGH-09 — Declared-book reverse still exhausts the 32-simulation budget for large targets · **Low-Medium · confirmed**

`bounded_hybrid_reverse_search` bisects `[1, hi]` under a **32-call** budget. The bracket is
`[1, seed]`, so any target whose sufficient offer exceeds ~2³⁰ raw units needs more than 30
bisections and terminates with *"Hybrid reverse simulation exceeded iteration cap"*. The new unit
test even encodes this: a `1u128 << 30` target consumes exactly 31 calls. For 6-decimal community
tokens, 2³⁰ raw ≈ 1.07e9 tokens — comfortably reachable for a well-funded pair, and far below the
`Uint128` domain the code claims to search.

Consequence is **quote liveness**, not fund loss: exact-output sizing becomes unavailable precisely
for the deepest, most competitive liquidity, pushing users toward unbounded-input swaps — the
sandwichable shape (Chain C). Note the pool-only path does *not* have this problem, so the exposure
is specific to book-hybrid hops, which is the official dApp's default.

*Recommendation.* Keep the 32-call *gas* budget but remove the *range* dependency: seed `lo` from the
pool-math lower bound (`total_offer_for_pool_leg(pool_math_min, hybrid)`) rather than from `1`, so the
bracket width is bounded by the book contribution rather than the absolute offer, and raise the cap
for this path only with a documented gas justification. Alternatively iterate on the **excess over
the pool-only solution**, which is bounded by the book's own depth.

#### SBHIGH-10 — Reverse sizing ignores the trader's discount tier (conservative) · **Low · confirmed, safe direction**

`min_pool_input_for_net_output` uses `effective_fee_bps` (the pair's base fee), while the forward
simulation applies the trader's `DISCOUNT_REGISTRY` tier. A discounted trader is quoted the
undiscounted inverse — always ≥ the true minimum, never below. The single forward validation
(`sim.return_amount < ask_target → error`) prevents any under-quote from reaching the response. Worth
an explicit test and a one-line note so a future "reverse ignores tier" report is recognised as
intentional and safe.

### 3.3 Factory / hooks / fee-discount / faucet / autolp / launcher (re-verified)

| Contract | Privilege model | Result |
|---|---|---|
| `factory` (1657 lines) | `ensure_governance` on every admin path (`AddWhitelistedCodeId`, `SetPairFee`, `SetPairTreasury*`, `SetPairCreationFee`, `SetPairHooks`, `SetDiscountRegistry*`, `SetLpAdmin*`, `SetPairPaused`, `SweepPair`, blacklist*, `RefreshPairAssetCodeIds*`, `UpdateConfig`); `CreatePair` is permissionless but requires both code IDs whitelisted, decimals ≤ cap, a creation fee with exact refund arithmetic, and a one-pair-per-block gate | **Sound.** No unguarded admin execute found. Pair instantiation uses `admin: governance` |
| `pair` (3956 lines) | 12 distinct `Unauthorized` sites; `Sweep` is **factory-only** and subtracts both the reserve *and* `PENDING_ESCROW_TOKEN{0,1}`; governance-only for fee/treasury/hooks/discount/paused/limit-config | **Sound.** Skim cannot touch reserves or pending escrow |
| `router` (1031) | `SetWrapMapper` requires the **factory's** governance (not a local admin) | **Sound** |
| `burn-hook` | `Hook` requires an allow-listed caller; `UpdateConfig`/`UpdateAllowedPairs` admin-only; burns `min(computed, own balance)`; `Burn` is `reply_on_error` and failures are swallowed so a hostile token cannot block a swap | **Sound** |
| `lp-burn-hook`, `tax-hook` | Same admin-gated shape | Sound (unchanged this pass) |
| `fee-discount`, `faucet`, `community-tax-autolp`, `community-token-launcher` | Unchanged this pass; previously reviewed | Carried forward |

### 3.4 `bridge-debt` / `bridge-debt-token` (live, `8f9dd98e`)

#### SBHIGH-15 — `RegisterMarket` validates the debt token but not the underlying · **Low-Medium · confirmed**

`inspect_debt_token` enforces: code id == the single `debt_code_id`, minter == this contract,
`debtor` sane, `total_supply == 0`, decimals match (`contract.rs:118-127`, `360-395`).
`underlying_decimals` performs **only** a `TokenInfo` query and returns its decimals
(`contract.rs:424-430`) — no `ContractInfo`/code-id check and no factory allowlist.

Practical exploitation is limited: `Issue` sends `Cw20ExecuteMsg::Mint` to the underlying, which
reverts unless this contract is its minter. But the asymmetry means a market can be registered
against an arbitrary contract that merely answers `TokenInfo`, and `Repay` will then `Burn` from that
contract, and `Recover` will `Transfer` from it. A hostile or non-CW20 "underlying" is a supported
configuration, which is a wider blast radius than the debt-token side implies.

*Recommendation.* Pin an `underlying_code_id` allowlist (reuse the factory's whitelisted code IDs, or
a dedicated `SetUnderlyingCodeId` behind `must_owner`) exactly as done for the debt token, and
re-verify it in `assert_market_consistent` on every issue/repay (that function already re-inspects
the debt token, so the hook exists).

#### SBHIGH-16 — Single-key owner with unbounded mint and sweep authority · **Low · confirmed (centralisation, live risk)**

`must_owner` is a bare `info.sender == config.owner`. There is no multisig, no timelock, and no
per-market cap: `Issue` can mint an arbitrary `amount` of the underlying to an arbitrary recipient
while incrementing `outstanding`; `Recover` can transfer any balance the contract holds (including
tokens users sent directly by mistake, since only the `Send`+`Repay` hook is tracked); `UpdateOwner`
is immediate and a typo is unrecoverable. With 10,000 units already issued and a UST1/USDT LP seeded
to treasury, owner-key compromise is a direct, unbounded economic event.

*Recommendation.* Move `owner` to the same governance contract that administers the factory (or a
2-of-2), add a timelock or a per-epoch mint cap to `Issue`, and emit a distinct attribute for
"unsolicited token recovery" so `Recover` is alertable.

### 3.5 `community-tax-token` remaining surfaces (re-verified)

- **`RegisterListedPair` is permissionless** but correctly re-validates the pair's self-reported
  `asset_infos` against the **factory's** `Pair` query for that same unordered key and against
  `pair_info.contract_addr == pair` (`pair_registry.rs`). An attacker cannot register a rogue pair.
  Side effect worth noting: registration is **irreversible** and also writes
  `PROTOCOL_EXEMPT[pair] = true`, so a listed pair is permanently transfer-tax-exempt.
- **Sink split** (`split_tax`) gives the last sink the whole remainder via
  `tax.checked_sub(allocated)`, and skips zero shares — conservation holds; sink count is capped at
  `MAX_SINKS = 4`.
- **Burn sinks** decrement `total_supply` and are capped by available balance because the debit's
  `checked_sub` runs first.
- **`MAX_WALLET` / cooldown** are evaluated on the *economic* kind (`trade_kind`) even when the
  manager exemption turns the tax into `Honest` (`tax.rs:265-269`, `551-560`), so #609 cannot
  disable launch guards. Verified correct.
- **`is_cooldown_subject`** excludes the token, listed pairs, the router (via `PROTOCOL_EXEMPT`) and
  `PROTOCOL_EXEMPT` entries. The router is stamped at instantiate from `msg.router`, so the
  cooldown/max-wallet path is consistent for the instantiate-time router — and inherits SBHIGH-01:
  a *new* router address would **not** be stamped, so if the `preview` gate were ever bypassed the new
  router itself would become a cooldown/max-wallet subject. Reinforces the recommendation in
  SBHIGH-01 to keep the address stable.

---

## 4. Indexer / Rust server / database findings

### SBHIGH-11 — The indexer unit suite is nondeterministic; merge-gate status and receipts are unreliable · **Medium · reproduced**

`indexer::pair_discovery` keeps a **process-global** negative cache
(`OnceLock<Mutex<HashMap<..>>>`, `PAIR_REJECT_CACHE_TTL = 1 h`, `PAIR_REJECT_CACHE_MAX = 10_000`,
`pair_discovery.rs:17-20`). Two tests — `discover_new_pair_negative_caches_unlisted_pair` and
`discover_new_pair_negative_caches_non_pair_contract` — both call `reject_cache_clear()` and then
both use the **same** `ATTACKER_PAIR` fixture address. Under `cargo test`'s default parallel
execution, whichever test populates the cache first makes the other short-circuit to
`RejectedCached` **without issuing the factory LCD query the test asserts on**.

Reproduction (this audit):

```
$ for i in 1 2 3; do cargo test --lib discover_new_pair; done
test result: FAILED. 1 passed; 1 failed   (assertion `left == right` failed  left: 0  right: 1)
test result: ok. 2 passed; 0 failed
test result: FAILED. 1 passed; 1 failed
```

Full-suite run: **469 passed / 1 failed**. Both `.gitlab-ci.yml` jobs that run Rust indexer tests
(`test-indexer-lib` → `cargo test --lib`, and `test-indexer-integration` → `cargo test --tests`)
execute this target, so pipelines are randomly red/green. Every prior report recording "indexer lib
470 passed" — including `docs/qa/merge-2026-10-04.md` — is a **lucky run**, not a stable gate.

*Recommendation.* Make the reject cache injectable per test (e.g. a `OnceLock` that tests can swap, or
key the cache on `(lcd.cache_namespace(), factory)` so distinct mock servers cannot collide), give the
two tests distinct fixture addresses, and consider `--test-threads=1` for this module. Until fixed,
treat indexer test results as advisory in review.

### SBHIGH-12 — Reject cache is not LCD-namespaced and is pollution-prone · **Low-Medium · confirmed**

Unlike `trading_tax_discovery::cache`, which keys on `state.lcd.cache_namespace()` plus router plus
policy, the pair-discovery reject cache is keyed only by pair address. In a multi-chain or
multi-endpoint deployment, a rejection learned against one LCD can suppress rediscovery against
another; and because the map is bounded at 10,000 entries with a simple oldest-first eviction, an
attacker who can induce discovery attempts for 10,000 foreign `action=swap` pair addresses can evict
legitimate entries and force repeated factory LCD traffic (amplification) during the eviction window.

*Recommendation.* Namespace the key by LCD identity, switch to a small LRU, and reject addresses
that are not plausibly pairs (e.g. length/bech32 prefix) *before* caching.

### SBHIGH-13 — Per-request LCD amplification on the solve path · **Low · confirmed, bounded**

`tax_rank::load_tax_rank_snapshot` spawns one discovery task per unique hop address
(`MAX_DISCOVERY_ASSETS = 64`) behind a **global 4-permit** semaphore with a 3 s per-query timeout and a
5 s TTL cache — a good design. However `router_funding_supported(state)` is issued **per request**
(uncached) and only when a v1 token is present, and `exact_output`'s `path_eligible` adds up to one
`is_paused` query per hop plus one `blacklist_check`. All are fail-closed and all sit under
`/api/v1/route/solve`, which is in the LCD-heavy rate-limited router — acceptable, but the exact-output
path additionally holds one of only `MAX_CONCURRENT_REQUESTS = 4` **global** permits for up to its
20 s budget, so a small number of clients (or one client opening four requests, each amplified by
SBHIGH-03) can starve every other exact-output user. Recommend a per-IP share of those permits.

### 4.1 Positive indexer findings (recorded to prevent re-litigation)

- **SQL injection: none found.** Every query is parameterised. The only dynamic SQL is
  `db/queries/protocol_liquidity.rs:163-180`, where `{table}`, `{key}` and `{bucket}` are drawn from a
  hard-coded `match` over `"hour" | "day" | "month"` and the file documents *"Trusted constants only;
  never accepts request-controlled SQL identifiers."* The `$1`/`$2` window and prune values are bound.
- **Tax discovery fails closed at every boundary.** `discover()` requires a registered code id or data
  hash before trusting a capability response; a `get_config` rejection still probes `tax_map` and
  `tax_info`; conflicting `tax_free` ∩ `route_tax` registrations return `Unsupported` rather than
  waiving; `compatible()` additionally requires the *live* instance's `router` to equal the configured
  router and bounds rates to ≤ 10 000 bps. `resolve_token_exempt` fails closed on LCD error.
- **Eligibility, not just adjustment.** `path_sells_middle_tax_hop` marks a path **ineligible** when any
  hop token is `unresolved` (including `Unavailable`), when a non-v1 legacy token would still charge a
  router hop, or when route-local funding is claimed but the router instance does not report
  `funding_version == 1`. Exact-output then drops the path rather than quoting it untaxed.
- **Tax rate vs. chain parity.** `sell_economic` and `tax_amount` are integer-identical to
  `route_tax::sell_from_gross`/`tax_amount` (same maximal-`S` binary search, same floor), and
  `inverse_net` is the exact inverse of `buy_from_gross`. Quote/execute divergence from rounding is
  therefore not expected — divergence would manifest as a **revert**, not a loss.
- **Route-cache TTL for v1 paths is clamped to the 5 s discovery TTL**
  (`route_solver.rs` `cache_ttl_for_hops` override), so a tax-rate change cannot be served stale from
  the route cache.
- **Caching/DoS hygiene:** `book_snapshot` caps response bytes (65 536 B) and orders (1 000/side);
  `RequestBodyLimitLayer` bounds the solve POST body; a 30 s `TimeoutLayer`, `CompressionLayer`,
  configured-origin CORS, per-IP and global rate limits, and an IPv4-only default listener (#282) are
  all present. `startup_logs_do_not_contain_database_password` and
  `startup_logs_do_not_contain_reorg_webhook_url` guard secret leakage into logs (#1205 redaction
  posture maintained).

---

## 5. Frontend findings (lower severity; the dApp is the least-trusted tier)

| # | Finding | Severity |
|---|---|---|
| FE-01 | `fundedHybridForQuote` in `swapRoutePreflight.ts` mirrors `funded_hybrid` exactly, including the unfavourable floor — quote/execute parity is **good**. No finding. | — |
| FE-02 | `useRouterTaxFunding` gates on `ROUTER_CONTRACT_ADDRESS` and caches 5 s; if the configured address is wrong, the UI can present **gross** budgets against a token that will reject them, producing a user-visible revert. The contract fails closed, so this is UX, not loss. Tie the address to the router actually verified in the migration inventory (see SBHIGH-01). | Low |
| FE-03 | Reverse-quote parity: the dApp calls `reverse_simulate_swap_operations` directly (see SBHIGH-03). Moving large trades onto the indexer's exact-output endpoint (which has rate limits, a work budget and fresh validation) would remove the browser as an amplification vector. | Low |
| FE-04 | 70 Playwright specs, **zero** in GitLab CI. The new `swap-receive-1376-tx.spec.ts` (the only true on-chain proof of the #1376 sizing) is therefore advisory. Still tracked under #421. | Medium (process) |

---

## 6. Access control & privilege matrix (consolidated)

| Principal | Authority | Guard | Status |
|---|---|---|---|
| Factory `governance` | All factory admin, pair instantiate/migrate, `Sweep`, fee/treasury/hooks/discount/pause, blacklist, code-id whitelist | `ensure_governance` on every admin execute | Verified |
| Pair `governance` | fee, treasury, hooks, discount registry, paused, LP admin, limit config, clean | 12 `Unauthorized` sites | Verified |
| Official router | `RouteTransfer` on tax tokens; router-sell funding | `is_official_router` **and** live `route_tax_context` matching trader/pair/tokens/gross/economic | Verified, with SBHIGH-01 (pin) and SBHIGH-02 (replay) |
| Tax-token `manager` | Paid-invoice settings (bps ≤ caps, treasury, sinks, exemptions, autolp, guards, minter) | UST1 invoice + SKU gating | Verified; see §7.1 economics |
| Tax-token `launcher` | `BindAutolp` | `info.sender == cfg.launcher` + one-shot `autolp.is_none()` | Verified |
| Anyone | `RegisterListedPair` | Factory `Pair` attestation | Verified |
| `bridge-debt` owner | `SetDebtCodeId`, `RegisterMarket`, `Issue`, `Recover`, `UpdateOwner` | `must_owner` (single key) | SBHIGH-15, SBHIGH-16 |
| burn-hook admin | Hook config + allowed pairs | `info.sender == config.admin` | Verified |
| Indexer | Read-only HTTP; no write path | Rate limits, body limits, timeouts, fail-closed discovery | Verified, SBHIGH-13 |

---

## 7. Tokenomics, economic and oracle considerations

### 7.1 Tax caps are self-declared (honeycot risk), not protocol-bounded
`MAX_TAX_BPS = 2500` (25%) is a **protocol** ceiling, but the binding ceiling is
`max_buy_bps` / `max_sell_bps` chosen by the token creator at instantiate, and raising the *current*
rate up to that cap is a routine paid-invoice manager action. A community token can therefore be
launched at 0% and raised to 25% later. The router's execute-time re-preview means a raise cannot
under-fill a live trade (it reverts, §3.1.2), so this is a **listing/disclosure** risk rather than a
theft vector — but it is the classic community-token honeycot pattern, and the dApp should display
*cap* alongside *current* rate. The 5 s indexer TTL bounds quote staleness to seconds.

### 7.2 Sink redirection and treasury capture
Sink credits are blocked from flowing to the router or to listed pairs, and burn sinks reduce
`total_supply`. Residual: the `AutoLp` sink credits the AutoLP sister, which is launcher-bound and
protocol-exempt — i.e. a manager can route tax into LP it (or its delegate) will withdraw as a
provider. This is a **disclosure** item, not a bug: `AutoLpConfig.lp_recipient` is manager-chosen.
Recommend the listing UI display the active sink graph.

### 7.3 Oracle / price-source manipulation (unchanged, still the top external risk)
- `indexer/src/indexer/hub_usd.rs` derives hub USD (cUSTC / UST1 / USTR) from the **largest-liquidity
  pool**. That is exactly the Nostra pattern: the selection criterion is attacker-influenceable
  (create/manipulate a pool that becomes the deepest), and this DEX publishes the resulting number.
- `indexer/src/api/gt.rs` mirrors reserves into a GeckoTerminal-shaped surface; a third party
  consuming it inherits the same exposure.
- Pair TWAP is now `Uint256`-cumulative (1.18.0, already rolled out and closed), so the in-pair
  oracle is overflow-safe; the *selection* problem is external to it.
- The repo's own consumers (`/protocol`, `/portfolio` marks, `hub-prices`) are display-only, so the
  damage is to **external** consumers — UST1 window, bridges, anything treating a CL8Y-DEX mark as
  collateral. → Chain S, Chain K.

### 7.4 MEV / slippage shape
- Router paths set `belief_price: None` and rely on per-hop `min_return` plus the
  "material pool leg" and "book leg requires a slippage floor" guards (`#307`/`#334`) — sound.
- Book-hybrid execution against a manipulable book is the residual exposure; combined with
  SBHIGH-09 (no exact-output for large book-hops) users are pushed to the unbounded-input shape. → Chain C.
- `deadline` is asserted once at router entry; because a route completes in one transaction, block
  time cannot advance mid-route, so this is correct rather than a gap.

---

## 8. Prior findings re-verified this pass

| Prior finding | Status now |
|---|---|
| #1324 pair TWAP Uint256 rollout complete and closed | **Re-verified closed.** Wall-clock trigger defused; nothing to do |
| Legacy wallet-funded router-hop taxes cannot silently resume | **Re-verified.** `capability()` refuses to fall back for any token answering `get_config`/`tax_map`/`tax_info`; the indexer marks such paths ineligible; the legacy `ExecuteSwapOperations` entry rejects taxed assets unless both bps are 0 |
| Playwright E2E absent from CI | **Still open** (FE-04) |
| Liquidity-quality false-collapse (#1380/#1382) | **Re-verified** the dynamic-SQL site is constant-only; the `quality`/`generation`/`coverage_identity` columns and the `DISTINCT ON … ORDER BY sampled_at DESC` terminal-sample selection correctly preserve degraded terminal rows instead of nulling them |
| Route-tax capability must be attested by immutable registration, not by a live response | **Re-verified** in `trading_tax_discovery` (`registered()` requires code id or data hash) and in the on-chain `capability()` (cw2 identity allowlist) |
| Frontend must not double-subtract buy tax | **Re-verified** — router returns post-tax `net` as the next hop's gross budget; the indexer mirrors; docs updated |
| Nostra/Moonwell/oracle-consumer chain | **Still the top external risk** (§7.3) |

---

## 9. Areas considered and explicitly cleared (so future audits do not redo them)

1. **Reentrancy across the whole route** — `SWAP_STATE` presence blocks re-entry; token `do_transfer`
   performs no external calls before its balance writes are committed in the same message.
2. **Flash-loan / atomic round-trip sandwich inside one route** — a trader can only sandwich
   *themselves*; there is no callback into the router between hops that would let an attacker act on
   intermediate state. (The `A→B→A` round trip is a real, if small, self-manipulation primitive and
   interacts with SBHIGH-05.)
3. **`Uint128` overflow in the new math** — `sell_from_gross` uses subtraction-based binary search;
   `buy_from_gross` and `pool_commission` are `Uint256`-widened; `min_pool_input_for_net_output`
   narrows explicitly with contextual errors. Tested at `u128::MAX`.
4. **Denial via zero/one-unit dust amounts** — `ZeroHopOutput`, `min_pool_input_for_net_output(0)` and
   `sell_from_gross(0)` all terminate deterministically; the router rejects empty/oversized operation
   lists and enforces `MAX_HOPS = 4`.
5. **Native-token confusion** — router rejects `NativeSwap` and non-token asset infos at every hop;
   the pair is CW20-only in `CreatePair`.
6. **Front-running the pair-creation one-per-block gate** — the gate is a rate limit, not an
   ordering guarantee; pair creation is fee-gated and code-id gated, so the residual is economic
   (slot scarcity), not a security bypass.
7. **Numeric/decimal confusion (6/18-decimal pairs)** — `asset_decimals`, `MAX_PAIR_ASSET_DECIMALS_BOOTSTRAP`,
   and `route_exact_output_reference::supported_hops` reject unsupported decimal mixes rather than
   guessing.
8. **Indexer secret leakage** — startup log tests cover DB password and reorg webhook URL; evidence
   endpoint redaction is in place; no `format!`-built SQL.

---

## 10. Positive security observations worth preserving through the migration

- Route-local funding v1 **removes** an entire class of failure: a trader can no longer be left paying
  a tax from their wallet that the quote did not model, and cannot be sandwiched into a wallet-funded
  extra debit.
- Execute authentication moved from a **caller-supplied hook field** to **live router state** — a
  meaningful improvement over the wallet-funded design it replaces.
- Pool reverse math is now exact, minimal, and overflow-safe, and the fee rounding is a single shared
  function across execute/simulate/inverse.
- The indexer is consistently **fail-closed** on tax discovery, and its 5 s TTL clamp prevents
  serving stale tax rates from the route cache.
- Route-listing gates, per-request concurrency caps, response byte caps, body limits, timeouts and
  rate limits are all present on the LCD-heavy surface.
- CI runs `cargo audit --deny warnings` for both workspaces, gitleaks over tracked files, and a log-secret
  linter; the build gate catches `tsc -b` breaks that unit tests miss.

---

## 11. Recommended work queue (ordered)

| Priority | Item | Finding |
|---|---|---|
| **P0 — before #1384 executes** | Write the "router address must be preserved" invariant into the runbook + QA checklist **and** add a preflight that compares every inventory token's `route_tax_capability.router` against the router being rolled out | SBHIGH-01 |
| **P0** | Fix the indexer test flake (injectable / namespaced reject cache, distinct fixture addresses); re-baseline all "indexer tests pass" receipts | SBHIGH-11 |
| **P1 — before #1384 executes** | Bound `reverse_greedy_offer` sim calls (32, matching the pair) and seed the bracket from pool math | SBHIGH-03 |
| **P1** | Add a consumption/idempotency marker to `RouteTransfer` | SBHIGH-02 |
| **P1** | Reject repeated-token hops in the router (or make the cooldown check unconditional), and add the regression test | SBHIGH-05 |
| **P2** | Document/enforce the proportional-tax invariant; add the `tax_model` discriminator or a two-point probe | SBHIGH-06 |
| **P2** | Seed the declared-book reverse bracket from pool math so the 32-call cap stops binding | SBHIGH-09 |
| **P2** | Namespace + LRU the pair-discovery reject cache | SBHIGH-12 |
| **P2** | Exempt refund legs from `max_wallet` | SBHIGH-08 |
| **P2** | Pin an `underlying_code_id` allowlist in `bridge-debt`; move owner to governance | SBHIGH-15/16 |
| **P3** | Deduplicate capability lookups; add gas-delta evidence for a 4-hop taxed route | SBHIGH-04 |
| **P3** | Per-IP share of the exact-output permits; cache `router_funding_supported` | SBHIGH-13 |
| **P3** | Get the Playwright job into GitLab CI (#421) | FE-04 / TST-02 |
| **P3** | Add a `UpdateRouter` path (invoice- or timelock-gated) so router rotation is possible at all | SBHIGH-01 (long-term) |
| **P3** | Display tax *caps* and the active sink graph in the listing/dApp surface | §7.1, §7.2 |

---

## 12. Residual risk statement

The highest residual risks after this pass are (a) **operational**: a mis-sequenced columbus-5 rollout
that changes the router address would take all taxed routing down with no recovery path
(SBHIGH-01), and (b) **external**: any consumer that treats a CL8Y-DEX spot or hub mark as a
collateral or settlement reference inherits the largest-liquidity-pool selection problem (§7.3).
Neither is a contract-logic defect; both are the kind of failure that only shows up under a real
migration, which is exactly why they are P0 above.

---

## 13. Finding index

| ID | Title | Severity | Area | Status |
|---|---|---|---|---|
| SBHIGH-01 | `CONFIG.router` write-once → new router address permanently bricks v1 routing | **High** | migration / token | Confirmed |
| SBHIGH-02 | `RouteTransfer` settlements not consumed (replayable) | Medium | token | Confirmed |
| SBHIGH-03 | Unbounded nested sim loop in public router reverse query | **Medium-High** | router | Confirmed |
| SBHIGH-04 | Capability discovery multiplies wasm queries on execute | Low-Medium | router | Confirmed |
| SBHIGH-05 | Repeated-token routes suppress launch-guard cooldown | Low-Medium | token/router | Confirmed |
| SBHIGH-06 | Reverse helpers assume proportional tax from one probe | Low-Medium | router/indexer | Confirmed |
| SBHIGH-07 | `funded_hybrid` floor favours the pool by ≤1 unit | Low | router | Confirmed |
| SBHIGH-08 | Residue refund can revert at `max_wallet` | Low | token | Confirmed |
| SBHIGH-09 | Declared-book reverse exhausts 32-sim budget for large targets | Low-Medium | pair | Confirmed |
| SBHIGH-10 | Reverse sizing ignores discount tier (safe direction) | Low | pair | Confirmed |
| SBHIGH-11 | Indexer unit suite nondeterministic; CI gate unreliable | **Medium** | indexer/CI | **Reproduced** |
| SBHIGH-12 | Reject cache not LCD-namespaced; pollution-prone | Low-Medium | indexer | Confirmed |
| SBHIGH-13 | Per-request LCD amplification / global-only permits | Low | indexer | Confirmed |
| SBHIGH-15 | `RegisterMarket` does not validate the underlying token | Low-Medium | bridge-debt | Confirmed |
| SBHIGH-16 | Single-key owner, unbounded mint + sweep (live) | Low | bridge-debt | Confirmed |
| FE-04 | Playwright E2E absent from CI (70 specs) | Medium (process) | CI | Open (#421) |

---

## 14. Exploit-chaining investigation

Method: take every confirmed finding above, every still-active prior-audit finding, and the
out-of-repo dependency set (columbus-5 wasmd/cosmwasm, the wrap mapper, the UST1 window, governance
contracts, LCD/RPC infrastructure, third-party consumers of published prices), and search for
combinations that turn a Low/Medium into a net economic event. Chains are graded by
**capital required**, **privilege required**, and **whether the protocol's own guards stop it**.

### Chain A — Pinned-router privilege persistence → replayable settlement → balance drain
**Inputs:** SBHIGH-01 + SBHIGH-02 + §3.5 (router stamped into `PROTOCOL_EXEMPT` at instantiate).

1. Governance performs the columbus-5 rollout and deploys route-tax v1 at a **new** router address
   (a completely reasonable reading of the runbook's "migrate the official router").
2. Every already-adopted community-tax token now rejects v1 routing (§3.1 SBHIGH-01) — taxed swaps
   are dead. Operationally this looks like "the migration didn't take" and the instinct is to roll
   back the tokens.
3. The **old** router address is still `CONFIG.router` in every token and can never be un-stamped.
   If, during incident response, the old router's wasm is migrated (the only lever available) to a
   build with any bug in `RouteTransfer`, that router retains authority to move token balances out of
   itself, repeatedly, with no consumption marker (SBHIGH-02), and the token-side attributes look like
   ordinary settlement.
4. Chained with SBHIGH-05: the same old router can be used to compose an `A→B→A` route that skips the
   trader's cooldown, laundering the activity past launch guards.

**Guard status:** the contract-level guards are sound; this chain is entirely **operational**. It is
prevented by (a) keeping the router address stable and (b) adding the consumption marker. Neither
requires user funds, only a governance action and a mistake.

### Chain B — Query amplification → quote-path outage → forced unbounded-input swaps → sandwich extraction · **highest expected value**
**Inputs:** SBHIGH-03 + SBHIGH-13 + §7.4 + Nostra-class manipulation.

1. An attacker calls `reverse_simulate_swap_operations` with `greedy` set on up to four hops. The
   router issues ~1,000 nested `HybridSimulation` queries (SBHIGH-03), each walking the limit book.
   This is unauthenticated, free, and repeatable.
2. The same attacker holds the four global exact-output permits (SBHIGH-13) so legitimate users get
   `429` / `504`, and the LCD/RPC that the dApp's own quotes depend on is saturated.
3. Users who cannot get a bounded quote fall back to **unbounded-input** swaps with `max_spread`
   only. A sandwicher front-runs with the knowledge that the victim's flow must take whatever the
   book offers.
4. Chained further: the attacker's own manipulation of the same pool (Nostra pattern) *improves* the
   sandwich, and the book-hybrid path — the one exact-output sizing could not cover for large targets
   anyway (SBHIGH-09) — is precisely the path most exposed.

**Guard status:** the router has **no** guard here (this is the core gap). The indexer's rate limits
mitigate the indexer-mediated path only; a direct LCD client bypasses them entirely. Fix SBHIGH-03
and this chain collapses.

### Chain C — Exact-output liveness gap + launch-guard cooldown bypass → same-block round-trip profit
**Inputs:** SBHIGH-09 + SBHIGH-05 + Spiral-class same-block thinking + §7.4.

1. For a deep book-hybrid pair, exact-output sizing fails at the 32-sim cap (SBHIGH-09), so large
   traders are pushed to input-exact swaps on the same book.
2. Within one transaction, an attacker uses an `A→B→A` route (same pair twice) to buy and re-sell
   into the book, and the authenticated trader's cooldown check is suppressed (SBHIGH-05), so the
   anti-snipe window that is supposed to make exactly this unprofitable does not apply.
3. `trading_enabled` and `max_wallet` still hold, so this is bounded to size and to the guard
   configuration — it is a *guard bypass*, not an unbounded drain, and it is Sybil-shaped (many
   wallets, one block), matching the Spiral incident class exactly.

**Guard status:** partially guarded. Rejecting repeated-token hops in the router fixes both halves.

### Chain D — Tax-cap raise between quote and execute · **verified NOT exploitable**
**Inputs:** §7.1 + SBHIGH-06.

A manager raises `sell_bps` between a user's quote and their execute. Because the router re-previews
at execute (§3.1.2) the economic input shrinks, the output shrinks, and the user's `minimum_receive`
fails → **revert**. The user cannot be under-filled. The only residual is a failed transaction and a
stale on-chain rate, which the 5 s indexer TTL bounds for clients. Recorded explicitly so this
plausible chain is not re-investigated as if it were a bug.

The *variant* that would be exploitable is a **non-proportional** tax (SBHIGH-06) combined with
per-hop min-return absent on one leg — e.g. a token whose tax depends on the destination or on a
holding period. The router's per-hop `min_return` and the final `minimum_receive` bound the loss to
"the quote was wrong and the trade reverts or fills marginally under the modelled rate". No path was
found to a silent, unbounded loss.

### Chain E — Indexer/contract rate disagreement → wrong gross budget submitted
**Inputs:** SBHIGH-06 + SBHIGH-11 + the 5 s discovery TTL.

The dApp computes a gross budget from an indexer quote; the router recomputes `economic` from its own
execute-time preview. If the indexer's snapshot is stale, or the token flipped to `Unavailable`
between quote and execute, the two disagree. Because the router's split is *always* recomputed from
the live preview and validated against its own conservation invariants
(`economic + tax ≤ gross`, `net + tax == gross` for buys), the outcome is a **revert**, never a
mis-funded swap. The residual risk is a temporarily unusable route for a token whose discovery is
flapping. Note SBHIGH-11 does **not** create a chain here: the flaky test is a test-isolation defect,
not a runtime one — the production reject cache is per-process and correct.

### Chain F — Bridge-debt owner key → unlimited mint into a live LP
**Inputs:** SBHIGH-16 + §3.4.

Owner-key compromise (or a malicious owner) allows unbounded `Issue` of the registered underlying to
any recipient, and `Recover` to sweep any balance the contract holds. If the underlying is a listed
DEX asset with a live pool, the attacker mints and dumps directly into that pool, extracting real
value from LPs while `outstanding` grows against a debtor who never authorised it. `RegisterMarket`'s
missing underlying code-id check (SBHIGH-15) widens the set of registrable "underlying" contracts.
Not chained with anything in-repo; it is a pure key-management chain, and the mitigation is
governance custody plus a mint cap.

### Chain G — Out-of-repo consumers of published marks (unchanged, highest systemic impact)
**Inputs:** §7.3 (hub-USD largest-liquidity selection, `/gt/` reserve mirroring) + Moonwell-class
"no code change needed" collateral pricing.

1. An attacker creates and manipulates a pool for a hub asset until it is the **deepest** pool for
   that ticker; the indexer's hub-USD job selects it, and the DEX's own public `/hub-prices`,
   `/overview` and `/protocol` numbers move with it.
2. Because the DEX publishes these as authoritative hub marks, any external consumer — a lending
   market, the UST1 window, a bridge valuation, an indexer such as GeckoTerminal reading the mirrored
   `/gt/` reserves — can be induced to overvalue thinly-backed collateral.
3. No CL8Y contract needs to be exploited for any of this; the pool-selection *criterion* is the
   vulnerability. This is the same shape as Nostra and Moonwell.

**Guard status:** no in-repo guard. The only in-repo mitigations are honesty of presentation
(`quality` / `coverage_identity` / `generation` on the #1380 liquidity surfaces) and the docs'
warnings. Recommendation: (a) select hub marks by a **manipulation-resistant** rule (e.g. median of
top-N by liquidity, or liquidity-weighted with a staleness/as-of floor) rather than deepest pool;
(b) publish `as_of` and reserve coverage next to every hub number (already partly done); (c) tell
external consumers in the API docs that these are spot marks, not oracle-grade.

### Chain H — Capability probing as a pre-trade enumeration oracle
**Inputs:** §3.1 (capability discovery) + §4.
`route_tax_capability` / `route_tax_preview` are public and unauthenticated, and the router's
`preview` calls are observable in transaction simulation. This lets any observer enumerate, per token,
whether it is a v1 community token, its configured router, and its exact current buy/sell bps. That
information is public by design (`TaxPreview` is a user-facing quote primitive) and low value on its
own, but combined with §7.1 it gives a precise, real-time map for "which tokens can be honeycotted
after listing" and for timing manager tax raises against pending user quotes. Accepted risk; note it
explicitly rather than leaving it implicit.

### Chain I — Cooldown/launch-guard as an anti-snipe control vs. MEV
Cross-reference, not a new finding: per-wallet cooldowns are **Sybil-inherently bypassable** (many
addresses, one block) and, in the routed path, additionally suppressible (SBHIGH-05). Any launch
economics or token distribution that assumes cooldown-based anti-snipe is assuming a control that
fails under exactly the conditions it is meant to address. Recommend treating cooldown as a
convenience, not a security boundary, in any token launch model.

---

## 15. Method notes and limitations

- **Not performed:** signed-transaction or live-chain verification, LocalTerra execution, Postgres
  integration suite, frontend vitest, Playwright, `cargo audit`, third-party smart-contract audit.
- **Firmware/ops surfaces not in scope:** Coolify deployment manifests, DNS/TLS, backup/restore,
  the buyback automation added in #1375 (noted in prior audits as a treasury-mover change with a new
  Hexxagon RPC trust anchor; re-check its credential handling and idempotency before the migration).
- **Every severity above is my engineering judgement**, not a third-party opinion. Items marked
  Confirmed are backed by a code reference in this document; SBHIGH-11 additionally by a reproduction
  command. Where I could not reach a conclusion, I said so rather than guessing.
