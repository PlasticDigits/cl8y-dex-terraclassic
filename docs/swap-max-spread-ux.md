# Swap max spread & price impact (dApp)

This document describes how the retail **Swap** page surfaces **price impact** and **max spread** checks so users are not surprised by thin-liquidity failures or unreadable chain errors.

**Related:** GitLab issue [**#134**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134) · [**#158**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/158) (single route row in trade summary) · [**#168**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/168) (MEV / public mempool disclosure in Swap Settings — [`docs/frontend.md#swap-mev-posture`](./frontend.md#swap-mev-posture)) · pair `assert_max_spread` in [`smartcontracts/contracts/pair/src/contract.rs`](../../smartcontracts/contracts/pair/src/contract.rs) · frontend [`SwapPage.tsx`](../../frontend-dapp/src/pages/SwapPage.tsx) · route label helper [`swapRouteDisplay.ts`](../../frontend-dapp/src/utils/swapRouteDisplay.ts) · math [`swapMaxSpread.ts`](../../frontend-dapp/src/utils/swapMaxSpread.ts) · LCD preflight [`swapRoutePreflight.ts`](../../frontend-dapp/src/services/terraclassic/swapRoutePreflight.ts) · tx copy [`humanizeTerraTxError.ts`](../../frontend-dapp/src/utils/humanizeTerraTxError.ts) · indexer + route LCD context: [`docs/indexer-invariants.md`](./indexer-invariants.md#frontend-expectations-read-path) · agent notes: [`skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](../skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md)

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
4. **Post-submit errors**: If a swap still fails with `Max spread assertion` in the log (wallet/LCD wording varies), [`terraBroadcast.ts`](../frontend-dapp/src/services/terraclassic/terraBroadcast.ts) maps the message to a short **Trade rejected: price impact…** string via `tryHumanizeTerraTxMessage` (surfaced through `TxResultAlert` → `humanizeUserFacingError`) so the alert is not a raw wasm stack trace.
5. **Pool-only swap gas (GitLab #134):** CW20 `send` → `swap` and top-level `swap` use the same buffered gas as a one-hop `execute_swap_operations` (**830k** wanted at default constants), not the legacy **600k** limit that caused `out of gas` after wallets showed a reduced fee (~23 LUNC vs ~36 LUNC). Station LocalTerra post-sign guards reject fee/gas rewrites below **95%** of the dApp envelope ([#127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127)).
6. **Route preview (GitLab #158):** The retail Swap page shows **at most one** token path in the **trade summary** card (same row family as **Price impact** and **Min received**). The path is **execution-aligned**: when the active quote includes indexer-shaped `router_operations` used for submit, that path wins over the client `findRoute` BFS so users never see two identical arrows with different labels (`Route (indexer)` vs `Route`). Implementation: [`swapRouteDisplay.ts`](../../frontend-dapp/src/utils/swapRouteDisplay.ts) `computeSwapRouteDisplay`. Agent checklist: [`skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](../skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md).

## Agent / third-party notes

- Changing **slippage** invalidates the simulation query (it is part of the React Query key) so preflight comparisons stay aligned with the submitted `max_spread`.
- If you change how indexer vs client routes are chosen for **submit**, update `computeSwapRouteDisplay` so the **Route** row still matches the tx path (GitLab #158).
- If you add a new swap entry path that uses the **router** but skips `preflightSwapRouteSpread`, multihop quotes may again show **0%** price impact while the chain rejects the trade — keep paths consistent with #134.
- Gas buffers for router swaps and pool-only CW20 swaps: [`docs/frontend.md`](./frontend.md) and [`skills/AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md) (GitLab #134: **830k** pool-only path, **95%** LocalTerra signed-fee/gas guard).
- MEV / mempool posture is **disclosure-only** on Swap Settings; slippage remains the executable protection. See [`docs/frontend.md#swap-mev-posture`](./frontend.md#swap-mev-posture) and [`skills/AGENTS_FRONTEND_MEV_POSTURE.md`](../skills/AGENTS_FRONTEND_MEV_POSTURE.md) (GitLab #168).
