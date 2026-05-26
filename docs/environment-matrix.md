# Environment matrix (local / testnet / mainnet)

Quick reference for **chain**, **LCD**, and **typical indexer** settings. Canonical network table: [README](../README.md#networks).

| Environment | Chain ID | Default LCD (examples) | Indexer notes |
|-------------|----------|--------------------------|---------------|
| **Local** | `localterra` | `http://localhost:1317` | Use `DATABASE_URL` to local Postgres; **`CORS_ORIGINS` must include every Vite origin you use** (`http://localhost:5173` and `http://127.0.0.1:5173` are distinct — [GitLab #131](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/131)); `RUN_MODE` usually unset (dev defaults for LCD OK). |
| **Testnet** | `rebel-2` | Public LCDs (see README) | Set explicit `LCD_URLS` and `RUN_MODE=prod` for production-style validation. |
| **Mainnet** | `columbus-5` | Operator-controlled LCDs; public mirrors exist | **Required:** `RUN_MODE=prod` and **non-default** `LCD_URLS` (see [`indexer/src/config.rs`](../indexer/src/config.rs)). Observability: **`tracing` logs only** — no `/metrics` ([GitLab #200](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/200)). |

## Related

- [Local development](local-development.md) — Docker, LocalTerra, Makefile.
- [Operator secrets](operator-secrets.md) — env vars and rotation.
- [Deployment guide](deployment-guide.md) — contract deploys per network.
