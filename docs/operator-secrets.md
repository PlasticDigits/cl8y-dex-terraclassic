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

## `RUN_MODE=prod`

- `RUN_MODE=prod` requires non-empty `DATABASE_URL`, `FACTORY_ADDRESS`, `CORS_ORIGINS`, and **LCD URLs that are not the built-in public default list** (`indexer/src/config.rs`).

## Observability

The indexer exposes **no Prometheus `/metrics` endpoint** ([GitLab #200](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/200)). Use **`tracing` logs** (configure `RUST_LOG` / log collectors) for block processing errors, LCD failures, and block timestamp fallback warnings — see [`indexer-invariants.md`](./indexer-invariants.md) and [`runbooks/indexer-reorg-replay-dedup.md`](./runbooks/indexer-reorg-replay-dedup.md).

**Reorg halt alerting (#362):** Alert on structured log field `event=indexer_reorg_halt` (target `indexer.reorg_halt`). Optionally set **`REORG_ALERT_WEBHOOK_URL`** on the indexer process for JSON webhook delivery (PagerDuty, Slack, etc.). Recovery: [`scripts/indexer-reorg-recover.sh`](../scripts/indexer-reorg-recover.sh) — dry-run first, `--apply` only after review.

## Chain signing keys

- **Hot wallets** for `terrad tx` should use hardware wallets or HSM-backed keys where possible.
- **Multisig** governance for factory/router/pair admin is required for production; see [Security model](../security-model.md).

## Rotation

- **Database:** Rotate DB password; update `DATABASE_URL` in your secret store / orchestrator; restart indexer.
- **LCD:** If an endpoint is compromised, switch `LCD_URLS` and monitor logs for block timestamp fallback warnings.

## Logs

- The indexer uses `tracing` for logs; **do not** log `DATABASE_URL` or bearer tokens. Configure log collectors to redact known patterns.
