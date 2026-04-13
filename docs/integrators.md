# Integrator reference

Audience: protocols, indexers, and wallets integrating with CL8Y pair hooks, hybrid swaps, and the on-chain limit book. End-user UX lives elsewhere.

## Hybrid swaps and post-swap hooks (invariant L7)

On a **hybrid** swap (pool + limit book in one execution), the pair invokes each registered hook with `AfterSwap` after settlement.

| Field | Meaning on hybrid txs |
|-------|------------------------|
| `return_asset.amount` | **Total** output to the receiver: book leg net **plus** pool leg net (same units as the ask asset). |
| `commission_amount` | **Pool leg only** — CW20 amount sent to `treasury` from the constant-product leg. |
| `spread_amount` | **Pool leg only** — TerraSwap-style spread metric from the pool leg. |

Book-side fees are collected inside the book match path (`limit_order_fill` events, treasury transfers in token0/token1 per side). Do **not** treat `commission_amount` in `AfterSwap` as the full protocol fee for the transaction.

Canonical references: [contracts-security-audit.md](./contracts-security-audit.md) (L7), [limit-orders.md](./limit-orders.md) (hooks + book).

## Limit book fees (maker / taker split)

Total limit-book fee rate matches the pair’s **effective** swap commission (`fee_bps` after the optional fee-discount registry), paid to `treasury`.

- **Maker half** is charged once when the order is placed (`Cw20HookMsg::PlaceLimitOrder`), from the escrowed CW20 amount. The resting order’s `remaining` is reduced accordingly.
- **Taker half** is charged on each **fill** against the book (same notional bases as before: bids — token1 `cost`; asks — token0 `fill`), and appears as `commission_amount` on `limit_order_fill` wasm events for that fill.

Updating only the **price** of an existing order (`ExecuteMsg::UpdateLimitOrderPrice`) re-links the order in the FIFO book **without** charging the maker placement fee again. Cancel + new placement pays a new maker-side half.

Details and tx attributes: [limit-orders.md](./limit-orders.md).

## Slippage: `max_spread` and `belief_price` (hybrid)

Slippage checks run in the pair after the book leg and pool leg are computed. See [ADR 0001](./adr/0001-hybrid-quoting-and-routing.md) for the high-level rule.

**Without `belief_price`:** The check compares `max_spread` to a ratio whose numerator is the pool leg’s constant-product spread metric (capped by pool gross output) and whose denominator is **pool gross output plus book net output to the taker** (`pool_net + pool_commission + book_return_net` in ask units). So the book leg scales the denominator even though the spread numerator comes from the pool leg.

**With `belief_price`:** Expected output is `offer_amount / belief_price` (in ask units). Actual output used in the inequality is `book_return_net + pool_net_to_receiver + pool_commission` (pool commission to treasury counts on the “actual” side).

These are **execution** semantics; accurate quoting for hybrid uses `HybridSimulation` / router ops with `hybrid` set (invariant L8).

## Route discovery and quotes (L8, `hybrid: null`)

The indexer exposes multi-hop routing under `/api/v1/route/solve` (see [indexer-invariants.md](./indexer-invariants.md) for full HTTP semantics).

| Method | Role |
|--------|------|
| **`GET`** | BFS path discovery. Response `router_operations` use **`terra_swap.hybrid: null` on every hop** — pool-only ops for backward-compatible clients. Optional `estimated_amount_out` when `amount_in` and `ROUTER_ADDRESS` are set uses LCD `simulate_swap_operations` on that pool-only shape. |
| **`POST`** | Same discovery, plus optional **`hybrid_by_hop`**: one entry per hop (`null` = pool-only that hop, or a `HybridSwapParams`-shaped object). The indexer merges these into `router_operations` and, when `amount_in` and `ROUTER_ADDRESS` are configured, runs the **same** LCD `simulate_swap_operations` the chain uses for the merged message — so quotes can include limit-book legs when your splits are valid. |

**Invariant L8:** Pool-only pair queries and router sims **without** `hybrid` do not model the book. For book-inclusive quotes you must set `hybrid` on the router op (or use pair `HybridSimulation` / `HybridReverseSimulation` directly). See [limit-orders.md](./limit-orders.md) and [ADR 0001](./adr/0001-hybrid-quoting-and-routing.md).

## Related docs

- [limit-orders.md](./limit-orders.md) — messages, pause, indexer, events.
- [contracts-security-audit.md](./contracts-security-audit.md) — invariant matrix.
- [ADR 0001](./adr/0001-hybrid-quoting-and-routing.md) — hybrid routing and quoting scope.
