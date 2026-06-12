# Operator secrets handling (DB, LCD, keys)

This document describes **how to handle secrets** when operating the indexer and deploying DEX contracts. **Never commit** secrets, `.env` files with live credentials, or private keys to git.

## Indexer

| Variable | Role | Notes |
|----------|------|--------|
| `DATABASE_URL` | Postgres connection | Use TLS to the DB provider when available; rotate credentials if leaked. |
| `LCD_URLS` | Comma-separated LCD endpoints | **Production:** Set `RUN_MODE=prod` and use **operator-controlled** LCD URLs (not the built-in public defaults). Optional API keys if your provider uses them—pass via env or sidecar, not in repo. |
| `CORS_ORIGINS` | Browser origin allowlist | Not a substitute for auth; restrict to your frontends. |
| `FACTORY_ADDRESS` | On-chain factory | Public address; not secret. |
| `FEE_DISCOUNT_ADDRESS`, `ROUTER_ADDRESS`, `USTC_DENOM` | Optional config | Same as factory—addresses are public. |
| `RATE_LIMIT_RPS` | Global per-IP API governor | Default **60** RPS. **`RUN_MODE=prod`:** `0` is clamped to **60** at startup ([#363](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/363)). Dev/QA may set `0` to disable the global layer (e.g. Playwright bursts). |
| `RATE_LIMIT_LCD_HEAVY_RPS` | Stricter per-IP limit on LCD-heavy routes | Default **10** RPS. **`RUN_MODE=prod`:** `0` is clamped to **10** ([#363](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/363)). Routes: limit-book family, `route/solve`, `route/solve/best`, `/cg/orderbook`, `/cmc/orderbook/*` — see [`skills/AGENTS_INDEXER_API_LCD_SECURITY.md`](../skills/AGENTS_INDEXER_API_LCD_SECURITY.md). |

## `RUN_MODE=prod`

- `RUN_MODE=prod` requires non-empty `DATABASE_URL`, `FACTORY_ADDRESS`, `CORS_ORIGINS`, and **LCD URLs that are not the built-in public default list** (`indexer/src/config.rs`).
- Production cannot disable rate limiting: `RATE_LIMIT_RPS=0` and `RATE_LIMIT_LCD_HEAVY_RPS=0` are clamped to **60** and **10** respectively at config load ([#363](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/363)).

## Observability

The indexer exposes **no Prometheus `/metrics` endpoint** ([GitLab #200](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/200)). Use **`tracing` logs** (configure `RUST_LOG` / log collectors) for block processing errors, LCD failures, and block timestamp fallback warnings — see [`indexer-invariants.md`](./indexer-invariants.md) and [`runbooks/indexer-reorg-replay-dedup.md`](./runbooks/indexer-reorg-replay-dedup.md).

## Chain signing keys

- **Hot wallets** for `terrad tx` should use hardware wallets or HSM-backed keys where possible.
- **Multisig** governance for factory/router/pair admin is required for production; see [Security model](../security-model.md).

## Rotation

- **Database:** Rotate DB password; update `DATABASE_URL` in your secret store / orchestrator; restart indexer.
- **LCD:** If an endpoint is compromised, switch `LCD_URLS` and monitor logs for block timestamp fallback warnings.

## Logs

- The indexer uses `tracing` for logs; **do not** log `DATABASE_URL` or bearer tokens. Configure log collectors to redact known patterns.
