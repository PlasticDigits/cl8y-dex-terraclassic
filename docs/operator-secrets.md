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
| `METRICS_BIND` | Enable Prometheus when non-empty | **Dedicated** listener address: `host:port` (for example `127.0.0.1:9095`). **Legacy:** a non-empty value **without** `:` binds `127.0.0.1:METRICS_PORT` (default port **9095**). **`GET /metrics` is not served on the public API port** (see [GitLab #125](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/125)). |
| `METRICS_PORT` | Default port for legacy `METRICS_BIND` flags | Default `9095` when `METRICS_BIND` has no `:`. |
| `DEPLOY_ENV` | Ops profile: `dev`, `qa` (or `staging`), `production` (or `prod`) | When **unset**, defaults to **production** if `RUN_MODE=prod`, else **dev**. In **production** profile, `METRICS_BIND` **must not** use `0.0.0.0` or `::` (use loopback and scrape via sidecar/reverse proxy). For test / QA hosts that must expose metrics on all interfaces, set `DEPLOY_ENV=qa` (or run without `RUN_MODE=prod` so default profile is dev). |

## `RUN_MODE=prod` and `DEPLOY_ENV`

- `RUN_MODE=prod` requires non-empty `DATABASE_URL`, `FACTORY_ADDRESS`, `CORS_ORIGINS`, and **LCD URLs that are not the built-in public default list** (`indexer/src/config.rs`).
- `DEPLOY_ENV` selects the **ops** profile for behaviors such as metrics bind policy. It defaults from `RUN_MODE` when unset (see `METRICS_BIND` row above).

## Chain signing keys

- **Hot wallets** for `terrad tx` should use hardware wallets or HSM-backed keys where possible.
- **Multisig** governance for factory/router/pair admin is required for production; see [Security model](../security-model.md).

## Rotation

- **Database:** Rotate DB password; update `DATABASE_URL` in your secret store / orchestrator; restart indexer.
- **LCD:** If an endpoint is compromised, switch `LCD_URLS` and monitor [`indexer_block_time_fallbacks_total`](../indexer/src/metrics.rs) for timestamp issues.

## Logs

- The indexer uses `tracing` for logs; **do not** log `DATABASE_URL` or bearer tokens. Configure log collectors to redact known patterns.
