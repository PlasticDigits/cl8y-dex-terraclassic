# Coolify / container images (no compose)

Single-service Dockerfiles for production soft launch. Do **not** add a compose file here — Coolify (or any orchestrator) runs each image independently.

| Image | Dockerfile | Public host |
|-------|------------|-------------|
| Indexer | [`indexer/Dockerfile`](./indexer/Dockerfile) | `https://indexer.dex.cl8y.com` |
| Frontend | [`frontend/Dockerfile`](./frontend/Dockerfile) | `https://dex.cl8y.com` |

Build context is the **repository root** for both.

See [`docs/runbooks/mainnet-soft-launch.md`](../docs/runbooks/mainnet-soft-launch.md) and [`skills/AGENTS_MAINNET_SOFT_LAUNCH.md`](../skills/AGENTS_MAINNET_SOFT_LAUNCH.md).
