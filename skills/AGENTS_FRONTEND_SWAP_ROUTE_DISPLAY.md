# Swap & trade market — single route row (retail UI)

Use this skill when changing **Swap** (`/`) or **Trade market** (`/trade/:pairAddr` → **Market** tab) quote UX, **route preview** copy, or anything that could show **two** token paths for the same trade (GitLab [**#158**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/158), [**#302**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/302)).

Both surfaces share [`computeSwapRouteDisplay`](../frontend-dapp/src/utils/swapRouteDisplay.ts) — one helper, one **Route** row per quote card, execution-aligned with submit on each page.

## Product invariant

- The dApp shows **one** human-readable **Route** (`TOKEN_A → TOKEN_B → …`) per active quote — no standalone “quote source” strip and no paired `Route (indexer)` / `Route` labels.
- **Swap** (`/`): the row lives in the **trade summary** card alongside **Expected slippage** (route-based when indexer provides `slippage_percent` — GitLab **#293**) and **Min received** (`data-testid="swap-route-summary"`).
- **Trade market** (`/trade`): the row lives inside the market quote card (`data-testid="trade-market-quote"`) when `marketRouteLine` is truthy (`data-testid="trade-market-route-summary"`). GitLab **#302** — mirrors swap layout.
- **Execution-aligned path:** Display must match submit on each surface:
  - **Swap:** `SwapPage` `swapMutation` prefers `indexerOperations`, then direct pair, then client multihop `route`.
  - **Trade market:** `TradeMarketOrderPanel` `swapMutation` uses `indexerOperations` via `swapOpsRequireRouter` → `executeMultiHopSwap`, else pair `swap` with hybrid params.
- **Indexer op precedence (#158):** When `indexerOperations` is non-empty, that path wins over any client BFS graph (Swap passes `clientRoute`; Trade passes `clientRoute: null` because the ticket is always a direct pair context).
- **Client BFS fallback label (#329):** When Swap **submit** uses client multihop `findRoute` without indexer `router_operations` (≥2 hops), a brief warning line appears under **`swap-route-summary`**: `data-testid="swap-route-source-client-fallback"`. Not shown for indexer, direct pair, or native wrap paths. Trade market: N/A (`clientRoute: null`).
- **Trade pool-only / hybrid-off:** With hybrid disabled or when the quote is pool-only (no indexer `router_operations`), `computeSwapRouteDisplay` still returns a direct `from → to` line via `isDirect: true` — the row renders whenever the market quote card is visible and `marketRouteLine` is non-null.

## Code map

| Concern | Location |
|--------|----------|
| Route string + precedence | [`frontend-dapp/src/utils/swapRouteDisplay.ts`](../frontend-dapp/src/utils/swapRouteDisplay.ts) — `computeSwapRouteDisplay` |
| Submit route source + client fallback copy | Same file — `deriveSwapSubmitRouteSource`, `SWAP_CLIENT_BFS_FALLBACK_COPY` |
| Unit tests | [`frontend-dapp/src/utils/swapRouteDisplay.test.ts`](../frontend-dapp/src/utils/swapRouteDisplay.test.ts), [`frontend-dapp/src/pages/SwapPage.test.tsx`](../frontend-dapp/src/pages/SwapPage.test.tsx) (`swap-route-source-client-fallback`, stale-submit **#356** `Calculating…` gate), [`frontend-dapp/src/utils/quoteDebounce.test.ts`](../frontend-dapp/src/utils/quoteDebounce.test.ts), [`frontend-dapp/src/hooks/useSubmitAlignedSimQuote.test.ts`](../frontend-dapp/src/hooks/useSubmitAlignedSimQuote.test.ts) (**#356**) |
| **Swap** route row | [`frontend-dapp/src/pages/SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) — `data-testid="swap-route-summary"` (trade summary grid) |
| **Swap** client BFS fallback label | Same file — under route row: `data-testid="swap-route-source-client-fallback"` when submit uses client multihop without indexer ops |
| **Swap** submit (must stay in sync with display) | Same file — `swapMutation` prefers `indexerOperations`, then direct pair, then client multihop `route`; `deriveSwapSubmitRouteSource` mirrors those branches; pay amount and quote fields from `useSubmitAlignedSimQuote` (**#356**) |
| **Trade market** route row | [`frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) — `data-testid="trade-market-route-summary"` inside `data-testid="trade-market-quote"` |
| **Trade market** quote source | Same file — `simQuery` calls `postRouteSolve` when hybrid on (`useHybridBook` + `willSubmitHybrid`); sets `indexerOperations` from `router_operations` |
| **Quote debounce ([#346](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/346))** | Swap + Trade market sim queries debounce pay amount and hybrid book leg (**350ms**, `useDebouncedValue`) and use `placeholderData: keepPreviousData` — see [`quoteDebounce.ts`](../frontend-dapp/src/utils/quoteDebounce.ts) |
| **Submit–quote alignment ([#356](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/356), [#360](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/360))** | When submit is allowed, on-chain pay raw, `minReceived`, `indexerOperations`, hybrid params (`book_input`, `max_maker_fills`), and route display all come from one debounced snapshot via [`useSubmitAlignedSimQuote`](../frontend-dapp/src/hooks/useSubmitAlignedSimQuote.ts) + [`buildSubmitAlignedSimPayload`](../frontend-dapp/src/utils/quoteDebounce.ts). Submit stays disabled while typed raw ≠ debounced key, live book leg ≠ debounced book leg, live max makers ≠ snapshotted max makers, placeholder data is shown, or `simQuery.isFetching` for the active key. |
| **Trade market** submit (must stay in sync with display) | Same file — `swapMutation` → `swapOpsRequireRouter` / `executeMultiHopSwap` or pair `swap` with hybrid; consumes `submitPayRaw` + matching `simData` from `useSubmitAlignedSimQuote` |

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
2. Confirm **exactly one** **Route** row in the trade summary (`swap-route-summary`; no `Route (indexer)` duplicate, no second route card above the grid).
3. Confirm **no** “Quote source:” strip on the main page (execution/hybrid callouts in `swap-execution-summary` may still appear when relevant).
4. **Indexer multihop:** With indexer `router_operations` (≥2 hops), confirm **no** `swap-route-source-client-fallback` label.
5. **Client BFS fallback:** Stop indexer or force a path with client multihop only → confirm `swap-route-source-client-fallback` appears under the route row; copy mentions client graph / not best execution.
6. Submit a small test swap (localnet) and confirm the on-chain path matches the displayed symbols (same hop count / ends).

### Trade market (`/trade/:pairAddr` → Market tab)

7. Open `/trade/<pairAddr>` → **Market** tab → enable **Use hybrid book + pool routing** → enter amount `1` → confirm **`trade-market-route-summary`** is visible with a **Route** label inside **`trade-market-quote`**.
8. **Multihop:** On a pair where indexer returns **≥ 2 hops** (e.g. EMBER→COBALT via `POST /route/solve`), confirm the route shows **≥ 3** token symbols and matches a small on-chain market submit hop count.
9. **Pool-only:** Hybrid off, amount set → quote card shows; route line is direct `PAY → RECEIVE` (or row absent only when `marketRouteLine` is null per table above).
10. Confirm **no** duplicate route labels on the market quote card (single **Route** row only; no client BFS fallback label — trade market does not submit via client BFS today).

### Submit–quote alignment (GitLab #356)

11. **Swap debounce skew:** Type `1`, wait for quote, append `0` quickly (`10`) → Swap stays **Calculating…** / disabled until quote refreshes for `10`; only then re-enables.
12. **Swap hybrid book skew (#360):** Enable limit book leg, pay `10`, book `2`, wait for quote, change book to `5` → Swap stays **Calculating…** / disabled until debounced book quote settles.
13. **Swap on-chain match:** With settled quote at amount A, submit → on-chain pay matches displayed quote amount (tx / balance).
14. **Trade market:** Repeat (11–12) on `/trade/:pairAddr` Market tab with hybrid on.
15. **Refetch guard:** With stable amount, during 10s sim refetch (`simQuery.isFetching`) → submit disabled until fetch completes.
16. **Max makers (#360):** With stable pay/book, change max maker fills → submit disabled until new sim settles.

## Closed scope (GitLab #302 / #329)

- **Client BFS fallback indicator (Swap — #329):** When submit uses client multihop `findRoute` without indexer `router_operations`, **`swap-route-source-client-fallback`** appears under **`swap-route-summary`**. **Trade market** passes `clientRoute: null` — indexer/pool-only direct line only; no client-BFS fallback label there unless product extends hybrid-off multihop.
- **Explainability (descoped for #302):** Token arrow only; no separate "best route" rationale beyond existing hybrid execution callouts and the client-BFS warning copy above.
