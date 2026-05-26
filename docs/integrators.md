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

These are **execution** semantics; all quoting uses `HybridSimulation` / `HybridReverseSimulation` (invariant L8; legacy `Simulation` removed in [#190](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/190)).

## Route discovery and quotes (L8)

The indexer exposes multi-hop routing under `/api/v1/route/solve` (see [indexer-invariants.md](./indexer-invariants.md) for full HTTP semantics).

| Method | Role |
|--------|------|
| **`GET`** | BFS path discovery (default **max 4 hops**). By default, `router_operations` use **`terra_swap.hybrid: null` on every hop** — pool-only ops for backward-compatible clients. Optional `estimated_amount_out` when `amount_in` and `ROUTER_ADDRESS` are set uses LCD `simulate_swap_operations` on that pool-only shape. With **`hybrid_optimize=true`** (requires `amount_in`), the indexer uses **max 3 hops**, optimizes per-hop splits via pair **`HybridSimulation`**, merges `hybrid` into ops, and returns `intermediate_tokens`, `quote_kind`, and `hybrid_notes` (see invariants doc). Pass **`pool_only=true`** to force pool-only ops even when `amount_in` is set. |
| **`GET /best`** | **Retail / Vyntrex best-execution path** ([GitLab **#189**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/189)): same as `hybrid_optimize=true` but **`amount_in` is required** and hybrid optimization always runs (max **3 hops**). Prefer this endpoint for server-chosen book+pool splits. |
| **`POST`** | Discovery (**max 4 hops**), plus optional **`hybrid_by_hop`**: one entry per hop (`null` = pool-only that hop, or a `HybridSwapParams`-shaped object). The indexer merges these into `router_operations` and, when `amount_in` and `ROUTER_ADDRESS` are configured, runs the **same** LCD `simulate_swap_operations` the chain uses for the merged message — so quotes can include limit-book legs when your splits are valid. |

**Invariant L8:** Pool-only quotes use `hybrid_simulation` with `book_input = 0` (helpers: `pool_only_hybrid_params`, `pool_only_hybrid_template`). Router ops with `hybrid: null` still get pool-only hybrid quotes on-chain. For book-inclusive quotes set non-zero `book_input` on the router op or pair query. See [limit-orders.md](./limit-orders.md), [ADR 0001](./adr/0001-hybrid-quoting-and-routing.md), and [skills/AGENTS_HYBRID_QUOTING.md](../skills/AGENTS_HYBRID_QUOTING.md).

## Vyntrex / Terraport hybrid event mapping (GitLab #189)

Hybrid swaps emit **Terraport-compatible baseline attrs** plus CL8Y leg breakdown. Vyntrex can parse baseline fields like Terraport; use the extensions for volume reconciliation.

| Terraport / baseline attr | CL8Y hybrid swap | Notes |
|---------------------------|------------------|-------|
| `offer_amount` | `offer_amount` | Total offer consumed (book + pool legs). |
| `return_amount` | `return_amount` | Total ask output to receiver (`pool_return_amount` + `book_return_amount`). |
| `spread_amount` | `spread_amount` | **Pool leg only** (TerraSwap-style). |
| `commission_amount` | `commission_amount` | **Pool leg only** (treasury from AMM leg). |
| *(none)* | `pool_return_amount` | Ask-side net from constant-product leg. |
| *(none)* | `book_return_amount` | Ask-side net from limit-book leg to taker. |
| *(none)* | `limit_book_offer_consumed` | Offer-side amount matched against the book (present when book leg > 0). |
| *(none)* | `effective_fee_bps` | Pair effective swap fee after discount registry. |

Indexer persistence and `/api/v1/pairs/{addr}/trades` expose the same columns. CG/CMC listing endpoints use **consolidated** totals in standard volume fields; optional `cl8y_extensions` / per-trade `pool_leg_volume` / `book_leg_volume` attribute book vs pool without double-counting — see [CG_CMC_COMPLIANCE.md](./CG_CMC_COMPLIANCE.md#consolidated-hybrid--pool-only-reporting-gitlab-189).

Full Terraport reference: [terraport.md](./terraport.md).

## Related docs

- [limit-orders.md](./limit-orders.md) — messages, pause, indexer, events.
- [contracts-security-audit.md](./contracts-security-audit.md) — invariant matrix.
- [ADR 0001](./adr/0001-hybrid-quoting-and-routing.md) — hybrid routing and quoting scope.
