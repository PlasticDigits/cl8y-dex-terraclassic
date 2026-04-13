# Indexer — Invariants, Business Logic, and Security

This document describes **on-chain indexing** and **read-only HTTP API** behavior for [`indexer/`](../indexer/). It pairs with [contracts-security-audit.md](./contracts-security-audit.md) (on-chain) and [security-model.md](./security-model.md) (protocol).

## Architecture

- **Indexer task:** LCD → parse txs → Postgres (swaps, candles, traders, positions, limit fills and lifecycle rows, liquidity events, hooks, oracle rows).
- **API task:** Axum handlers (primarily `GET`) → SQLx (parameterized) → JSON; `POST /api/v1/route/solve` for hybrid-aware route merge and optional LCD simulation.
- **Shared state:** `AppState` (pool, LCD client, USTC price cache, ticker map cache, orderbook cache, optional `router_address` for route simulation).

## API invariants

| Invariant | Enforcement | Unhappy path | Tests |
|-----------|-------------|--------------|-------|
| Read-only surface | `GET` on all routes except **`POST /api/v1/route/solve`** (JSON body, no writes to indexer DB); CORS allows `GET` and `POST` | Other `POST` → **405** | [`api_route_solve.rs`](../indexer/tests/api_route_solve.rs) |
| No SQL injection on dynamic ordering | Leaderboard `sort` matched to fixed columns in Rust + API allowlist | Unknown `sort` → **400** | [`security.rs`](../indexer/tests/security.rs), [`api_traders.rs`](../indexer/tests/api_traders.rs) |
| Candle `interval` allowlist | `VALID_INTERVALS` | Bad / injection-like string → **400** | `security.rs` |
| Numeric query caps | `.min(200)`, `.min(500)`, `.min(1000)` on limits/depth | Oversized `limit`/`depth` clamped | `security.rs` (pairs candles/trades, oracle history, trader trades), [`api_cg.rs`](../indexer/tests/api_cg.rs), [`api_cmc.rs`](../indexer/tests/api_cmc.rs) |
| CG `type` filter | Only `buy`, `sell`, or omit | Invalid → **400** | `api_cg.rs` |
| Ticker / market pair shape | Exactly one `_`, non-empty `BASE` and `TARGET` | `A_B_C`, `_`, `A_`, malformed → **400** | `security.rs` (`cg_ticker_id_attack_matrix`), `api::cg_ticker_tests`, **proptest** on `cg_ticker_segments` |
| Unknown pair / token | DB lookup miss | **404** with short message | Various `api_*.rs` |
| Internal errors sanitized | `internal_err()` → `"Internal server error"` | DB/LCD errors never echo sqlx/SQL | `security.rs` |
| Hooks errors | Same `internal_err()` as rest of API | No raw DB text | Code path in [`hooks.rs`](../indexer/src/api/hooks.rs) |
| CORS | Allowlist from `CORS_ORIGINS` | Disallowed `Origin` → no `ACA-O` | `security.rs` |
| Abuse: rate limit | `tower_governor` when `RATE_LIMIT_RPS > 0` | Sustained burst → **429** | `security.rs` |
| Abuse: slow handlers | `TimeoutLayer` 30s | **408** Request Timeout | Documented; no slow-query test |
| Abuse: response size | `CompressionLayer` | Reduces bandwidth cost | Operational |
| LCD amplification | Orderbook responses cached 30s per `(pair, depth)` | Repeated CG/CMC orderbook hits reuse cache | [`orderbook_sim.rs`](../indexer/src/api/orderbook_sim.rs), [`api_orderbook_lcd_mock.rs`](../indexer/tests/api_orderbook_lcd_mock.rs) (wiremock LCD) |
| Ticker resolution load | Full ticker→pair map cached 30s | Reduces DB scans on CG/CMC | [`api/mod.rs`](../indexer/src/api/mod.rs) |
| Route discovery | `GET /api/v1/route/solve` — BFS over indexed pairs (max 4 hops); `token_in` / `token_out` must match `assets.contract_address` (native-only assets without a contract address are not routable) | Unknown token → **400**; no path → **404** | [`route_solver.rs`](../indexer/src/api/route_solver.rs), [`api_route_solve.rs`](../indexer/tests/api_route_solve.rs) |
| Route hybrid merge + simulation | `POST /api/v1/route/solve` with JSON `{ token_in, token_out, amount_in?, hybrid_by_hop? }`. When `hybrid_by_hop` is set, its length **must equal** the number of hops; each entry is `null` (pool-only hop) or a `HybridSwapParams`-shaped object (`pool_input`, `book_input`, `max_maker_fills`, `book_start_hint`). Response `router_operations` reflects merged `hybrid` fields. Optional `estimated_amount_out` uses LCD `simulate_swap_operations` on the router when `amount_in` and `ROUTER_ADDRESS` are set (router validates leg sums). | Bad hybrid length → **400**; router/LCD query error → **400** with generic message (no raw LCD stack) | `api_route_solve.rs` |
| Route simulation (GET) | Optional `estimated_amount_out` when `amount_in` is set **and** `ROUTER_ADDRESS` env is configured | LCD `simulate_swap_operations` with pool-only ops | Same; requires live LCD in production |
| Pair liquidity history | `GET /api/v1/pairs/{addr}/liquidity-events` — `limit`/`before` capped like trades | Unknown pair → **404** | [`api_pairs.rs`](../indexer/tests/api_pairs.rs) |
| Limit placements / cancellations | `GET /api/v1/pairs/{addr}/limit-placements`, `.../limit-cancellations` | Unknown pair → **404** | Same |
| On-chain book (LCD proxy) | `GET /api/v1/pairs/{addr}/order-book-head`, `.../limit-book-shallow` | Unknown pair → **404**; LCD failure → **502**; `depth` clamped (max 20) | [`api_limit_book_lcd_mock.rs`](../indexer/tests/api_limit_book_lcd_mock.rs), [`limit-orders.md`](./limit-orders.md) |
| Hooks OpenAPI | `GET /api/v1/hooks` documented under Swagger **Hooks** tag | Same error handling as other read routes | [`api_hooks.rs`](../indexer/tests/api_hooks.rs) |

## Indexing invariants

| Invariant | Enforcement | Unhappy path | Tests |
|-----------|-------------|--------------|-------|
| Block time usable | RFC3339 parse; else `tracing::warn` + `Utc::now()` | Misaligned candles (documented risk) | Manual / logs; see [Block time fallback and candle skew](#block-time-fallback-and-candle-skew) |
| Swap dedup | Unique index on `(tx_hash, pair_id)`; `INSERT ... ON CONFLICT DO NOTHING` in [`insert_swap`](../indexer/src/db/queries/swap_events.rs) | Replay skipped; optional `trade_exists` fast path in parser | Migration `20260326120000_*`; [`parser.rs`](../indexer/src/indexer/parser.rs) |
| Wasm attributes | `wasm_attr_last`: duplicate keys → last wins | Matches CosmWasm multi-attribute events | [`parser.rs`](../indexer/src/indexer/parser.rs) unit tests + fuzz |
| Candle price positive | `price <= 0` → skip update | No zero/negative OHLC from bad ratio | [`candle_skip_zero_price.rs`](../indexer/tests/candle_skip_zero_price.rs), `merge_candle_ohlc` unit tests |
| Candle OHLC consistency | `high ≥ low`, `high ≥ close`, `low ≤ close`, `open` unchanged on update | Enforced by merge logic | Unit tests + **proptest** on `merge_candle_ohlc` ([`candle_builder.rs`](../indexer/src/indexer/candle_builder.rs)) |
| Position net quote | After sell leg, `net_position_quote` clamped to ≥ 0 | Oversell does not leave negative inventory | [`position_tracker_clamp.rs`](../indexer/tests/position_tracker_clamp.rs), `net_quote_after_sell` unit tests |
| Oracle storage | Non-finite `f64` → safe `BigDecimal` default | No NaN/Inf in DB from conversion | [`oracle.rs`](../indexer/src/indexer/oracle.rs) unit tests |

## Attack paths (off-chain API)

1. **Query injection via `sort` / `interval`** — Blocked by allowlists (no string pasting into SQL for those).
2. **Ticker confusion / extra underscores** — Blocked by strict `BASE_TARGET` shape (exactly two non-empty segments).
3. **DoS: huge limits** — Blocked by caps on trades, candles, hooks, oracle history, CG trades, orderbook depth.
4. **DoS: expensive DB per request** — Ticker map cache reduces repeated full scans for CG/CMC.
5. **DoS: LCD hammering via orderbook** — Short-TTL cache on simulated orderbook.
6. **DoS: connection exhaustion** — Rate limit + timeout (defense in depth; edge still needs WAF/reverse proxy).
7. **Information leak via errors** — `internal_err` for hooks and other handlers.
8. **CORS token theft** — Browser enforces origin; server only echoes allowlisted origins.

## Running tests

- **Library tests (no Postgres):** `cd indexer && cargo test --lib` — includes parser/candle/oracle fuzz-style tests, **proptest** on `merge_candle_ohlc` and `cg_ticker_segments`, and invariant unit tests.
- **Integration tests:** Require PostgreSQL and migrations (e.g. CI service or `TEST_DATABASE_URL`). `cd indexer && cargo test --tests`. Orderbook routes are also covered with a **wiremock** stub of the LCD `pool` smart query ([`tests/common/lcd_mock.rs`](../indexer/tests/common/lcd_mock.rs)).

Integration tests **fail fast** if the database is unreachable (see [`tests/common/mod.rs`](../indexer/tests/common/mod.rs)).

**Shared DB / flaky tests:** If you see sporadic duplicate-key or FK errors against one database, run tests with serialized parallelism as documented in [Testing — Shared Postgres and test parallelism](./testing.md#shared-postgres-and-test-parallelism) (`cargo test --tests -j 1 -- --test-threads=1`).

## Block time fallback and candle skew

Block timestamps come from the LCD transaction response (`tx_responses[0].timestamp`). In [`indexer/src/indexer/poller.rs`](../indexer/src/indexer/poller.rs), `parse_block_time`:

- Parses RFC3339 into UTC when valid.
- If the timestamp is **missing** or **invalid**, logs a **warning** and uses **`Utc::now()`** (wall-clock) for that block’s events.

**Risk:** Event times and candle bucket boundaries can **diverge** from true chain time—**OHLC intervals may skew** relative to block time. Mitigations:

- Run a **reliable LCD** close to your chain; monitor logs for the warning strings and the Prometheus counter `indexer_block_time_fallbacks_total` when metrics are enabled (`METRICS_BIND` non-empty; see [`docs/operator-secrets.md`](./operator-secrets.md)).
- After prolonged LCD issues, consider **re-indexing** from a known height (see [runbook: reorg / replay / dedup](./runbooks/indexer-reorg-replay-dedup.md)).

## Maintenance

When adding a new query parameter that influences SQL or ordering:

1. Use an allowlist or parameterized query only.
2. Cap numeric inputs.
3. Add a **happy** and **unhappy** test (400/404/429 as appropriate).
4. Update this matrix if the invariant is user-visible.
