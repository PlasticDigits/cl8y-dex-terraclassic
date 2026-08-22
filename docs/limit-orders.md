# Limit orders and hybrid swaps

This document is the implementation reference for the hybrid AMM + FIFO limit book. Canonical message shapes live in [`smartcontracts/packages/dex-common/src/pair.rs`](../smartcontracts/packages/dex-common/src/pair.rs).

## Swap page: hybrid vs pool-only estimates

<a id="swap-ui-hybrid-vs-pool-only-estimates"></a>

The Swap UI must show **before submit** whether execution is **hybrid (pool + limit book)** vs **pool-only**, and when submit includes a book leg the receive line must use the **same hybrid simulation** as execution (indexer `POST /route/solve` and/or LCD `hybrid_simulation` with matching `pool_input` / `book_input` — never pool-only `simulate_swap` while hybrid submit is configured; [GitLab **#418**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/418), L8 in [contracts-security-audit.md](./contracts-security-audit.md)). Implementation: [`frontend-dapp/src/pages/SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx), [`directHybridQuote.ts`](../frontend-dapp/src/utils/directHybridQuote.ts), and [`swapDisclosure.ts`](../frontend-dapp/src/utils/swapDisclosure.ts). Product/QA: [GitLab #111](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/111).

**MEV / mempool posture** (separate from hybrid routing): txs use the **public mempool** and **slippage** is the on-chain sandwich guard — no MEV-protection toggle in this build. Documented in [`docs/frontend.md#swap-mev-posture`](./frontend.md#swap-mev-posture) only (no Swap/Trade UI panel; [GitLab #299](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/299)). See also [GitLab #168](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/168) · [`skills/AGENTS_FRONTEND_MEV_POSTURE.md`](../skills/AGENTS_FRONTEND_MEV_POSTURE.md).

**Invariants**

- **Pool-only quotes** use `HybridSimulation` with `book_input = 0` ([`pool_only_hybrid_params`](../smartcontracts/packages/dex-common/src/pair.rs)); they do not walk the on-chain book.
- **Direct CW20 + “limit book leg” in Settings:** `pool_input` / `book_input` must sum to the pay amount; the same split is computed in one place for UI, simulation (`postRouteSolve` when used), and `swap` submit (`getDirectHybridBookSplit` vs [`HybridSwapParams`](../smartcontracts/packages/dex-common/src/pair.rs) fields).
- **Empty manual book (Swap + Trade, [#501](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/501), [#596](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/596)):** On Swap Settings and Trade market **Advanced**, an empty **Book leg** means no caller-declared override (`book_input = 0` for the Advanced POST path). **Default quoting** (always-on hybrid, empty book) uses indexer **`GET /api/v1/route/solve`**, which may still allocate an interior pool/book split. Typed book amount → Advanced override via [`quoteDirectHybridSwap`](../frontend-dapp/src/utils/directHybridQuote.ts) (`POST` with fixed `hybrid_by_hop`). Shared GET helper: [`cw20RouteSolveQuote.ts`](../frontend-dapp/src/utils/cw20RouteSolveQuote.ts). The official dApp has **no hybrid opt-out**; pool-only is integrator `pool_only=true` or a custom frontend ([`AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md`](../skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md)).
- **Execution notice when hybrid on + empty manual book ([#492](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/492)):** do **not** show “Pool only — add a book leg in Settings…”. Hybrid Settings already apply; pool-only is expected when there is no manual leg (and/or no takeable resting limits). Display helper: [`getDirectHybridSettingsExecutionSummary`](../frontend-dapp/src/utils/swapDisclosure.ts) returns `{ show: false }` — silence over instructional fluff ([`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](../skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md)).
- **Hybrid quote = execute ([#418](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/418), [#501](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/501), [#596](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/596)):** default Swap/Trade market quotes use GET solver hybrid params on submit (`hybridFromSingleHopIndexerOps`). When a **manual** book leg is active (Advanced), quotes use [`quoteDirectHybridSwap`](../frontend-dapp/src/utils/directHybridQuote.ts) (indexer `POST`, then LCD `hybrid_simulation` with the same split). There is **no** pool-only receive line with a configured manual book leg; retail hybrid is **always on** (no opt-in / opt-out checkbox).
- **Book leg amount input ([GitLab #169](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/169)):** Settings **Book leg override** and Trade market **Book leg override** use the same decimal draft validation as Swap **You Pay** — only digits and one `.`; invalid keys are rejected with no error UI. See [frontend.md § Decimal amount inputs](./frontend.md#decimal-amount-inputs) and [`decimalAmountInput.ts`](../frontend-dapp/src/utils/decimalAmountInput.ts).
- **Indexer routes with `indexer_hybrid_lcd` / `indexer_hybrid_lcd_degraded`:** the main panel shows an **“Indexer hybrid”** execution line (not only quote disclosure / the alert block).
- **Playwright hybrid E2E ([GitLab #193](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/193)):** default CI/local setup seeds a resting **bid** via [`scripts/e2e-seed-hybrid-book.sh`](../scripts/e2e-seed-hybrid-book.sh) and asserts on-chain `limit_order_fill` + positive `book_return_amount`; see [`docs/testing.md`](./testing.md) and [`skills/AGENTS_E2E_HYBRID_SWAP.md`](../skills/AGENTS_E2E_HYBRID_SWAP.md).
- **Playwright limit place/cancel tx E2E ([GitLab #195](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/195), strict chain [#201](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201)):** [`frontend-dapp/e2e/limit-orders-tx.spec.ts`](../frontend-dapp/e2e/limit-orders-tx.spec.ts) picks the first **unpaused** dual-CW20 factory pair (LCD `is_paused`), funds escrow via [`scripts/e2e-provision-dev-wallet.sh`](../scripts/e2e-provision-dev-wallet.sh), and **fails** (no env-only skip) when place/cancel cannot broadcast or wasm actions are missing. Pause semantics: invariant **L6** — [`docs/contracts-security-audit.md`](./contracts-security-audit.md); agent playbooks [`skills/AGENTS_E2E_LIMIT_ORDERS_TX.md`](../skills/AGENTS_E2E_LIMIT_ORDERS_TX.md), [`skills/AGENTS_E2E_STRICT_CHAIN.md`](../skills/AGENTS_E2E_STRICT_CHAIN.md).

**Not duplicated elsewhere:** if you change pay split rules, update `getDirectHybridBookSplit` and the `swap` mutation in `SwapPage` together. For merge/CI follow-up on this repo, the **babysit** Cursor agent skill (keep a PR merge-ready) is the intended loop.

## Exchange API “orderbook” vs on-chain limit book

CoinGecko/CoinMarketCap [`GET /cg/orderbook`](./CG_CMC_COMPLIANCE.md#get-cgorderbook) and [`GET /cmc/orderbook/:market_pair`](./CG_CMC_COMPLIANCE.md#get-cmcorderbookmarket_pair) return a **hybrid-simulated** level-2 book: AMM curve-walk levels **plus** merged resting FIFO limits (**#220**). That is still **not** a live CEX L2 feed — use `limit-book` below for the on-chain book walk without pool overlay.

**Resting limits** are on-chain: query the pair contract with `LimitOrder { order_id }` and `OrderBookHead { side }` via LCD or any CosmWasm client. After an expiry park, `LimitOrder` returns an error — use **`ExpiredLimitRefund { order_id }`** (`null` if already claimed).

**Typed custody status (GitLab [#505](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/505)):** contract callers that hold local `order_id`s should use **`OrderStatus { order_id }`** → `OrderStatusResponse` (`Active` / `ParkedRefund` / `Unknown`). This is a **successful** JSON response even when the id has no custody row (`Unknown`). Do **not** treat LCD/transport/`StdError` failures as `Unknown`. Do **not** treat `Unknown` as proof of fill — filled, cancelled, claimed, and never-placed all share that bucket; vaults must keep their own cancel/fill ledger. `order_id == 0` errors. Existing `LimitOrder` / `ExpiredLimitRefund` semantics are unchanged. Agent playbook: [`skills/AGENTS_ORDER_STATUS_QUERY.md`](../skills/AGENTS_ORDER_STATUS_QUERY.md); invariant **L21** in [contracts-security-audit.md](./contracts-security-audit.md).

The **indexer also proxies** book walks for integrators and the dApp (see [ADR 0002: Limit book surfacing](./adr/0002-limit-book-surfacing.md)):

- **`GET /api/v1/pairs/{addr}/order-book-head?side=bid|ask`** — JSON `{ "head_order_id": <u64> | null }` from LCD `OrderBookHead`.
- **`GET /api/v1/pairs/{addr}/limit-book?side=bid|ask&limit=L&after_order_id=OPTIONAL`** — **paginated** walk from head or keyset cursor along `next` (default `limit` 50, max 100 per HTTP response). Optional **`price_from`** + **`price_to`** return the contiguous in-band slice only ([GitLab **#267**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/267); [integrators.md § Insert hints & price window](./integrators.md#insert-hints-price-window-gitlab-267)). Response includes `orders`, `has_more`, and `next_after_order_id` (pass the latter as `after_order_id` for the next page). LCD errors → **502**; invalid cursor / side mismatch → **400**.
- **`GET /api/v1/pairs/{addr}/limit-book/insert-hints?side=bid|ask&prices=p1,p2,...`** — batch **insert-hint** resolution in one walk (max **100** prices); never guesses across pagination gaps ([#267](./integrators.md#insert-hints-price-window-gitlab-267)).
- **`GET /api/v1/pairs/{addr}/limit-book-shallow?side=bid|ask&depth=N`** — legacy small preview walk (default depth 10, max 20); prefer `limit-book` for full depth.

**Trader-scoped history (fills, cancellations, swaps on a pair, CSV):** [GitLab **#163**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/163) — `GET /api/v1/traders/{addr}/limit-fills`, `.../limit-cancellations`, `.../trades` with optional `pair=` and `format=csv`. Invariants: [`indexer-invariants.md`](./indexer-invariants.md); dApp: [`frontend.md` § Wallet swap and limit history](./frontend.md#wallet-swap-limit-history), [`skills/AGENTS_FRONTEND_ORDER_HISTORY.md`](../skills/AGENTS_FRONTEND_ORDER_HISTORY.md).

For multihop routing the indexer exposes route discovery via [`GET /api/v1/route/solve`](./indexer-invariants.md) (**hybrid-aware by default** when `amount_in` is set, max **4 hops**; legacy **`pool_only=true`** — GitLab [**#191**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/191), [**#101**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/101), [**#323**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/323)) and **hybrid merge + router quote** via [`POST /api/v1/route/solve`](./indexer-invariants.md) when the client sends `hybrid_by_hop` aligned with the discovered hops (see ADR 0001).

## Messages (CosmWasm)

### Swap with Pattern C (`Cw20HookMsg::Swap`)

- **`hybrid`:** optional [`HybridSwapParams`](../smartcontracts/packages/dex-common/src/pair.rs): `pool_input`, `book_input` (must sum to the CW20 `amount`), `max_maker_fills`, optional `book_start_hint` (order id).
- **Match walk:** If `book_start_hint` is set and that order id still exists **on the side being matched** (bid hint for `match_bids`, ask hint for `match_asks`), matching starts from that id; wrong-side, stale, or missing hints fall back to the book head with no error. Each walked order must match the active matcher side (defense in depth — GitLab [**#272**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/272)). See invariant **L17** in [contracts-security-audit.md](./contracts-security-audit.md); agent playbook [skills/AGENTS_BOOK_MATCH_HINT_SECURITY.md](../skills/AGENTS_BOOK_MATCH_HINT_SECURITY.md).

### Place / cancel limit (GitLab [#206](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/206))

- **`Cw20HookMsg::PlaceLimitOrderBatch`:** `side`, `orders[]` each with `price`, `amount` (gross escrow for that rung), `max_adjust_steps`, optional `expires_at`, optional **`hint_after_order_id`** (GitLab **#261** — per-rung predecessor for O(1) insert; explicit client hint wins over internal batch chaining from the prior successful rung). The CW20 `send` **amount must equal the sum** of per-rung `amount` values. **Same side per batch** (bid escrows token1, ask escrows token0). **Validation is all-or-nothing** (empty batch, cap, amount mismatch, invalid price/expiry/**price band** ([#467](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/467)), maker fee too large → whole tx reverts). **Book-walk is partial** ([#206](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/206) design note): if a rung hits `LimitInsertStepsExceeded`, that rung is skipped, escrow for skipped rungs is **CW20-refunded** to the maker, and later rungs still attempt; if **no** rung places, the tx reverts (`LimitBatchNoRungsPlaced`).
- **`Cw20HookMsg::PlaceLimitOrderLadder`:** `ladder` with `start_price`, `end_price`, `count` (≥ 2), `total_amount`, `distribution` (`equal`), shared `max_adjust_steps` / `expires_at`, optional **`hint_after_order_id`** on the **head-most rung in book order** (GitLab **#266** — boundary anchor for deep-book placement; resolved client-side via indexer **#267**). Expanded on-chain to the same rules as batch.
- **Batch wasm attrs:** `action=place_limit_order_batch`, `batch_count`, `batch_skipped_count`, `batch_refund_amount`, then one `action=place_limit_order` per successful rung. On-chain, CosmWasm often emits **columnar** attrs (repeated `action` keys, then parallel `order_id` / `price` / … columns); the indexer zips those into one `limit_order_placements` row per rung (see `parse_limit_order_placements_columnar` in `indexer/src/indexer/parser.rs`).
- **Gas (dApp estimate):** one `increase_allowance` + one CW20 `send` → batch/ladder hook. Limits in [`terraGas.ts`](../frontend-dapp/src/services/terraclassic/terraGas.ts): base `400_000` + `180_000` × rung count vs **N** separate placements at ~`950_000` each — see [§ Batch / ladder gas savings](#batch-ladder-gas-savings).
- **Rung cap (governance):** each pair stores `max_batch_rungs` (query `limit_order_config`). Factory governance sets defaults via `UpdateConfig { default_limit_batch_max_rungs }` and per-pair caps via `SetPairLimitBatchMax`. Hard ceiling: `MAX_LIMIT_BATCH_RUNGS_HARD_CAP` (**100**; LocalTerra gas — GitLab [**#263**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/263)) in `dex-common`.
- **Retail single order:** use batch with `orders.len() == 1` (dApp path). Optional **`hint_after_order_id`** on the batch item when the client knows the predecessor ([GitLab **#261**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/261)).
- **Batch placement storage (GitLab #247):** `execute_place_limit_orders_batch` reserves all rung ids with one `ORDER_NEXT_ID` write and accumulates `PENDING_ESCROW_*` in memory, saving once per token side touched after the loop (skipped rungs still consume ids — same gap semantics as sequential singles). Helpers: `reserve_order_id_block`, `insert_bid_with_id` / `insert_ask_with_id` with `update_escrow: false` in [`orderbook.rs`](../smartcontracts/contracts/pair/src/orderbook.rs).
- **`MAX_ADJUST_STEPS`:** each rung uses its own `max_adjust_steps` when walking from the book head. Hard caps: `MAX_ADJUST_STEPS_HARD_CAP` / `MAX_MAKER_FILLS_HARD_CAP` in `dex-common::pair`.
- **Fees:** Total limit-book fee rate matches the pair’s **effective** swap commission (`fee_bps` after the optional fee-discount registry). The pair charges **half** to the maker at placement (from the escrowed CW20, sent to `treasury`; the resting order’s `remaining` is reduced) and **half** on each book fill (taker leg), same notional bases as before (bids: token1 `cost`; asks: token0 fill). **Placement** uses `limit_discount_bps` when set ([#514](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/514), invariant **I13**); the **taker** half on a fill still uses the taker’s swap `discount_bps` (unregistered retail still pays half of the full pair fee). See [`docs/reference/fee-discount-tiers.md`](./reference/fee-discount-tiers.md) and [`docs/integrators.md`](./integrators.md).
- **`hint_after_order_id` / batch insert hints (GitLab #256, per-rung wire #261, directional fallback #265, book-order traversal #266):** optional predecessor order id for **O(1) insertion** when verified. On placement (`insert_bid` / `insert_ask`), price update (`UpdateLimitOrderPrice`), and batch/ladder placement (each rung may pass **`hint_after_order_id`**; ladder also accepts an optional **boundary anchor** on `LimitOrderLadderSpec`; batch inserts rungs in **book-sort order** (composite `(price, order_id)`) while preserving **input/id assignment** and emitting attrs in input order — interior rungs chain a **thread cursor** from the prior successful insert for O(0)-load verify when the next key brackets the cursor), the pair loads the hinted order (+ at most its `next` neighbor) and accepts the hint only when: the order exists on the correct side and is still linked in the DLL; the new `(price, order_id)` sorts **after** the hint and **before** the hint’s `next` (or tail). When O(1) verify fails but the hint is still a **valid anchor**, the pair walks **`prev` toward head** (new sorts before hint) or **`next` toward tail** from `hint.next` (new sorts after hint’s successor) within the same `max_adjust_steps` budget — no second head walk. **Stale** (id absent), **wrong-side**, or **unlinked** hints fall back to bounded **head** walk (`LimitInsertStepsExceeded` unchanged). Malicious hints cannot weaken ordering — worst case equals today’s bounded head walk. Integrators with book topology (indexer `limit-book` walk, dApp deep book) should pass the expected predecessor id on single-rung batch items, ladder boundary rungs, and price updates. dApp resolver: [`limitBookInsertHint.ts`](../frontend-dapp/src/utils/limitBookInsertHint.ts) ([GitLab **#261**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/261), ladder anchor via **#267**). See invariant **L14** in [contracts-security-audit.md](./contracts-security-audit.md); agent playbooks [skills/AGENTS_FRONTEND_LIMIT_ORDER_PLACEMENT_GAS.md](../skills/AGENTS_FRONTEND_LIMIT_ORDER_PLACEMENT_GAS.md), [skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md](../skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md), [skills/AGENTS_FRONTEND_DEEP_ORDER_BOOK.md](../skills/AGENTS_FRONTEND_DEEP_ORDER_BOOK.md).
- **`ExecuteMsg::CancelLimitOrder`:** `order_id`. Only the stored **owner** may cancel. Applies only while the order row exists in on-chain `LimitOrder` storage (not after an expiry park — use claim below).
- **`ExecuteMsg::CancelLimitOrders`:** `order_ids: Vec<u64>` (≤ pair `max_batch_rungs`, hard cap **100**). **All-or-nothing:** every id must belong to `info.sender`; duplicate ids revert; any failure reverts the whole tx (unlike batch placement’s partial book-walk skip). Refunds **aggregate** into at most **two** CW20 transfers (token0 asks + token1 bids). Wasm summary: `action=cancel_limit_orders_batch`, `batch_count`, then columnar `cancel_limit_order` / `limit_order_cancelled` / `owner` per id ([GitLab **#246**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/246)).
- **`ExecuteMsg::ClaimExpiredLimitOrder`:** `order_id`. Owner-only. After an order is **parked** into `EXPIRED_LIMIT_CLAIMS` (TTL expiry, dust flush, blacklist, or force-clean — see [§ Park reason](#expired-limit-park-reason-gitlab-504)), escrow is refunded here — same token routing as cancel. Query **`ExpiredLimitRefund { order_id }`** for a pending row (`null` if none); includes **`reason`** ([#504](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/504)). **Blocked while the pair is paused** (same emergency gate as **`CancelLimitOrder`** — GitLab [**#120**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120)).
- **`ExecuteMsg::ClaimExpiredLimitOrders`:** `order_ids: Vec<u64>` — same cap, owner-only, all-or-nothing, and ≤ 2 CW20 refund transfers as batch cancel. Reasons may be mixed; claim is reason-agnostic. Summary attrs: `action=claim_expired_limit_orders_batch`, `batch_count`, columnar `claim_expired_limit_order` / `order_id` ([GitLab **#246**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/246)).
- **`ExecuteMsg::UpdateLimitOrderPrice`:** `order_id`, `price`, `hint_after_order_id`, `max_adjust_steps`. Owner-only; re-links the order in the FIFO book at a new price **without** charging the maker placement fee again (same `order_id` and `remaining`). **`PENDING_ESCROW_*` unchanged** (no token movement). Blocked while paused (**L6**). Rejects expired orders. dApp: book **Edit** → change price only → one tx via [`updateLimitOrderPrice`](../frontend-dapp/src/services/terraclassic/pair.ts) ([GitLab **#247**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/247)); size/side/expiry changes still require cancel + place.
- **`ExecuteMsg::CleanLimitBook`:** `side`, `max_orders` (1…`MAX_LIMIT_CLEAN_ORDERS_HARD_CAP` = **100**), optional **`start_hint`** (order id — if absent or not on-book for that side, walk from head), optional **`max_steps`** (nodes visited; absent or `0` → **`MAX_CLEAN_SCAN_STEPS` (500)** — mirrors matcher scan sizing, [#274](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/274)). **Permissionless** (any `sender`); **no CW20 movement** — parks eligible rows into `EXPIRED_LIMIT_CLAIMS` via the same path as taker expiry (**L1**). **Blocked while paused** (**L6**). Eligibility: (1) **time-expired** — `expires_at` set and `block_time >= expires_at`; (2) **force-clean** — when governance sets `min_remaining_token0` / `min_remaining_token1` (query `limit_clean_config`) **above zero**, live orders with `remaining` **below** the side threshold (token0 for asks, token1 for bids); force-clean does **not** require `expires_at` and clears it on the parked refund row. Default thresholds **0 / 0** → only time-expired orders. Wasm summary: `action=clean_limit_book`, `cleaned_count`, `time_expired_count`, `force_expired_count`, `cap_hit`, optional **`scan_capped=true`**, optional **`resume_cursor`** (order id to continue when traversal budget exhausted before list end). Per park still emits **`limit_order_expired_parked`** (optional `force_expired=true`). **Not** factory CW20 excess recovery — see [`ExecuteMsg::Sweep`](./limit-orders.md#factory-cw20-excess-sweep) below. Implementation: [`limit_book_clean.rs`](../smartcontracts/contracts/pair/src/limit_book_clean.rs) ([GitLab **#263**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/263), bounded walk [#274](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/274)).
- **`ExecuteMsg::UpdateLimitCleanConfig`:** `min_remaining_token0`, `min_remaining_token1`. Factory-only (same auth as `UpdateLimitOrderConfig`). Factory governance wrapper: `SetPairLimitCleanConfig`.

<a id="factory-cw20-excess-sweep"></a>

### Factory CW20 excess recovery (`Sweep`) vs limit book clean

| Message | Who | Purpose |
|---------|-----|---------|
| **`ExecuteMsg::Sweep`** (pair) | Factory only | Recover **donated / accidental** CW20 above `reserves + pending_escrow` — unrelated to limit orders |
| **`ExecuteMsg::CleanLimitBook`** | Anyone | Park **expired / dust** limit rows into `EXPIRED_LIMIT_CLAIMS` — makers claim later |

Integrators: [integrators.md § Limit book clean](./integrators.md#limit-book-clean-gitlab-263).

<a id="permissionless-limit-book-clean"></a>

### Permissionless limit book clean (GitLab #263)

**When to call:** expired backlog at the book head (hybrid swap parks ≤ 15/side), after mass expiry, or when governance enables dust thresholds. **Traversal:** each tx visits at most **`MAX_CLEAN_SCAN_STEPS` (500)** nodes; when `scan_capped=true`, resubmit with **`start_hint`** / emitted **`resume_cursor`** until the side is clear ([#274](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/274)). **Gas:** write-heavy (unlink + claim row + event per order); benchmark ≤ **100** parks per tx on LocalTerra.

**Future watcher (docs only — no in-repo bot):** an indexer/cron can poll `GET .../limit-placements?status=parked_expired` and `limit-book` head walks; when expired-at-head count or parked backlog exceeds a policy threshold, submit `CleanLimitBook` with `start_hint` from the last walk cursor. Playbook: [`skills/AGENTS_LOCALNET_TRADING_SWARM.md`](../skills/AGENTS_LOCALNET_TRADING_SWARM.md).

### Router

- Each `SwapOperation::TerraSwap` may include `hybrid: Option<HybridSwapParams>` (same fields as the pair hook). `None` is legacy pool-only.
- **`SimulateSwapOperations` / `ReverseSimulateSwapOperations`:** when `hybrid` is unset on a hop, the router still queries pair **`HybridSimulation`** / **`HybridReverseSimulation`** with pool-only params (same as `pool_only_hybrid_params` / `pool_only_hybrid_template`). When `hybrid` is set, legs must sum to the per-hop offer and the router passes those params through. See [contracts-security-audit.md](./contracts-security-audit.md) invariant **L8** and [GitLab #190](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/190).

### Pair quoting (removed legacy queries)

- Legacy `Simulation` / `ReverseSimulation` queries were **removed** ([#190](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/190)). Integrators must use **`hybrid_simulation`** / **`hybrid_reverse_simulation`** only. Agent playbook: [`skills/AGENTS_HYBRID_QUOTING.md`](../skills/AGENTS_HYBRID_QUOTING.md).
- **`HybridReverseSimulation` search bounds ([GitLab #257](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/257)):** The pair seeds the upper offer from constant-product reverse math on the pool leg (`hybrid_reverse.rs`), then binary-searches for the **minimal** total offer whose hybrid return ≥ `ask_asset.amount`. Full hybrid sims are capped at **`MAX_HYBRID_REVERSE_SIM_CALLS` = 32** per query (replaces the prior 128-step exponential ramp). Fee discount is still resolved **once** per query ([#238](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/238)). Regression: `limit_order_tests::hybrid_reverse_sim_minimal_offer_invariant`.

### Pause (governance)

- When the pair is **paused**, `Receive` is blocked (no swap, no new limit orders), **`CancelLimitOrder` is blocked**, **`ClaimExpiredLimitOrder` is blocked**, and **`CleanLimitBook` is blocked** (including force-clean) — active resting limits and parked expired refund rows stay locked until governance unpauses (see [contracts-security-audit.md](./contracts-security-audit.md) **L6**, GitLab [**#120**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120), [**#263**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/263)).
- **`IsPaused` query:** `{ "is_paused": {} }` → `{ "paused": bool }` so frontends can show accurate pause copy without guessing from failed transactions.

### Blacklisted maker (GitLab #468)

<a id="blacklisted-maker-gitlab-468"></a>

When governance **`BlacklistWallet`** blocks a maker **after** they already have resting limits:

- The maker cannot place, cancel, claim, or update (same as other wallet-blacklist gates).
- **Hybrid match walks** (`match_bids` / `match_asks`) probe each stepped order's `owner` via factory `BlacklistCheck` — blacklisted makers are **never filled** and do not receive offer-token CW20 in the taker tx.
- When park budget allows (`MAX_EXPIRED_PARKS_PER_SWAP`), the order unlinks into **`EXPIRED_LIMIT_CLAIMS`** via `park_limit_order_for_clean(..., reason=Blacklisted)` — same escrow accounting as time-expiry (**L1**). The maker claims after **`UnblacklistWallet`**. Query/event **`reason=Blacklisted`** / `blacklisted` (GitLab [#504](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/504)).
- **`HybridSimulation`** skips blacklisted makers read-only (no park) so quotes align with execute for the same chain snapshot.
- Clean-wallet takers may still swap against non-blacklisted makers behind a blacklisted head row.

Regression: `blacklist_tests::blacklisted_maker_resting_limit_not_filled_taker_can_still_swap`. Invariant **L19** in [contracts-security-audit.md](./contracts-security-audit.md).

<a id="expired-limit-park-reason-gitlab-504"></a>

### Park reason discriminator (GitLab #504)

`EXPIRED_LIMIT_CLAIMS` / `ExpiredLimitRefund` / `ClaimExpiredLimitOrder*` keep historical “expired*” names, but **a parked refund row does not mean the order went unfilled**. Match-time dust flush, blacklist park, and governance force-clean share the same claim map.

| `reason` (query JSON / wasm) | Rust variant | Typical `expires_at` | Historical `force_expired` | Meaning |
|------------------------------|--------------|----------------------|------------------------------|---------|
| `expired` | `Expired` | `Some(..)` | absent | TTL reached; usually unfilled (or leftover after earlier partial fills still on book) |
| `dust_filled` | `DustFilled` | `None` | `true` | Post-fill remainder &lt; **10** units — order traded to near-completion (#264) |
| `force_cleaned` | `ForceCleaned` | `None` | `true` | Governance dust threshold via `CleanLimitBook` (#263) |
| `blacklisted` | `Blacklisted` | `None` | `true` | Maker wallet blacklisted during match walk (#468) |
| omitted / `null` | n/a | varies | varies | Pre-#504 row — **unknown**; do not infer fill vs expiry |

JSON wire uses **snake_case** (`#[cw_serde]`), matching wasm attrs — not PascalCase.

**Invariants for integrators**

1. Prefer **`reason`** over `expires_at` and over wasm **`force_expired`**.
2. **`force_expired=true` means “parked though not a TTL expiry”** — inverted vs a naive reading. It is kept for indexer back-compat; new consumers should read `reason`.
3. **`expires_at == null` is multi-way** (DustFilled / ForceCleaned / Blacklisted) — never treat null as “filled”.
4. Claim economics are unchanged: refund **`remaining`** only; `reason` is observability. No lifetime `filled_amount` on the pair (serve fills from indexer history if needed).
5. Set at each park call site — never re-derive reason from `(force_expired, expires_at)` alone.

Agent playbook: [`skills/AGENTS_EXPIRED_LIMIT_PARK_REASON.md`](../skills/AGENTS_EXPIRED_LIMIT_PARK_REASON.md). Types: [`ExpiredLimitParkReason`](../smartcontracts/packages/dex-common/src/pair.rs).

### Expiry (`expires_at`)

- If **`expires_at`** is set and a hybrid (or future) match walk reaches that order when **`block_time >= expires_at`**, the contract **does not** match it. The order is **removed from the DLL**, a row is stored in **`EXPIRED_LIMIT_CLAIMS`** with **`reason=Expired`**, and **`PENDING_ESCROW_*` is left unchanged** until the maker calls **`ClaimExpiredLimitOrder`** (which CW20-transfers and then decrements pending — same economics as cancel).
- **Park cap ([GitLab #250](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/250), raised [#254](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/254), benchmarked [#309](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/309)):** Each `match_bids` / `match_asks` walk during hybrid swap parks at most **`MAX_EXPIRED_PARKS_PER_SWAP` (15)** expired orders (`dex-common`). Additional expired orders at the book head are **skipped without storage writes** (`cur = next_ptr; continue`) until a later swap parks them or the maker **`CancelLimitOrder`** / **`ClaimExpiredLimitOrder`** (after a park). Swap attrs when the cap bites: `expired_parks_used`, `expired_parks_capped=true`, `expired_parks_skipped`. **`max_maker_fills`** still limits profitable fills only; expired parks do not increment `makers_used`. Skipped expired rows may remain visible on LCD/indexer book APIs until parked elsewhere — see invariant **L5** in [contracts-security-audit.md](./contracts-security-audit.md) and [skills/AGENTS_TERRACLASSIC_GAS.md](../skills/AGENTS_TERRACLASSIC_GAS.md).
- **Scan step budget ([GitLab #254](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/254), raised [#262](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/262)):** Each book walk counts **every** doubly-linked list iteration (fills, parks, skips, zero-remaining continues) against **`MAX_SCAN_STEPS` (500)** — decoupled from **`MAX_MAKER_FILLS_HARD_CAP` (100)**. Sized for ~**19k gas per step** vs a **15M** dApp envelope (~789 steps) with conservative margin. When the budget is exhausted, the walk stops early; unfilled book budget rolls to the pool leg (existing spread rules). Swap attr: `scan_steps_capped=true`. **`HybridSimulation`** applies the same step budget so quotes match execute when the cap binds.
- **Head-clog griefing (low severity, [GitLab #289](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/289)):** An adversary can stack **> `MAX_SCAN_STEPS`** expired orders at the book head so a **head-only** hybrid taker (no **`book_start_hint`**) hits the scan cap before live liquidity — book leg under-fills; pool spillover or slippage revert. **Mitigations (accepted, no matcher change in #289):** (1) **Takers** pass a **same-side** **`book_start_hint`** past the expired prefix (**L17**, [#272](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/272)) — [`resolve_match_start_hint`](../smartcontracts/contracts/pair/src/orderbook.rs); (2) **Keepers** clear backlog with resumable **`CleanLimitBook`** (**#274**); (3) **Small clogs** (≤ park cap + scan budget) clear in one swap (`hybrid_walk_three_expired_bids_all_parked_then_fills_live_bid`). **Future (docs only):** optional per-order placement fee to treasury if spam becomes material; re-benchmark **`MAX_EXPIRED_PARKS_PER_SWAP`** on pair wasm redeploy ([#309](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/309)).
- The taker transaction emits a wasm event **`limit_order_expired_parked`** (`action`, `order_id`, `maker`, `side`, `remaining`, **`reason`**, optional `force_expired`). **No** CW20 is sent to the maker in that taker tx — this keeps `balance − reserves − pending_escrow` aligned and fixes the stranded-funds / mis-sweep issue described in GitLab [**#120**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120).
- **`CancelLimitOrder`** only operates on **`ORDERS`**; it cannot fire after a park, so there is **no** path to refund the same escrow twice.
- While an order is still live on the book but past `expires_at`, the owner may **`CancelLimitOrder`** if the pair is unpaused — useful before any taker walks the book.

<a id="expired-park-benchmark-gitlab-309"></a>

#### Expired-park benchmark (GitLab #309)

**Goal:** evidence-based validation of **`MAX_EXPIRED_PARKS_PER_SWAP` (15)** vs Terra Classic **15M** dApp gas ceiling and wasm tx size after post-#254 orderbook features (dust flush #264, scan cap #254, adjust steps #265).

**Methodology**

| Item | Value |
|------|-------|
| Build | `make build-optimized` (CosmWasm optimizer wasm) + `make deploy-local` |
| Book setup | `N` expired bids at book head (`expires_at` in the near future); chain time advanced past expiry |
| Swap | Book-only hybrid: `pool_input = 0`, `book_input = 50_000`, `max_maker_fills = 8` (CW20 `send` → pair `swap`) |
| Isolation | One fresh factory pair per sweep case (`VERIFY309_PAIR_INDEX` base + case offset) |
| Measure | `gas_used`, `limit_order_expired_parked` event count, serialized tx bytes, swap attrs `expired_parks_*` |
| Acceptance ceiling | `gas_used < 12_000_000` (~20% headroom under **15M**); tx bytes `< 1_048_576` |

**LocalTerra results (optimized wasm, 2026-06-05)**

| N expired at head | gas_used | park events | tx bytes | parks_used | skipped |
|-------------------|----------|-------------|----------|------------|---------|
| 1 | 413_250 | 1 | 1_623 | 1 | 0 |
| 5 | 529_043 | 5 | 1_623 | 5 | 0 |
| 10 | 673_976 | 10 | 1_623 | 10 | 0 |
| 15 | 819_194 | 15 | 1_624 | 15 | 0 |
| 20 | 837_841 | 15 | 1_624 | 15 | 5 |
| 25 | 846_429 | 15 | 1_624 | 15 | 10 |
| 30 | 854_701 | 15 | 1_624 | 15 | 15 |

Marginal cost on optimized wasm: **~29k gas per park** (storage unlink + `EXPIRED_LIMIT_CLAIMS` write + wasm event); **~3.7k gas per skipped** expired head (pointer advance only). Isolated worst case at the current cap (**15** parks, `N ≥ 15`) ≈ **855k** `gas_used` — far below the **12M** headroom target.

**Chosen cap: retain 15.** Raising parks is gas-feasible in isolation, but (1) **`MAX_SCAN_STEPS` (500)** is the binding traversal budget on deep expired prefixes — additional head orders beyond **15** are skipped cheaply and clear on the next taker swap or via maker cancel/claim; (2) combined worst-case hybrid envelope with **`MAX_MAKER_FILLS_HARD_CAP` (100)** + full scan budget is already budgeted offline at **~7.7M** ([#262](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/262)); extra parks add **~29k** each on-chain vs **8k** in the dApp offline formula (conservative). **Governance:** changing the on-chain constant requires pair wasm redeploy.

**Regression / reproduction**

- Integration: `limit_order_tests::{expired_parks_benchmark_event_counts_sweep, hybrid_walk_at_cap_parks_all_expired_bids_without_capped_attr, hybrid_walk_pool_only_swap_has_no_expired_park_attrs}` (`cargo test expired_parks_benchmark`).
- Live stack: `make verify-issue-309` (requires `make start` + `make deploy-local`).

### Post-swap hooks and hybrid

- For hybrid swaps, `AfterSwap.return_asset.amount` is the **total** output (book + pool legs). `AfterSwap.commission_amount` is the **total protocol commission** in the ask asset (pool + book taker fees); `spread_amount` is **pool leg only**. Book-side fees are also on `limit_order_fill` events and swap attrs (`book_commission_amount` when book leg > 0). See invariant **L7** in [contracts-security-audit.md](./contracts-security-audit.md), [integrators.md](./integrators.md), and [hooks README](../smartcontracts/contracts/hooks/README.md) (GitLab [#196](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/196)).

## Ordering (composite key, FIFO)

For each side, the book is a strict total order:

- **Price** is **token1 per token0** (same convention as the pool).
- **Bids:** sort by **descending** `price`, then **ascending** `order_id` (higher price first; at equal price, **lower** `order_id` is ahead in the queue — older orders first).
- **Asks:** sort by **ascending** `price`, then **ascending** `order_id` (lower ask price first; FIFO at equal price by `order_id`).

## Execution order in `execute_swap`

When `hybrid` is set: the pair consumes the **book leg** first (up to `max_maker_fills` distinct makers), then routes the **pool leg** (including any book remainder rolled per contract logic). Hooks, spread checks, and fee discount (`trader`) follow the existing swap path. The **pool** leg uses full **`effective_fee_bps`**; each **book fill** charges the **taker half** of `effective_fee_bps` on the fill notional (maker half was paid at order placement). The swap response still exposes a single `effective_fee_bps` attribute for the taker context.

**CW20 transfer aggregation ([GitLab #248](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/248)):** Book match (`match_bids` / `match_asks`) accumulates payouts in memory — `maker_payouts` (offer token, one entry per distinct maker owner), `return_net`, and `commission_total` (ask token). After the pool leg, `execute_swap` builds aggregated submessages in order: **maker payouts** (offer token) → **one ask-token transfer to the receiver** (`book_return + pool_return`) → **one ask-token transfer to treasury** (`book_commission + pool_commission`). Zero amounts omit the transfer. Per-fill **`limit_order_fill`** wasm events are unchanged (indexer/analytics). Invariant **L10** in [contracts-security-audit.md](./contracts-security-audit.md); agent playbook [skills/AGENTS_TERRACLASSIC_GAS.md](../skills/AGENTS_TERRACLASSIC_GAS.md).

**Pending-escrow storage batching ([GitLab #255](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/255)):** During book match, `PENDING_ESCROW_TOKEN1` (bid fills) and `PENDING_ESCROW_TOKEN0` (ask fills) are decremented **once per token side per invocation** — fill costs accumulate in memory during the loop, then a single `checked_sub` + save runs after the walk (same final balance as sequential per-fill subtracts; underflow errors unchanged). **Dust flush ([#264](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/264)):** sub-threshold post-fill remainders add to the same batched subtract before save. Placement, cancel, claim, and time-expiry park paths are unchanged. `simulate_match_*` does not touch escrow. Invariant **L13** in [contracts-security-audit.md](./contracts-security-audit.md); agent playbook [skills/AGENTS_TERRACLASSIC_GAS.md](../skills/AGENTS_TERRACLASSIC_GAS.md).

<a id="match-time-dust-flush-gitlab-264"></a>

### Match-time dust flush (GitLab #264)

Integer rounding during hybrid book fills can leave **1–9 smallest-unit** remainders on resting limits (`floor(fill × price)` on bids). Those rows are economically exhausted but previously stayed in `ORDERS` until cancel or **`CleanLimitBook`** ([#263](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/263)).

**Protocol constant:** `LIMIT_ORDER_DUST_FLUSH_THRESHOLD = 10` in [`dex-common::pair`](../smartcontracts/packages/dex-common/src/pair.rs) (not governance-configurable in v1).

**Execute:** After each successful fill in `match_bids` / `match_asks`, when **`0 < remaining < 10`** (token1 for bids, token0 for asks):

1. Fill batched subtract already decremented **`cost`** from **`PENDING_ESCROW_*`** (**L13**); dust **`remaining`** is unchanged at park (same **L1** economics as time-expiry park — maker claim decrements pending).
2. **`park_limit_order_for_clean(..., reason=DustFilled)`** — unlink `ORDERS`, store dust in **`EXPIRED_LIMIT_CLAIMS`**; **no** CW20 in the swap tx.
3. Emit **`limit_order_expired_parked`** with **`reason=dust_filled`** and **`force_expired=true`** (indexer → `parked_expired` lifecycle today; prefer `reason` once parsed — [#504](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/504)).

**Unchanged:** `remaining = 0` → unlink only (**no** claim row); **`remaining ≥ 10`** → partial save. Dust flush **does not** count toward **`MAX_EXPIRED_PARKS_PER_SWAP` (15)** — only time-expired / blacklist parks during the walk do.

**Integrator footgun (#504):** do **not** treat a parked row as “expired unfilled.” Use query/event **`reason=dust_filled`**. See [§ Park reason discriminator](#expired-limit-park-reason-gitlab-504).

**Simulation:** `simulate_match_*` zeroes in-memory sub-threshold remainders so **`HybridSimulation`** quotes match execute (**L8**).

**Maker recovery:** **`ClaimExpiredLimitOrder`** / batch claim — same pause gate (**L6**) and CW20 routing as time-expiry parks. Distinction vs **`CleanLimitBook`:** proactive at fill time with fixed **10**-unit threshold; vs **cancel:** only while row is still in `ORDERS`.

Invariant **L16** in [contracts-security-audit.md](./contracts-security-audit.md); agent playbooks [skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md](../skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md), [skills/AGENTS_LOCALNET_TRADING_SWARM.md](../skills/AGENTS_LOCALNET_TRADING_SWARM.md).

<a id="zero-cost-fill-skip-gitlab-470"></a>

### Zero-cost fill skip (GitLab #470)

When **`floor(fill × price) = 0`** while **`fill > 0`** (typical when the resting limit price is **&lt; 1** token1 per token0 — allowed across mismatched CW20 decimals), a naive fill would debit maker escrow without crediting the counter leg (ask: maker loses token0, receives 0 token1; bid: symmetric).

**Execute + simulation:** `match_bids` / `match_asks` and **`simulate_match_*`** skip the order (`continue`) — the taker's remaining budget cannot afford a price-honoring fill at that granularity. Indexer Postgres mirror (`db_orderbook_sim`) matches on-chain math (**L8**).

**Not** the same as match-time dust flush (**L16**): dust flush runs **after** a successful fill with non-zero `cost`; zero-cost skip prevents the fill entirely.

Invariant **L18** in [contracts-security-audit.md](./contracts-security-audit.md); agent playbook [skills/AGENTS_BOOK_MATCH_HINT_SECURITY.md](../skills/AGENTS_BOOK_MATCH_HINT_SECURITY.md).

<a id="limit-price-band-gitlab-467"></a>

### Limit price band (GitLab #467)

**Price** on limit orders is **token1 per token0** (same basis as pool pricing). The stored / matched value is **raw** base units (`fill × price`). Placement (`PlaceLimitOrderBatch` / ladder), ladder expansion, and **`UpdateLimitOrderPrice`** enforce the band on the **human-scale** price `raw × 10^(decimals0 − decimals1)` ([#529](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/529)):

| Constant | Value | Role |
|----------|-------|------|
| [`MIN_LIMIT_PRICE`](../smartcontracts/packages/dex-common/src/limit_placement.rs) | **1e-9** (human) | Rejects former attack vector `Decimal::raw(1)` = 1e-18 on like-decimal pairs where `1/price` overflows `Uint128::checked_mul_floor` on realistic taker budgets |
| [`MAX_LIMIT_PRICE`](../smartcontracts/packages/dex-common/src/limit_placement.rs) | **1e9** (human) | Symmetric upper bound; a 6-vs-18 pair at ~79 human (raw ~7.9e13) is in-band |

Sub-unity prices inside the band (e.g. **0.1**, **0.4**) remain valid — see zero-cost fill skip (**L18** / #470).

**Match belt-and-suspenders:** if a legacy resting row predates the band, `match_bids` / `match_asks` and **`simulate_match_*`** skip the maker on reciprocal math overflow instead of reverting the whole hybrid swap.

Invariant **L20** in [contracts-security-audit.md](./contracts-security-audit.md); verification: `make verify-issue-467`, `make verify-issue-529`. Agent playbook: [skills/AGENTS_LIMIT_PRICE_DECIMALS.md](../skills/AGENTS_LIMIT_PRICE_DECIMALS.md).

**Frontend hybrid gas ([GitLab #249](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/249), scan cap [#260](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/260), ceiling [#262](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/262)):** Terra Classic does not refund unused gas — the dApp sizes `Fee.gas` from the route quote’s `max_maker_fills` **plus** conservative book-walk overhead when `book_input > 0`, not a flat **15M** per hop unless the envelope hits the ceiling. Formula (one hop, book leg):

`gasWanted = min(15_000_000, max(600_000, 550_000 + 65_000 × (max_maker_fills + 2) + 950 × max(0, scanSteps − (max_maker_fills + 2)) + 8_000 × expiredParks))`

where offline defaults are `scanSteps = MAX_SCAN_STEPS` (**500**, [#254](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/254), [#262](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/262)) and `expiredParks = MAX_EXPIRED_PARKS_PER_SWAP` (**15**, [#250](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/250)). Optional indexer hints may tighten both when the book head is known clean. Pool-only hybrid (`book_input = 0`) uses the buffered one-hop pool envelope (**840k**). Multi-hop sums per-hop estimates; **≥ 2 hops** still use the router. **Single-hop** with a known pair uses CW20 `send` → pair `swap` (no router `SubMsg`/`reply` overhead). Unknown quote → flat **15M** fallback. On-chain maker cap: **`MAX_MAKER_FILLS_HARD_CAP` = 100** ([#262](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/262)). Code: [`hybridSwapGas.ts`](../frontend-dapp/src/services/terraclassic/hybridSwapGas.ts), [`hybridBookWalkLimits.ts`](../frontend-dapp/src/services/terraclassic/hybridBookWalkLimits.ts), [`swapRouting.ts`](../frontend-dapp/src/services/terraclassic/swapRouting.ts). On-chain caps: [`dex-common::pair`](../smartcontracts/packages/dex-common/src/pair.rs). LocalTerra `gas_used` benchmarks: [#252](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/252).

## Indexer route solver

- **`GET /api/v1/route/solve`** — query params: `token_in`, `token_out` (CW20 addresses indexed in `assets`), optional `amount_in` (raw integer string; triggers **default hybrid optimization**), optional `pool_only` / deprecated `hybrid_optimize` / `max_maker_fills` (see [indexer-invariants.md](./indexer-invariants.md), GitLab [**#191**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/191)).
- Returns `hops` (pair + offer/ask tokens per hop), `router_operations` (merged hybrid params when `amount_in` is set; `hybrid: null` per hop when `pool_only=true` or no `amount_in`).
- Optional **`estimated_amount_out`:** set when `amount_in` is provided **and** `ROUTER_ADDRESS` is configured; the indexer calls the router `simulate_swap_operations` query on LCD.
- **Running indexer integration tests:** route tests live under [`indexer/tests/api_route_solve.rs`](../indexer/tests/api_route_solve.rs). They need Postgres; if multiple tests share one DB, use the serialized commands in [Testing — Shared Postgres and test parallelism](./testing.md#shared-postgres-and-test-parallelism).

## Indexer limit book (LCD proxy)

Design record: [ADR 0002](./adr/0002-limit-book-surfacing.md). Endpoints above require the pair address to exist in the indexer DB (**404** if unknown).

**LCD cost (SLO-style):** each `limit-book` page does at most **1 + N** successful smart queries in the steady state: one `order_book_head` when `after_order_id` is omitted, otherwise one `limit_order` lookup for the cursor, plus **one `limit_order` query per returned order** (up to `limit`). There is no server-side caching of book walks; LCD throttling surfaces as **502** from the indexer. Clients should use the smallest `limit` that fits their UI and paginate with `after_order_id` instead of repeating full walks.

## Tx attributes (indexer / analytics)

CosmWasm responses use **attributes** (visible in tx logs as events). Useful keys on the **pair** contract:

| Attribute | When |
|-----------|------|
| `action` = `place_limit_order_batch` | Batch / ladder placement summary |
| `batch_count`, `batch_skipped_count`, `batch_refund_amount` | Batch summary (skipped rungs + CW20 refund) |
| `action` = `place_limit_order` | Limit placed (repeat per successful rung in a batch) |
| `limit_order_placed`, `order_id` | Same tx |
| `side` (`bid` / `ask`), `price`, `owner` | Same tx (for indexers); omitted on older pair code |
| `maker_fee_amount`, `effective_fee_bps` | Same tx (placement fee split vs fills) |
| `expires_at` | Same tx when set |
| `action` = `update_limit_order_price` | Owner changed limit price in place |
| `action` = `cancel_limit_order` | Cancel |
| `action` = `claim_expired_limit_order` | Expired-limit refund (after park) |
| `limit_order_cancelled`, `owner` | Same tx |
| `action` = `limit_order_expired_parked` | Order removed from book into claim queue (wasm event; taker tx or **`clean_limit_book`**) — TTL, dust, blacklist, or force-clean |
| `reason` | Park discriminator (`expired` / `dust_filled` / `force_cleaned` / `blacklisted`) — GitLab [#504](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/504) |
| `force_expired` = `true` | Historical: parked though **not** a TTL expiry ([#263](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/263) / [#264](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/264) / [#468](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/468)). Prefer **`reason`**. |
| `action` = `clean_limit_book` | Permissionless clean summary: `cleaned_count`, `time_expired_count`, `force_expired_count`, `cap_hit` ([#263](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/263)) |
| `action` = `swap` | Any swap |
| `book_return_amount`, `pool_return_amount`, `return_amount` | Hybrid breakdown |
| `limit_book_offer_consumed` | When the book leg consumed offer token |
| `book_commission_amount` | Book taker commission total (ask asset) when book leg > 0 ([#196](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/196)) |
| `expired_parks_used`, `expired_parks_capped`, `expired_parks_skipped` | Hybrid book walk parked ≤ **15** expired limits; skipped head count when cap reached ([#250](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/250), [#254](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/254)) |
| `scan_steps_capped` | Book walk hit **`MAX_SCAN_STEPS`** (500) before `max_maker_fills` or list end ([#254](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/254), [#262](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/262)) |
| `action` = `limit_order_fill` | One **wasm event per maker fill** (not on the main swap attribute list) |
| `order_id`, `side` (`bid` / `ask`), `maker`, `price` | Per fill |
| `token0_amount`, `token1_amount`, `commission_amount` | Raw amounts in pair token0 / token1; `commission_amount` is the **taker** half for that fill (bid: token1; ask: token0) |
| `swap_index` | 0-based ordinal of the parent swap on this pair within the tx (GitLab [#331](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/331)); matches `swap_events.swap_index` / indexer `(tx_hash, pair_id, swap_index)` |

### Wasm attribute coverage vs indexer nulls (operators)

| Pair build | `place_limit_order` wasm attrs | `cancel_limit_order` wasm attrs | Indexer `limit_order_placements` / `limit_order_cancellations` |
|------------|-------------------------------|----------------------------------|-------------------------------------------------------------------|
| **Current** (main branch pair) | `side`, `price`, `owner`, `expires_at` when set | `owner` | Metadata columns populated when attrs appear in tx logs |
| **Legacy** (older deployed wasm omitting attrs) | May omit `side`, `price`, `owner` | May omit `owner` | Corresponding DB columns stay **null**; rows still keyed by `pair_id`, `order_id`, `tx_hash`, heights/timestamps |
| **`limit_order_fill` events** | Per-fill `order_id`, `side`, `maker`, `price`, amounts, `commission_amount`, **`swap_index`** | — | Indexed in `limit_order_fills`; aligns with on-chain book fills; `swap_index` links fill → parent swap ([#331](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/331)) |

The **indexer** persists `pool_return_amount`, `book_return_amount`, and `limit_book_offer_consumed` on `swap_events`, stores each `limit_order_fill` in `limit_order_fills`, and indexes wasm `place_limit_order` / `cancel_limit_order` into **`limit_order_placements`** and **`limit_order_cancellations`**. It decodes **`limit_order_expired_parked`** (taker tx; GitLab [**#120**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120)) and **`claim_expired_limit_order`** so each placement row tracks **`lifecycle_status`** (`active` → `parked_expired` → `refunded`) and **`remaining_escrow`** from the park event ([**#142**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/142)). **LCD note:** some REST paths flatten multiple logical wasm emissions into **one** `wasm` attribute stream where **`action=swap` or `transfer` appears after** earlier actions; the indexer scans **every** `action` value per event so **`limit_order_expired_parked`**, **`claim_expired_limit_order`**, and **`limit_order_fill`** rows still index when the stream ends with swap/transfer attrs ([**#141**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/141), [**#269**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/269) — contract still emits distinct CosmWasm events; the gap was off-chain parsing). HTTP: **`GET /api/v1/pairs/{addr}/trades`** includes hybrid fields and optional **`effective_fee_bps`** when present; **`GET /api/v1/pairs/{addr}/limit-fills`** and **`GET /api/v1/pairs/{addr}/limit-orders/{order_id}/fills`** expose per-maker fills; **`GET /api/v1/pairs/{addr}/limit-placements`** returns placement rows **without** a matching indexed cancel for the same `(pair, order_id)` (full cancel history remains on **`.../limit-cancellations`** — [GitLab **#135**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/135)), includes **`lifecycle_status`** on every row, defaults to **`active` + `parked_expired`** (excludes terminal **`refunded`**), and accepts **`?status=`** `active` \| `parked_expired` \| `refunded` \| `all`; **`GET /api/v1/pairs/{addr}/order-book-head`**, **`.../limit-book`**, and **`.../limit-book-shallow`** proxy on-chain book state (see [ADR 0002](./adr/0002-limit-book-surfacing.md)).

<a id="batch-ladder-gas-savings"></a>

## Batch / ladder gas savings (GitLab #206)

Retail **single** limit placement uses the batch hook with `orders.len() == 1`. A **ladder** uses one allowance + one `place_limit_order_ladder` (expanded to batch on-chain).

| Flow | Signed txs (after allowance) | dApp gas limit model ([`terraGas.ts`](../frontend-dapp/src/services/terraclassic/terraGas.ts)) |
|------|------------------------------|--------------------------------------------------------------------------------------------------|
| **N separate** limits | N × (`increase_allowance` + `send`) = **2N** wallet prompts | N × (`PLACE_LIMIT_ORDER_GAS_LIMIT` ≈ 950k) + N allowance envelopes |
| **One batch / ladder** | **1** `send` | `PLACE_LIMIT_ORDER_BATCH_BASE_GAS_LIMIT` (400k) + `PLACE_LIMIT_ORDER_BATCH_PER_RUNG_GAS_LIMIT` (180k) × N |

Example (5 rungs, fee math only — actual gas used varies by chain): batch attach ≈ **1.3M** gas units vs 5×950k ≈ **4.75M** for separate placements; native LUNC preflight uses `estimateLimitOrderBatchPlaceSequenceUlunaFeesTotal(rungCount)` ([`transactions.ts`](../frontend-dapp/src/services/terraclassic/transactions.ts)). UI copy: [`LimitOrderLadderPanel`](../frontend-dapp/src/components/trade/LimitOrderLadderPanel.tsx) · agent skill [`skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md`](../skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md).

<a id="batch-cancel-claim-gas-savings"></a>

## Batch cancel / claim gas savings (GitLab #246)

| Flow | Signed txs | dApp gas limit model ([`terraGas.ts`](../frontend-dapp/src/services/terraclassic/terraGas.ts)) |
|------|------------|--------------------------------------------------------------------------------------------------|
| **N separate cancels** | N | N × `CANCEL_LIMIT_ORDER_GAS_LIMIT` (450k) |
| **One `cancel_limit_orders` batch** | **1** | `CANCEL_LIMIT_ORDER_BATCH_BASE_GAS_LIMIT` (400k) + `CANCEL_LIMIT_ORDER_BATCH_PER_ORDER_GAS_LIMIT` (80k) × N |
| **N separate claims** | N | N × `CLAIM_EXPIRED_LIMIT_ORDER_GAS_LIMIT` (450k) |
| **One `claim_expired_limit_orders` batch** | **1** | same formula as batch cancel |

Example (10 orders): batch cancel attach ≈ **1.2M** vs 10×450k ≈ **4.5M** for separate cancels. Service: [`cancelLimitOrders` / `claimExpiredLimitOrders`](../frontend-dapp/src/services/terraclassic/pair.ts) · [`useLimitOrderCancelMutation`](../frontend-dapp/src/hooks/useLimitOrderCancelMutation.ts) (accepts `number | number[]`).

## dApp: retail form (wires, invariants)

Implementation: [`LimitOrdersPage`](../frontend-dapp/src/pages/LimitOrdersPage.tsx) (**Single** + **Ladder** on `/limits`), [`TradeOrderTicket`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx) (single limit on `/trade`), shared [`LimitOrderMyPlacementsPanel`](../frontend-dapp/src/components/trade/LimitOrderMyPlacementsPanel.tsx), components under [`frontend-dapp/src/components/trade/`](../frontend-dapp/src/components/trade/) ([issue #110](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/110)). Pure helpers: [`limitOrderExpiry.ts`](../frontend-dapp/src/utils/limitOrderExpiry.ts), [`limitOrderEscrowBalanceGate.ts`](../frontend-dapp/src/utils/limitOrderEscrowBalanceGate.ts) ([GitLab #130](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/130)), [`limitOrderNativeGasBalanceGate.ts`](../frontend-dapp/src/utils/limitOrderNativeGasBalanceGate.ts) ([GitLab #132](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/132)), [`limitOrderPriceReference.ts`](../frontend-dapp/src/utils/limitOrderPriceReference.ts) + [`limitOrderPricePlaceGate.ts`](../frontend-dapp/src/utils/limitOrderPricePlaceGate.ts) ([GitLab #154](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/154), **#166** pool fallback + hard submit gate), [`limitOrderFeeSummary.ts`](../frontend-dapp/src/utils/limitOrderFeeSummary.ts) (effective + maker placement bps for pre-submit copy — [GitLab **#157**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/157)), [`useLimitOrderPriceRefBundle.ts`](../frontend-dapp/src/hooks/useLimitOrderPriceRefBundle.ts), [`limitPlacementLifecycle.ts`](../frontend-dapp/src/utils/limitPlacementLifecycle.ts) ([GitLab #141](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/141)).

- **`expires_at`:** the UI may use local `datetime-local`, “24h / 7d / no expiry” presets, or (Advanced) a raw **Unix second** value; all map to the same `expires_at` field described under [Expiry (`expires_at`)](#expiry-expires_at). If both friendly controls and raw seconds are set, they must agree — they are one logical value in state.
- **`max_adjust_steps` (placement gas):** the retail default is **Medium → 32** on-chain steps. Advanced **Placement gas (book walk)** on `/trade` and `/limit` exposes **Low (16)**, **Medium (32)**, **High (128)**, and **Custom (1–256)** — higher caps spend more placement gas but improve reliability on deep books ([GitLab **#204**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/204)). Constants and mapping: [`limitOrderExpiry.ts`](../frontend-dapp/src/utils/limitOrderExpiry.ts); UI: [`LimitOrderAdvancedLimitSettings.tsx`](../frontend-dapp/src/components/trade/LimitOrderAdvancedLimitSettings.tsx). On-chain field semantics: [Messages (CosmWasm)](#messages-cosmwasm). Third-party agents: [`skills/AGENTS_FRONTEND_LIMIT_ORDER_PLACEMENT_GAS.md`](../skills/AGENTS_FRONTEND_LIMIT_ORDER_PLACEMENT_GAS.md).
- **Escrow `amount`:** the CW20 `send` amount uses the same balance query and **Max** affordance as the swap form so users see spendable balance before `increase_allowance` + `place_limit_order`.
- **Trade `/trade` ticket — market vs limit ([GitLab #152](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/152), [#501](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/501), [#596](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/596)):** `TradeOrderTicket` exposes **Limit** and **Market** tabs. **Market** executes a CW20 `send` → pair `swap` (Pattern C **hybrid** when the solver or Advanced override supplies it) with **global slippage** from `useDexStore`, `increase_allowance` preflight, native **two-tx** gas gate via `estimateMarketPairSwapSequenceUlunaFeesTotal` + `evaluateMarketSwapNativeGasPlaceGate` ([`transactions.ts`](../frontend-dapp/src/services/terraclassic/transactions.ts), [`limitOrderNativeGasBalanceGate.ts`](../frontend-dapp/src/utils/limitOrderNativeGasBalanceGate.ts)). **Default quotes** use indexer **`GET /api/v1/route/solve`** (same always-on best-execution path as Swap) via [`quoteCw20ViaRouteSolve`](../frontend-dapp/src/utils/cw20RouteSolveQuote.ts); submit uses solver `hybrid` from `indexerOperations`. **Advanced** manual book leg uses **`POST /api/v1/route/solve`** (`quoteDirectHybridSwap`). There is **no** hybrid-off / pool-only toggle. See [§ Swap page: hybrid vs pool-only estimates](#swap-ui-hybrid-vs-pool-only-estimates). Implementation: [`TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx). Agent skills: [`skills/AGENTS_HYBRID_QUOTING.md`](../skills/AGENTS_HYBRID_QUOTING.md), [`skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md`](../skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md).
- **Limit pre-submit summary vs market quote ([GitLab #157](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/157)):** the **Limit** path shows **before Place** that resting orders are filled **over time** (no taker slippage / pool price impact / min-received line), repeats **% deviation vs reference**, shows **maker placement fee** (`floor(effective/2)` bps) from pair fee + optional discount query, and a **minimum LUNC** line for the two-tx place sequence. **Market** tab keeps the simulation receive / min-received UX. Docs: [`docs/frontend.md` § Limit order pre-submit summary](./frontend.md#trade-page-limit-order-pre-submit-summary); [`LimitOrderPreSubmitSummary.tsx`](../frontend-dapp/src/components/trade/LimitOrderPreSubmitSummary.tsx).
- **Trade `/trade` ticket — post-only style limit preflight (#152):** before broadcasting `place_limit_order`, the UI blocks **bid price ≥ best ask** and **ask price ≤ best bid** using head rows from indexer **`GET .../limit-book?side=&limit=1`** (same token1/token0 price convention as the contract). This is a **client-only** guard for clearer UX; the on-chain pair does not reject “marketable” limit prices by itself.
- **Preflight vs on-chain balance (GitLab [#130](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/130)):** the dApp must **not** broadcast `increase_allowance` when the signed human amount (converted to raw units) **exceeds** the wallet’s CW20 balance for the escrow token, or when balance is **still loading** / **unreadable** — `increase_allowance` can succeed with zero balance, then `place_limit_order` fails at `transfer_from`, burning gas twice. Implementation: shared gate [`limitOrderEscrowBalanceGate.ts`](../frontend-dapp/src/utils/limitOrderEscrowBalanceGate.ts) + inline UI [`LimitOrderEscrowPlaceGuardMessage.tsx`](../frontend-dapp/src/components/trade/LimitOrderEscrowPlaceGuardMessage.tsx); wired from [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx) and [`LimitOrdersPage.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.tsx).
- **Limit price vs tape / pool reference (GitLab [#154](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/154), [#166](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/166), [#495](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/495)):** the trade ticket and standalone limit page show **last-trade** token1/token0 when the indexer tape is available; otherwise they resolve an **AMM pool spot** from on-chain reserves (LCD `pool` query) when decimals are known, and **block** place when no reference can be computed (no silent bypass). **% deviation**, **headline-scaled USD** (tape headline only), and **submit gate** use the resolved reference. **Deviation chips** are side-aware (bid below ref, ask above ref; near-market `0` uses a tiny epsilon so equality stays invalid). Invariants: [`docs/frontend.md` § Trade page — limit order price field](./frontend.md#trade-page-limit-order-price); agent skill [`skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](../skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md).
- **Escrow amount — headline USD + Bid/Ask sizing (GitLab [#155](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/155)):** the **Amount** field shows a **headline-scaled USD** line using `escrowAmountUsdAnchorNotional` (same tape headline + `refToken1PerToken0` as the limit price USD anchor). **Bid ↔ Ask** clears manually typed amounts; **Max** reapplies full balance after the new escrow token balance loads. Docs: [`docs/frontend.md` § Limit place — escrow amount](./frontend.md#limit-place-escrow-amount); [`useLimitOrderForm.ts`](../frontend-dapp/src/hooks/useLimitOrderForm.ts).
- **Post-placement indexer poll (GitLab [#131](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/131)):** after a successful place tx, the UI polls **`GET /api/v1/traders/{addr}/limit-placements?pair=`** (or pair-scoped fallback) to resolve the new **`order_id`** and pre-fill cancel. **Local dev:** ensure indexer **`CORS_ORIGINS`** includes the Vite **`Origin`** you use (`localhost` vs `127.0.0.1` — see [`docs/frontend.md` § Local dev indexer CORS](./frontend.md#local-dev-indexer-cors)). Failures log **`[limit-place] indexer poll failed:`** via [`warnIndexerPlacementPollFailed`](../frontend-dapp/src/utils/warnIndexerPlacementPollFailed.ts).
- **Cancel UX vs indexer (GitLab [#135](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/135)):** the indexer **`limit-placements`** feed omits orders once a matching **`limit-cancellations`** row exists; the dApp **invalidates** both queries after a successful cancel, **blocks** submit when the typed id is already in the indexed cancel list, and **humanizes** CosmWasm `LimitOrder` map “not found” errors via [`tryHumanizeTerraTxMessage`](../frontend-dapp/src/utils/humanizeTerraTxError.ts) (wired from [`transactions.ts`](../frontend-dapp/src/services/terraclassic/transactions.ts); helpers in [`limitOrderCancelUserMessage.ts`](../frontend-dapp/src/utils/limitOrderCancelUserMessage.ts)).
- **Open-row Cancel reconciliation (GitLab [#530](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/530)):** indexer `lifecycle_status=active` is **not** proof the row is still in `ORDERS`. Hybrid fills write `limit_order_fills` only — they do **not** flip placement lifecycle — so a fully filled order can still render as a green `●` until the dApp reconciles. Before offering **Cancel**, the panel prefers LCD **`OrderStatus`** (`queryOrderStatus`) plus indexer fills / cancellations / a local post-broadcast cancel set. See [§ Open-row Cancel reconciliation](#open-row-cancel-reconciliation-gitlab-530) and [`skills/AGENTS_FRONTEND_LIMIT_CANCEL_OPEN.md`](../skills/AGENTS_FRONTEND_LIMIT_CANCEL_OPEN.md).
- **Trade `/trade` order book — deep pagination ([GitLab #194](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/194)):** **`OrderBookPanel`** on `/trade` and `/limits` loads **`GET .../limit-book`** via **`useLimitBookInfinite`** (45 rows/page, **Load more depth**). Invariants: [`docs/frontend.md` § Trade page — deep order book pagination](./frontend.md#trade-page-deep-order-book); skill [`skills/AGENTS_FRONTEND_DEEP_ORDER_BOOK.md`](../skills/AGENTS_FRONTEND_DEEP_ORDER_BOOK.md).
- **Trade `/trade` order book — row cancel / edit / cancel-all (GitLab [#162](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/162), [#178](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/178), **#246** batch cancel, **#247** price edit):** the paginated **Bids / Asks** tables show **`#order_id`** on each row; rows whose **`owner`** matches the connected wallet expose **Edit** (prefills the limit ticket with `orderId` + remaining size; **price-only** change submits **`UpdateLimitOrderPrice`** in one tx — same `order_id`, no maker fee, no CW20; changing size/side/expiry shows cancel-first copy) and **×** cancel (shared `useLimitOrderCancelMutation` with the ticket’s **Manage** section). **Cancel all mine** submits **`CancelLimitOrders`** in **one tx** when multiple active placements exist (single-id path still uses `CancelLimitOrder`). Gas: `gasLimitForLimitOrderCancelBatch(n)` and **`UPDATE_LIMIT_ORDER_PRICE_GAS_LIMIT`** (350k) in [`terraGas.ts`](../frontend-dapp/src/services/terraclassic/terraGas.ts). Invariants: [`docs/frontend.md` § Trade page — order book row actions](./frontend.md#trade-book-row-actions); skill [`skills/AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md`](../skills/AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md).
- **Parked-expired recovery (GitLab [#141](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/141); indexer [#142](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/142); batch claim [#246](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/246); **Claim all parked** [#253](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/253); retail UX [#419](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/419)):** default **`GET .../limit-placements`** includes **`lifecycle_status`** **`active`** and **`parked_expired`**. Wallet-scoped listing uses **`GET /api/v1/traders/{addr}/limit-placements?pair=`** ([GitLab **#217**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/217)). The dApp **`LimitOrderMyPlacementsPanel`** ([`LimitOrderMyPlacementsPanel.tsx`](../frontend-dapp/src/components/trade/LimitOrderMyPlacementsPanel.tsx)) is the **primary** open-orders surface on `/limits` and the `/trade` **Limit** tab (above place/cancel forms): **My open limits** lists resting rows with per-row **Cancel** (shared `useLimitOrderCancelMutation`), splits **Expired — refund pending** vs **Dust — claim remaining** (`remaining_escrow < 10` → **Claim dust**), and shows **Claim refund** / **Claim all parked** wired to **`ClaimExpiredLimitOrder`** / batch claim. Maker placement fee copy in [`LimitOrderPreSubmitSummary.tsx`](../frontend-dapp/src/components/trade/LimitOrderPreSubmitSummary.tsx) uses retail percent labels (`bpsToPercentLabel`). **Market** hybrid tab surfaces **min return** + 10% pool-leg copy in [`TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx). Manual **Cancel by order ID** remains in a collapsed **Advanced** block. **Claim/cancel disabled while paused** (L6). Helpers: [`limitPlacementLifecycle.ts`](../frontend-dapp/src/utils/limitPlacementLifecycle.ts), [`limitOrderFeeSummary.ts`](../frontend-dapp/src/utils/limitOrderFeeSummary.ts). Third-party agents: [`skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](../skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md).

**Invariants (#141 parked-expired UX; #253 Claim all parked)**

- **Indexer fields:** UI trusts **`lifecycle_status`** from the indexer default listing (**`active`** \| **`parked_expired`**); legacy rows without the column are treated as **`active`**. **`remaining_escrow`** is shown for parked rows (bid → token1 decimals, ask → token0 decimals).
- **Contract:** per-row **Claim refund** submits **`claim_expired_limit_order`**; **Claim all parked** (≥ 2 rows) submits **`claim_expired_limit_orders`** with deduped ids from indexed **`parked_expired`** rows only (≤ **100** ids per tx — [`MAX_LIMIT_BATCH_RUNGS_HARD_CAP`](../smartcontracts/packages/dex-common/src/limit_placement.rs); UI chunks larger sets with one confirm per chunk). **Book clean:** permissionless **`clean_limit_book`** parks expired/dust rows — makers still claim via the same paths (**#263**). Gas: single **`CLAIM_EXPIRED_LIMIT_ORDER_GAS_LIMIT`**; batch **`gasLimitForLimitOrderCancelBatch(n)`** in [`terraGas.ts`](../frontend-dapp/src/services/terraclassic/terraGas.ts) ([#246](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/246)). **Confirm copy ([#259](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/259)):** **`confirmExpiredClaimBatchMessage`** in [`limitExpiredClaimBatch.ts`](../frontend-dapp/src/utils/limitExpiredClaimBatch.ts) appends **est. ~X LUNC gas** from **`estimateFeeUlunaAmountForGasLimit(gasLimitForLimitOrderCancelBatch(chunkSize))`** plus optional savings vs N× single-claim; per-chunk when **N > 30**.
- **Single parked row:** no **Claim all** control; per-row **Claim refund** unchanged.
- **Queries:** successful claim **invalidates** `limitPlacements`, pair book previews, and **`tokenBalance`** so escrow updates after refund.

<a id="open-row-cancel-reconciliation-gitlab-530"></a>

### Open-row Cancel reconciliation (GitLab #530)

Retail **My open limits** must not show a green `●` with a dead or mute **Cancel**. Report class: `●order #1 · Sell UST1 · 82.044… · placed 2026-08-15T14:21:43` (indexer-active ask after the order left `ORDERS`, typically a full hybrid fill).

**Invariants (F530-1–F530-8)**

| Id | Rule |
|----|------|
| **F530-1** | Prefer LCD `OrderStatus` (`queryOrderStatus` / `QueryMsg::OrderStatus`) before offering Cancel. `Active` → Cancel (unless pause / blacklist / gas). `ParkedRefund` → **Claim refund**, not Cancel. |
| **F530-2** | LCD / transport / decode **failure is not `Unknown`** (**L21**). Keep Cancel on indexer-active rows; #135 still humanizes a race. |
| **F530-3** | Successful `Unknown` is classified from indexer evidence only: fill row → **Filled**; cancellation or local post-broadcast id → **Already cancelled**; else **No longer on the book**. Never treat `Unknown` as proof of fill. |
| **F530-4** | Disabled Cancel states **why** (paused / restricted / filled / already cancelled / gone). No mute `Cancel`. |
| **F530-5** | `/trade` compact panel sits **above** `trade-ticket-submit-footer` (not a child). Cancel `elementFromPoint` must hit the button, not Place limit. Testids unchanged: `trade-cancel-placement-{id}`, `limits-page-cancel-placement-{id}`, `trade-book-cancel-{bid\|ask}-{id}`, `trade-ticket-placements-anchor`. |
| **F530-6** | Owner-only + **L6** pause unchanged. Cancel payload is factory `order_id` on the selected pair — UST1 invert (#524) is display-only. |
| **F530-7** | After a successful cancel, remember the id locally so indexer lag cannot re-enable Cancel (I9). Mutation also refuses LCD `Unknown` / `ParkedRefund` (A2 / A3). |
| **F530-8** | Indexer `process_limit_order_fill` still does **not** flip `lifecycle_status`. Default-open listing can include filled-as-`active` until LCD + fills reclassify the row. Do not add a third execute path. |

Implementation: [`limitPlacementOpenReconcile.ts`](../frontend-dapp/src/utils/limitPlacementOpenReconcile.ts), [`useLimitOrderStatuses.ts`](../frontend-dapp/src/hooks/useLimitOrderStatuses.ts), [`LimitOrderMyPlacementsPanel.tsx`](../frontend-dapp/src/components/trade/LimitOrderMyPlacementsPanel.tsx), [`useLimitOrderCancelMutation.ts`](../frontend-dapp/src/hooks/useLimitOrderCancelMutation.ts). Verify: `make verify-issue-530`. Playbooks: [`AGENTS_FRONTEND_LIMIT_CANCEL_OPEN.md`](../skills/AGENTS_FRONTEND_LIMIT_CANCEL_OPEN.md), [`AGENTS_ORDER_STATUS_QUERY.md`](../skills/AGENTS_ORDER_STATUS_QUERY.md), [`AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](../skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md).

- **Preflight vs native LUNC for two fees (GitLab [#132](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/132)):** the dApp must **not** broadcast `increase_allowance` when bank **uluna** is below the **sum** of the two fee envelopes the UI will attach (`increase_allowance` then CW20 `send` → `place_limit_order`), or when LUNC balance is **loading** / **unreadable** — otherwise the first tx can succeed and burn gas while the user cannot complete the second. Required uluna is computed with the same gas limits and `effectiveGasPriceUluna()` as broadcast: [`estimateLimitOrderPlaceSequenceUlunaFeesTotal`](../frontend-dapp/src/services/terraclassic/transactions.ts) + gate [`limitOrderNativeGasBalanceGate.ts`](../frontend-dapp/src/utils/limitOrderNativeGasBalanceGate.ts); hook [`useNativeUlunaBalance.ts`](../frontend-dapp/src/hooks/useNativeUlunaBalance.ts) (React Query key `['tokenBalance', address, 'uluna']`).
- **Ladder place (GitLab [#206](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/206)):** [`LimitOrderLadderPanel`](../frontend-dapp/src/components/trade/LimitOrderLadderPanel.tsx) on `/limits` — price range, rung count, total escrow, equal split preview, **escrow + native LUNC gates** via [`useLimitLadderPlaceGates.ts`](../frontend-dapp/src/hooks/useLimitLadderPlaceGates.ts) (total human amount + `gasLimitForLimitOrderBatch(rungCount)`), and **post-only crossing guard per rung** ([#297](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/297)) via [`describeLimitCrossingBlocker`](../frontend-dapp/src/utils/limitOrderNonCrossing.ts) + [`useTradeBestBookPrices`](../frontend-dapp/src/hooks/useTradeBestBookPrices.ts) (same client-only semantics as the retail limit form [#152](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/152)). One `increase_allowance` for **total** escrow. Playwright: [`limit-orders-tx.spec.ts`](../frontend-dapp/e2e/limit-orders-tx.spec.ts) 5-rung ladder · skill [`skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md`](../skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md).
- **Ladder create options without wallet (GitLab [#494](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/494)):** selecting **Ladder** on `/limits` must mount the ladder create form whenever a pair is selected — **do not** gate `LimitOrderLadderPanel` on wallet `address` (Single mode does not). When disconnected, show start/end price, rungs, total pay, expiry/advanced, preview, and an enabled **Connect Wallet** CTA (`openWalletModal`); apply escrow/LUNC place gates only after connect (same pattern as Single / [`TradeOrderTicket`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx)). Invariant **§14** in [`skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md`](../skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md). Vitest: [`LimitOrderLadderPanel.disconnect.test.tsx`](../frontend-dapp/src/components/trade/__tests__/LimitOrderLadderPanel.disconnect.test.tsx) · smoke E2E: [`limit-orders.spec.ts`](../frontend-dapp/e2e/limit-orders.spec.ts).
- **Ladder total escrow aggregation (GitLab [#233](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/233)):** [`sumLadderAmountsRaw`](../frontend-dapp/src/utils/limitOrderLadder.ts) must **add** per-rung raw amounts as `BigInt` (initial accumulator `0n`). A reduce that coerces to string (e.g. `(acc + BigInt(r)).toString()` with `acc` starting at `'0'`) concatenates rung strings and produces an invalid Uint128 for `increase_allowance` / CW20 `send`. Submit path: [`LimitOrderLadderPanel`](../frontend-dapp/src/components/trade/LimitOrderLadderPanel.tsx) → [`placeLimitOrderLadderWithAllowance`](../frontend-dapp/src/services/terraclassic/pair.ts). Vitest: [`limitOrderLadder.test.ts`](../frontend-dapp/src/utils/__tests__/limitOrderLadder.test.ts) · agent skill invariant §3 in [`skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md`](../skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md).
- **Ladder escrow balance hook (GitLab [#231](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/231)):** [`useLimitLadderPlaceGates`](../frontend-dapp/src/hooks/useLimitLadderPlaceGates.ts) must import a **real** CW20 balance hook — [`useLimitOrderEscrowBalance`](../frontend-dapp/src/hooks/useLimitOrderEscrowBalance.ts) (React Query key `['tokenBalance', wallet, terra1…]`, shared with `/trade` and `/limits` retail limit place). [`useTokenBalance`](../frontend-dapp/src/hooks/useTokenBalance.ts) is a **re-export only**; a missing file breaks Vite on any route that transitively imports the ladder module (often surfaced after `rm -rf node_modules`). Vitest: [`useTokenBalance.test.ts`](../frontend-dapp/src/hooks/__tests__/useTokenBalance.test.ts).
- **Offline / stalled broadcast (GitLab [#173](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/173)):** `executeTerraContract` caps **`broadcastTx`** / **`pollTx`** so Place limit / cancel do not hang forever; see [`docs/frontend.md` § Transaction broadcast / confirmation timeout](./frontend.md#terra-tx-broadcast-timeout) and [`skills/AGENTS_FRONTEND_TX_BROADCAST_TIMEOUT.md`](../skills/AGENTS_FRONTEND_TX_BROADCAST_TIMEOUT.md).
- **Related — pool add liquidity, three fees (GitLab [#147](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/147)):** CW20/CW20 **Provide Liquidity** uses **three** sequential txs (`increase_allowance` ×2 then `provide_liquidity`) with the same “sum native fees before first broadcast” pattern as [#132](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/132): [`estimateProvideLiquidityCw20SequenceUlunaFeesTotal`](../frontend-dapp/src/services/terraclassic/transactions.ts) + [`provideLiquidityNativeGasBalanceGate.ts`](../frontend-dapp/src/utils/provideLiquidityNativeGasBalanceGate.ts). See [`docs/frontend.md` § Pool page](./frontend.md#pool-page--provide-liquidity-ui-invariants). **Rollback** (`provide_liquidity` fails after allowances): [`pair.ts`](../frontend-dapp/src/services/terraclassic/pair.ts) sends **both** `decrease_allowance` messages in **one** [`executeTerraContractMulti`](../frontend-dapp/src/services/terraclassic/transactions.ts) tx (one prompt / one fee).
**Invariants (#130 escrow + #132 native gas preflight)**

- **Raw comparison (escrow):** gate compares `toRawAmount(amountHuman, escrowDecimals)` to the LCD/CW20 balance string as **BigInt** (same units as the allowance / hook `send` amount).
- **Uluna comparison (gas):** gate compares bank `uluna` balance (BigInt string from LCD) to `estimateLimitOrderPlaceSequenceUlunaFeesTotal()` — must stay aligned with `getGasLimitForTx` + `estimateTerraClassicFee` in [`transactions.ts`](../frontend-dapp/src/services/terraclassic/transactions.ts) when those gas limits or fee math change.
- **Conservative when uncertain:** if either balance query is **loading**, **errored**, or **missing `data`**, the combined gate is closed — no allowance tx.
- **Empty human amount:** escrow gate returns no inline message; native gas gate is open (no “add LUNC” noise until the user enters an amount). Submit remains blocked by the zero-amount path.
- **UI + mutation:** the Place button is disabled when either gate is closed, and `mutationFn` re-evaluates both gates so a stale click cannot reach the broadcast layer.
- **Broadcast path:** retail place uses **`placeLimitOrderWithAllowance`** (optional **`hintAfterOrderId`** from deep book — [#261](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/261)) → **`executeCw20AllowanceThen`** → **`broadcastTerraExecuteContracts`** twice (`increase_allowance`, then `place_limit_order_batch`). Same stack as swaps/pool txs ([`terraBroadcast.ts`](../frontend-dapp/src/services/terraclassic/terraBroadcast.ts), [`terraGas.ts`](../frontend-dapp/src/services/terraclassic/terraGas.ts)). Station signing: see [`docs/frontend.md` § Station extension signing](./frontend.md#station-extension-signing) ([#127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127), [#208](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/208)).
- **Inline copy precedence:** when both gates would show text, the UI shows the escrow message first until the escrow gate has no message (then native gas), so users see CW20 issues before LUNC.

**Docs / work splits:** when splitting large follow-up changes into reviewable pieces, the **split-to-prs** skill in the Cursor *skills* family is the intended workflow (small branches, one concern per change).

## Example JSON (logical shapes)

`Cw20HookMsg::PlaceLimitOrderBatch` — single retail rung with insert hint (GitLab **#261**):

```json
{
  "place_limit_order_batch": {
    "side": "bid",
    "orders": [
      {
        "price": "0.95",
        "amount": "1000000",
        "max_adjust_steps": 16,
        "expires_at": null,
        "hint_after_order_id": 42
      }
    ]
  }
}
```

`Cw20HookMsg::PlaceLimitOrder` (inside CW20 `send.msg`):

```json
{
  "place_limit_order": {
    "side": "bid",
    "price": "1.0",
    "hint_after_order_id": null,
    "max_adjust_steps": 32,
    "expires_at": null
  }
}
```

`Cw20HookMsg::Swap` with Pattern C (book leg only; amounts are raw integer strings):

```json
{
  "swap": {
    "belief_price": null,
    "max_spread": "1",
    "to": null,
    "deadline": null,
    "trader": null,
    "hybrid": {
      "pool_input": "0",
      "book_input": "1000000",
      "max_maker_fills": 8,
      "book_start_hint": null
    }
  }
}
```
