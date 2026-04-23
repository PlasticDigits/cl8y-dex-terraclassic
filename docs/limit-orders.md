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

**Resting limits** are on-chain: query the pair contract with `LimitOrder { order_id }` and `OrderBookHead { side }` via LCD or any CosmWasm client. The **indexer also proxies** those reads for integrators and the dApp (see [ADR 0002: Limit book surfacing](./adr/0002-limit-book-surfacing.md)):

- **`GET /api/v1/pairs/{addr}/order-book-head?side=bid|ask`** — JSON `{ "head_order_id": <u64> | null }` from LCD `OrderBookHead`.
- **`GET /api/v1/pairs/{addr}/limit-book?side=bid|ask&limit=L&after_order_id=OPTIONAL`** — **paginated** walk from head or keyset cursor along `next` (default `limit` 50, max 100 per HTTP response). Response includes `orders`, `has_more`, and `next_after_order_id` (pass the latter as `after_order_id` for the next page). LCD errors → **502**; invalid cursor / side mismatch → **400**.
- **`GET /api/v1/pairs/{addr}/limit-book-shallow?side=bid|ask&depth=N`** — legacy small preview walk (default depth 10, max 20); prefer `limit-book` for full depth.

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
- **`ExecuteMsg::CancelLimitOrder`:** `order_id`. Only the stored **owner** may cancel.
- **`ExecuteMsg::UpdateLimitOrderPrice`:** `order_id`, `price`, `hint_after_order_id`, `max_adjust_steps`. Owner-only; re-links the order in the FIFO book at a new price **without** charging the maker placement fee again (same `order_id` and `remaining`).

### Router

- Each `SwapOperation::TerraSwap` may include `hybrid: Option<HybridSwapParams>` (same fields as the pair hook). `None` is legacy pool-only.
- **`SimulateSwapOperations` / `ReverseSimulateSwapOperations`:** when `hybrid` is unset, the router uses each hop’s pool-only `Simulation` / `ReverseSimulation`. When `hybrid` is set, the router queries the pair’s **`HybridSimulation`** / **`HybridReverseSimulation`** (read-only book walk + pool leg), so quotes align with Pattern C for the same on-chain book snapshot. Legacy pool-only `Simulation` remains for integrators that do not pass `hybrid`. See [contracts-security-audit.md](./contracts-security-audit.md) invariant **L8**.

### Pair `Simulation` query

- The pair’s `Simulation` / `ReverseSimulation` queries use **reserves only** (no book). Off-chain tooling must not treat them as hybrid-aware.

### Pause (governance)

- When the pair is **paused**, `Receive` is blocked (no swap, no new limit orders) and **`CancelLimitOrder` is blocked** — resting limit escrow stays locked until governance unpauses (see [contracts-security-audit.md](./contracts-security-audit.md) **L6**).
- **`IsPaused` query:** `{ "is_paused": {} }` → `{ "paused": bool }` so frontends can show accurate pause copy without guessing from failed transactions.

### Expiry (`expires_at`)

- If **`expires_at`** is set and a swap’s match walk reaches that order when **`block_time >= expires_at`**, the order is **unlinked**, pending escrow is decremented for its remaining size, and **no** CW20 transfer to the maker is performed in that transaction. Tokens stay in the pair contract and follow normal **sweep** rules (excess over reserves + pending). **Cancel** still refunds the maker while the order exists and is unexpired.

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
| `limit_order_cancelled`, `owner` | Same tx |
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

The **indexer** persists `pool_return_amount`, `book_return_amount`, and `limit_book_offer_consumed` on `swap_events`, stores each `limit_order_fill` in `limit_order_fills`, and indexes wasm `place_limit_order` / `cancel_limit_order` into **`limit_order_placements`** and **`limit_order_cancellations`**. HTTP: **`GET /api/v1/pairs/{addr}/trades`** includes hybrid fields and optional **`effective_fee_bps`** when present; **`GET /api/v1/pairs/{addr}/limit-fills`** and **`GET /api/v1/pairs/{addr}/limit-orders/{order_id}/fills`** expose per-maker fills; **`GET /api/v1/pairs/{addr}/limit-placements`** and **`.../limit-cancellations`** list lifecycle events; **`GET /api/v1/pairs/{addr}/order-book-head`**, **`.../limit-book`**, and **`.../limit-book-shallow`** proxy on-chain book state (see [ADR 0002](./adr/0002-limit-book-surfacing.md)).

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
