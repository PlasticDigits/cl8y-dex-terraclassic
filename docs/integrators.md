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
| **`GET /api/v1/pairs/{addr}/limit-book?side=bid\|ask&limit=L&after_order_id=OPTIONAL`** | **Paginated** walk along on-chain `next` pointers. Default **`limit=50`**, max **100** per response. Response: `{ side, orders[], has_more, next_after_order_id }`. Pass **`next_after_order_id`** as **`after_order_id`** for the next page. Optional **`price_from`** + **`price_to`** return only the contiguous in-band slice (see [§ Insert hints & price window](#insert-hints-price-window-gitlab-267)). |
| **`GET /api/v1/pairs/{addr}/limit-book/insert-hints?side=bid\|ask&prices=p1,p2,...`** | Batch **insert-hint** resolution in one LCD walk (max **100** prices). Response: `{ side, hints[], budget_exhausted }` per [§ Insert hints & price window](#insert-hints-price-window-gitlab-267). |
| **`GET /api/v1/pairs/{addr}/limit-book-shallow?side=bid\|ask&depth=N`** | Legacy preview (default **10**, max **20**). Prefer **`limit-book`** for pro depth. |

**Errors:** unknown pair → **404**; LCD failure → **502**; invalid cursor / side mismatch → **400**. When **`RATE_LIMIT_RPS > 0`**, sustained abuse → **429** ([indexer-invariants.md](./indexer-invariants.md)).

**LCD cost:** each **`limit-book`** page costs up to **1 + limit** smart queries (head or cursor lookup + one `limit_order` per returned row). No server-side caching of arbitrary deep walks — **clients paginate**.

**Not the on-chain FIFO book:** CoinGecko/CoinMarketCap **`/cg/orderbook`** and **`/cmc/orderbook`** return **hybrid-simulated** depth (AMM curve walk + merged resting limits; GitLab **#220**) — not a live CEX L2 feed. On-chain FIFO book: **`limit-book`** above; listing spec: [CG_CMC_COMPLIANCE.md](./CG_CMC_COMPLIANCE.md).

OpenAPI: served from the indexer Swagger UI (`/swagger-ui/`). Regression tests: [`api_limit_book_lcd_mock.rs`](../indexer/tests/api_limit_book_lcd_mock.rs), [`api_limit_book_deep.rs`](../indexer/tests/api_limit_book_deep.rs), [`api_limit_book_insert_hints.rs`](../indexer/tests/api_limit_book_insert_hints.rs).

### Insert hints & price window (GitLab **#267**) {#insert-hints-price-window-gitlab-267}

Indexer primitives for deep-book ladders ([#268](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/268)) and contract anchor hints ([#266](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/266)). **Frontends must use these HTTP routes** — no direct LCD/RPC book walks in production UI.

**`insert-hints`** — comma-separated **`prices`** (≤ **100**). Each hint:

| Field | Meaning |
|-------|---------|
| `predecessor_order_id` | On-chain `hint_after_order_id` when `resolved` is true |
| `resolved` | `false` when the walk cannot reach the slot (**never guess**) |
| `reason` | `"head"` (better than head / empty book), or `"pagination_gap"` (budget or tail not reached) |
| `budget_exhausted` | Top-level flag when the **101** LCD query cap stopped the walk early |

Semantics match [limit-orders.md § Ordering](./limit-orders.md#ordering-composite-key-fifo) and [`limitBookInsertHint.ts`](../frontend-dapp/src/utils/limitBookInsertHint.ts) (parity-tested in Rust).

**Price window** — on **`limit-book`**, set both **`price_from`** and **`price_to`** (positive decimals). Band is inclusive:

- **Bids** (head = highest price): `price_from` ≥ `price_to`; returns orders with `price_to ≤ price ≤ price_from`.
- **Asks** (head = lowest price): `price_from` ≤ `price_to`; returns orders with `price_from ≤ price ≤ price_to`.

Same **`limit`**, **`after_order_id`**, **`has_more`**, and **101** LCD budget as paginated `limit-book`. Routes sit on the **LCD-heavy** governor ([`skills/AGENTS_INDEXER_API_LCD_SECURITY.md`](../skills/AGENTS_INDEXER_API_LCD_SECURITY.md)).

### Batch placement insert hints (GitLab **#261**)

When placing via **`PlaceLimitOrderBatch`**, each `orders[]` entry may include optional **`hint_after_order_id`** (`null`/omit = head walk only). **`PlaceLimitOrderLadder`** accepts optional **`ladder.hint_after_order_id`** on the **head-most rung in book order** only (GitLab **#266**; resolved via indexer **#267**). On-chain, batch inserts traverse rungs in **book-sort order** (composite `(price, order_id)`) while preserving input **id assignment** and **wasm attr order**; interior rungs reuse an **`InsertThreadCursor`** from the prior successful insert when bracketed (O(0)-load verify). On-chain verify + fallback: invariant **L14** in [contracts-security-audit.md](./contracts-security-audit.md) ([#256](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/256), directional near-miss fallback [#265](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/265), book-order traversal [#266](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/266)).

**Client resolver (recommended):** prefer **`GET .../limit-book/insert-hints`** ([#267](#insert-hints-price-window-gitlab-267)) for batch ladder bands; otherwise walk paginated **`limit-book`** pages for the target side (head → tail). For insert price `P`:

- **Bids** (descending price, ascending `order_id`): return the `order_id` of the order immediately **before** the insert slot; `null` for head insert (price better than loaded head) or when **`has_more`** and the slot is past the last loaded row (pagination gap).
- **Asks** (ascending price): same rule with ask sort order ([limit-orders.md § Ordering](./limit-orders.md#ordering-composite-key-fifo)).
- At **equal price**, hint = last loaded order at that price level (new ids are higher → FIFO tail).
- **Never guess** across pagination gaps; stale hints are safe (bounded head walk when anchor unusable; directional walk from valid near-miss hints — [#265](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/265)).

Reference implementation: [`limitBookInsertHint.ts`](../frontend-dapp/src/utils/limitBookInsertHint.ts). dApp wire: [`placeLimitOrderWithAllowance`](../frontend-dapp/src/services/terraclassic/pair.ts). Agent playbooks: [`skills/AGENTS_FRONTEND_DEEP_ORDER_BOOK.md`](../skills/AGENTS_FRONTEND_DEEP_ORDER_BOOK.md), [`skills/AGENTS_FRONTEND_LIMIT_ORDER_PLACEMENT_GAS.md`](../skills/AGENTS_FRONTEND_LIMIT_ORDER_PLACEMENT_GAS.md), [`skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md`](../skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md).

**dApp reference:** [`skills/AGENTS_FRONTEND_DEEP_ORDER_BOOK.md`](../skills/AGENTS_FRONTEND_DEEP_ORDER_BOOK.md).

## Limit book clean (GitLab #263) {#limit-book-clean-gitlab-263}

Permissionless pair execute **`clean_limit_book`** parks time-expired and/or governance **dust** orders into **`EXPIRED_LIMIT_CLAIMS`** (no CW20 in that tx — makers use **`claim_expired_limit_order`** later). **Not** factory **`sweep`** (excess CW20 recovery).

| Field | Meaning |
|-------|---------|
| `side` | `bid` or `ask` |
| `max_orders` | 1…**100** (`MAX_LIMIT_CLEAN_ORDERS_HARD_CAP`) |
| `start_hint` | Optional order id to start the DLL walk; invalid/absent → **head** |

Query **`limit_clean_config`** for per-side `min_remaining_token0` (asks) / `min_remaining_token1` (bids). **0** disables force-clean on that side. Governance: factory **`set_pair_limit_clean_config`**.

**Pause:** blocked while `is_paused` (invariant **L6**). **Indexer:** `limit_order_expired_parked` still drives **`parked_expired`**; optional wasm attr **`force_expired=true`** on dust parks.

Canonical: [limit-orders.md § Permissionless limit book clean](./limit-orders.md#permissionless-limit-book-clean), invariant **L15** in [contracts-security-audit.md](./contracts-security-audit.md), [`limit_book_clean.rs`](../smartcontracts/contracts/pair/src/limit_book_clean.rs). **No in-repo watcher** — operators may cron `clean_limit_book` from indexer backlog signals ([`skills/AGENTS_LOCALNET_TRADING_SWARM.md`](../skills/AGENTS_LOCALNET_TRADING_SWARM.md)).

## Match-time dust flush (GitLab #264) {#match-time-dust-flush-gitlab-264}

During hybrid **`match_bids` / `match_asks`**, post-fill remainders **`0 < remaining < 10`** (raw escrow units: token1 bids, token0 asks) are **auto-parked** into **`EXPIRED_LIMIT_CLAIMS`** with **`force_expired=true`** — no separate keeper tx, no CW20 in the swap. Constant: **`LIMIT_ORDER_DUST_FLUSH_THRESHOLD`** in [`dex-common::pair`](../smartcontracts/packages/dex-common/src/pair.rs). Makers claim via **`claim_expired_limit_order`** (same as time-expiry / governance dust parks).

**vs #263:** `CleanLimitBook` is permissionless, governance-threshold, and async; match-time flush is proactive with a fixed **10**-unit protocol threshold at fill time.

**Indexer:** existing `limit_order_expired_parked` + `force_expired=true` → **`parked_expired`** (no parser change expected).

Canonical: [limit-orders.md § Match-time dust flush](./limit-orders.md#match-time-dust-flush-gitlab-264), invariant **L16** in [contracts-security-audit.md](./contracts-security-audit.md), [`orderbook.rs`](../smartcontracts/contracts/pair/src/orderbook.rs). Agent playbook: [`skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](../skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md).

## Slippage: `max_spread` and `belief_price` (hybrid)

Slippage checks run in the pair after the book leg and pool leg are computed. See [ADR 0001](./adr/0001-hybrid-quoting-and-routing.md) for the high-level rule. Canonical implementation: [`dex_common::max_spread`](../smartcontracts/packages/dex-common/src/max_spread.rs) (invariant **L9**, [GitLab **#197**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/197)). Pool-only swaps are the special case `book_return_net = 0`.

**Without `belief_price`:** The check compares `max_spread` to a ratio whose numerator is the pool leg’s constant-product spread metric (capped by pool gross output) and whose denominator is **pool gross output plus book net output to the taker** (`pool_net + pool_commission + book_return_net` in ask units). So the book leg scales the denominator even though the spread numerator comes from the pool leg.

**With `belief_price`:** Expected output is `offer_amount / belief_price` (in ask units). Actual output used in the inequality is `book_return_net + pool_net_to_receiver + pool_commission` (pool commission to treasury counts on the “actual” side).

These are **execution** semantics; all quoting uses `HybridSimulation` / `HybridReverseSimulation` (invariant L8; legacy `Simulation` removed in [#190](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/190)). Frontend preflight mirrors the no-belief formula in [`swapMaxSpread.ts`](../frontend-dapp/src/utils/swapMaxSpread.ts) — see [`docs/swap-max-spread-ux.md`](./swap-max-spread-ux.md).

## Route discovery and quotes (L8)

The indexer exposes multi-hop routing under `/api/v1/route/solve` (see [indexer-invariants.md](./indexer-invariants.md) for full HTTP semantics).

| Method | Role |
|--------|------|
| **`GET`** | Without `amount_in`: **first** BFS path discovery (**max 3 hops**). With `amount_in`: **global best execution** ([GitLab **#209**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/209), [ADR 0002](./adr/0002-global-best-execution-route-solver.md)) — up to 5 simple paths, joint per-hop hybrid splits, highest `estimated_amount_out` wins. Response includes `solver_version`, `paths_considered`, `optimality_scope`, `lcd_hybrid_queries`, `intermediate_tokens`, `quote_kind`, `hybrid_notes`. Legacy **`pool_only=true`** → pool-only ops (**max 4 hops**). See [**#191**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/191). |
| **`GET /best`** | **Retail / Vyntrex alias** ([GitLab **#189**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/189)): same `global_v1` engine as default GET; **`amount_in` required**. |
| **`POST`** | BFS discovery (**max 4 hops**), plus optional **`hybrid_by_hop`** for **integrator overrides** (one entry per hop). Merges into `router_operations` and runs LCD `simulate_swap_operations` when configured. |

**Invariant L8:** Pool-only quotes use `hybrid_simulation` with `book_input = 0` (helpers: `pool_only_hybrid_params`, `pool_only_hybrid_template`). Router ops with `hybrid: null` still get pool-only hybrid quotes on-chain. For book-inclusive quotes set non-zero `book_input` on the router op or pair query. **Fee discounts:** pass optional `trader` (and `sender` when the CW20 sender differs, e.g. trusted router) on `HybridSimulation` / router `SimulateSwapOperations` so quotes match execute for registered CL8Y tiers; omit `trader` for undiscounted (full fee) quotes ([#238](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/238)). Indexer **`GET/POST /api/v1/route/solve`** accept the same optional fields; hybrid GET cache keys include normalized `trader` when set ([#245](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/245)). See [limit-orders.md](./limit-orders.md), [ADR 0001](./adr/0001-hybrid-quoting-and-routing.md), [skills/AGENTS_HYBRID_QUOTING.md](../skills/AGENTS_HYBRID_QUOTING.md), [skills/AGENTS_FEE_DISCOUNT_TIERS.md](../skills/AGENTS_FEE_DISCOUNT_TIERS.md).

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

Indexer persistence and `/api/v1/pairs/{addr}/trades` expose the same columns plus CG/CMC aliases `pool_leg_volume` / `book_leg_volume` when hybrid attrs are indexed. CG/CMC listing endpoints use **consolidated** totals in standard volume fields; optional `cl8y_extensions` attribute book vs pool without double-counting — see [CG_CMC_COMPLIANCE.md](./CG_CMC_COMPLIANCE.md#consolidated-hybrid--pool-only-reporting-gitlab-189).

**Volume reconciliation (headline vs legs vs fills):** [integrators-hybrid-volume.md](./integrators-hybrid-volume.md) ([GitLab #216](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/216)). Agent playbook: [skills/AGENTS_INTEGRATOR_HYBRID_VOLUME.md](../skills/AGENTS_INTEGRATOR_HYBRID_VOLUME.md).

Full Terraport reference: [terraport.md](./terraport.md).

## Related docs

- [limit-orders.md](./limit-orders.md) — messages, pause, indexer, events.
- [contracts-security-audit.md](./contracts-security-audit.md) — invariant matrix.
- [ADR 0001](./adr/0001-hybrid-quoting-and-routing.md) — hybrid routing and quoting scope.
- [ADR 0002](./adr/0002-global-best-execution-route-solver.md) — global best execution (#209).
- [integrators-hybrid-volume.md](./integrators-hybrid-volume.md) — consolidated vs leg vs fill volumes (#216).
