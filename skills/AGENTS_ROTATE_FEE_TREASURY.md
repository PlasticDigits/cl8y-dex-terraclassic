# Agent playbook: rotate DEX trading fees to CMM

Use when swap / limit-book commissions must go to the **ustr-cmm CMM treasury** instead of the DEX governance multisig.

## Target

`terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2`

Do **not** use `terra1zlmv2…` (governance / wasm admin only).

## Invariant (F4)

1. Factory `UpdateConfig { treasury }` updates **new** pairs and pair-creation uluna only.
2. Live pairs keep instantiate-time `FEE_CONFIG.treasury` until `SetPairTreasury*`.
3. Pair `UpdateTreasury` is factory-only. Direct pair execute from an EOA is `Unauthorized`.
4. `SetPairTreasuryAll` fails when `PAIR_COUNT` > 10 — use `SetPairTreasuryBatch`.
5. Soft-launch columbus-5 has 10 pairs, so All is enough after migrate.

## Ops

```bash
DRY_RUN=1 ./scripts/rotate-fee-treasury.sh
./scripts/rotate-fee-treasury.sh
```

Runbook: [`docs/runbooks/rotate-fee-treasury.md`](../docs/runbooks/rotate-fee-treasury.md).

## Tests

`factory_tests::set_pair_treasury_all_rotates_existing_pairs_and_swap_fees_follow`  
`set_pair_treasury_batch_paginates_and_covers_all_pairs`  
`set_pair_treasury_all_rejects_when_pair_count_exceeds_cap`  
`test_pair_update_treasury_unauthorized`
