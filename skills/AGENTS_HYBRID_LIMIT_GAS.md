# Agent playbook: Hybrid limit placement, fill, and cancel gas (#1329)

Use when changing retail limit placement, book insertion caps, hybrid book+pool fills, limit cancellation fees, or the related fee preflight.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [`docs/limit-orders.md`](../docs/limit-orders.md) | Message behavior, insertion caps, gas formulas, and rollback guarantees |
| [`docs/frontend.md`](../docs/frontend.md) | Retail gas estimator and signed fee flow |
| [`terraGas.ts`](../frontend-dapp/src/services/terraclassic/terraGas.ts) | Message-shape gas limits for place, edit, fill, and cancel |
| [`transactions.ts`](../frontend-dapp/src/services/terraclassic/transactions.ts) | LUNC fee reserve for allowance + place and update-price executes |
| [`LimitOrderNativeGasBalanceGate`](../frontend-dapp/src/utils/limitOrderNativeGasBalanceGate.ts) | Prevent the first allowance tx when the wallet cannot fund the full place sequence |
| [`pair::execute_swap`](../smartcontracts/contracts/pair/src/contract.rs) | Hybrid split validation, book matching, pool remainder, and atomic failure |
| [`orderbook.rs`](../smartcontracts/contracts/pair/src/orderbook.rs) | Bounded insertion and hybrid walk behavior |
| [invariant L5 / L25](../docs/contracts-security-audit.md) | On-chain bounded walks, dApp gas envelope, and failure rollback |

## Invariants

1. **Fee limits come from the execute message.** Do not accept `gas` or `credit` from the URL, user input, or a quote. `getGasLimitForTx` reads only the message's supported action and `max_adjust_steps`; missing steps use Medium (32), values clamp to the contract hard cap (256).
2. **Insertion envelope stays aligned across callers.** Batch/ladder gas is `1,000,000 + 180,000 × rung_count + 25,000 × sum(rung max_adjust_steps)`. A legacy single place uses `1,200,000 + 25,000 × steps`; `UpdateLimitOrderPrice` uses `350,000 + 25,000 × steps`. Revisit the increment when new LocalTerra or Columbus-5 gas measurements show a higher use.
3. **Budget skipped-rung retries.** Pair batches sort rungs by book key and thread a verified insert cursor after a successful insert (#266). A rung that exceeds its step cap is skipped before advancing that cursor, so later rungs may perform another bounded head walk. Sum the per-rung caps; do not assume one maximum cap bounds the full batch. If contract insertion ordering or skip semantics change, revisit the envelope and tests together.
4. **Credit preflight uses the same envelope.** Limit page, trade ticket, ladder gates, gas summary, and native Max reserve must receive the current `max_adjust_steps`. The preflight covers `increase_allowance` plus the place hook so the first tx is not signed when the remaining fee is unavailable.
5. **Hybrid split and failure are atomic.** `pool_input + book_input` must equal that hop's offer. The book leg may leave a remainder for the pool; if the pool cannot take it, return the specific liquidity error and let the full transaction revert. A failed execute must not leave maker escrow, fills, or fill events persisted.
6. **Keep cancel separate.** Cancel is a single pair execute with a CW20 refund submessage. Preserve its measured 1,000,000 gas envelope unless an on-chain `gas_used` measurement requires a change.

## Verification

```bash
make verify-issue-1329
# With LocalTerra deployed and the local indexer running:
VERIFY_ISSUE_1329_CHAIN=1 make verify-issue-1329
```

The LocalTerra E2E checks LCD `gas_used < gas_wanted` for limit place, cancel, and hybrid fill. It is not Columbus-5 production sign-off. A Columbus-5 place/fill/cancel that needs a wallet, keyring, or multisig signature is operator work; use a `cl8y-pm` inbox card with the DEX issue URL.

Key regressions: `transactions.test.ts`, `terraGas.retailShapes.test.ts`, `limitOrderNativeGasBalanceGate.test.ts`, `humanizeTerraTxError.test.ts`, `swapQueryParams.test.ts`, `limit_order_tests::hybrid_book_fill_reverts_when_pool_cannot_take_remainder`, and `e2e/limit-orders-tx.spec.ts` / `e2e/hybrid-swap.spec.ts`.

Related playbooks: [`AGENTS_FRONTEND_LIMIT_ORDER_PLACEMENT_GAS.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PLACEMENT_GAS.md), [`AGENTS_LIMIT_ORDER_BATCH_LADDER.md`](./AGENTS_LIMIT_ORDER_BATCH_LADDER.md), and [`AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md).
