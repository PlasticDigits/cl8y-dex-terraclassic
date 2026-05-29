# Agent playbook: Batch / ladder limit orders (GitLab #206)

Use when changing **multi-rung limit placement** on-chain, in the indexer, or in the dApp (`/limits` ladder panel, gas preflight, E2E).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/limit-orders.md § Place / cancel limit](../docs/limit-orders.md#place--cancel-limit-gitlab-206) | Messages, partial book-walk, batch attrs |
| [docs/limit-orders.md § Batch / ladder gas savings](../docs/limit-orders.md#batch-ladder-gas-savings) | Gas model vs N separate placements |
| [`dex-common` `limit_placement.rs`](../smartcontracts/packages/dex-common/src/limit_placement.rs) | Ladder expansion (`equal` only) |
| [`pair` `limit_placement.rs`](../smartcontracts/contracts/pair/src/limit_placement.rs) | Batch execute, refund, per-rung attrs |
| [`pair.rs`](../smartcontracts/packages/dex-common/src/pair.rs) | `Cw20HookMsg::PlaceLimitOrderBatch` / `PlaceLimitOrderLadder` |
| [`terraGas.ts`](../frontend-dapp/src/services/terraclassic/terraGas.ts) | `gasLimitForLimitOrderBatch`, batch base + per-rung |
| [`transactions.ts`](../frontend-dapp/src/services/terraclassic/transactions.ts) | `estimateLimitOrderBatchPlaceSequenceUlunaFeesTotal` |
| [`LimitOrderLadderPanel.tsx`](../frontend-dapp/src/components/trade/LimitOrderLadderPanel.tsx) | Ladder UI + gates |
| [`useLimitLadderPlaceGates.ts`](../frontend-dapp/src/hooks/useLimitLadderPlaceGates.ts) | Escrow + LUNC preflight for total escrow |
| [`limitOrderLadder.ts`](../frontend-dapp/src/utils/limitOrderLadder.ts) | Client ladder preview (must match `expand_limit_ladder`) |
| [`indexer/parser.rs`](../indexer/src/indexer/parser.rs) | One `limit_order_placements` row per `action=place_limit_order` |

## Invariants

1. **One side per batch** — bid uses token1 CW20; ask uses token0. Mixed sides → separate txs.
2. **CW20 `send` amount** = sum of per-rung gross `amount` fields (before maker fee split on-chain).
3. **One allowance** covers total escrow for the batch/ladder path.
4. **Validation errors** revert the whole tx; **`LimitInsertStepsExceeded`** on a rung skips that rung, refunds its escrow, continues; **zero** successful rungs → `LimitBatchNoRungsPlaced`.
5. **Indexer** scans every `action=place_limit_order` in the wasm stream (including after `place_limit_order_batch`); do not use `wasm_attr_last` for placement parsing ([#141](../docs/limit-orders.md)).
6. **Gas preflight** for ladder uses `estimateLimitOrderBatchPlaceSequenceUlunaFeesTotal(rungCount)` — keep aligned with `getGasLimitForTx` for `place_limit_order_batch` / `place_limit_order_ladder` ([#132](./AGENTS_TERRACLASSIC_GAS.md)).
7. **Retail single** order uses batch with one item ([`placeLimitOrderWithAllowance`](../frontend-dapp/src/services/terraclassic/pair.ts)).

## Tests to run after changes

```bash
# Contracts (batch + partial + ladder)
cd smartcontracts && cargo test -p cl8y-dex-tests limit_batch place_limit_order_ladder -- --nocapture

# Frontend unit
cd frontend-dapp && npm test -- limitOrderLadder limitOrderBatchGasSummary useLimitLadderPlaceGates

# E2E (LocalTerra, 5 workers)
bash scripts/e2e-provision-dev-wallet.sh
cd frontend-dapp && npx playwright test e2e/limit-orders-tx.spec.ts --project=e2e-tx
```

## Related skills

- Limit place/cancel E2E: [`AGENTS_E2E_LIMIT_ORDERS_TX.md`](./AGENTS_E2E_LIMIT_ORDERS_TX.md)
- Placement gas presets: [`AGENTS_FRONTEND_LIMIT_ORDER_PLACEMENT_GAS.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PLACEMENT_GAS.md)
- Terra gas / two-tx sequences: [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md)

## GitLab

- [#206](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/206) — batch / ladder feature + verification checklist
