# SEC-I Attacker Prompt Pack

**Date:** 2026-06-30
**Checklist item:** SEC-I01
**Scope:** cl8y-dex-terraclassic -- CosmWasm DEX on Terra Classic (columbus-5)

---

## What this document is

This is the deliverable for SEC-I01. It contains:
1. **Prompts** -- adversarial LLM prompts derived from historical exploit themes in the checklist preamble, parameterized for this repo.
2. **Generated hypotheses** -- what the LLM produces when run with each prompt against this codebase.
3. **Triage** -- QA verdict: Covered, Not Applicable, Risk Accepted, or Real Gap.

Real Gap items feed into SEC-I02.

---

## Historical themes modeled

| # | Source | Theme |
|---|---|---|
| T1 | Terra / Astroport IBC-hooks 2022 | IBC callback reentrancy / infinite-mint |
| T2 | Osmosis pool-share bug 2022 | LP share inflation via donate-and-extract |
| T3 | Levana oracle / update-timing | Stale or manipulable quote used for settlement |
| T4 | Malicious CW20 registration | Fee-on-transfer or rebase token draining escrow |
| T5 | Admin key compromise | Governance key abuse or replay |
| T6 | Frontend wallet / signing confusion | Phishing via misleading signing modal |
| T7 | Indexer / API amplification | LLM agent crawling LCD-heavy routes |
| T8 | SQL / query injection | Malicious sort, interval, or filter parameters |
| T9 | Event spoofing / fake pair indexing | Clone pair emitting events to pollute indexer |
| T10 | Stale deployment / missed patch | Code ID mismatch, wrong env address |

---

## Summary table

| ID | Theme | Hypothesis | Verdict |
|---|---|---|---|
| H01 | T1 | IBC hooks reentrancy via submessage reply chain | Not Applicable |
| H02 | T2 | LP share inflation via first-depositor share price manipulation | Covered |
| H03 | T2 | Repeated add/remove profit extraction (Osmosis-style loop) | Covered |
| H04 | T3 | Sandwich via large reserve shift before multihop settlement | Risk Accepted |
| H05 | T4 | Fee-on-transfer CW20 drains limit order escrow | Real Gap |
| H06 | T5 | Governance key replay or spoofed factory message | Covered |
| H07 | T6 | Signing modal shows symbol only -- malicious indexer substitutes pair contract | Real Gap |
| H08 | T6 | Expert Mode persistence in localStorage enables repeated high-slippage swaps | Risk Accepted |
| H09 | T6 | Indexer-controlled route mismatch between displayed path and submitted operations | Real Gap |
| H10 | T7 | IPv6 /64 rotation bypasses per-IP rate limit on LCD-heavy routes | Risk Accepted |
| H11 | T7 | Concurrent limit-book depth walks fan out unbounded LCD queries | Covered |
| H12 | T8 | SQL injection via sort or interval query parameters | Not Applicable |
| H13 | T9 | Fake pair contract emits wasm events to inject synthetic volume | Covered |
| H14 | T9 | Empty FACTORY_ADDRESS ships to production, skips provenance check | Real Gap |
| H15 | T10 | Stale env address causes frontend to call wrong factory or router | Partially Covered |
| H16 | Novel | Blacklist UI gate bypassed via 15-second React Query stale cache | Risk Accepted |
| H17 | Novel | Limit order partial fill followed by cancel returns full original escrow | Covered |

---

## Prompts and generated hypotheses

---

### H01 -- IBC hooks reentrancy via submessage reply chain

**Prompt:**

> You are a security researcher attacking a CosmWasm DEX on Terra Classic. The pair contract dispatches after-swap hooks as CosmosMsg::Wasm::Execute submessages. The router chains hops using reply() handlers with SWAP_REPLY_ID. Identify whether an attacker can craft a malicious hook or a crafted CW20 send reply that reenters the pair or router contract mid-execution to manipulate reserves or escrow balances before the outer transaction commits.

**Generated hypothesis:**

If a hook dispatched during hop N can trigger a message that creates a new swap on the same pair before hop N+1 reads SWAP_STATE, reserves seen by hop N+1 would be post-hook post-reentry reserves. An attacker could deposit liquidity during the hook window, shift reserves, and withdraw after hop N+1 settles at a manipulated price.

**Triage: Not Applicable**

- Pair contract writes reserves to storage (pair/src/contract.rs line ~1143) before dispatching any hook messages. State changes are fully committed before hooks execute.
- CosmWasm message-passing model does not allow callbacks into the calling contract during the same message execution. Hook submessages execute after the pair contract's Response is fully returned.
- No IBC receive, ack, or timeout handlers exist in any contract. Zero cross-chain callback surface.
- Router reply handler checks SWAP_STATE is empty before initializing, preventing double-init replay.

**No issue required.**

---

### H02 -- LP share inflation via first-depositor share price manipulation

**Prompt:**

> You are attacking a CosmWasm AMM pair. The first depositor receives LP shares proportional to sqrt(amount_a * amount_b). An attacker who is the first depositor can deposit a tiny amount (1 uluna / 1 utoken) to make total_supply = 1, then donate a large amount directly to reserves without minting LP tokens (bypassing provide_liquidity). This makes each LP token worth a large amount. A subsequent depositor providing significant liquidity receives 0 LP tokens (rounded down) while their tokens stay in the pool. Show whether this attack works on this pair.

**Generated hypothesis:**

First depositor mints 1 LP token by depositing 1 uluna / 1 utoken. Then directly transfers 1,000,000 utoken to the pair contract via CW20. Reserves become 1,000,001 / 1,000,001 but total_supply = 1. Next depositor providing 999,999 receives floor(999,999 * 1 / 1,000,001) = 0 LP tokens. Their funds are absorbed.

**Triage: Covered**

The pair contract burns MINIMUM_LIQUIDITY = 1,000 tokens on the first deposit (pair/src/contract.rs lines 1614-1622). Total_supply is always at least 1,000 after the first deposit. The economic cost to execute the inflation attack is at minimum 1,000x larger than without the burn. Same defense as Uniswap v2 and Astroport.

**No issue required.**

---

### H03 -- Repeated add/remove profit extraction (Osmosis-style loop)

**Prompt:**

> Show whether a depositor can repeatedly add and then remove liquidity to extract value from rounding in LP share issuance and redemption.

**Generated hypothesis:**

Each provide_liquidity issues floor(min(a * total_supply / reserve_a, b * total_supply / reserve_b)) LP tokens. Each withdraw_liquidity returns floor(lp * reserve / total_supply). A depositor who receives fewer LP tokens than proportional and immediately withdraws receives less than they put in. Repeated round trips extract value.

**Triage: Covered**

Floor division always rounds down shares issued, meaning the depositor loses fractional value on entry, not gains. Withdrawals also floor-divide, returning less than proportional reserves. Repeated round trips lose money, not gain. The rounding direction goes against the attacker on both legs.

**No issue required.**

---

### H04 -- Sandwich via large reserve shift before multihop settlement

**Prompt:**

> A multihop swap's quote is computed off-chain. The attacker front-runs by shifting reserves in intermediate pools before the victim's transaction executes. Show how to extract value from a 3-hop LUNC -> X -> Y -> USTC route using this timing gap.

**Generated hypothesis:**

Attacker front-runs by buying X on the LUNC/X pair and Y on the X/Y pair, shifting both reserves. The victim executes at worse rates on both intermediate hops. Attacker back-runs by selling X and Y, extracting the spread. Only max_spread per hop constrains the victim's loss.

**Triage: Risk Accepted**

Standard AMM sandwich. Mitigations in place: max_spread enforced per hop by each pair contract (pair/src/contract.rs lines 824-878). min_return enforced globally by the router. For hybrid swaps, min_return is required per hop (router lines 203, 422). The limit book component reduces surface by allowing exact price limits. This risk applies to all AMMs on Terra Classic with no private mempool. Documented and accepted for small-TVL launch posture.

**No issue required.** (Reference: SEC-D05 covers quote advisory vs. settlement distinction.)

---

### H05 -- Fee-on-transfer CW20 drains limit order escrow

**Prompt:**

> The pair contract records an escrow amount when a limit order is placed using PENDING_ESCROW_TOKEN0 / PENDING_ESCROW_TOKEN1. The CW20 Receive handler credits the declared amount field from the CW20 Send message. If the CW20 deducts a fee on transfer, the pair receives less than declared but credits the full declared amount to escrow. When the order is cancelled and escrow is returned, the pair transfers the full declared amount from its token balance. Show whether enough repeated place-and-cancel cycles drain the pair's token balance below the sum of all PENDING_ESCROW values.

**Generated hypothesis:**

An attacker places 1,000 limit bid orders for 1,000 utoken each with a 1% fee-on-transfer token. Each transfer delivers 990 utoken. Total declared escrow: 1,000,000 utoken. Total actual balance: 990,000 utoken. Attacker triggers cancels on all 1,000 orders. The pair attempts to return 1,000 utoken per order. At order 991, the pair's real balance is exhausted.

**Triage: Real Gap**

The maker fee is taken at placement time (limit_placement.rs lines 183-287): maker_fee = declared_amount * maker_bps / 10000. PENDING_ESCROW records declared_amount - maker_fee. A CW20 Transfer message for total_maker_fee is queued to treasury in the same Response. So with declared_amount=1000, cw20 fee-on-transfer=1% (10 utoken), maker_fee=5 utoken:

- Pair receives: 990 (real balance)
- PENDING_ESCROW += 995 (declared - maker_fee)
- Treasury transfer queued: 5
- Pair balance after treasury transfer: 990 - 5 = 985
- Shortfall per order: 10 (the cw20 fee-on-transfer amount)

The maker fee being taken immediately reduces the escrowed amount correctly, but does not close the gap. The shortfall equals exactly the CW20 fee-on-transfer amount per order.

The CW20 code ID whitelist (factory/src/contract.rs CreatePair) gates on `code_id` at pair creation. GitLab [#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582) closes the post-listing `MsgMigrateContract` hole: pair instantiate pins live ids; write paths re-check pin + factory `IsCodeIdWhitelisted` (invariant **F6**). A fee-on-transfer implementation **behind a still-whitelisted and still-pinned** code_id remains an ops/audit gap — do not whitelist FoT templates. Balance-delta escrow assertion is **not** implemented (H-01).

**Action required for I02:** Document that whitelisted code IDs must be explicitly audited to confirm absence of fee-on-transfer and rebase mechanics. Post-listing `MsgMigrateContract` is mitigated on-chain by pin + whitelist re-check ([#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582)). Do **not** add balance-delta escrow math.

---

### H06 -- Governance key replay or spoofed factory message

**Prompt:**

> With the factory governance private key, show the minimum set of factory messages required to execute a full governance compromise: drain treasury, intercept swap fees, register malicious hooks, and lock out the legitimate team.

**Generated hypothesis:**

Attacker calls UpdateConfig (treasury = attacker_wallet, default_fee_bps = 9900). Calls AddHook to register attacker hook on all pairs. Calls AddWhitelistedCodeId for a backdoored CW20, then CreatePair to use it. Calls RotateGovernance to transfer control to attacker key.

**Triage: Covered**

Standard admin key compromise scenario (SEC-D10). Documented in docs/security-model.md and docs/operator-secrets.md. Governance intended as multisig for production. Emergency rotation procedure exists. Named risk for single hot wallet at small-TVL launch.

**No issue required.**

---

### H07 -- Signing modal shows symbol only -- malicious indexer substitutes pair contract

**Prompt:**

> The swap signing modal displays "Swap LUNC -> USTC" with amount, max_spread, and min_return but NOT the pair contract address. The frontend uses the indexer's router_operations to determine which pair contracts to call. A malicious indexer can return the correct token symbols but route through an attacker-controlled pair contract. Show what the attacker returns and what the user sees vs. what is actually submitted.

**Generated hypothesis:**

Malicious indexer returns router_operations pointing to terra1attacker_pair. Frontend validates token_in == LUNC_ADDRESS and token_out == USTC_ADDRESS (SwapPage.tsx line ~531). Both check out. Route is accepted. Signing modal shows "Swap LUNC -> USTC, max_spread 2%, min_return 95 USTC." User approves. Transaction calls attacker's pair contract.

Min_return is enforced on-chain: if attacker pair returns fewer tokens than min_return, the transaction reverts. However, if the attacker pair subtly skims an amount below the min_return threshold (within slippage tolerance), the user sees a successful swap but the attacker extracts the skim. The user has no way to verify the pair contract address before signing.

**Triage: Real Gap**

Confirmed by code review. SwapPage.tsx validates token_in and token_out only. SwapPreSubmitSummary.tsx (lines ~48, 70) shows symbols only. No pair contract address in the signing modal. A malicious VITE_INDEXER_URL can substitute pair contracts. Impact is bounded by min_return. Operator-secrets.md notes VITE_INDEXER_URL must be HTTPS-only and operator-controlled, but the signing modal does not surface the pair address for user verification. Maps to SEC-E07.

**Action required for I02:** File issue for pair contract address disclosure in signing modal.

---

### H08 -- Expert Mode persistence in localStorage enables repeated high-slippage swaps

**Prompt:**

> Expert Mode is enabled by typing a confirmation phrase once and persists in localStorage. Show how a social engineering attack uses this to cause a user to approve high-slippage swaps without per-transaction confirmation.

**Generated hypothesis:**

Attacker publishes a support article instructing users to enable Expert Mode to resolve "Slippage is too high" errors. User enables it. Attacker (controlling a low-liquidity pair or the indexer) causes a high-slippage swap to present without the 30% block. User submits.

**Triage: Risk Accepted**

Expert Mode requires explicit confirmation phrase typed by the user (ExpertModeModal.tsx line ~15) with a visible warning. Persistence is standard DEX design (Uniswap, Astroport). The social engineering vector exists for any DEX with this feature. Not a code defect. SEC-A01 user communication docs can note Expert Mode implications.

**No issue required.**

---

### H09 -- Indexer-controlled route mismatch between displayed path and submitted operations

**Prompt:**

> Show whether a malicious indexer can serve a response where the displayed intermediate tokens differ from the actual router_operations token path, and whether the frontend catches this mismatch before submission.

**Generated hypothesis:**

Malicious indexer returns intermediate_tokens: [tokenX] for display (user sees LUNC -> X -> USTC) but router_operations routing through LUNC -> MALICIOUS -> USTC. Frontend validates only token_in and token_out (SwapPage.tsx line ~531). Intermediate addresses are not re-derived from router_operations and compared against the displayed path. The enrichment function adds min_return per hop but does not cross-validate intermediate token addresses.

**Triage: Real Gap**

Confirmed by code review. swapRouteDisplay.ts deriveSwapSubmitRouteSource (lines 115-131) uses indexer operations for both display and submission without cross-validating intermediate token addresses. Impact bounded by min_return. Related to H07 -- the same trust-the-indexer problem applied to the route path rather than the terminal pair.

**Action required for I02:** File issue for route intermediate token verification before submission. May combine with H07 into one issue.

---

### H10 -- IPv6 /64 rotation bypasses per-IP rate limit on LCD-heavy routes

**Prompt:**

> IPv6 attackers can rotate through different /64 prefixes from a single /48 allocation. Each prefix gets its own 10 RPS allowance on LCD-heavy routes. Show the effective attack rate from a single /48.

**Generated hypothesis:**

From a /48 (65,536 /64 prefixes) each with 10 RPS: effective attack rate 655,360 RPS against limit-book routes that fan out multiple LCD queries per request.

**Triage: Risk Accepted**

Indexer defaults to IPv4-only (api/mod.rs Domain::IPV4). IPv6 only when API_IPV6_ENABLED=1 is explicitly set. operator-secrets.md documents the IPv6 risk. Operators enabling IPv6 must accept this or add additional controls (Cloudflare, /64-level bucketing). Acceptable for small-TVL launch with IPv4 default.

**No issue required.** (Reference: SEC-F04 covers IPv6 anti-abuse strategy documentation requirement.)

---

### H11 -- Concurrent limit-book depth walks fan out unbounded LCD queries

**Prompt:**

> Show whether a single request to limit-book with a large depth parameter fans out more than 1 LCD query, and whether the depth parameter is bounded.

**Generated hypothesis:**

depth=10000 triggers 10,000 LCD queries per request.

**Triage: Covered**

limit-book-shallow has a clamped depth parameter. limit-book uses cursor pagination (after_order_id), not a depth parameter. 10 RPS per-IP on LCD-heavy routes bounds fanout per client.

**No issue required.**

---

### H12 -- SQL injection via sort or interval query parameters

**Prompt:**

> An attacker submits GET /api/v1/pairs?sort=id;DROP TABLE pairs;-- or GET /api/v1/pairs/{addr}/candles?interval=1m';DROP TABLE candles;--. Show whether these strings reach raw SQL execution.

**Generated hypothesis:**

If sort or interval is interpolated directly into a format!() SQL string, the injected payload drops tables.

**Triage: Not Applicable**

All sort parameters are validated against a Rust enum; unrecognized values are rejected 400 before reaching the database. Interval parameters are validated against VALID_INTERVALS ["1m","5m","15m","1h","4h","1d","1w"]. All remaining user inputs use SQLx QueryBuilder with push_bind() parameterized binding. The format!() call in candle rebuild uses the match arm result, not user input. No SQL injection vectors identified.

**No issue required.**

---

### H13 -- Fake pair contract emits wasm events to inject synthetic volume

**Prompt:**

> You deploy a CosmWasm contract at terra1fake that emits events identical to a real pair's swap events. The indexer picks up these events and calls discover_new_pair(terra1fake). Show whether terra1fake gets indexed.

**Generated hypothesis:**

Attacker deploys fake pair. Indexer calls discover_new_pair and then verify_factory_provenance(factory, terra1fake, [LUNC, USTC]). Factory's pair query for (LUNC, USTC) returns the real pair address terra1real, not terra1fake. Provenance check fails. terra1fake is not indexed.

**Triage: Covered**

Factory provenance check in pair_discovery.rs (lines 157-199) queries the factory for the asset tuple and confirms the factory maps those assets to the exact pair address that emitted the event. A clone pair cannot pass this check.

**No issue required.**

---

### H14 -- Empty FACTORY_ADDRESS ships to production, skips provenance check

**Prompt:**

> Show what happens if an operator deploys the indexer to production with FACTORY_ADDRESS="", and how an attacker exploits this.

**Generated hypothesis:**

FACTORY_ADDRESS="" causes verify_factory_provenance to log a warning and return Ok(()). All contracts emitting swap events are indexed without provenance validation. Attacker deploys pair clone, emits swap events, indexer indexes synthetic volume. Operators and users see fake pair data.

**Triage: Real Gap**

The code path exists (pair_discovery.rs lines 163-168). RUN_MODE=prod requires non-empty FACTORY_ADDRESS, but if RUN_MODE is not set to prod, FACTORY_ADDRESS can be empty without rejection. No CI guard confirms non-empty FACTORY_ADDRESS in staging deploy scripts. Distinct from issue #442 (which covers address match verification, not the empty-address production guard).

**Action required for I02:** Add FACTORY_ADDRESS non-empty assertion to post-deploy verification or launch-checklist, independent of RUN_MODE.

---

### H15 -- Stale env address causes frontend to call wrong factory or router on mainnet

**Prompt:**

> During a mainnet migration, factory or router is redeployed. Operator updates indexer env but forgets to update VITE_FACTORY_ADDRESS or VITE_ROUTER_ADDRESS. Show what breaks and whether users lose funds.

**Generated hypothesis:**

Frontend sends blacklist_check to old factory. New pairs registered in new factory are not reflected in old factory's blacklist state. Swap submissions to old VITE_ROUTER_ADDRESS fail. No funds lost (transactions revert), but frontend is broken.

**Triage: Partially Covered**

Tracked under existing issue #442 (env-to-chain address comparison gap). If #442 is resolved with a script cross-checking VITE_* vars against on-chain state, this scenario is caught pre-deploy.

**No new issue required.** Tracked under #442.

---

## Novel hypotheses (not from the 10 historical themes)

---

### H16 -- Blacklist UI gate bypassed via 15-second React Query stale cache

**Hypothesis:** Wallet blacklisted between two swaps submitted within 15 seconds. Second swap submitted without re-checking blacklist. On-chain contract rejects. User loses gas only.

**Triage: Risk Accepted.** On-chain contract enforces blacklist at execution time regardless of UI state. 15-second stale window is a UX issue, not a security hole. Acceptable for small TVL.

---

### H17 -- Limit order partial fill followed by cancel returns full original escrow

**Hypothesis:** If execute_cancel_limit_order loads original order without fill-reduced remaining, user receives more than owed.

**Triage: Covered.** Orderbook fill function updates order.remaining in storage atomically during fill matching. Cancel reads current remaining (post-fill) from ORDERS storage. Underflow check (checked_sub) on PENDING_ESCROW prevents over-withdrawal regardless.

---

## Triage summary for SEC-I02

Items requiring follow-up action:

| ID | Action |
|---|---|
| H05 | Document whitelist code ID audit requirement: no fee-on-transfer, no rebase. Post-listing migrate is on-chain pin+recheck ([#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582), **F6**). Do not add FoT balance-delta math. |
| H07 | File issue: signing modal should display pair contract address alongside token symbols. |
| H09 | File issue (may combine with H07): route intermediate token addresses should be cross-validated between display path and submitted router_operations. |
| H14 | Add FACTORY_ADDRESS non-empty assertion to post-deploy verification or launch-checklist, independent of RUN_MODE. |

---

## Verification checklist (SEC-I01)

- [x] Prompt pack saved as `help/sec-i-prompt-pack.md`
- [x] All 10 historical exploit themes from checklist preamble addressed (T1-T10)
- [x] Each prompt parameterized with actual file paths and line numbers from this repo
- [x] Each hypothesis has a triage verdict: Covered / Not Applicable / Risk Accepted / Real Gap
- [x] Real Gap items mapped to required follow-up actions for SEC-I02
- [x] Novel hypotheses from codebase review included (H16, H17)
