# Agent playbook: mainnet soft launch (non-economic)

Use when deploying or verifying the **columbus-5 soft launch** with non-economic CW20s, Coolify hosts **dex.cl8y.com** / **indexer.dex.cl8y.com**, or changing soft-launch defaults / Dockerfiles.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [`docs/runbooks/mainnet-soft-launch.md`](../docs/runbooks/mainnet-soft-launch.md) | Operator runbook + invariants **SL1–SL7** |
| [`docs/runbooks/soft-launch-faucet.md`](../docs/runbooks/soft-launch-faucet.md) | Soft-launch faucet / Mint page (**F1–F12**, GitLab #473) |
| [`scripts/lib/mainnet-soft-launch-defaults.sh`](../scripts/lib/mainnet-soft-launch-defaults.sh) | Token/pair catalog, deploy key, code ID defaults |
| [`scripts/deploy-dex-mainnet-soft-launch.sh`](../scripts/deploy-dex-mainnet-soft-launch.sh) | Single deploy script (`cl8ydeploy`) |
| [`scripts/lib/terrad-host.sh`](../scripts/lib/terrad-host.sh) | Host terrad (no LocalTerra Docker) |
| [`docker/indexer/Dockerfile`](../docker/indexer/Dockerfile) | Indexer image (Coolify) |
| [`docker/frontend/Dockerfile`](../docker/frontend/Dockerfile) | Frontend nginx image (Coolify) |
| [`docs/reference/fee-discount-tiers.md`](../docs/reference/fee-discount-tiers.md) | Tier ladder (I7 includes soft-launch defaults) |
| [`docs/reference/governance-multisig.md`](../docs/reference/governance-multisig.md) | Admin / governance address |

## Rules of thumb

1. **Do not** whitelist Terraport/GDEX/economic CW20 code IDs on the soft-launch path (**SL1/SL2**).
2. **Reuse** mintable code ID **10184** and cw20-base **6036** on mainnet unless forcing a fresh store.
3. **Deploy key** pays gas; **multisig** is `--admin` / governance / treasury (**SL4**). Host txs use `--gas-prices=28.325uluna` (not a flat fee). Keyring auto-detects `file` when `~/.terra/keyring-file/<key>.info` exists (avoids bech32-of-name errors from wrong `--keyring-backend os`).
4. **No compose** under `docker/` — Coolify uses the Dockerfiles directly.
5. **HTTPS only** for `VITE_INDEXER_URL` and production CORS origin `https://dex.cl8y.com`.
6. **Local Vite + remote soft-launch hosts:** do not widen production CORS. Use the Vite same-origin proxy (`/__dev/indexer`, `/__dev/lcd`) — [`AGENTS_FRONTEND_LOCAL_REMOTE_CORS_PROXY.md`](./AGENTS_FRONTEND_LOCAL_REMOTE_CORS_PROXY.md).
6. Keep fee-discount tiers aligned: `make check-fee-discount-tier-docs`.
7. Regression: `make test-mainnet-soft-launch-defaults` (includes `DRY_RUN=1` when wasm artifacts exist).

## Quick commands

```bash
make test-mainnet-soft-launch-defaults
make check-fee-discount-tier-docs
DRY_RUN=1 make deploy-mainnet-soft-launch
DRY_RUN=1 make deploy-soft-launch-faucet   # after soft-launch addresses exist (#473)
```

## Related

- [`AGENTS_SOFT_LAUNCH_FAUCET.md`](./AGENTS_SOFT_LAUNCH_FAUCET.md) — faucet deploy + Mint UI (#473)
- [`AGENTS_DEPLOY_TRACE.md`](./AGENTS_DEPLOY_TRACE.md) — deploy audit record
- [`AGENTS_LAUNCH_GO_NO_GO.md`](./AGENTS_LAUNCH_GO_NO_GO.md) — Phase 5 gate
- [`AGENTS_FEE_DISCOUNT_TIERS.md`](./AGENTS_FEE_DISCOUNT_TIERS.md) — tier wire format
- [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md) — Vite prod guards
