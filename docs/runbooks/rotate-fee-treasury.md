# Rotate DEX trading fees to the CMM treasury

Swap and limit-book commissions go to each pair’s `FEE_CONFIG.treasury`, snapshotted from factory `config.treasury` at `CreatePair`. Soft launch set that pointer to the DEX governance multisig. The intended sink is the **ustr-cmm CMM treasury**.

| Role | Address |
|------|---------|
| CMM treasury (target) | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` |
| DEX governance / wasm admin (signer) | `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` |
| Factory (columbus-5) | `terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea` |

`UpdateConfig { treasury }` alone is **not** enough: it only changes new pairs and the uluna pair-creation fee. Live pairs need `SetPairTreasury*` after a wasm migrate that adds pair `UpdateTreasury`.

## Prerequisites

1. Optimized artifacts: `make build-optimized` → `smartcontracts/artifacts/cl8y_dex_factory.wasm` + `cl8y_dex_pair.wasm`.
2. Factory wasm **1.7.0** (`SetPairTreasury` / `All` / `Batch`) and pair wasm **1.11.0** (`UpdateTreasury`).
3. Signer is factory `config.governance` **and** wasm `--admin` (2-of-3 multisig).
4. Live columbus-5 factory has **12** pairs (2026-08-15) — over the All cap of **10**. The script uses `SetPairTreasuryBatch`.

## One script

```bash
DRY_RUN=1 ./scripts/rotate-fee-treasury.sh
# LocalTerra (reads frontend-dapp/.env.local):
ROTATE_TREASURY_LOCAL=1 ./scripts/rotate-fee-treasury.sh
# columbus-5 (governance / wasm-admin key):
./scripts/rotate-fee-treasury.sh
```

The script: stores factory + pair wasm → migrates factory → migrates every registered pair → `UpdateConfig { treasury }` → `SetPairTreasuryAll` (or Batch) → queries factory `config.treasury` and each pair `get_fee_config.treasury`.

Skip store when code IDs are already on-chain.

**columbus-5 (2026-08-15):** pair code **11577**, factory code **11578**, `config.treasury` + all 12 pair `GetFeeConfig.treasury` = CMM, `pair_code_id` **11577**. Store was permissionless (`cl8ydeploy`); migrate / `UpdateConfig` / `SetPairTreasury*` required the 2-of-3.

```bash
# Resume after store. Signer MUST be the 2-of-3 (terra1zlmv2…), not cl8ydeploy.
TERRAD_HOST_KEY=<multisig-or-signer-flow> \
  ROTATE_TREASURY_SKIP_STORE=1 \
  ROTATE_TREASURY_PAIR_CODE_ID=11577 \
  ROTATE_TREASURY_FACTORY_CODE_ID=11578 \
  ./scripts/rotate-fee-treasury.sh
```

`cl8ydeploy` can **store** code. It cannot `migrate`, `UpdateConfig`, or `SetPairTreasury*`. Those need wasm admin + factory `governance` = `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` (generate-only → 2-of-3 `tx sign --multisig` → `multisign` → broadcast). Same flow as [governance key rotation §2](./governance-key-rotation.md#2-rotate-the-wasm-contract-admin).

Bare `terrad` defaults `--node` to **localhost:26657**. `--gas auto` and `tx sign` must hit columbus-5:

`--node https://terra-classic-rpc.publicnode.com:443`

Helper (uses that RPC + `multisig_2of3` / `multisig1` / `multisig2`):

```bash
rm -f unsigned.json sig1.json sig2.json signed.json
FACTORY=terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea
CMM=terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2

# one tx at a time (passphrase prompts)
./scripts/multisig-2of3-host-tx.sh wasm migrate "$FACTORY" 11578 '{}'
# then each pair: ./scripts/multisig-2of3-host-tx.sh wasm migrate "$PAIR" 11577 '{}'
./scripts/multisig-2of3-host-tx.sh wasm execute "$FACTORY" \
  "{\"update_config\":{\"treasury\":\"$CMM\"}}"
./scripts/multisig-2of3-host-tx.sh wasm execute "$FACTORY" \
  "{\"set_pair_treasury_batch\":{\"treasury\":\"$CMM\",\"start_after\":null,\"limit\":10}}"
./scripts/multisig-2of3-host-tx.sh wasm execute "$FACTORY" \
  "{\"set_pair_treasury_batch\":{\"treasury\":\"$CMM\",\"start_after\":9,\"limit\":10}}"
```

## Manual messages (after migrate)

```bash
CMM=terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2
FACTORY=terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea

# 1) Factory pointer (new pairs + pair-creation LUNC)
terrad tx wasm execute "$FACTORY" \
  "{\"update_config\":{\"treasury\":\"$CMM\"}}" \
  --from <gov-multisig> --gas auto --gas-adjustment 1.4 --gas-prices=28.325uluna

# 2) Live pairs (≤10)
terrad tx wasm execute "$FACTORY" \
  "{\"set_pair_treasury_all\":{\"treasury\":\"$CMM\"}}" \
  --from <gov-multisig> --gas auto --gas-adjustment 1.4 --gas-prices=28.325uluna
```

More than 10 pairs — loop `set_pair_treasury_batch` with `start_after` / `limit` until `has_more=false` (same cursor as `SetDiscountRegistryBatch`).

## Verify

```bash
# Factory
terrad query wasm contract-state smart "$FACTORY" '{"config":{}}'
# Each pair
terrad query wasm contract-state smart "$PAIR" '{"get_fee_config":{}}'
```

Both `config.treasury` and every `fee_config.treasury` must equal `terra16j5u6…`. Then:

```bash
export VERIFY_CONFIG_EXPECT_TREASURY=terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2
make qa-verify-deploy-config
```

Invariant **F4** in [`docs/contracts-security-audit.md`](../contracts-security-audit.md).
