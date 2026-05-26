# Integrator reference

Audience: protocols, indexers, and wallets integrating with CL8Y pair hooks, hybrid swaps, and the on-chain limit book. End-user UX lives elsewhere.

## Hybrid swaps and post-swap hooks (invariant L7)

On a **hybrid** swap (pool + limit book in one execution), the pair invokes each registered hook with `AfterSwap` after settlement.

| Field | Meaning |
|-------|---------|
| `return_asset.amount` | **Total** output to the receiver: book leg net **plus** pool leg net (same units as the ask asset). |
| `commission_amount` | **Total protocol commission** in the ask asset: pool leg treasury transfer **plus** book taker fees (sum of per-fill commissions). Matches `HybridSimulation.commission_amount`. |
| `spread_amount` | **Pool leg only** — TerraSwap-style spread metric from the pool leg. |

Per-fill book taker fees are also attributed on `limit_order_fill` events (and swap txs emit `book_commission_amount` when the book leg runs). Swap **baseline** attrs keep pool-only `commission_amount` for Terraport/Vyntrex compatibility — see [Vyntrex mapping](#vyntrex--terraport-hybrid-event-mapping-gitlab-189).

**Breaking change (GitLab [#196](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/196)):** hooks previously received pool-leg `commission_amount` only on hybrid txs; they now receive the total above.

Canonical references: [contracts-security-audit.md](./contracts-security-audit.md) (L7), [limit-orders.md](./limit-orders.md) (hooks + book), [hooks README](../smartcontracts/contracts/hooks/README.md), [skills/AGENTS_HOOK_COMMISSION.md](../skills/AGENTS_HOOK_COMMISSION.md).

## Limit book fees (maker / taker split)

Total limit-book fee rate matches the pair’s **effective** swap commission (`fee_bps` after the optional fee-discount registry), paid to `treasury`.

- **Maker half** is charged once when the order is placed (`Cw20HookMsg::PlaceLimitOrder`), from the escrowed CW20 amount. The resting order’s `remaining` is reduced accordingly.
- **Taker half** is charged on each **fill** against the book (same notional bases as before: bids — token1 `cost`; asks — token0 `fill`), and appears as `commission_amount` on `limit_order_fill` wasm events for that fill.

Updating only the **price** of an existing order (`ExecuteMsg::UpdateLimitOrderPrice`) re-links the order in the FIFO book **without** charging the maker placement fee again. Cancel + new placement pays a new maker-side half.

Details and tx attributes: [limit-orders.md](./limit-orders.md).

## On-chain limit book (LCD proxy) {#on-chain-limit-book-lcd-proxy}

Resting **FIFO limit orders** live on each pair contract. The indexer exposes read-only HTTP that **proxies CosmWasm smart queries** on LCD — same JSON shapes as on-chain `OrderBookHead` / `LimitOrder` ([ADR 0002](./adr/0002-limit-book-surfacing.md), [GitLab **#194**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/194)).

| Endpoint | Purpose |
|----------|---------|
| **`GET /api/v1/pairs/{addr}/order-book-head?side=bid\|ask`** | Best `order_id` on that side, or `null` if empty. |
| **`GET /api/v1/pairs/{addr}/limit-book?side=bid\|ask&limit=L&after_order_id=OPTIONAL`** | **Paginated** walk along on-chain `next` pointers. Default **`limit=50`**, max **100** per response. Response: `{ side, orders[], has_more, next_after_order_id }`. Pass **`next_after_order_id`** as **`after_order_id`** for the next page. |
| **`GET /api/v1/pairs/{addr}/limit-book-shallow?side=bid\|ask&depth=N`** | Legacy preview (default **10**, max **20**). Prefer **`limit-book`** for pro depth. |

**Errors:** unknown pair → **404**; LCD failure → **502**; invalid cursor / side mismatch → **400**. When **`RATE_LIMIT_RPS > 0`**, sustained abuse → **429** ([indexer-invariants.md](./indexer-invariants.md)).

**LCD cost:** each **`limit-book`** page costs up to **1 + limit** smart queries (head or cursor lookup + one `limit_order` per returned row). No server-side caching of arbitrary deep walks — **clients paginate**.

**Not the AMM book:** CoinGecko/CoinMarketCap **`/cg/orderbook`** and **`/cmc/orderbook`** simulate pool curve depth — not the on-chain limit book ([CG_CMC_COMPLIANCE.md](./CG_CMC_COMPLIANCE.md)).

OpenAPI: served from the indexer Swagger UI (`/swagger-ui/`). Regression tests: [`api_limit_book_lcd_mock.rs`](../indexer/tests/api_limit_book_lcd_mock.rs), [`api_limit_book_deep.rs`](../indexer/tests/api_limit_book_deep.rs).

**dApp reference:** [`skills/AGENTS_FRONTEND_DEEP_ORDER_BOOK.md`](../skills/AGENTS_FRONTEND_DEEP_ORDER_BOOK.md).

## Slippage: `max_spread` and `belief_price` (hybrid)

Slippage checks run in the pair after the book leg and pool leg are computed. See [ADR 0001](./adr/0001-hybrid-quoting-and-routing.md) for the high-level rule.

**Without `belief_price`:** The check compares `max_spread` to a ratio whose numerator is the pool leg’s constant-product spread metric (capped by pool gross output) and whose denominator is **pool gross output plus book net output to the taker** (`pool_net + pool_commission + book_return_net` in ask units). So the book leg scales the denominator even though the spread numerator comes from the pool leg.

**With `belief_price`:** Expected output is `offer_amount / belief_price` (in ask units). Actual output used in the inequality is `book_return_net + pool_net_to_receiver + pool_commission` (pool commission to treasury counts on the “actual” side).

These are **execution** semantics; all quoting uses `HybridSimulation` / `HybridReverseSimulation` (invariant L8; legacy `Simulation` removed in [#190](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/190)).

## Route discovery and quotes (L8)

The indexer exposes multi-hop routing under `/api/v1/route/solve` (see [indexer-invariants.md](./indexer-invariants.md) for full HTTP semantics).

| Method | Role |
|--------|------|
| **`GET`** | BFS path discovery (**default max 3 hops**). When `amount_in` is set, hybrid optimization runs **by default**: per-hop splits via pair **`HybridSimulation`**, merged `hybrid` in `router_operations`, optional `estimated_amount_out` from LCD `simulate_swap_operations` when `ROUTER_ADDRESS` is configured. Returns `intermediate_tokens`, `quote_kind`, and `hybrid_notes`. Legacy integrators may pass **`pool_only=true`** for pool-only ops (**max 4 hops**, `hybrid: null` on every hop). Without `amount_in`, GET returns route discovery only. See GitLab [**#191**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/191). |
| **`GET /best`** | **Retail / Vyntrex best-execution alias** ([GitLab **#189**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/189)): same hybrid engine as default GET with `amount_in`; **`amount_in` is required**. |
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
| *(none)* | `book_commission_amount` | Book taker fees (ask asset); present when book leg > 0 ([#196](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/196)). |
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
