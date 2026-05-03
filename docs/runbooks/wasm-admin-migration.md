# Runbook: Wasm admin migration and upgrade checklist

Use this checklist when **migrating** or **upgrading** CosmWasm contracts (factory, pair, router, fee-discount, hooks) with an **admin** set (governance multisig). It does **not** replace the full deploy narrative—see [Deployment guide](../deployment-guide.md) and [Pool-only v2 launch](launch-checklist.md).

## Pre-flight

- [ ] **Artifact:** Production wasm from **workspace-optimizer** only (`make build-optimized` / CI optimizer workflow). Do not upload ad-hoc `cargo wasm` from PR builds to mainnet.
- [ ] **Checksums:** Record `wasm-checksums.txt` and code IDs for audit trail.
- [ ] **Governance:** `admin` on contracts is the intended multisig or DAO; verify with `terrad query wasm contract <addr>`.

## Migration / upgrade steps

1. **Build** optimized wasm artifacts per contract.
2. **Store** new wasm on chain; note new `code_id`.
3. **Migrate** each contract that supports `Migrate` (or follow contract-specific upgrade path in `smartcontracts/`).
4. **Verify** state after migration:
   - Factory: `get_config`, pair whitelist; after **factory 1.1.0** migrate, `pair_addr_reg` is populated from `pair_index` (see [GitLab #122](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/122), [contracts reference § Factory storage](../contracts-terraclassic.md#factory-storage--upgrades)).
   - Pair: fee config, hooks, limit-order state as applicable.
   - Router: factory address, trusted paths.
   - Fee-discount: tiers, trusted router flags.
5. **Smoke:** Pool query + optional LCD simulation (see deployment guide and smoke scripts).

## Commands (illustrative)

Replace placeholders, node, and fees per your network.

```bash
terrad tx wasm store artifacts/cl8y_dex_pair.wasm \
  --from <wallet> --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna --chain-id <chain-id> --node <rpc-url>

terrad tx wasm migrate <pair_addr> <new_code_id> '{}' \
  --from <admin> --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna --chain-id <chain-id> --node <rpc-url>
```

## Admin rotation

- Use **`update_admin`** (where supported) only through governance process; verify new admin on-chain before revoking old keys.

## References

- [Deployment guide](../deployment-guide.md) — store, instantiate, instantiate2.
- [Security model](../security-model.md) — governance and treasury.
- [Contracts reference](../contracts-terraclassic.md) — message shapes.
