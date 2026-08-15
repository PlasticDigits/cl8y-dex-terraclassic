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
4. Soft-launch factory has **10** pairs, so `SetPairTreasuryAll` fits the default cap. More than 10 pairs → use `SetPairTreasuryBatch`.

## One script

```bash
DRY_RUN=1 ./scripts/rotate-fee-treasury.sh
# LocalTerra (reads frontend-dapp/.env.local):
ROTATE_TREASURY_LOCAL=1 ./scripts/rotate-fee-treasury.sh
# columbus-5 (governance / wasm-admin key):
./scripts/rotate-fee-treasury.sh
```

The script: stores factory + pair wasm → migrates factory → migrates every registered pair → `UpdateConfig { treasury }` → `SetPairTreasuryAll` (or Batch) → queries factory `config.treasury` and each pair `get_fee_config.treasury`.

Skip store when code IDs are already on-chain:

```bash
ROTATE_TREASURY_SKIP_STORE=1 \
  ROTATE_TREASURY_FACTORY_CODE_ID=<id> \
  ROTATE_TREASURY_PAIR_CODE_ID=<id> \
  ./scripts/rotate-fee-treasury.sh
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
