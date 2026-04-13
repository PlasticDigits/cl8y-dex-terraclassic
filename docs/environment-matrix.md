# Environment matrix (local / testnet / mainnet)

Quick reference for **chain**, **LCD**, and **typical indexer** settings. Canonical network table: [README](../README.md#networks).

| Environment | Chain ID | Default LCD (examples) | Indexer notes |
|-------------|----------|--------------------------|---------------|
| **Local** | `localterra` | `http://localhost:1317` | Use `DATABASE_URL` to local Postgres; `CORS_ORIGINS` includes `http://localhost:5173` for Vite; `RUN_MODE` usually unset (dev defaults for LCD OK). |
| **Testnet** | `rebel-2` | Public LCDs (see README) | Set explicit `LCD_URLS` and `RUN_MODE=prod` for production-style validation. |
| **Mainnet** | `columbus-5` | Operator-controlled LCDs; public mirrors exist | **Required:** `RUN_MODE=prod` and **non-default** `LCD_URLS` (see [`indexer/src/config.rs`](../indexer/src/config.rs)). |

## Related

- [Local development](local-development.md) — Docker, LocalTerra, Makefile.
- [Operator secrets](operator-secrets.md) — env vars and rotation.
- [Deployment guide](deployment-guide.md) — contract deploys per network.
