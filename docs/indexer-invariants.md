# Indexer — Invariants, Business Logic, and Security

This document describes **on-chain indexing** and **read-only HTTP API** behavior for [`indexer/`](../indexer/). It pairs with [contracts-security-audit.md](./contracts-security-audit.md) (on-chain) and [security-model.md](./security-model.md) (protocol).

## Architecture

- **Indexer task:** LCD → parse txs → Postgres (swaps, candles, traders, positions, hooks, oracle rows).
- **API task:** Axum GET handlers → SQLx (parameterized) → JSON.
- **Shared state:** `AppState` (pool, LCD client, USTC price cache, ticker map cache, orderbook cache, optional `router_address` for route simulation).

## API invariants

| Invariant | Enforcement | Unhappy path | Tests |
|-----------|-------------|--------------|-------|
| Read-only surface | Only `GET` routes; CORS allows `GET` | `POST` etc. rejected by router | Implicit |
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

## Indexing invariants

| Invariant | Enforcement | Unhappy path | Tests |
|-----------|-------------|--------------|-------|
| Block time usable | RFC3339 parse; else `tracing::warn` + `Utc::now()` | Misaligned candles (documented risk) | Manual / logs |
| Swap dedup | `(tx_hash, pair_id)` | Replay not double-counted | Implicit DB unique usage |
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

## Maintenance

When adding a new query parameter that influences SQL or ordering:

1. Use an allowlist or parameterized query only.
2. Cap numeric inputs.
3. Add a **happy** and **unhappy** test (400/404/429 as appropriate).
4. Update this matrix if the invariant is user-visible.
