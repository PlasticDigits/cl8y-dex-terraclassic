# Operator secrets handling (DB, LCD, keys)

This document describes **how to handle secrets** when operating the indexer and deploying DEX contracts. **Never commit** secrets, `.env` files with live credentials, or private keys to git.

## Indexer

| Variable | Role | Notes |
|----------|------|--------|
| `DATABASE_URL` | Postgres connection | Use TLS to the DB provider when available; rotate credentials if leaked. |
| `LCD_URLS` | Comma-separated LCD endpoints | **Production:** Set `RUN_MODE=prod` and use **operator-controlled** LCD URLs (not the built-in public defaults). Optional API keys if your provider uses them—pass via env or sidecar, not in repo. |
| `CORS_ORIGINS` | Browser origin allowlist | Not a substitute for auth; restrict to your frontends. |
| `VITE_INDEXER_URL` (frontend) | Browser quote / charts API | **Production:** HTTPS only. MITM or a compromised indexer can serve misleading routes while pools remain valid on-chain ([#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/378)). |
| `VITE_WC_PROJECT_ID` (frontend) | WalletConnect project | Required for production `vite build`; do not rely on the dev-only shared default. |
| Token `logo_url` (indexer DB) | Display metadata | Update only after **human review** of symbol + logo host (allowlisted in dApp). Untrusted URLs render as blockies. |
| `FACTORY_ADDRESS` | On-chain factory | Public address; not secret. |
| `FEE_DISCOUNT_ADDRESS`, `ROUTER_ADDRESS`, `USTC_DENOM` | Optional config | Same as factory—addresses are public. |
| `REORG_ALERT_WEBHOOK_URL` | Reorg halt webhook | Optional. POST JSON on chain reorg halt (GitLab #362). Use your paging/Slack endpoint; not a public API. |
| `RATE_LIMIT_RPS` | Global per-IP API governor | Default **60** RPS. **`RUN_MODE=prod`:** `0` is clamped to **60** with a startup warning ([#363](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/363)). Dev/QA may set `0` to disable the global layer (e.g. Playwright bursts). When **both** `RATE_LIMIT_RPS` and `RATE_LIMIT_LCD_HEAVY_RPS` are **0**, startup logs a **DoS-risk warning** regardless of mode ([#379](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/379)). |
| `RATE_LIMIT_LCD_HEAVY_RPS` | Stricter per-IP limit on LCD-heavy routes | Default **10** RPS. **`RUN_MODE=prod`:** `0` is clamped to **10** with a startup warning ([#363](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/363)). |

## Rate limits (production)

LCD-heavy routes fan out multiple upstream LCD `smart` queries per HTTP request. They use a **separate** `tower_governor` layer in addition to the global limit. Full route list and invariants: [`skills/AGENTS_INDEXER_API_LCD_SECURITY.md`](../skills/AGENTS_INDEXER_API_LCD_SECURITY.md), [`indexer-invariants.md`](./indexer-invariants.md).

| Profile | `RATE_LIMIT_RPS` | `RATE_LIMIT_LCD_HEAVY_RPS` | Notes |
|---------|------------------|----------------------------|--------|
| **Mainnet / prod** | **60** (min) | **10** (min) | Set `RUN_MODE=prod`. Do not set either to `0`. |
| **QA / staging** | **60** | **10** | Match prod limits unless load-testing with isolated infra. |
| **Local dev / Playwright** | **0** (global off) | **10** (default) | `deploy-dex-local.sh` writes `RATE_LIMIT_RPS=0` and `RATE_LIMIT_LCD_HEAVY_RPS=10` so UI bursts do not 429 on `/health` while route/solve stays bounded. |

**429 response:** HTTP **429 Too Many Requests** with `Retry-After` and `x-ratelimit-*` headers (`tower_governor` `use_headers()`). Empty or minimal body — clients should back off and respect `Retry-After`. Keys on **socket peer IP** only (no trusted `X-Forwarded-For`).

**Integrator guidance:** debounced swap/trade quotes (`/api/v1/route/solve/best`) should stay under **10 RPS per IP**; use CG/CMC orderbook caches instead of hammering `limit-book` walks. See [`integrators.md`](./integrators.md).

## `RUN_MODE=prod`

- `RUN_MODE=prod` requires non-empty `DATABASE_URL`, `FACTORY_ADDRESS`, `CORS_ORIGINS`, and **LCD URLs that are not the built-in public default list** (`indexer/src/config.rs`).
- Production cannot disable rate limiting: `RATE_LIMIT_RPS=0` and `RATE_LIMIT_LCD_HEAVY_RPS=0` are clamped to **60** and **10** respectively at config load ([#363](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/363)). Startup logs effective values; warnings are emitted if env had `0` for either knob. When **both** are `0` in env, an additional **dual-disable** warning is logged ([#379](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/379)).

## Observability

The indexer exposes **no Prometheus `/metrics` endpoint** ([GitLab #200](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/200)). Use **`tracing` logs** (configure `RUST_LOG` / log collectors) for block processing errors, LCD failures, and block timestamp fallback warnings — see [`indexer-invariants.md`](./indexer-invariants.md) and [`runbooks/indexer-reorg-replay-dedup.md`](./runbooks/indexer-reorg-replay-dedup.md). On reorg halt, alert on stderr prefix **`INDEXER_REORG_HALT`** or tracing target **`indexer_reorg_halt`**; optional `REORG_ALERT_WEBHOOK_URL` for webhook delivery.

**Reorg halt alerting (#362):** Alert on structured log field `event=indexer_reorg_halt` (target `indexer.reorg_halt`). Optionally set **`REORG_ALERT_WEBHOOK_URL`** on the indexer process for JSON webhook delivery (PagerDuty, Slack, etc.). Recovery: [`scripts/indexer-reorg-recover.sh`](../scripts/indexer-reorg-recover.sh) — dry-run first, `--apply` only after review.

## Chain signing keys

- **Hot wallets** for `terrad tx` should use hardware wallets or HSM-backed keys where possible.
- **Multisig** governance for factory/router/pair admin is required for production; see [Security model](../security-model.md).

## Frontend deploy secrets

| Variable | Role | Notes |
|----------|------|--------|
| `VITE_WC_PROJECT_ID` | WalletConnect Cloud project | **Required** for `vite build --mode production`. Do not rely on a shared default in source ([#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378)). |
| `VITE_INDEXER_URL` | Browser → indexer API | **HTTPS only** on public sites. Pin to operator-controlled origin; see [Security model § Off-chain trust](./security-model.md#off-chain-trust-boundaries-frontend). |
| `VITE_DEV_MNEMONIC` | Simulated wallet (dev only) | Must **not** be set for staging/production builds unless `VITE_ALLOW_DEV_MNEMONIC=local-only` with explicit operator approval. |

## Rotation

- **Database:** Rotate DB password; update `DATABASE_URL` in your secret store / orchestrator; restart indexer.
- **LCD:** If an endpoint is compromised, switch `LCD_URLS` and monitor logs for block timestamp fallback warnings.

## Logs

- The indexer uses `tracing` for logs; **do not** log `DATABASE_URL`, `REORG_ALERT_WEBHOOK_URL`, bearer tokens, or mnemonics. Configure log collectors to redact known patterns.
- **Automated guard (SEC-F13 / GitLab #433):** `make lint-indexer-log-secrets` greps `indexer/src/` for secret field names inside `tracing::` macro arguments and exits nonzero on match. Unit tests in [`startup.rs`](../indexer/src/startup.rs) capture startup INFO output and assert a dummy DATABASE_URL password is absent. CI job `lint-indexer-log-secrets` runs the grep on default branch and MRs touching indexer sources.
