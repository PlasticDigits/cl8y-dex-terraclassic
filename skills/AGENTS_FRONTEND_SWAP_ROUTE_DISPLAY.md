# Swap & trade market — single route row (retail UI)

Use this skill when changing **Swap** (`/`) or **Trade market** (`/trade/:pairAddr` → **Market** tab) quote UX, **route preview** copy, or anything that could show **two** token paths for the same trade (GitLab [**#158**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/158), [**#302**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/302)).

Both surfaces share [`computeSwapRouteDisplay`](../frontend-dapp/src/utils/swapRouteDisplay.ts) — one helper, one **Route** row per quote card, execution-aligned with submit on each page.

## Product invariant

- The dApp shows **one** human-readable **Route** (`TOKEN_A → TOKEN_B → …`) per active quote — no standalone “quote source” strip and no paired `Route (indexer)` / `Route` labels.
- **Swap** (`/`): the row lives in the **trade summary** card alongside **Price impact** and **Min received** (`data-testid="swap-route-summary"`).
- **Trade market** (`/trade`): the row lives inside the market quote card (`data-testid="trade-market-quote"`) when `marketRouteLine` is truthy (`data-testid="trade-market-route-summary"`). GitLab **#302** — mirrors swap layout.
- **Execution-aligned path:** Display must match submit on each surface:
  - **Swap:** `SwapPage` `swapMutation` prefers `indexerOperations`, then direct pair, then client multihop `route`.
  - **Trade market:** `TradeMarketOrderPanel` `swapMutation` uses `indexerOperations` via `swapOpsRequireRouter` → `executeMultiHopSwap`, else pair `swap` with hybrid params.
- **Indexer op precedence (#158):** When `indexerOperations` is non-empty, that path wins over any client BFS graph (Swap passes `clientRoute`; Trade passes `clientRoute: null` because the ticket is always a direct pair context).
- **Trade pool-only / hybrid-off:** With hybrid disabled or when the quote is pool-only (no indexer `router_operations`), `computeSwapRouteDisplay` still returns a direct `from → to` line via `isDirect: true` — the row renders whenever the market quote card is visible and `marketRouteLine` is non-null.

## Code map

| Concern | Location |
|--------|----------|
| Route string + precedence | [`frontend-dapp/src/utils/swapRouteDisplay.ts`](../frontend-dapp/src/utils/swapRouteDisplay.ts) — `computeSwapRouteDisplay` |
| Unit tests | [`frontend-dapp/src/utils/swapRouteDisplay.test.ts`](../frontend-dapp/src/utils/swapRouteDisplay.test.ts) |
| **Swap** layout (trade summary grid) | [`frontend-dapp/src/pages/SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) — `data-testid="swap-route-summary"` |
| **Swap** submit (must stay in sync with display) | Same file — `swapMutation` prefers `indexerOperations`, then direct pair, then client multihop `route` |
| **Trade market** layout (quote card) | [`frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) — `data-testid="trade-market-route-summary"` inside `trade-market-quote` |
| **Trade market** quote source | Same file — `simQuery` calls `postRouteSolve` when hybrid on (`useHybridBook` + `willSubmitHybrid`); sets `indexerOperations` from `router_operations` |
| **Trade market** submit (must stay in sync with display) | Same file — `swapMutation` → `swapOpsRequireRouter` / `executeMultiHopSwap` or pair `swap` with hybrid |

## When the trade market route row appears

| Condition | Route line |
|-----------|------------|
| Amount &gt; 0, quote loaded (`simQuery.data`) | Row may render (parent `trade-market-quote` visible) |
| Hybrid on + indexer returns `router_operations` (≥2 hops) | Multihop path (≥3 symbols), same precedence as Swap |
| Hybrid off or pool-only quote (no `indexerOperations`) | Direct `PAY → RECEIVE` via `isDirect` branch |
| `marketRouteLine` is `null` | Row hidden (no `trade-market-route-summary` in DOM) — e.g. missing from/to tokens |

Hybrid / L8 quoting detail: [`docs/swap-max-spread-ux.md`](../docs/swap-max-spread-ux.md), [`docs/indexer-invariants.md`](../docs/indexer-invariants.md#frontend-expectations-read-path). Indexer outage copy: [`AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md`](./AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md).

## Docs cross-links

- **Hybrid book leg amount input (no raw `BigInt` errors):** [GitLab **#169**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/169), [`AGENTS_FRONTEND_DECIMAL_AMOUNT_INPUT.md`](./AGENTS_FRONTEND_DECIMAL_AMOUNT_INPUT.md).
- [`docs/swap-max-spread-ux.md`](../docs/swap-max-spread-ux.md) — frontend invariant **#6** (route preview) and price-impact context (**#134**).
- [`docs/indexer-invariants.md`](../docs/indexer-invariants.md#frontend-expectations-read-path) — indexer `router_operations` vs LCD spread preflight; GET `/route/solve` hybrid default ([GitLab **#191**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/191)).
- [`docs/frontend.md`](../docs/frontend.md#swap-page-integration) — Swap page integration.
- [`docs/frontend.md`](../docs/frontend.md#trade-page-market-context) — Trade market ticket + route preview (**#302**).

## Regression checklist (manual)

### Swap (`/`)

1. Pick a CW20 pair that quotes via **indexer hybrid** with **≥ 2 hops** where the client BFS also finds a path.
2. Confirm **exactly one** **Route** row in the trade summary (no `Route (indexer)` duplicate, no second route card above the grid).
3. Confirm **no** “Quote source:” strip on the main page (execution/hybrid callouts in `swap-execution-summary` may still appear when relevant).
4. Submit a small test swap (localnet) and confirm the on-chain path matches the displayed symbols (same hop count / ends).

### Trade market (`/trade/:pairAddr` → Market tab)

5. Open `/trade/<pairAddr>` → **Market** tab → enable **Use hybrid book + pool routing** → enter amount `1` → confirm `trade-market-route-summary` is visible with a **Route** label inside `trade-market-quote`.
6. **Multihop:** On a pair where indexer returns **≥ 2 hops** (e.g. EMBER→COBALT via `POST /route/solve`), confirm the route shows **≥ 3** token symbols and matches a small on-chain market submit hop count.
7. **Pool-only:** Hybrid off, amount set → quote card shows; route line is direct `PAY → RECEIVE` (or row absent only when `marketRouteLine` is null per table above).
8. Confirm **no** duplicate route labels on the market quote card (single **Route** row only).

## Open scope (GitLab #302 — verification)

- **Client BFS fallback indicator (not implemented):** On **Swap**, when indexer `route/solve` is unavailable and the active quote falls back to client `findRoute`, the **Route** row still renders but there is **no** user-facing label that the path source changed (grep: no `client fallback` / route-source copy in `SwapPage` or `TradeMarketOrderPanel`). **Trade market** passes `clientRoute: null` into `computeSwapRouteDisplay` — indexer/pool-only direct line only; client-BFS fallback is a Swap concern unless product extends hybrid-off multihop there.
- **Explainability:** Token arrow only; no “best route” rationale beyond existing hybrid execution callouts.
- Close **#302** only when route display on `/trade` **and** fallback-indicator acceptance are both met or explicitly descoped on the issue.
