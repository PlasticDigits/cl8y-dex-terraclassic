# Coolify / container images (no compose)

Single-service Dockerfiles for production soft launch. Do **not** add a compose file here — Coolify (or any orchestrator) runs each image independently.

| Image | Dockerfile | Public host |
|-------|------------|-------------|
| Indexer | [`indexer/Dockerfile`](./indexer/Dockerfile) | `https://indexer.dex.cl8y.com` |
| Frontend | [`frontend/Dockerfile`](./frontend/Dockerfile) | `https://dex.cl8y.com` |

Build context is the **repository root** for both.

Public indexer also serves `GET /api/v1/defillama/daily` for Llama dimension adapters ([#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631), [`docs/DEFILLAMA.md`](../docs/DEFILLAMA.md)). TVL stays on-chain (factory `Pool {}`); do not point Llama at `/cg/tickers` `liquidity_in_usd`.

See [`docs/runbooks/mainnet-soft-launch.md`](../docs/runbooks/mainnet-soft-launch.md) and [`skills/AGENTS_MAINNET_SOFT_LAUNCH.md`](../skills/AGENTS_MAINNET_SOFT_LAUNCH.md).
