# Agent playbook: LocalTerra trading bot swarm

Use this skill when working on **localnet-only** scripted trading volume for UI / indexer stress (GitLab [**#119**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/119)), or when wiring automation around `packages/localnet-trading-swarm`.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [`packages/localnet-trading-swarm/README.md`](../packages/localnet-trading-swarm/README.md) | Invariants, env vars, `--dry-run` / `--stats`, funding model (full factory CW20 enumeration) |
| [`scripts/localnet-trading-swarm.sh`](../scripts/localnet-trading-swarm.sh) | Repo-root entrypoint (`npm run start` in the package) |
| [`scripts/e2e-provision-dev-wallet.sh`](../scripts/e2e-provision-dev-wallet.sh) | Prior art for factory `pairs` → unique CW20 → `Mint` |
| [`scripts/deploy-dex-local.sh`](../scripts/deploy-dex-local.sh) | Writes `frontend-dapp/.env.local`, seeds pair liquidity |
| [`docs/local-development.md`](../docs/local-development.md) | Local stack prerequisites |
| [`docs/testing.md`](../docs/testing.md) | “Trading swarm for UI load” subsection |

## Rules of thumb

1. **Never** point the swarm at public RPC/LCD or mainnet/testnet — it exits unless LCD reports `localterra` and optional `VITE_NETWORK=local`.
2. **Do not** commit `SWARM_BOT_MNEMONIC` or generated mnemonics; CI and gitleaks expectations match other wallet docs ([`AGENTS_BUNDLE_DEV_WALLET.md`](./AGENTS_BUNDLE_DEV_WALLET.md)).
3. **Funding:** Prefer the built-in idempotent path (`test1` bank sends + CW20 `Mint` over **all** factory tokens). Do not shrink mint coverage to “only CL8Y/LUNC_C/USTC_C” unless product explicitly changes #119.
4. **Liquidity guards** in `liquidityGuards.ts` are heuristics aligned with default `deploy-dex-local` reserves; if you change seed liquidity in the deploy script, revisit constants and the README table together.
5. **CI:** package tests are `cd packages/localnet-trading-swarm && npm ci && npm run test:run` (no chain).

## Cross-links

- Dev wallet / mnemonic bundle safety: [`AGENTS_BUNDLE_DEV_WALLET.md`](./AGENTS_BUNDLE_DEV_WALLET.md)  
- Gas limits for swap shapes: [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md)
