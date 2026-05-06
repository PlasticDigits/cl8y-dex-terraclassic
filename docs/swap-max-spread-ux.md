# Swap max spread & price impact (dApp)

This document describes how the retail **Swap** page surfaces **price impact** and **max spread** checks so users are not surprised by thin-liquidity failures or unreadable chain errors.

**Related:** GitLab issue [**#134**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134) · pair `assert_max_spread` in [`smartcontracts/contracts/pair/src/contract.rs`](../../smartcontracts/contracts/pair/src/contract.rs) · frontend [`SwapPage.tsx`](../../frontend-dapp/src/pages/SwapPage.tsx) · math [`swapMaxSpread.ts`](../../frontend-dapp/src/utils/swapMaxSpread.ts) · LCD preflight [`swapRoutePreflight.ts`](../../frontend-dapp/src/services/terraclassic/swapRoutePreflight.ts) · tx copy [`humanizeTerraTxError.ts`](../../frontend-dapp/src/utils/humanizeTerraTxError.ts)

## Invariants (on-chain, pair, no `belief_price`)

For each pool hop the contract compares:

- `spread_cmp = min(spread_amount, pool_gross)` where `pool_gross = pool_return_amount + commission_amount`.
- `total_gross_out = pool_gross + book_return_net` (for pool-only swaps, `book_return_net = 0`).
- The hop **fails** if `spread_cmp / total_gross_out > max_spread` (strict inequality), where `max_spread` is the decimal the user supplies via **Slippage tolerance** (`slippageTolerance%` → `max_spread = slippageTolerance / 100` as a string, e.g. `1%` → `"0.01"`).

The router applies the **same** `max_spread` to **each** hop in `execute_swap_operations` ([`router` execute path](../../smartcontracts/contracts/router/src/contract.rs)).

## Invariants (frontend)

1. **Direct pool quote** (`pair` `simulation`): **Price impact** uses the pool’s `return_amount`, `commission_amount`, and `spread_amount` in the same ratio style as the UI historically used; this matches the pool-only branch of `assert_max_spread` for typical swaps.
2. **Router / indexer / native multihop quotes**: LCD `simulate_swap_operations` returns only the **final output amount** — it does **not** return per-hop spread. The dApp runs an extra **sequential preflight**: for each hop it resolves the pair via the factory, then calls `simulation` or `hybrid_simulation` on the pair with the **cumulative offer amount** for that hop (mirroring the router query loop). It records the **worst** hop spread % (contract formula above) and whether **any** hop would exceed the user’s `max_spread`.
3. **Submit guard**: If preflight finds `anyHopExceedsMaxSpread`, the Swap button stays disabled and an **Insufficient liquidity for this trade size** panel explains the constraint (GitLab #134).
4. **Post-submit errors**: If a swap still fails with `Max spread assertion` in the log (wallet/LCD wording varies), `transactions.ts` maps the message to a short **Trade rejected: price impact…** string via `tryHumanizeTerraTxMessage` so the alert is not a raw wasm stack trace.

## Agent / third-party notes

- Changing **slippage** invalidates the simulation query (it is part of the React Query key) so preflight comparisons stay aligned with the submitted `max_spread`.
- If you add a new swap entry path that uses the **router** but skips `preflightSwapRouteSpread`, multihop quotes may again show **0%** price impact while the chain rejects the trade — keep paths consistent with #134.
- Gas buffers for router swaps remain documented under [`docs/frontend.md`](./frontend.md) and [`skills/AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md).
