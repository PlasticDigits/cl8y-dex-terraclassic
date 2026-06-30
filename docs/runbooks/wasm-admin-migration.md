# Runbook: Wasm admin migration and upgrade checklist

Use this checklist when **migrating** or **upgrading** CosmWasm contracts (factory, pair, router, fee-discount, hooks) with an **admin** set (governance multisig). It does **not** replace the full deploy narrative—see [Deployment guide](../deployment-guide.md) and [Pool-only v2 launch](launch-checklist.md).

## Pre-flight

- [ ] **Artifact:** Production wasm from **workspace-optimizer** only (`make build-optimized`). Do not upload ad-hoc `cargo wasm` from dev checks to mainnet ([docs/testing.md § CI](../testing.md#ci)).
- [ ] **Deploy trace (SEC-D12):** Record on the launch tracking issue ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)) using [`docs/templates/deploy-trace.md`](../templates/deploy-trace.md):
  - [ ] **Git SHA:** `git rev-parse HEAD` — paste output
  - [ ] **Terra Classic chain version:** `terrad version` or `terrad status --node <rpc> | jq -r .node_info.version` — paste output
  - [ ] **Contract code IDs** for factory, pair, router, fee-discount (and hook contracts if migrated)
  - [ ] **`wasm-checksums.txt`** artifact hashes from `smartcontracts/artifacts/wasm-checksums.txt`
  - [ ] **Post-migration verification command output** (state queries + smoke script — paste or link log)
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

## Automated regression (SEC-C14)

CI and local dev exercise migration **state preservation** without archived prior wasm artifacts:

```bash
make test-contracts
# or: cd smartcontracts && cargo test migration_tests
```

[`smartcontracts/tests/src/migration_tests.rs`](../smartcontracts/tests/src/migration_tests.rs) (GitLab [#405](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/405)) populates factory (governance, treasury, pair registry, LP admin, wallet/token/pair blacklists), pair (fee config, discount registry link, limit book), and fee-discount (tiers, registration, trusted router), downgrades cw2 version in `cw-multi-test` storage, migrates to the current code id, and asserts query snapshots are unchanged. Pair downgrade rejection is covered by `pair_migration_checks_version`. Cross-link: [contracts security audit § C14](../contracts-security-audit.md).

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

## Rollback vs forward-fix (SEC-H09)

Contract incidents require governance action — there is no instant “redeploy” like the frontend or indexer. When a post-migrate bug is discovered:

1. **Pause affected pairs** if funds are at risk — [emergency commands § Pause](./emergency-commands.md#1-pause-a-pair).
2. Choose **forward-fix migrate** (new `code_id` + `Migrate`) vs **migrate back** to a prior stored `code_id` vs **pause-and-wait** using the criteria in [rollback-decision.md § Contract](./rollback-decision.md#3-contract-incident) ([#445](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/445)).
3. Record governance tx hashes in the [incident timeline](../templates/incident-dex-indexer.md#incident-timeline).

**Limitations:** migrate-back requires the prior wasm still on chain, admin keys retained, and compatible contract state — see the runbook before broadcasting.

## References

- [Deployment guide](../deployment-guide.md) — store, instantiate, instantiate2.
- [Rollback decision runbook](./rollback-decision.md) — forward-fix vs migrate-back for all surfaces (SEC-H09).
- [Security model](../security-model.md) — governance and treasury.
- [Contracts reference](../contracts-terraclassic.md) — message shapes.
