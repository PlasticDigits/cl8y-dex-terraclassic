# Limit orders and hybrid swaps

This document is the implementation reference for the hybrid AMM + FIFO limit book. Canonical message shapes live in [`smartcontracts/packages/dex-common/src/pair.rs`](../smartcontracts/packages/dex-common/src/pair.rs).

## Exchange API “orderbook” vs on-chain limit book

CoinGecko/CoinMarketCap [`GET /cg/orderbook`](./CG_CMC_COMPLIANCE.md#get-cgorderbook) and [`GET /cmc/orderbook/:market_pair`](./CG_CMC_COMPLIANCE.md#get-cmcorderbookmarket_pair) return an **AMM-simulated** level-2 book (walking the bonding curve). That is **not** the FIFO limit book stored on pairs.

**Resting limits** are on-chain: query the pair contract with `LimitOrder { order_id }` and `OrderBookHead { side }` via LCD or any CosmWasm client. The indexer does not currently expose these as HTTP proxy endpoints; [`GET /api/v1/route/solve`](./indexer-invariants.md) only discovers routes and returns pool-only `hybrid: null` operations (clients patch `hybrid` off-chain if needed).

## Messages (CosmWasm)

### Swap with Pattern C (`Cw20HookMsg::Swap`)

- **`hybrid`:** optional [`HybridSwapParams`](../smartcontracts/packages/dex-common/src/pair.rs): `pool_input`, `book_input` (must sum to the CW20 `amount`), `max_maker_fills`, optional `book_start_hint` (order id).
- **Match walk:** If `book_start_hint` is set and that order id still exists, matching starts from that id; otherwise it starts from the book head (see `orderbook::match_bids` / `match_asks`).
- **`MAX_ADJUST_STEPS`:** placement uses `PlaceLimitOrder { max_adjust_steps }` to cap the linear walk when finding an insert position from the **book head**. Hard caps: `MAX_ADJUST_STEPS_HARD_CAP` / `MAX_MAKER_FILLS_HARD_CAP` in `dex-common::pair`.

### Place / cancel limit

- **`Cw20HookMsg::PlaceLimitOrder`:** `side`, `price`, `hint_after_order_id`, `max_adjust_steps`, optional **`expires_at`** (Unix seconds; `null` = no expiry). If set, it must be **strictly greater** than the block time at placement.
- **`hint_after_order_id`:** reserved for future indexer-assisted insertion. The **current implementation ignores this field** and always walks from the book head (same as `find_insert_bid` / `find_insert_ask` in the pair crate). Clients may send `null`; wire compatibility is preserved.
- **`ExecuteMsg::CancelLimitOrder`:** `order_id`. Only the stored **owner** may cancel.

### Router

- Each `SwapOperation::TerraSwap` may include `hybrid: Option<HybridSwapParams>` (same fields as the pair hook). `None` is legacy pool-only.
- **`SimulateSwapOperations` / `ReverseSimulateSwapOperations`:** the router **does not** apply `hybrid` when querying the pair — it only uses each hop’s pool `Simulation` / `ReverseSimulation`, which are **AMM-only** and do not include limit-book fills. Quotes can diverge from executed hybrid swaps whenever the on-chain book is non-empty. See [contracts-security-audit.md](./contracts-security-audit.md) invariant **L8**.

### Pair `Simulation` query

- The pair’s `Simulation` / `ReverseSimulation` queries use **reserves only** (no book). Off-chain tooling must not treat them as hybrid-aware.

### Pause (governance)

- When the pair is **paused**, `Receive` is blocked (no swap, no new limit orders). **`CancelLimitOrder` is not paused** — makers can cancel resting orders and receive escrow refunds while the pair is paused (see [contracts-security-audit.md](./contracts-security-audit.md) **L6**).

### Expiry (`expires_at`)

- If **`expires_at`** is set and a swap’s match walk reaches that order when **`block_time >= expires_at`**, the order is **unlinked**, pending escrow is decremented for its remaining size, and **no** CW20 transfer to the maker is performed in that transaction. Tokens stay in the pair contract and follow normal **sweep** rules (excess over reserves + pending). **Cancel** still refunds the maker while the order exists and is unexpired.

### Post-swap hooks and hybrid

- For hybrid swaps, `AfterSwap.return_asset.amount` is the **total** output (book + pool legs). `commission_amount` and `spread_amount` in the hook payload reflect the **pool leg only**; book-side fees are sent to `treasury` inside the book match path. Hooks and indexers must not assume `commission_amount` is the full fee for the transaction. See invariant **L7** in [contracts-security-audit.md](./contracts-security-audit.md).

## Ordering (composite key, FIFO)

For each side, the book is a strict total order:

- **Price** is **token1 per token0** (same convention as the pool).
- **Bids:** sort by **descending** `price`, then **ascending** `order_id` (higher price first; at equal price, **lower** `order_id` is ahead in the queue — older orders first).
- **Asks:** sort by **ascending** `price`, then **ascending** `order_id` (lower ask price first; FIFO at equal price by `order_id`).

## Execution order in `execute_swap`

When `hybrid` is set: the pair consumes the **book leg** first (up to `max_maker_fills` distinct makers), then routes the **pool leg** (including any book remainder rolled per contract logic). Hooks, spread checks, and fee discount (`trader`) follow the existing swap path.

## Indexer route solver

- **`GET /api/v1/route/solve`** — query params: `token_in`, `token_out` (CW20 addresses indexed in `assets`), optional `amount_in` (raw integer string).
- Returns `hops` (pair + offer/ask tokens per hop), `router_operations` (TerraSwap ops with `hybrid: null` for pool-only routing).
- Optional **`estimated_amount_out`:** set when `amount_in` is provided **and** `ROUTER_ADDRESS` is configured; the indexer calls the router `simulate_swap_operations` query on LCD.
- **Running indexer integration tests:** route tests live under [`indexer/tests/api_route_solve.rs`](../indexer/tests/api_route_solve.rs). They need Postgres; if multiple tests share one DB, use the serialized commands in [Testing — Shared Postgres and test parallelism](./testing.md#shared-postgres-and-test-parallelism).

## Tx attributes (indexer / analytics)

CosmWasm responses use **attributes** (visible in tx logs as events). Useful keys on the **pair** contract:

| Attribute | When |
|-----------|------|
| `action` = `place_limit_order` | Limit placed |
| `limit_order_placed`, `order_id` | Same tx |
| `action` = `cancel_limit_order` | Cancel |
| `limit_order_cancelled` | Same tx |
| `action` = `swap` | Any swap |
| `book_return_amount`, `pool_return_amount`, `return_amount` | Hybrid breakdown |
| `limit_book_offer_consumed` | When the book leg consumed offer token |

Fine-grained per-fill lines are not required for basic sync; the indexer can combine on-chain queries (`LimitOrder`, `OrderBookHead`) with these attributes for reconciliation.

The **tx indexer** does not yet persist hybrid/limit-specific attributes into dedicated tables; treat chain queries and logs as authoritative for book-level analytics until extended.

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
