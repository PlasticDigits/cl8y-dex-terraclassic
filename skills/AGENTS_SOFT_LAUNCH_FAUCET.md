# Agent playbook: soft-launch faucet (GitLab #473)

Use when deploying, verifying, or changing the **soft-launch CW20 faucet** / dApp **Mint** page (`/mint`).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [`docs/runbooks/soft-launch-faucet.md`](../docs/runbooks/soft-launch-faucet.md) | Operator runbook + invariants **F1–F14** |
| [`docs/runbooks/mainnet-soft-launch.md`](../docs/runbooks/mainnet-soft-launch.md) | Soft-launch SL1–SL7 (unchanged by faucet) |
| [`scripts/deploy-soft-launch-faucet.sh`](../scripts/deploy-soft-launch-faucet.sh) | Mainnet store / instantiate / `AddMinter` |
| [`scripts/deploy-dex-local.sh`](../scripts/deploy-dex-local.sh) | LocalTerra faucet parity (Phase 2a) |
| [`smartcontracts/contracts/faucet/`](../smartcontracts/contracts/faucet/) | Contract (`Drip`, pause, allowlist, cooldown) |
| [`frontend-dapp/src/pages/MintPage.tsx`](../frontend-dapp/src/pages/MintPage.tsx) | Mint UI |
| [`frontend-dapp/src/services/terraclassic/faucet.ts`](../frontend-dapp/src/services/terraclassic/faucet.ts) | LCD query + execute |
| [`frontend-dapp/src/services/terraclassic/terraGas.ts`](../frontend-dapp/src/services/terraclassic/terraGas.ts) | **`FAUCET_DRIP_GAS_LIMIT` (400k)** for `{ drip }` — must not fall through to **`BASE_GAS_LIMIT` (200k)** ([#474](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/474) / [#475](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/475)) |
| [`frontend-dapp/src/utils/terraAccountSequence.ts`](../frontend-dapp/src/utils/terraAccountSequence.ts) | Code-32 sequence parse + one-shot retry plan ([#499](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/499)) |
| [`deployments/mainnet-soft-launch/faucet-trace.md`](../deployments/mainnet-soft-launch/faucet-trace.md) | Deploy audit (after live deploy) |

## Rules of thumb

1. **Allowlist addresses only** — never trust client symbols; exclude QUARTZ/PEARL (**F1/F4**).
2. **Fixed drip + global wallet cooldown** — defaults `100000000` / `300s` (**F2/F3**).
3. **`cl8ydeploy` stays primary minter**; faucet is additional via `AddMinter` (**F5/F6**). No multisig minter handoff for noneconomic gems.
4. **Do not** put faucet code id on factory CW20 whitelist (**F7** / **SL1–SL2**).
5. **Hide Mint nav** unless `VITE_FAUCET_ADDRESS` is set (**F11**). Use `includeMint` on `getHeaderMoreMenuItems` / `getMobileMoreMenuItems`.
6. **Emergency:** faucet `Pause` + CW20 `RemoveMinter`; pause preserves cooldown map (**F9**).
7. User pays gas (**F12**). Happy path needs no indexer (**F10**).
8. **Mint drip gas envelope ([#474](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/474) / [#475](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/475)):** `{ drip }` must map to **`FAUCET_DRIP_GAS_LIMIT`** in `getGasLimitForTx` — not **`BASE_GAS_LIMIT`**. Verify: `make verify-issue-475`. Playbook: [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md).
9. **Account sequence mismatch (#499 / F14):** do **not** add Mint-page-only sequence logic. Shared **`broadcastTerraExecuteContracts`** refreshes sequence at sign, retries once on code 32, then **`Wallet out of sync. Try again.`**. Playbook: [`AGENTS_FRONTEND_TX_BROADCAST_TIMEOUT.md`](./AGENTS_FRONTEND_TX_BROADCAST_TIMEOUT.md); copy length: [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md).

## Quick commands

```bash
make build-optimized
cd smartcontracts && cargo test -p cl8y-dex-tests faucet_tests -- --test-threads=1
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- src/pages/MintPage.test.tsx src/utils/__tests__/faucetCooldown.test.ts src/services/terraclassic/__tests__/faucet.test.ts src/components/common/navItems.test.ts src/services/terraclassic/__tests__/terraGas.retailShapes.test.ts src/services/terraclassic/__tests__/terraBroadcastRecovery.test.ts src/utils/__tests__/terraAccountSequence.test.ts
make verify-issue-475
DRY_RUN=1 make deploy-soft-launch-faucet
```

## Related

- Soft launch: [`AGENTS_MAINNET_SOFT_LAUNCH.md`](./AGENTS_MAINNET_SOFT_LAUNCH.md)
- Shell nav: [`AGENTS_FRONTEND_SHELL_NAV.md`](./AGENTS_FRONTEND_SHELL_NAV.md) — Mint is conditional More item
- Deploy audit: [`AGENTS_DEPLOY_TRACE.md`](./AGENTS_DEPLOY_TRACE.md)
- Prod Vite / Coolify: [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md)
- Gas: [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md) — retail inventory / drip envelope (#475)
- Tx broadcast / code-32 retry: [`AGENTS_FRONTEND_TX_BROADCAST_TIMEOUT.md`](./AGENTS_FRONTEND_TX_BROADCAST_TIMEOUT.md) ([#499](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/499))
- Retail errors: [`AGENTS_FRONTEND_USER_ERRORS.md`](./AGENTS_FRONTEND_USER_ERRORS.md)
