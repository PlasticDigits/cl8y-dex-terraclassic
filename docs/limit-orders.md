# Limit orders and hybrid swaps

This document is the implementation reference for the hybrid AMM + FIFO limit book. Canonical message shapes live in [`smartcontracts/packages/dex-common/src/pair.rs`](../smartcontracts/packages/dex-common/src/pair.rs).

## Swap page: hybrid vs pool-only estimates

<a id="swap-ui-hybrid-vs-pool-only-estimates"></a>

The Swap UI must show **before submit** whether execution is **hybrid (pool + limit book)** vs **pool-only** when a book leg is configured, and it must not hide the fact that a **pool `Simulation` quote** can disagree with a **submitted hybrid** (see L8 in [contracts-security-audit.md](./contracts-security-audit.md)). Implementation: [`frontend-dapp/src/pages/SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) and the pure split helper [`frontend-dapp/src/utils/swapDisclosure.ts`](../frontend-dapp/src/utils/swapDisclosure.ts). Product/QA: [GitLab #111](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/111).

**Invariants**

- **Pool `Simulation` / pool-only multihop sim** does not include the on-chain book; the pair’s `Simulation` query is reserves-only.
- **Direct CW20 + “limit book leg” in Settings:** `pool_input` / `book_input` must sum to the pay amount; the same split is computed in one place for UI, simulation (`postRouteSolve` when used), and `swap` submit (`getDirectHybridBookSplit` vs [`HybridSwapParams`](../smartcontracts/packages/dex-common/src/pair.rs) fields).
- **When the receive line is still pool-only but a book leg is active:** the UI sets `receiveQuoteIsPoolOnlyWithConfiguredBookLeg` and shows copy under “You receive” (hybrid fill may differ).
- **Indexer routes with `indexer_hybrid_lcd` / `indexer_hybrid_lcd_degraded`:** the main panel shows an **“Indexer hybrid”** execution line (not only quote disclosure / the alert block).

**Not duplicated elsewhere:** if you change pay split rules, update `getDirectHybridBookSplit` and the `swap` mutation in `SwapPage` together. For merge/CI follow-up on this repo, the **babysit** Cursor agent skill (keep a PR merge-ready) is the intended loop.

## Exchange API “orderbook” vs on-chain limit book

CoinGecko/CoinMarketCap [`GET /cg/orderbook`](./CG_CMC_COMPLIANCE.md#get-cgorderbook) and [`GET /cmc/orderbook/:market_pair`](./CG_CMC_COMPLIANCE.md#get-cmcorderbookmarket_pair) return an **AMM-simulated** level-2 book (walking the bonding curve). That is **not** the FIFO limit book stored on pairs.

**Resting limits** are on-chain: query the pair contract with `LimitOrder { order_id }` and `OrderBookHead { side }` via LCD or any CosmWasm client. After an expiry park, `LimitOrder` returns an error — use **`ExpiredLimitRefund { order_id }`** (`null` if already claimed). The **indexer also proxies** those reads for integrators and the dApp (see [ADR 0002: Limit book surfacing](./adr/0002-limit-book-surfacing.md)):

- **`GET /api/v1/pairs/{addr}/order-book-head?side=bid|ask`** — JSON `{ "head_order_id": <u64> | null }` from LCD `OrderBookHead`.
- **`GET /api/v1/pairs/{addr}/limit-book?side=bid|ask&limit=L&after_order_id=OPTIONAL`** — **paginated** walk from head or keyset cursor along `next` (default `limit` 50, max 100 per HTTP response). Response includes `orders`, `has_more`, and `next_after_order_id` (pass the latter as `after_order_id` for the next page). LCD errors → **502**; invalid cursor / side mismatch → **400**.
- **`GET /api/v1/pairs/{addr}/limit-book-shallow?side=bid|ask&depth=N`** — legacy small preview walk (default depth 10, max 20); prefer `limit-book` for full depth.

**Trader-scoped history (fills, cancellations, swaps on a pair, CSV):** [GitLab **#163**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/163) — `GET /api/v1/traders/{addr}/limit-fills`, `.../limit-cancellations`, `.../trades` with optional `pair=` and `format=csv`. Invariants: [`indexer-invariants.md`](./indexer-invariants.md); dApp: [`frontend.md` § Wallet swap and limit history](./frontend.md#wallet-swap-limit-history), [`skills/AGENTS_FRONTEND_ORDER_HISTORY.md`](../skills/AGENTS_FRONTEND_ORDER_HISTORY.md).

For multihop routing the indexer exposes route discovery via [`GET /api/v1/route/solve`](./indexer-invariants.md) (default pool-only `hybrid: null`; optional **`hybrid_optimize`** for server-chosen splits, max 3 hops — [**#101**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/101)) and **hybrid merge + router quote** via [`POST /api/v1/route/solve`](./indexer-invariants.md) when the client sends `hybrid_by_hop` aligned with the discovered hops (see ADR 0001).

## Messages (CosmWasm)

### Swap with Pattern C (`Cw20HookMsg::Swap`)

- **`hybrid`:** optional [`HybridSwapParams`](../smartcontracts/packages/dex-common/src/pair.rs): `pool_input`, `book_input` (must sum to the CW20 `amount`), `max_maker_fills`, optional `book_start_hint` (order id).
- **Match walk:** If `book_start_hint` is set and that order id still exists, matching starts from that id; otherwise it starts from the book head (see `orderbook::match_bids` / `match_asks`).
- **`MAX_ADJUST_STEPS`:** placement uses `PlaceLimitOrder { max_adjust_steps }` to cap the linear walk when finding an insert position from the **book head**. Hard caps: `MAX_ADJUST_STEPS_HARD_CAP` / `MAX_MAKER_FILLS_HARD_CAP` in `dex-common::pair`.

### Place / cancel limit

- **`Cw20HookMsg::PlaceLimitOrder`:** `side`, `price`, `hint_after_order_id`, `max_adjust_steps`, optional **`expires_at`** (Unix seconds; `null` = no expiry). If set, it must be **strictly greater** than the block time at placement.
- **Fees:** Total limit-book fee rate matches the pair’s **effective** swap commission (`fee_bps` after the optional fee-discount registry). The pair charges **half** to the maker at placement (from the escrowed CW20, sent to `treasury`; the resting order’s `remaining` is reduced) and **half** on each book fill (taker leg), same notional bases as before (bids: token1 `cost`; asks: token0 fill). See [`docs/integrators.md`](./integrators.md).
- **`hint_after_order_id`:** reserved for future indexer-assisted insertion. The **current implementation ignores this field** and always walks from the book head (same as `find_insert_bid` / `find_insert_ask` in the pair crate). Clients may send `null`; wire compatibility is preserved.
- **`ExecuteMsg::CancelLimitOrder`:** `order_id`. Only the stored **owner** may cancel. Applies only while the order row exists in on-chain `LimitOrder` storage (not after an expiry park — use claim below).
- **`ExecuteMsg::ClaimExpiredLimitOrder`:** `order_id`. Owner-only. After a match walk **parks** an expired order (see **Expiry** below), escrow is refunded here — same token routing as cancel. Query **`ExpiredLimitRefund { order_id }`** for a pending row (`null` if none). **Blocked while the pair is paused** (same emergency gate as **`CancelLimitOrder`** — GitLab [**#120**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120)).
- **`ExecuteMsg::UpdateLimitOrderPrice`:** `order_id`, `price`, `hint_after_order_id`, `max_adjust_steps`. Owner-only; re-links the order in the FIFO book at a new price **without** charging the maker placement fee again (same `order_id` and `remaining`).

### Router

- Each `SwapOperation::TerraSwap` may include `hybrid: Option<HybridSwapParams>` (same fields as the pair hook). `None` is legacy pool-only.
- **`SimulateSwapOperations` / `ReverseSimulateSwapOperations`:** when `hybrid` is unset, the router uses each hop’s pool-only `Simulation` / `ReverseSimulation`. When `hybrid` is set, the router queries the pair’s **`HybridSimulation`** / **`HybridReverseSimulation`** (read-only book walk + pool leg), so quotes align with Pattern C for the same on-chain book snapshot. Legacy pool-only `Simulation` remains for integrators that do not pass `hybrid`. See [contracts-security-audit.md](./contracts-security-audit.md) invariant **L8**.

### Pair `Simulation` query

- The pair’s `Simulation` / `ReverseSimulation` queries use **reserves only** (no book). Off-chain tooling must not treat them as hybrid-aware.

### Pause (governance)

- When the pair is **paused**, `Receive` is blocked (no swap, no new limit orders), **`CancelLimitOrder` is blocked**, and **`ClaimExpiredLimitOrder` is blocked** — active resting limits and parked expired refund rows stay locked until governance unpauses (see [contracts-security-audit.md](./contracts-security-audit.md) **L6**, GitLab [**#120**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120)).
- **`IsPaused` query:** `{ "is_paused": {} }` → `{ "paused": bool }` so frontends can show accurate pause copy without guessing from failed transactions.

### Expiry (`expires_at`)

- If **`expires_at`** is set and a hybrid (or future) match walk reaches that order when **`block_time >= expires_at`**, the contract **does not** match it. The order is **removed from the DLL**, a row is stored in **`EXPIRED_LIMIT_CLAIMS`**, and **`PENDING_ESCROW_*` is left unchanged** until the maker calls **`ClaimExpiredLimitOrder`** (which CW20-transfers and then decrements pending — same economics as cancel).
- The taker transaction emits a wasm event **`limit_order_expired_parked`** (`action`, `order_id`, `maker`, `side`, `remaining`). **No** CW20 is sent to the maker in that taker tx — this keeps `balance − reserves − pending_escrow` aligned and fixes the stranded-funds / mis-sweep issue described in GitLab [**#120**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120).
- **`CancelLimitOrder`** only operates on **`ORDERS`**; it cannot fire after a park, so there is **no** path to refund the same escrow twice.
- While an order is still live on the book but past `expires_at`, the owner may **`CancelLimitOrder`** if the pair is unpaused — useful before any taker walks the book.

### Post-swap hooks and hybrid

- For hybrid swaps, `AfterSwap.return_asset.amount` is the **total** output (book + pool legs). `commission_amount` and `spread_amount` in the hook payload reflect the **pool leg only**; book-side fees go to `treasury` (maker half at placement + **taker** half per `limit_order_fill`, not the full pair fee in one event). Hooks and indexers must not assume `commission_amount` is the full fee for the transaction. See invariant **L7** in [contracts-security-audit.md](./contracts-security-audit.md) and [integrators.md](./integrators.md).

## Ordering (composite key, FIFO)

For each side, the book is a strict total order:

- **Price** is **token1 per token0** (same convention as the pool).
- **Bids:** sort by **descending** `price`, then **ascending** `order_id` (higher price first; at equal price, **lower** `order_id` is ahead in the queue — older orders first).
- **Asks:** sort by **ascending** `price`, then **ascending** `order_id` (lower ask price first; FIFO at equal price by `order_id`).

## Execution order in `execute_swap`

When `hybrid` is set: the pair consumes the **book leg** first (up to `max_maker_fills` distinct makers), then routes the **pool leg** (including any book remainder rolled per contract logic). Hooks, spread checks, and fee discount (`trader`) follow the existing swap path. The **pool** leg uses full **`effective_fee_bps`**; each **book fill** charges the **taker half** of `effective_fee_bps` on the fill notional (maker half was paid at order placement). The swap response still exposes a single `effective_fee_bps` attribute for the taker context.

## Indexer route solver

- **`GET /api/v1/route/solve`** — query params: `token_in`, `token_out` (CW20 addresses indexed in `assets`), optional `amount_in` (raw integer string), optional `hybrid_optimize` / `max_maker_fills` (see [indexer-invariants.md](./indexer-invariants.md)).
- Returns `hops` (pair + offer/ask tokens per hop), `router_operations` (default `hybrid: null` per hop; merged hybrid params when `hybrid_optimize=true`).
- Optional **`estimated_amount_out`:** set when `amount_in` is provided **and** `ROUTER_ADDRESS` is configured; the indexer calls the router `simulate_swap_operations` query on LCD.
- **Running indexer integration tests:** route tests live under [`indexer/tests/api_route_solve.rs`](../indexer/tests/api_route_solve.rs). They need Postgres; if multiple tests share one DB, use the serialized commands in [Testing — Shared Postgres and test parallelism](./testing.md#shared-postgres-and-test-parallelism).

## Indexer limit book (LCD proxy)

Design record: [ADR 0002](./adr/0002-limit-book-surfacing.md). Endpoints above require the pair address to exist in the indexer DB (**404** if unknown).

**LCD cost (SLO-style):** each `limit-book` page does at most **1 + N** successful smart queries in the steady state: one `order_book_head` when `after_order_id` is omitted, otherwise one `limit_order` lookup for the cursor, plus **one `limit_order` query per returned order** (up to `limit`). There is no server-side caching of book walks; LCD throttling surfaces as **502** from the indexer. Clients should use the smallest `limit` that fits their UI and paginate with `after_order_id` instead of repeating full walks.

## Tx attributes (indexer / analytics)

CosmWasm responses use **attributes** (visible in tx logs as events). Useful keys on the **pair** contract:

| Attribute | When |
|-----------|------|
| `action` = `place_limit_order` | Limit placed |
| `limit_order_placed`, `order_id` | Same tx |
| `side` (`bid` / `ask`), `price`, `owner` | Same tx (for indexers); omitted on older pair code |
| `maker_fee_amount`, `effective_fee_bps` | Same tx (placement fee split vs fills) |
| `expires_at` | Same tx when set |
| `action` = `update_limit_order_price` | Owner changed limit price in place |
| `action` = `cancel_limit_order` | Cancel |
| `action` = `claim_expired_limit_order` | Expired-limit refund (after park) |
| `limit_order_cancelled`, `owner` | Same tx |
| `action` = `limit_order_expired_parked` | Expired order removed from book into claim queue (wasm event; taker tx) |
| `action` = `swap` | Any swap |
| `book_return_amount`, `pool_return_amount`, `return_amount` | Hybrid breakdown |
| `limit_book_offer_consumed` | When the book leg consumed offer token |
| `action` = `limit_order_fill` | One **wasm event per maker fill** (not on the main swap attribute list) |
| `order_id`, `side` (`bid` / `ask`), `maker`, `price` | Per fill |
| `token0_amount`, `token1_amount`, `commission_amount` | Raw amounts in pair token0 / token1; `commission_amount` is the **taker** half for that fill (bid: token1; ask: token0) |

### Wasm attribute coverage vs indexer nulls (operators)

| Pair build | `place_limit_order` wasm attrs | `cancel_limit_order` wasm attrs | Indexer `limit_order_placements` / `limit_order_cancellations` |
|------------|-------------------------------|----------------------------------|-------------------------------------------------------------------|
| **Current** (main branch pair) | `side`, `price`, `owner`, `expires_at` when set | `owner` | Metadata columns populated when attrs appear in tx logs |
| **Legacy** (older deployed wasm omitting attrs) | May omit `side`, `price`, `owner` | May omit `owner` | Corresponding DB columns stay **null**; rows still keyed by `pair_id`, `order_id`, `tx_hash`, heights/timestamps |
| **`limit_order_fill` events** | Per-fill `order_id`, `side`, `maker`, `price`, amounts, `commission_amount` | — | Indexed in `limit_order_fills`; aligns with on-chain book fills |

The **indexer** persists `pool_return_amount`, `book_return_amount`, and `limit_book_offer_consumed` on `swap_events`, stores each `limit_order_fill` in `limit_order_fills`, and indexes wasm `place_limit_order` / `cancel_limit_order` into **`limit_order_placements`** and **`limit_order_cancellations`**. It decodes **`limit_order_expired_parked`** (taker tx; GitLab [**#120**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120)) and **`claim_expired_limit_order`** so each placement row tracks **`lifecycle_status`** (`active` → `parked_expired` → `refunded`) and **`remaining_escrow`** from the park event ([**#142**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/142)). **LCD note:** some REST paths flatten multiple logical wasm emissions into **one** `wasm` attribute stream where **`action=swap` appears after** `limit_order_expired_parked`; the indexer scans **every** `action` value per event so parked rows still update ([**#141**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/141) — contract still emits distinct CosmWasm events; the gap was off-chain parsing). HTTP: **`GET /api/v1/pairs/{addr}/trades`** includes hybrid fields and optional **`effective_fee_bps`** when present; **`GET /api/v1/pairs/{addr}/limit-fills`** and **`GET /api/v1/pairs/{addr}/limit-orders/{order_id}/fills`** expose per-maker fills; **`GET /api/v1/pairs/{addr}/limit-placements`** returns placement rows **without** a matching indexed cancel for the same `(pair, order_id)` (full cancel history remains on **`.../limit-cancellations`** — [GitLab **#135**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/135)), includes **`lifecycle_status`** on every row, defaults to **`active` + `parked_expired`** (excludes terminal **`refunded`**), and accepts **`?status=`** `active` \| `parked_expired` \| `refunded` \| `all`; **`GET /api/v1/pairs/{addr}/order-book-head`**, **`.../limit-book`**, and **`.../limit-book-shallow`** proxy on-chain book state (see [ADR 0002](./adr/0002-limit-book-surfacing.md)).

## dApp: retail form (wires, invariants)

Implementation: [`LimitOrdersPage`](../frontend-dapp/src/pages/LimitOrdersPage.tsx), [`TradeOrderTicket`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx), shared [`LimitOrderMyPlacementsPanel`](../frontend-dapp/src/components/trade/LimitOrderMyPlacementsPanel.tsx), components under [`frontend-dapp/src/components/trade/`](../frontend-dapp/src/components/trade/) ([issue #110](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/110)). Pure helpers: [`limitOrderExpiry.ts`](../frontend-dapp/src/utils/limitOrderExpiry.ts), [`limitOrderEscrowBalanceGate.ts`](../frontend-dapp/src/utils/limitOrderEscrowBalanceGate.ts) ([GitLab #130](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/130)), [`limitOrderNativeGasBalanceGate.ts`](../frontend-dapp/src/utils/limitOrderNativeGasBalanceGate.ts) ([GitLab #132](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/132)), [`limitOrderPriceReference.ts`](../frontend-dapp/src/utils/limitOrderPriceReference.ts) + [`limitOrderPricePlaceGate.ts`](../frontend-dapp/src/utils/limitOrderPricePlaceGate.ts) ([GitLab #154](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/154), **#166** pool fallback + hard submit gate), [`limitOrderFeeSummary.ts`](../frontend-dapp/src/utils/limitOrderFeeSummary.ts) (effective + maker placement bps for pre-submit copy — [GitLab **#157**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/157)), [`useLimitOrderPriceRefBundle.ts`](../frontend-dapp/src/hooks/useLimitOrderPriceRefBundle.ts), [`limitPlacementLifecycle.ts`](../frontend-dapp/src/utils/limitPlacementLifecycle.ts) ([GitLab #141](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/141)).

- **`expires_at`:** the UI may use local `datetime-local`, “24h / 7d / no expiry” presets, or (Advanced) a raw **Unix second** value; all map to the same `expires_at` field described under [Expiry (`expires_at`)](#expiry-expires_at). If both friendly controls and raw seconds are set, they must agree — they are one logical value in state.
- **`max_adjust_steps`:** the retail default is **32**; “max adjust” / “book insert walk” (labels vary) cap the on-chain book-head insert path — see [Messages (CosmWasm)](#messages-cosmwasm) and [Ordering](#ordering-composite-key-fifo) context. The UI may expose a larger numeric or preset; still bounded by the pair’s hard cap in `pair.rs` (UI clamp 256 in [`limitOrderExpiry.ts`](../frontend-dapp/src/utils/limitOrderExpiry.ts) constants).
- **Escrow `amount`:** the CW20 `send` amount uses the same balance query and **Max** affordance as the swap form so users see spendable balance before `increase_allowance` + `place_limit_order`.
- **Trade `/trade` ticket — market vs limit ([GitLab #152](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/152)):** `TradeOrderTicket` exposes **Limit** and **Market** tabs. **Market** executes a CW20 `send` → pair `swap` (Pattern C **hybrid** when enabled) with **global slippage** from `useDexStore`, `increase_allowance` preflight, native **two-tx** gas gate via `estimateMarketPairSwapSequenceUlunaFeesTotal` + `evaluateMarketSwapNativeGasPlaceGate` ([`transactions.ts`](../frontend-dapp/src/services/terraclassic/transactions.ts), [`limitOrderNativeGasBalanceGate.ts`](../frontend-dapp/src/utils/limitOrderNativeGasBalanceGate.ts)). Quotes prefer indexer **`POST /api/v1/route/solve`** with `hybrid_by_hop` aligned to the UI split, then fall back to pair `hybrid_simulation` / pool `simulation` — see [`docs/limit-orders.md` § Swap page: hybrid vs pool-only estimates](#swap-ui-hybrid-vs-pool-only-estimates) (L8 disclosure when the pool line is shown with a book leg configured). Implementation: [`TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx).
- **Limit pre-submit summary vs market quote ([GitLab #157](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/157)):** the **Limit** path shows **before Place** that resting orders are filled **over time** (no taker slippage / pool price impact / min-received line), repeats **% deviation vs reference**, shows **maker placement fee** (`floor(effective/2)` bps) from pair fee + optional discount query, and a **minimum LUNC** line for the two-tx place sequence. **Market** tab keeps the simulation receive / min-received UX. Docs: [`docs/frontend.md` § Limit order pre-submit summary](./frontend.md#trade-page-limit-order-pre-submit-summary); [`LimitOrderPreSubmitSummary.tsx`](../frontend-dapp/src/components/trade/LimitOrderPreSubmitSummary.tsx).
- **Trade `/trade` ticket — post-only style limit preflight (#152):** before broadcasting `place_limit_order`, the UI blocks **bid price ≥ best ask** and **ask price ≤ best bid** using head rows from indexer **`GET .../limit-book?side=&limit=1`** (same token1/token0 price convention as the contract). This is a **client-only** guard for clearer UX; the on-chain pair does not reject “marketable” limit prices by itself.
- **Preflight vs on-chain balance (GitLab [#130](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/130)):** the dApp must **not** broadcast `increase_allowance` when the signed human amount (converted to raw units) **exceeds** the wallet’s CW20 balance for the escrow token, or when balance is **still loading** / **unreadable** — `increase_allowance` can succeed with zero balance, then `place_limit_order` fails at `transfer_from`, burning gas twice. Implementation: shared gate [`limitOrderEscrowBalanceGate.ts`](../frontend-dapp/src/utils/limitOrderEscrowBalanceGate.ts) + inline UI [`LimitOrderEscrowPlaceGuardMessage.tsx`](../frontend-dapp/src/components/trade/LimitOrderEscrowPlaceGuardMessage.tsx); wired from [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx) and [`LimitOrdersPage.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.tsx).
- **Limit price vs tape / pool reference (GitLab [#154](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/154), [#166](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/166)):** the trade ticket and standalone limit page show **last-trade** token1/token0 when the indexer tape is available; otherwise they resolve an **AMM pool spot** from on-chain reserves (LCD `pool` query) when decimals are known, and **block** place when no reference can be computed (no silent bypass). **% deviation**, **headline-scaled USD** (tape headline only), and **submit gate** use the resolved reference. Invariants: [`docs/frontend.md` § Trade page — limit order price field](./frontend.md#trade-page-limit-order-price); agent skill [`skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](../skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md).
- **Escrow amount — headline USD + Bid/Ask sizing (GitLab [#155](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/155)):** the **Amount** field shows a **headline-scaled USD** line using `escrowAmountUsdAnchorNotional` (same tape headline + `refToken1PerToken0` as the limit price USD anchor). **Bid ↔ Ask** clears manually typed amounts; **Max** reapplies full balance after the new escrow token balance loads. Docs: [`docs/frontend.md` § Limit place — escrow amount](./frontend.md#limit-place-escrow-amount); [`useLimitOrderForm.ts`](../frontend-dapp/src/hooks/useLimitOrderForm.ts).
- **Post-placement indexer poll (GitLab [#131](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/131)):** after a successful place tx, the UI polls [`GET /api/v1/pairs/{addr}/limit-placements`](./indexer-invariants.md) to resolve the new **`order_id`** and pre-fill cancel. **Local dev:** ensure indexer **`CORS_ORIGINS`** includes the Vite **`Origin`** you use (`localhost` vs `127.0.0.1` — see [`docs/frontend.md` § Local dev indexer CORS](./frontend.md#local-dev-indexer-cors)). Failures log **`[limit-place] indexer poll failed:`** via [`warnIndexerPlacementPollFailed`](../frontend-dapp/src/utils/warnIndexerPlacementPollFailed.ts).
- **Cancel UX vs indexer (GitLab [#135](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/135)):** the indexer **`limit-placements`** feed omits orders once a matching **`limit-cancellations`** row exists; the dApp **invalidates** both queries after a successful cancel, **blocks** submit when the typed id is already in the indexed cancel list, and **humanizes** CosmWasm `LimitOrder` map “not found” errors via [`tryHumanizeTerraTxMessage`](../frontend-dapp/src/utils/humanizeTerraTxError.ts) (wired from [`transactions.ts`](../frontend-dapp/src/services/terraclassic/transactions.ts); helpers in [`limitOrderCancelUserMessage.ts`](../frontend-dapp/src/utils/limitOrderCancelUserMessage.ts)).
- **Trade `/trade` order book — row cancel / edit / cancel-all (GitLab [#162](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/162)):** the paginated **Bids / Asks** tables show **`#order_id`** on each row; rows whose **`owner`** matches the connected wallet expose **Edit** (prefills the limit ticket — replace flow still requires canceling the resting order first) and **×** cancel (shared `useLimitOrderCancelMutation` with the ticket’s **Manage** section). **Cancel all mine** walks active indexed placements. Invariants: [`docs/frontend.md` § Trade page — order book row actions](./frontend.md#trade-book-row-actions); skill [`skills/AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md`](../skills/AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md).
- **Parked-expired recovery (GitLab [#141](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/141); indexer [#142](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/142)):** default **`GET .../limit-placements`** includes **`lifecycle_status`** **`active`** and **`parked_expired`**. The dApp **`LimitOrderMyPlacementsPanel`** ([`LimitOrderMyPlacementsPanel.tsx`](../frontend-dapp/src/components/trade/LimitOrderMyPlacementsPanel.tsx)) renders active rows separately from parked-expired rows (amber highlight), shows **Claim refund** wired to **`ExecuteMsg::ClaimExpiredLimitOrder`** ([`claimExpiredLimitOrder`](../frontend-dapp/src/services/terraclassic/pair.ts)), and humanizes **`NoExpiredLimitClaim`** via [`limitClaimUserMessage.ts`](../frontend-dapp/src/utils/limitClaimUserMessage.ts). **Claim is disabled while the pair is paused** (L6 — same policy as cancel; GitLab [**#120**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120)); place/cancel stay blocked. Pure lifecycle helpers: [`limitPlacementLifecycle.ts`](../frontend-dapp/src/utils/limitPlacementLifecycle.ts). Third-party agents: [`skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](../skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md).

**Invariants (#141 parked-expired UX)**

- **Indexer fields:** UI trusts **`lifecycle_status`** from the indexer default listing (**`active`** \| **`parked_expired`**); legacy rows without the column are treated as **`active`**. **`remaining_escrow`** is shown for parked rows (bid → token1 decimals, ask → token0 decimals).
- **Contract:** only **`claim_expired_limit_order`** is submitted from **Claim refund**; gas limit **`CLAIM_EXPIRED_LIMIT_ORDER_GAS_LIMIT`** tracks cancel-class envelopes in [`transactions.ts`](../frontend-dapp/src/services/terraclassic/transactions.ts).
- **Queries:** successful claim **invalidates** `limitPlacements`, pair book previews, and **`tokenBalance`** so escrow updates after refund.

- **Preflight vs native LUNC for two fees (GitLab [#132](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/132)):** the dApp must **not** broadcast `increase_allowance` when bank **uluna** is below the **sum** of the two fee envelopes the UI will attach (`increase_allowance` then CW20 `send` → `place_limit_order`), or when LUNC balance is **loading** / **unreadable** — otherwise the first tx can succeed and burn gas while the user cannot complete the second. Required uluna is computed with the same gas limits and `effectiveGasPriceUluna()` as broadcast: [`estimateLimitOrderPlaceSequenceUlunaFeesTotal`](../frontend-dapp/src/services/terraclassic/transactions.ts) + gate [`limitOrderNativeGasBalanceGate.ts`](../frontend-dapp/src/utils/limitOrderNativeGasBalanceGate.ts); hook [`useNativeUlunaBalance.ts`](../frontend-dapp/src/hooks/useNativeUlunaBalance.ts) (React Query key `['tokenBalance', address, 'uluna']`).
- **Related — pool add liquidity, three fees (GitLab [#147](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/147)):** CW20/CW20 **Provide Liquidity** uses **three** sequential txs (`increase_allowance` ×2 then `provide_liquidity`) with the same “sum native fees before first broadcast” pattern as [#132](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/132): [`estimateProvideLiquidityCw20SequenceUlunaFeesTotal`](../frontend-dapp/src/services/terraclassic/transactions.ts) + [`provideLiquidityNativeGasBalanceGate.ts`](../frontend-dapp/src/utils/provideLiquidityNativeGasBalanceGate.ts). See [`docs/frontend.md` § Pool page](./frontend.md#pool-page--provide-liquidity-ui-invariants). **Rollback** (`provide_liquidity` fails after allowances): [`pair.ts`](../frontend-dapp/src/services/terraclassic/pair.ts) sends **both** `decrease_allowance` messages in **one** [`executeTerraContractMulti`](../frontend-dapp/src/services/terraclassic/transactions.ts) tx (one prompt / one fee).
**Invariants (#130 escrow + #132 native gas preflight)**

- **Raw comparison (escrow):** gate compares `toRawAmount(amountHuman, escrowDecimals)` to the LCD/CW20 balance string as **BigInt** (same units as the allowance / hook `send` amount).
- **Uluna comparison (gas):** gate compares bank `uluna` balance (BigInt string from LCD) to `estimateLimitOrderPlaceSequenceUlunaFeesTotal()` — must stay aligned with `getGasLimitForTx` + `estimateTerraClassicFee` in [`transactions.ts`](../frontend-dapp/src/services/terraclassic/transactions.ts) when those gas limits or fee math change.
- **Conservative when uncertain:** if either balance query is **loading**, **errored**, or **missing `data`**, the combined gate is closed — no allowance tx.
- **Empty human amount:** escrow gate returns no inline message; native gas gate is open (no “add LUNC” noise until the user enters an amount). Submit remains blocked by the zero-amount path.
- **UI + mutation:** the Place button is disabled when either gate is closed, and `mutationFn` re-evaluates both gates so a stale click cannot reach `executeTerraContract`.
- **Broadcast `Fee` (gas price × `gas_wanted`):** placement is **`increase_allowance`** (base gas) then **`send` → `place_limit_order`**. The dApp must attach enough **uluna** in `Fee.amount` for Station / network minimum gas price; with **`VITE_NETWORK=local`**, Station connect runs **`experimentalSuggestChain`** before and after extension connect, the cosmes **`KeplrExtension`** patch passes **`preferNoSetFee`** on each sign call, and the patched **`StationController`** uses **amino** signing when **`chainId`** matches LocalTerra (**case-insensitive**) **or** the account is Ledger (**`isNanoLedger`**) (see [`docs/frontend.md` § Terra Classic gas limits](./frontend.md#terra-classic-gas-limits), [`patches/@goblinhunt+cosmes+0.0.71-ghunt.21.patch`](../frontend-dapp/patches/@goblinhunt+cosmes+0.0.71-ghunt.21.patch)). **QA:** verify with **both Station and Terra Classic Keplr** on LocalTerra — extension fee handling can diverge. [GitLab #127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127).
- **Inline copy precedence:** when both gates would show text, the UI shows the escrow message first until the escrow gate has no message (then native gas), so users see CW20 issues before LUNC.

**Docs / work splits:** when splitting large follow-up changes into reviewable pieces, the **split-to-prs** skill in the Cursor *skills* family is the intended workflow (small branches, one concern per change).

## Example JSON (logical shapes)

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
