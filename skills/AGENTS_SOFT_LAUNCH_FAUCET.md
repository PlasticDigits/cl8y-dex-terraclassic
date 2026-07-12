# Agent playbook: soft-launch faucet (GitLab #473)

Use when deploying, verifying, or changing the **soft-launch CW20 faucet** / dApp **Mint** page (`/mint`).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [`docs/runbooks/soft-launch-faucet.md`](../docs/runbooks/soft-launch-faucet.md) | Operator runbook + invariants **F1–F12** |
| [`docs/runbooks/mainnet-soft-launch.md`](../docs/runbooks/mainnet-soft-launch.md) | Soft-launch SL1–SL7 (unchanged by faucet) |
| [`scripts/deploy-soft-launch-faucet.sh`](../scripts/deploy-soft-launch-faucet.sh) | Mainnet store / instantiate / `AddMinter` |
| [`scripts/deploy-dex-local.sh`](../scripts/deploy-dex-local.sh) | LocalTerra faucet parity (Phase 2a) |
| [`smartcontracts/contracts/faucet/`](../smartcontracts/contracts/faucet/) | Contract (`Drip`, pause, allowlist, cooldown) |
| [`frontend-dapp/src/pages/MintPage.tsx`](../frontend-dapp/src/pages/MintPage.tsx) | Mint UI |
| [`frontend-dapp/src/services/terraclassic/faucet.ts`](../frontend-dapp/src/services/terraclassic/faucet.ts) | LCD query + execute |
| [`deployments/mainnet-soft-launch/faucet-trace.md`](../deployments/mainnet-soft-launch/faucet-trace.md) | Deploy audit (after live deploy) |

## Rules of thumb

1. **Allowlist addresses only** — never trust client symbols; exclude QUARTZ/PEARL (**F1/F4**).
2. **Fixed drip + global wallet cooldown** — defaults `100000000` / `300s` (**F2/F3**).
3. **`cl8ydeploy` stays primary minter**; faucet is additional via `AddMinter` (**F5/F6**). No multisig minter handoff for noneconomic gems.
4. **Do not** put faucet code id on factory CW20 whitelist (**F7** / **SL1–SL2**).
5. **Hide Mint nav** unless `VITE_FAUCET_ADDRESS` is set (**F11**). Use `includeMint` on `getHeaderMoreMenuItems` / `getMobileMoreMenuItems`.
6. **Emergency:** faucet `Pause` + CW20 `RemoveMinter`; pause preserves cooldown map (**F9**).
7. User pays gas (**F12**). Happy path needs no indexer (**F10**).

## Quick commands

```bash
make build-optimized
cd smartcontracts && cargo test -p cl8y-dex-tests faucet_tests -- --test-threads=1
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- src/pages/MintPage.test.tsx src/utils/__tests__/faucetCooldown.test.ts src/services/terraclassic/__tests__/faucet.test.ts src/components/common/navItems.test.ts
DRY_RUN=1 make deploy-soft-launch-faucet
```

## Related

- Soft launch: [`AGENTS_MAINNET_SOFT_LAUNCH.md`](./AGENTS_MAINNET_SOFT_LAUNCH.md)
- Shell nav: [`AGENTS_FRONTEND_SHELL_NAV.md`](./AGENTS_FRONTEND_SHELL_NAV.md) — Mint is conditional More item
- Deploy audit: [`AGENTS_DEPLOY_TRACE.md`](./AGENTS_DEPLOY_TRACE.md)
- Prod Vite / Coolify: [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md)
- Gas: [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md)
