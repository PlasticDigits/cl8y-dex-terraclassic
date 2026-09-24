# Agent playbook: pair storage-key migration compatibility ([#1232](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1232))

Use when changing pair `migrate`, adding a required pair `Item`, reviewing a pair wasm upgrade, or checking a legacy pair after migrate. This is storage initialization, not discount wiring or TWAP arithmetic.

## Current verification status

Reviewed `origin/main` at `54c4868e`: `pair::migrate` backfills order, escrow, limit configuration, and asset-code items, but it does **not** backfill `DISCOUNT_REGISTRY` or `ORACLE_STATE`. Runtime code still hard-loads those items. The existing `migration_tests::pair_migration_preserves_fee_registry_lp_admin_and_limit_book` checks preservation of populated state, not a layout missing the keys. Keep #1232 open until the missing-key behavior and idempotence are implemented and tested.

The newer [#1324](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1324) is an ops request to migrate columbus-5 pairs to pair cw2 1.18.0 while retaining observations. Its issue body explicitly keeps #1232 separate; a live migrate or `Observe` smoke does not verify these backfills.

## Required invariants

| Id | Rule |
|----|------|
| **PM1** | Missing `DISCOUNT_REGISTRY` → save `None`. Preserve an existing `Some(addr)` and an explicit stored `None`. Do not copy the factory pointer or run factory wiring from pair migrate. `DISCOUNT_REGISTRY` is `Item<Option<Addr>>`; absent storage is not the same as stored `None`. |
| **PM2** | Missing `ORACLE_STATE` → save the instantiate default from `DEFAULT_OBSERVATION_CARDINALITY` (currently 360), `index: 0`, `cardinality_initialized: 0`. Preserve existing state. |
| **PM3** | Do not write or reset `OBSERVATIONS` or any cumulative in migrate. Keep `oracle_update` as the first writer that seeds an empty observation slot. Do not weaken hard `.load()` calls into silent fallback fees or empty oracle results. |
| **PM4** | Preserve all existing pair migrate backfills (`ORDER_NEXT_ID`, pending escrow, limit configs, and `ASSET_CODE_IDS`) and `cw2::ensure_from_older_version`. |

## Code and tests

- Pair migration: [`contract.rs`](../smartcontracts/contracts/pair/src/contract.rs) (`migrate`, `instantiate`, `oracle_update`, simulation/query hard-load paths).
- Storage declarations: [`state.rs`](../smartcontracts/contracts/pair/src/state.rs).
- Oracle default: [`oracle.rs`](../smartcontracts/packages/dex-common/src/oracle.rs) (`DEFAULT_OBSERVATION_CARDINALITY`).
- Existing integration preservation test: [`migration_tests.rs`](../smartcontracts/tests/src/migration_tests.rs) `pair_migration_preserves_fee_registry_lp_admin_and_limit_book`.
- Require a missing-key migrate fixture, assertions for `None` and the empty oracle default, nonzero-reserve `oracle_update`, and an idempotence case preserving `Some(addr)`, explicit `None`, existing `OracleState`, and `OBSERVATIONS`. Cover the hard-load paths after the backfill; `GetDiscountRegistry` alone is insufficient because it uses `may_load`.

After implementation, run:

```bash
cd smartcontracts && cargo test -p cl8y-dex-pair migrate_backfill_tests -- --test-threads=1
cd smartcontracts && cargo test -p cl8y-dex-pair oracle_overflow_tests -- --test-threads=1
make test-contracts
```

Do not use a live-chain migrate as the acceptance test. Do not use #1324 to imply this storage fix shipped.

## Cross-links

- [Contract reference: pair storage-key migration](../docs/contracts-terraclassic.md#pair-storage-key-migration-compatibility-1232)
- [Audit invariant C14](../docs/contracts-security-audit.md)
- [Migration test guide](../docs/testing.md#pair-storage-migration-compatibility-1232)
- [Discount-registry factory snapshot](./AGENTS_FACTORY_DISCOUNT_REGISTRY.md) — #535 / #536 / #538 wiring is separate.
- [TWAP cumulative Uint256](./AGENTS_TWAP_CUMULATIVE_U256.md) — #465 / #1224 / #1231 arithmetic is separate.
- [Asset code-id pin](./AGENTS_CW20_CODE_ID_PIN.md) — #582 is the prior `ASSET_CODE_IDS` backfill precedent.
- [Wasm admin migration regression](../docs/runbooks/wasm-admin-migration.md)
