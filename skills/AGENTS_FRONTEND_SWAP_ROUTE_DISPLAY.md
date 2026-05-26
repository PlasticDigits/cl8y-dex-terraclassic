# Swap page — single route row (retail UI)

Use this skill when changing **Swap** (`/`) quote UX, **route preview** copy, or anything that could show **two** token paths for the same trade (GitLab [**#158**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/158)).

## Product invariant

- The dApp shows **one** human-readable **Route** (`TOKEN_A → TOKEN_B → …`) in the **trade summary** card alongside **Price impact** and **Min received**.
- There is **no** standalone “quote source” strip on the main swap flow for multihop quotes.
- The path must be **execution-aligned** with `SwapPage`’s submit mutation: if `simQuery.data.indexerOperations` is non-empty, the UI uses the indexer path (prefer `indexerIntermediateTokens` when present, else derive from operations). Only when indexer ops are **not** used for submit may the UI fall back to the client `findRoute` graph.

## Code map

| Concern | Location |
|--------|----------|
| Route string + precedence | [`frontend-dapp/src/utils/swapRouteDisplay.ts`](../frontend-dapp/src/utils/swapRouteDisplay.ts) — `computeSwapRouteDisplay` |
| Unit tests | [`frontend-dapp/src/utils/swapRouteDisplay.test.ts`](../frontend-dapp/src/utils/swapRouteDisplay.test.ts) |
| Layout (trade summary grid) | [`frontend-dapp/src/pages/SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) — `data-testid="swap-route-summary"` |
| Submit path (must stay in sync with display) | Same file — `swapMutation` prefers `indexerOperations`, then direct pair, then client multihop `route` |

## Docs cross-links

- **Hybrid book leg amount input (no raw `BigInt` errors):** [GitLab **#169**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/169), [`AGENTS_FRONTEND_DECIMAL_AMOUNT_INPUT.md`](./AGENTS_FRONTEND_DECIMAL_AMOUNT_INPUT.md).
- [`docs/swap-max-spread-ux.md`](../docs/swap-max-spread-ux.md) — frontend invariant **#5** (route preview) and price-impact context (**#134**).
- [`docs/indexer-invariants.md`](../docs/indexer-invariants.md#frontend-expectations-read-path) — indexer `router_operations` vs LCD spread preflight; GET `/route/solve` hybrid default ([GitLab **#191**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/191)).
- [`docs/frontend.md`](../docs/frontend.md#swap-page-integration) — Swap page integration section.

## Regression checklist (manual)

1. Pick a CW20 pair that quotes via **indexer hybrid** with **≥ 2 hops** where the client BFS also finds a path.
2. Confirm **exactly one** **Route** row in the trade summary (no `Route (indexer)` duplicate, no second route card above the grid).
3. Confirm **no** “Quote source:” strip on the main page (execution/hybrid callouts in `swap-execution-summary` may still appear when relevant).
4. Submit a small test swap (localnet) and confirm the on-chain path matches the displayed symbols (same hop count / ends).
