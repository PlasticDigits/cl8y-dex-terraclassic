# Agent playbook: indexer API LCD security (GitLab #239)

## When to use

You are changing **LCD-proxied HTTP routes**, **rate limiting**, or **502 error bodies** on the indexer API. Gap findings **H6** (error leakage) and **H7** (LCD amplification) from [`gaps/GAP_1780200149.md`](../gaps/GAP_1780200149.md).

## Invariants

| ID | Invariant | Enforcement |
|----|-----------|-------------|
| **H6** | Client **502** bodies never include `LcdError` text, LCD URLs, or wasm paths | [`lcd_gateway_err`](../indexer/src/api/errors.rs) |
| **H7** | LCD-heavy routes rate-limited separately; prod cannot disable global/heavy limits with `0` | [`api/mod.rs`](../indexer/src/api/mod.rs), [`config.rs`](../indexer/src/config.rs) |
| **H7b** | Deep `limit-book` / `insert-hints` / price-window ≤ **101** LCD queries per request | [`LIMIT_BOOK_LCD_QUERY_BUDGET`](../indexer/src/api/limit_book_lcd.rs); [integrators.md § #267](../docs/integrators.md#insert-hints-price-window-gitlab-267) |
| **H7c** | Route `global_v1` documents ≤ **`LCD_HYBRID_SIM_BUDGET`** hybrid sims | [`best_execution.rs`](../indexer/src/api/best_execution.rs) |

Human matrix: [`docs/indexer-invariants.md`](../docs/indexer-invariants.md). Hybrid route behavior: [`AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md`](./AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md).

## LCD-heavy routes (stricter governor)

These paths use **`RATE_LIMIT_LCD_HEAVY_RPS`** (default **10**) in addition to **`RATE_LIMIT_RPS`** (default **60**):

- `GET /api/v1/pairs/{addr}/order-book-head`
- `GET /api/v1/pairs/{addr}/limit-book-shallow`
- `GET /api/v1/pairs/{addr}/limit-book`
- `GET /api/v1/pairs/{addr}/limit-book/insert-hints` ([#267](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/267))
- `GET|POST /api/v1/route/solve`
- `GET /api/v1/route/solve/best`

Legitimate frontend polling (e.g. deep book, route preview) should stay under **10 RPS per client IP**; use indexer caches on CG/CMC orderbook paths instead of hammering limit-book.

## Configuration

| Env | Default (dev) | Prod note |
|-----|---------------|-----------|
| `RATE_LIMIT_RPS` | 60 | If set to `0`, prod forces **60** |
| `RATE_LIMIT_LCD_HEAVY_RPS` | 10 | If set to `0`, prod forces **10** |
| `RUN_MODE=prod` | — | Requires operator `LCD_URLS` (no public defaults) |

See [`indexer/.env.example`](../indexer/.env.example).

## Tests

```bash
cd indexer && cargo test --lib
cd indexer && cargo test --test security -j 1 -- --test-threads=1
```

Key cases in security.rs: sanitized LCD 502 body, LCD-heavy 429 under global limit off, global 429 burst.

Manual check (failing LCD):

```bash
curl -sS -o /tmp/body.txt -w "%{http_code}" "http://127.0.0.1:3001/api/v1/pairs/terra1.../order-book-head?side=bid"
grep -E 'https?://|LCD query failed|cosmwasm' /tmp/body.txt && echo FAIL || echo OK
```

## Do not regress

- [`internal_err()`](../indexer/src/api/mod.rs) for SQL/DB paths (500, not 502).
- CORS allowlist, query caps, 30s timeout — [`indexer/tests/security.rs`](../indexer/tests/security.rs).
- Router `simulate_swap_operations` failures stay **400** with generic message ([`route_solver.rs`](../indexer/src/api/route_solver.rs) `maybe_simulate`).
