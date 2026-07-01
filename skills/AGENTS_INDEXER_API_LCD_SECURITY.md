# Agent playbook: indexer API LCD security (GitLab #239)

## When to use

You are changing **LCD-proxied HTTP routes**, **rate limiting**, or **502 error bodies** on the indexer API. Gap findings **H6** (error leakage) and **H7** (LCD amplification) from [`gaps/GAP_1780200149.md`](../gaps/GAP_1780200149.md).

## Invariants

| ID | Invariant | Enforcement |
|----|-----------|-------------|
| **H6** | Client **502** bodies never include `LcdError` text, LCD URLs, or wasm paths | [`lcd_gateway_err`](../indexer/src/api/errors.rs) |
| **H6a** | LCD client **WARN** logs omit upstream URL/host and response bodies; full detail at **DEBUG** ([#379](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/379)) | [`lcd/mod.rs`](../indexer/src/lcd/mod.rs) |
| **H7** | LCD-heavy routes rate-limited separately; prod cannot disable global/heavy limits with `0` | [`api/mod.rs`](../indexer/src/api/mod.rs), [`config.rs`](../indexer/src/config.rs) |
| **H7d** | Startup warns when **both** `RATE_LIMIT_RPS=0` and `RATE_LIMIT_LCD_HEAVY_RPS=0` ([#379](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/379)) | [`config.rs`](../indexer/src/config.rs) |
| **H7e** | Non-prod refuses startup when **both** rate limits are `0` and `API_BIND` is non-loopback unless **`ALLOW_ZERO_RATE_LIMITS=1`** ([#458](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/458)) | [`config.rs`](../indexer/src/config.rs) `bind_is_loopback`, `ConfigError::ZeroRateLimitNonLoopbackBind` |
| **H8** | `GET max_maker_fills` clamped to on-chain **`MAX_MAKER_FILLS_HARD_CAP` (100)** | [`hybrid_limits.rs`](../indexer/src/hybrid_limits.rs), [`route_solver.rs`](../indexer/src/api/route_solver.rs); benchmark [`docs/benchmarks/max-maker-fills-route-solve.md`](../docs/benchmarks/max-maker-fills-route-solve.md) |
| **H9** | `POST /api/v1/route/solve` body ≤ **128 KiB** → else **413** | [`api/mod.rs`](../indexer/src/api/mod.rs) `RequestBodyLimitLayer` |
| **H7b** | Deep `limit-book` / `insert-hints` / price-window ≤ **101** LCD queries per request | [`LIMIT_BOOK_LCD_QUERY_BUDGET`](../indexer/src/api/limit_book_lcd.rs); [integrators.md § #267](../docs/integrators.md#insert-hints-price-window-gitlab-267) |
| **H7c** | Route `global_v1` documents ≤ **`LCD_HYBRID_SIM_BUDGET`** hybrid sims | [`best_execution.rs`](../indexer/src/api/best_execution.rs) |
| **H6b** | `GET /api/v1/health/fee-discount` returns only `configured`, `fee_discount_registry_ok`, `consecutive_lcd_failures` — no LCD URLs, upstream bodies, or per-trader registry state ([#373](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/373)) | [`fee_discount_health.rs`](../indexer/src/api/fee_discount_health.rs), [`api_fee_discount_health.rs`](../indexer/tests/api_fee_discount_health.rs) |
| **F13** | Startup INFO logs never emit `database_url`, webhook URLs, mnemonics, or bearer tokens ([#433](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/433)) | [`startup.rs`](../indexer/src/startup.rs), `make lint-indexer-log-secrets`, [`operator-secrets.md`](../docs/operator-secrets.md) § Logs |

Human matrix: [`docs/indexer-invariants.md`](../docs/indexer-invariants.md). Hybrid route behavior: [`AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md`](./AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md).

## LCD-heavy routes (stricter governor)

These paths use **`RATE_LIMIT_LCD_HEAVY_RPS`** (default **10**) in addition to **`RATE_LIMIT_RPS`** (default **60**):

- `GET /api/v1/pairs/{addr}/order-book-head`
- `GET /api/v1/pairs/{addr}/limit-book-shallow`
- `GET /api/v1/pairs/{addr}/limit-book`
- `GET /api/v1/pairs/{addr}/limit-book/insert-hints` ([#267](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/267))
- `GET|POST /api/v1/route/solve`
- `GET /api/v1/route/solve/best`
- `GET /cg/orderbook` and `GET /cmc/orderbook/{market_pair}` — same `orderbook_sim` LCD fanout as native book routes ([#278](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/278))

Legitimate frontend polling (e.g. deep book, route preview) should stay under **10 RPS per client IP**; use indexer caches on CG/CMC orderbook paths instead of hammering limit-book.

## Configuration

| Env | Default (dev) | Prod note |
|-----|---------------|-----------|
| `RATE_LIMIT_RPS` | 60 | If set to `0`, prod forces **60** |
| `RATE_LIMIT_LCD_HEAVY_RPS` | 10 | If set to `0`, prod forces **10** ([#363](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/363)) |
| `ALLOW_ZERO_RATE_LIMITS` | unset | Set `1` to allow dual-zero on a non-loopback `API_BIND` in non-prod ([#458](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/458)) |
| `API_BIND` | `127.0.0.1` | Loopback binds allow dual-zero without override; `0.0.0.0`/public IPs require limits or opt-out ([#458](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/458)) |

**Dev disable ([#355](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/355)):** `RATE_LIMIT_RPS=0` disables only the **global** layer. LCD-heavy routes stay limited unless **`RATE_LIMIT_LCD_HEAVY_RPS=0`** too. Set **both** to `0` for fully unlimited local QA on **loopback** — startup emits a **DoS-risk warning** ([#379](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/379)). On a **non-loopback** bind, dual-zero refuses startup unless **`ALLOW_ZERO_RATE_LIMITS=1`** ([#458](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/458)). Local deploy keeps `API_BIND=127.0.0.1` and `RATE_LIMIT_LCD_HEAVY_RPS=10`.
| `API_IPV6_ENABLED` | off | When off (default), API binds **IPv4-only** and rejects IPv6 `API_BIND` ([#282](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/282)) |
| `RUN_MODE=prod` | — | Requires operator `LCD_URLS` (no public defaults) |

See [`indexer/.env.example`](../indexer/.env.example) and [`docs/operator-secrets.md`](../docs/operator-secrets.md) (prod vs QA vs local profiles, GitLab **#363**).

**429 shape:** HTTP **429** with `Retry-After` and `x-ratelimit-*` headers; minimal body. Burst size = `2 × RPS` per governor layer.

## Tests

```bash
cd indexer && cargo test --lib
cd indexer && cargo test --test security -j 1 -- --test-threads=1
make lint-indexer-log-secrets
```

Key cases in security.rs: sanitized LCD 502 body, LCD-heavy 429 under global limit off (native book + CG/CMC orderbook — **#278**), global 429 burst, blacklist-check LCD **502** (**#379**), POST route solve oversized body **413** (**#379**), capped list `limit` upper bound (`limit=99999`) and lower bound (`limit=-1` / `limit=0` clamp to **1** — **SEC-F05** / **#431**). Startup log secret guard (**SEC-F13** / **#433**): `cargo test --lib startup::tests` + `make lint-indexer-log-secrets`.

Broader SQL-backed list lower-bound sweep: [`api_limit_lower_bound.rs`](../indexer/tests/api_limit_lower_bound.rs) (**#317**). LCD `limit-book` / orderbook `depth` lower bounds: [`api_limit_book_lcd_mock.rs`](../indexer/tests/api_limit_book_lcd_mock.rs), [`api_orderbook_lcd_mock.rs`](../indexer/tests/api_orderbook_lcd_mock.rs).

**Frontend 429 copy (SEC-E04 / GitLab #426):** indexer/LCD HTTP **429** is classified in [`indexerErrors.ts`](../frontend-dapp/src/utils/indexerErrors.ts) (`isIndexerRateLimitError`) and humanized via [`INDEXER_RATE_LIMIT_RETRY_MESSAGE`](../frontend-dapp/src/utils/marketDataServiceCopy.ts). Vitest: `SwapPage.test.tsx` mocks indexer 429 and asserts calm retry guidance — see [docs/testing.md § SEC-E04](../docs/testing.md#indexer-http-429-calm-retry-copy-sec-e04-gitlab-426). Distinct from on-chain wrap-mapper rate limit ([`AGENTS_FRONTEND_SWAP_SAFETY_CTA.md`](./AGENTS_FRONTEND_SWAP_SAFETY_CTA.md)).

Manual check (failing LCD):

```bash
curl -sS -o /tmp/body.txt -w "%{http_code}" "http://127.0.0.1:3001/api/v1/pairs/terra1.../order-book-head?side=bid"
grep -E 'https?://|LCD query failed|cosmwasm' /tmp/body.txt && echo FAIL || echo OK
```

## Do not regress

- [`internal_err()`](../indexer/src/api/mod.rs) for SQL/DB paths (500, not 502).
- CORS allowlist, query caps, 30s timeout — [`indexer/tests/security.rs`](../indexer/tests/security.rs).
- Router `simulate_swap_operations` failures stay **400** with generic message ([`route_solver.rs`](../indexer/src/api/route_solver.rs) `maybe_simulate`).
