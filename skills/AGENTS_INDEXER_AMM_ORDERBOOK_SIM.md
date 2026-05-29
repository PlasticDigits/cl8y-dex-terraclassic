# Indexer: hybrid-simulated orderbook (CG/CMC)

Use when changing **`indexer/src/api/orderbook_sim.rs`**, **`indexer/src/api/hybrid_orderbook_sim.rs`**, **`/cg/orderbook`**, **`/cmc/orderbook/*`**, or CG/CMC compliance docs for synthetic depth.

GitLab **#220** (hybrid merge), **#210** (AMM pool leg), **#222** / **#224** (listing compliance: timestamps + CMC array wrapper).

## Do not confuse

| Surface | What it is |
|---------|------------|
| `orderbook_sim.rs` | Pool **curve walk** + cache + LCD pool/fee; calls hybrid merge when enabled |
| `hybrid_orderbook_sim.rs` | **Merge** pool levels + limit LCD levels for listing APIs |
| Pair `limit-book` / `limit_book_lcd.rs` | On-chain **FIFO** resting orders (integrator + dApp) |
| `tests/common/lcd_mock.rs` | Wiremock **LCD HTTP** — use `start_pool_query_mock` (empty book) or `start_hybrid_orderbook_mock` |

## Normative spec

- [`docs/CG_CMC_COMPLIANCE.md`](../docs/CG_CMC_COMPLIANCE.md) § Hybrid Orderbook Simulation + § AMM Orderbook Simulation (pool leg)
- [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) — query `depth` ≤ 100 **total** (Openware), cache 30s per `(pair, depth, fee_bps, bid_head, ask_head)`
- [`docs/limit-orders.md`](../docs/limit-orders.md) — limit `price` / `remaining` units
- GitLab **#220** (hybrid merge), **#210** (pool leg), **#221** (Openware depth split), **#222** (timestamps), **#105** (stub catalog)

## Orderbook `timestamp` (#222)

| Endpoint | Field | Type | Unit |
|----------|-------|------|------|
| `GET /cg/orderbook` | `timestamp` | JSON number (`i64`) | Unix **milliseconds** |
| `GET /cmc/orderbook/*` | `timestamp` | JSON number (`i64`) | Unix **seconds** (same as `/cmc/trades`) |
| `GET /cmc/orderbook/*` | (root) | JSON **array** | One book object per Openware Peatio (**#224**) |

Implementation: [`listing_timestamps.rs`](../indexer/src/api/listing_timestamps.rs). **Breaking:** pre-#222 responses used RFC3339 strings; pre-#224 CMC orderbook was a bare object.

## Depth query (Openware / CMC — #221)

- `cap_orderbook_depth` / `levels_per_side` in `orderbook_sim.rs`
- `depth=100` → **50** bids + **50** asks; default **20** → **10+10**; `depth=1` → **1+1**
- Cache keys use **requested** total `depth`, not per-side count
- Do **not** use Kujira FIN per-side semantics on CG/CMC listing endpoints

## Merge rules (#220)

1. Fetch pool book via `walk_amm_book` (reserves + `fee_bps`).
2. Fetch up to `levels_per_side(requested_depth)` resting orders per side via `fetch_limit_book_page` (bounded by `LIMIT_BOOK_PAGE_MAX`).
3. Limit → CG level: `[price, base_qty]` — asks use `remaining` (token0); bids floor(`remaining` quote / price).
4. Merge: sort (bids desc, asks asc), sum qty at same price string, truncate to **per-side** cap.
5. **Not** a live L2 feed — disclosure in compliance doc.

## Env

- `ORDERBOOK_HYBRID` — default **on**; set `0` / `false` for pool-only (pre-#220).

## Math (pool leg — must stay aligned with pair pool swap)

- Integer `u128` reserves; **`ceil_div`** for new opposite reserve
- Step size: `R0 * (i / levels_per_side) * 0.10` for `i` in `1..=levels_per_side` (from query `depth`)
- Fee: `gross * fee_bps / 10000` on swap output (LCD `get_fee_config`, DB `pairs.fee_bps` fallback)

## Tests

```bash
cd indexer && cargo test --lib orderbook -- --test-threads=1
cd indexer && cargo test --test api_orderbook_lcd_mock -- --test-threads=1
```

Unit tests: `hybrid_orderbook_sim.rs`, `orderbook_sim.rs` `#[cfg(test)]`, `listing_timestamps.rs` (ms/s skew). Integration: `tests/api_orderbook_lcd_mock.rs` (wiremock LCD + Postgres; asserts numeric timestamps per **#222**).
