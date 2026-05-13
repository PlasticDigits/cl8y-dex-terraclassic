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
| Candle default time window | `GET .../candles` without `from`/`to`: **`from` = `to` − 90 days**, **`to` = now** (RFC 3339 bounds on `open_time`) | Short windows (historically 7 days) made charts **empty** when indexed swaps were older than the window (common on LocalTerra / QA) while `trades` still returned rows — see [`pairs.rs`](../indexer/src/api/pairs.rs) `DEFAULT_CANDLE_LOOKBACK_DAYS` | `api_pairs.rs` |
| Numeric query caps | `.min(200)`, `.min(500)`, `.min(1000)` on limits/depth | Oversized `limit`/`depth` clamped | `security.rs` (pairs candles/trades, oracle history, trader trades), [`api_cg.rs`](../indexer/tests/api_cg.rs), [`api_cmc.rs`](../indexer/tests/api_cmc.rs) |
| CG `type` filter | Only `buy`, `sell`, or omit | Invalid → **400** | `api_cg.rs` |
| Ticker / market pair shape | Exactly one `_`, non-empty `BASE` and `TARGET` | `A_B_C`, `_`, `A_`, malformed → **400** | `security.rs` (`cg_ticker_id_attack_matrix`), `api::cg_ticker_tests`, **proptest** on `cg_ticker_segments` |
| Unknown pair / token | DB lookup miss | **404** with short message | Various `api_*.rs` |
| Internal errors sanitized | `internal_err()` → `"Internal server error"` | DB/LCD errors never echo sqlx/SQL | `security.rs` |
| Hooks errors | Same `internal_err()` as rest of API | No raw DB text | Code path in [`hooks.rs`](../indexer/src/api/hooks.rs) |
| CORS | Allowlist from `CORS_ORIGINS` | Disallowed `Origin` → no `ACA-O` | `security.rs` |
| Abuse: rate limit | `tower_governor` when `RATE_LIMIT_RPS > 0` | Sustained burst → **429** | `security.rs` |
| Prometheus `/metrics` | Served only on **`METRICS_BIND`** (dedicated TCP listener), never on the public API listener; **`0.0.0.0` / `::` forbidden** when `DEPLOY_ENV` is production (default under `RUN_MODE=prod`) | Misconfiguration → indexer refuses to start | `config::tests` in [`config.rs`](../indexer/src/config.rs); GitLab [**#125**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/125); [`operator-secrets.md`](./operator-secrets.md) |
| Abuse: slow handlers | `TimeoutLayer` 30s | **408** Request Timeout | Documented; no slow-query test |
| Abuse: response size | `CompressionLayer` | Reduces bandwidth cost | Operational |
| LCD amplification | Orderbook responses cached 30s per `(pair, depth)` | Repeated CG/CMC orderbook hits reuse cache | [`orderbook_sim.rs`](../indexer/src/api/orderbook_sim.rs), [`api_orderbook_lcd_mock.rs`](../indexer/tests/api_orderbook_lcd_mock.rs) (wiremock LCD) |
| Ticker resolution load | Full ticker→pair map cached 30s | Reduces DB scans on CG/CMC | [`api/mod.rs`](../indexer/src/api/mod.rs) |
| Route discovery | `GET /api/v1/route/solve` — BFS over indexed pairs (**max 4 hops** unless `hybrid_optimize=true`, then **max 3 hops**); `token_in` / `token_out` must match `assets.contract_address` (native-only assets without a contract address are not routable) | Unknown token → **400**; no path → **404** | [`route_solver.rs`](../indexer/src/api/route_solver.rs), [`api_route_solve.rs`](../indexer/tests/api_route_solve.rs) |
| Route GET hybrid optimize | `GET` query: `hybrid_optimize=true` requires `amount_in`. Indexer runs a **sequential per-hop** split search using pair `HybridSimulation` (grid on `book_input`), merges best `hybrid` into `router_operations`, then optional router `simulate_swap_operations`. Response adds `intermediate_tokens`, `quote_kind`, `hybrid_notes`. Short-TTL in-memory cache keys `(token_in, token_out, amount_bucket, max_maker_fills)`. | `hybrid_optimize` without `amount_in` → **400**; LCD failure during optimization → **502**; router sim error → **400** (same as POST) | `api_route_solve.rs`, [`hybrid_route_opt.rs`](../indexer/src/api/hybrid_route_opt.rs) |
| Route hybrid merge + simulation | `POST /api/v1/route/solve` with JSON `{ token_in, token_out, amount_in?, hybrid_by_hop? }`. When `hybrid_by_hop` is set, its length **must equal** the number of hops; each entry is `null` (pool-only hop) or a `HybridSwapParams`-shaped object (`pool_input`, `book_input`, `max_maker_fills`, `book_start_hint`). Response `router_operations` reflects merged `hybrid` fields. Optional `estimated_amount_out` uses LCD `simulate_swap_operations` on the router when `amount_in` and `ROUTER_ADDRESS` are set (router validates leg sums). | Bad hybrid length → **400**; router/LCD query error → **400** with generic message (no raw LCD stack) | `api_route_solve.rs` |
| Route simulation (GET) | Optional `estimated_amount_out` when `amount_in` is set **and** `ROUTER_ADDRESS` env is configured | LCD `simulate_swap_operations` (pool-only or hybrid-merged ops) | Same; requires live LCD in production |
| Pair liquidity history | `GET /api/v1/pairs/{addr}/liquidity-events` — `limit`/`before` capped like trades | Unknown pair → **404** | [`api_pairs.rs`](../indexer/tests/api_pairs.rs) |
| Limit placements / cancellations | `GET .../limit-placements` omits `(pair_id, order_id)` present in `limit_order_cancellations`; each row includes **`lifecycle_status`** (`active`, `parked_expired`, `refunded`) with **`remaining_escrow`** when parked ([GitLab **#142**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/142)); default listing is **`active` + `parked_expired`** (excludes **`refunded`**); **`?status=`** filters `active` \| `parked_expired` \| `refunded` \| `all`; `.../limit-cancellations` lists indexed cancels | Unknown pair → **404**; bad **`status`** → **400** | [`api_pairs.rs`](../indexer/tests/api_pairs.rs), [`limit_order_parked_lifecycle.rs`](../indexer/tests/limit_order_parked_lifecycle.rs); [GitLab **#135**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/135), [**#142**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/142) |
| **Trader history (swaps, limit fills, cancels)** | `GET /api/v1/traders/{addr}/trades` — optional **`pair=`** pair contract filter (**404** unknown pair); optional **`format=csv`** → **`text/csv`** attachment (UTF-8) with stable header row; default **`format=json`**. **`GET .../limit-fills`** — rows where indexed **`maker`** equals `{addr}`; optional **`pair=`** filters to that pair. **`GET .../limit-cancellations`** — rows where indexed **`owner`** equals `{addr}`; optional **`pair=`** filter. Same **`format`**, **`limit`**, **`before`** caps (**`limit` ≤ 200**). Swap JSON adds optional **`commission_amount`** / **`spread_amount`** when indexed. | Bad **`format`** (not `json` or `csv`) → **400** | [`api_traders.rs`](../indexer/tests/api_traders.rs), [`security.rs`](../indexer/tests/security.rs); [GitLab **#163**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/163); [`docs/frontend.md` § Wallet swap and limit history](./frontend.md#wallet-swap-limit-history), [`skills/AGENTS_FRONTEND_ORDER_HISTORY.md`](../skills/AGENTS_FRONTEND_ORDER_HISTORY.md) |
| On-chain book (LCD proxy) | `GET /api/v1/pairs/{addr}/order-book-head`, `.../limit-book` (paginated), `.../limit-book-shallow` (legacy preview) | Unknown pair → **404**; LCD failure → **502**; `limit-book` `limit` clamped (max 100); `limit-book-shallow` `depth` clamped (max 20); bad book cursor / side → **400** | [`api_limit_book_lcd_mock.rs`](../indexer/tests/api_limit_book_lcd_mock.rs), [`api_limit_book_deep.rs`](../indexer/tests/api_limit_book_deep.rs), [`limit-orders.md`](./limit-orders.md) |
| Hooks OpenAPI | `GET /api/v1/hooks` documented under Swagger **Hooks** tag | Same error handling as other read routes | [`api_hooks.rs`](../indexer/tests/api_hooks.rs) |

### Local dev CORS: `localhost` vs `127.0.0.1` {#local-dev-cors-localhost-vs-127001}

The browser sends `Origin` using the hostname from the address bar. `http://localhost:5173` and `http://127.0.0.1:5173` are different origins; if `CORS_ORIGINS` lists only one of them, fetches from the other fail CORS even when the indexer returns **200**. Include both spellings (and Vite preview ports if used) in dev — see [`indexer/.env.example`](../indexer/.env.example), [`scripts/deploy-dex-local.sh`](../scripts/deploy-dex-local.sh), and [GitLab **#131**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/131). After placing a limit, the dApp polls `limit-placements`; failures are logged as **`[limit-place] indexer poll failed:`** via [`warnIndexerPlacementPollFailed`](../frontend-dapp/src/utils/warnIndexerPlacementPollFailed.ts).

## Frontend expectations (read path)

The indexer may legitimately return **empty candle arrays** for a pair/interval with no buckets yet. The dApp must treat that as **success**, not a silent chart failure. UI invariants and lightweight-charts vs TradingView widget naming are documented under [Trade page — price chart invariants](./frontend.md#trade-page--price-chart-invariants) (GitLab [**#113**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/113), [**#150**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/150)).

When validating LocalTerra flows with **browser wallets**, fee broadcast quirks are **frontend/wallet** concerns — test **both Station and Terra Classic Keplr** when checking gas/fee fixes (**`experimentalSuggestChain`**, **per-sign `preferNoSetFee` in patched cosmes `KeplrExtension`**, **Station LocalTerra (`chainId` case-insensitive) or Ledger → amino signing** in patched **`StationController`**, **min gas price** — [`docs/frontend.md` § Terra Classic gas limits](./frontend.md#terra-classic-gas-limits), GitLab [**#127**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127)); the indexer only observes txs once included on-chain.

**Pair pause vs limit claim:** On-chain **`ClaimExpiredLimitOrder`** is rejected while the pair is paused (same **`assert_not_paused`** gate as cancel — [contracts-security-audit.md](./contracts-security-audit.md) **L6**, GitLab [**#120**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120)). Bots and UIs must not broadcast claim txs during a pause; the dApp mirrors this on **Claim refund** ([`skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](../skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md)).

**Router `simulate_swap_operations` vs spread:** Route solve responses (and the dApp’s matching wallet LCD `simulate_swap_operations` query) carry **router_operations** plus an optional **`estimated_amount_out`**, but the router simulation payload does **not** include per-hop **spread** — only the final amount. The retail Swap page therefore runs a **sequential pair-level preflight** (factory + `simulation` / `hybrid_simulation`) so **price impact** and submit gating align with on-chain **`max_spread`** per hop. Invariants and agent notes: [swap max spread / price impact UX](./swap-max-spread-ux.md) (GitLab [**#134**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134)), [`docs/frontend.md` § Swap Page Integration](./frontend.md#swap-page-integration), [`skills/AGENTS_LOCALNET_TRADING_SWARM.md`](../skills/AGENTS_LOCALNET_TRADING_SWARM.md).

## Indexing invariants

| Invariant | Enforcement | Unhappy path | Tests |
|-----------|-------------|--------------|-------|
| Block time usable | RFC3339 parse; else `tracing::warn` + `Utc::now()` | Misaligned candles (documented risk) | Manual / logs; see [Block time fallback and candle skew](#block-time-fallback-and-candle-skew) |
| Swap dedup | Unique index on `(tx_hash, pair_id)`; `INSERT ... ON CONFLICT DO NOTHING` in [`insert_swap`](../indexer/src/db/queries/swap_events.rs) | Replay skipped; optional `trade_exists` fast path in parser | Migration `20260326120000_*`; [`parser.rs`](../indexer/src/indexer/parser.rs) |
| Limit placement lifecycle | Parser applies **`limit_order_expired_parked`** (`remaining`) then **`claim_expired_limit_order`** on `limit_order_placements`; transitions **`active` → `parked_expired` → `refunded`** | Replay idempotent (updates match at-most-one row per event) | [`parser.rs`](../indexer/src/indexer/parser.rs); [`limit_order_lifecycle.rs`](../indexer/src/db/queries/limit_order_lifecycle.rs); [`limit_order_parked_lifecycle.rs`](../indexer/tests/limit_order_parked_lifecycle.rs); GitLab [**#142**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/142) |
| Limit placement HTTP feed | `list_placements_for_pair` SQL excludes rows whose `(pair_id, order_id)` exists in `limit_order_cancellations` | Stale UI listing already-cancelled ids as “active” | [`limit_order_lifecycle.rs`](../indexer/src/db/queries/limit_order_lifecycle.rs); `api_pairs.rs`; GitLab [**#135**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/135) |
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

- Run a **reliable LCD** close to your chain; monitor logs for the warning strings and the Prometheus counter `indexer_block_time_fallbacks_total` when metrics are enabled (`METRICS_BIND` non-empty; scrape the **metrics** listener, not the API port — see [`docs/operator-secrets.md`](./operator-secrets.md) and GitLab [**#125**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/125)).
- After prolonged LCD issues, consider **re-indexing** from a known height (see [runbook: reorg / replay / dedup](./runbooks/indexer-reorg-replay-dedup.md)).

## Dapp read model (pools list)

The Liquidity pool browser (`/pool`) **lists and sorts** pairs from the indexer API; it **tags** each row against the on-chain factory pair set used for the swap router graph. Counts and badges are documented in [Frontend guide — Liquidity pools list (indexer vs factory)](./frontend.md#liquidity-pools-list-indexer-vs-factory) (see [glab#112](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/112)).

### Factory LCD: pair enumeration vs governance gas (agents)

When the indexer (or any tool) syncs pairs from the factory via LCD, use **paginated** `Pairs` / `GetPairCount` and **avoid** assuming unbounded factory queries are cheap on-chain—the factory keeps a sequential `pair_index` specifically for discovery-style iteration, including resolving `start_after` in pagination (see [Smart contract reference — Factory storage & upgrades](./contracts-terraclassic.md#factory-storage--upgrades)). That design is separate from **per-pair governance** messages (`SetPairFee`, `SweepPair`, etc.), which must use an **O(1)** reverse map (`pair_addr_reg`) so gas stays bounded at high pair counts ([GitLab **#122**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/122)). Third-party agents should follow the same split: iterate for **listing** only; never reintroduce linear “is this pair registered?” scans in contract code paths that validate a single address—see [`skills/AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md).

## Maintenance

When adding a new query parameter that influences SQL or ordering:

1. Use an allowlist or parameterized query only.
2. Cap numeric inputs.
3. Add a **happy** and **unhappy** test (400/404/429 as appropriate).
4. Update this matrix if the invariant is user-visible.
