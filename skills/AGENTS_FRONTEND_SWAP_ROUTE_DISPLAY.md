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
- **Indexer route path reconciliation (#450 / SEC-I02 H09):** When indexer `intermediate_tokens` disagrees with `router_operations`, the route row is updated to the ops-derived path and **`swap-route-intermediate-reconciled`** warns the user. Submit still uses indexer ops (not client BFS or pool-only fallback). Helper: `reconcileSwapRouteIntermediateTokens` in [`swapRouteDisplay.ts`](../frontend-dapp/src/utils/swapRouteDisplay.ts).
- **Trade pool-only / hybrid-off:** With hybrid disabled or when the quote is pool-only (no indexer `router_operations`), `computeSwapRouteDisplay` still returns a direct `from → to` line via `isDirect: true` — the row renders whenever the market quote card is visible and `marketRouteLine` is non-null.

## Code map

| Concern | Location |
|--------|----------|
| Route string + precedence | [`frontend-dapp/src/utils/swapRouteDisplay.ts`](../frontend-dapp/src/utils/swapRouteDisplay.ts) — `computeSwapRouteDisplay` |
| Intermediate path cross-check (#450 / SEC-I02 H09) | Same file — `swapRouteIntermediateTokensAligned`, `reconcileSwapRouteIntermediateTokens`, `SWAP_ROUTE_INTERMEDIATE_RECONCILED_COPY` |
| Submit route source + client fallback copy | Same file — `deriveSwapSubmitRouteSource`, `SWAP_CLIENT_BFS_FALLBACK_COPY` |
| Unit tests | [`frontend-dapp/src/utils/swapRouteDisplay.test.ts`](../frontend-dapp/src/utils/swapRouteDisplay.test.ts), [`frontend-dapp/src/pages/SwapPage.test.tsx`](../frontend-dapp/src/pages/SwapPage.test.tsx) (`swap-route-source-client-fallback`, stale-submit **#356** `Calculating…` gate), [`frontend-dapp/src/utils/quoteDebounce.test.ts`](../frontend-dapp/src/utils/quoteDebounce.test.ts), [`frontend-dapp/src/hooks/useSubmitAlignedSimQuote.test.ts`](../frontend-dapp/src/hooks/useSubmitAlignedSimQuote.test.ts) (**#356**) |
| **On-chain route alignment (SEC-E07 / #428)** | [`frontend-dapp/e2e/swap-route-alignment-tx.spec.ts`](../frontend-dapp/e2e/swap-route-alignment-tx.spec.ts) — Playwright `e2e-tx`: direct pair + multihop; compares `swap-route-summary` symbols to wasm `swap` `offer_asset`/`ask_asset` hops via [`route-alignment-e2e.ts`](../frontend-dapp/e2e/helpers/route-alignment-e2e.ts) |
| **Swap** route row | [`frontend-dapp/src/pages/SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) — `data-testid="swap-route-summary"` (trade summary grid) |
| **Swap** pre-sign summary (#409 / SEC-D11) | [`SwapPreSubmitSummary.tsx`](../frontend-dapp/src/components/swap/SwapPreSubmitSummary.tsx) in SwapPage — `swap-pre-submit-summary`, `swap-confirm-*` |
| **Swap** client BFS fallback label | Same file — under route row: `data-testid="swap-route-source-client-fallback"` when submit uses client multihop without indexer ops |
| **Swap** indexer route reconciliation (#450) | Same file — `data-testid="swap-route-intermediate-reconciled"` when `intermediate_tokens` disagreed with `router_operations` |
| **Swap** direct-hybrid amount reconciliation (#471) | [`directHybridQuote.ts`](../frontend-dapp/src/utils/directHybridQuote.ts) — wallet `hybrid_simulation` receive + slippage; `data-testid="swap-direct-hybrid-amount-reconciled"` when indexer `estimated_amount_out` disagreed; Trade market: `trade-market-amount-reconciled` |
| **Swap** submit (must stay in sync with display) | Same file — `swapMutation` prefers `indexerOperations`, then direct pair, then client multihop `route`; `deriveSwapSubmitRouteSource` mirrors those branches; pay amount and quote fields from `useSubmitAlignedSimQuote` (**#356**) |
| **Swap Settings — retail vs Advanced (#413)** | [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) — default panel: slippage, transaction deadline, Expert Mode (`#swap-slippage-settings`). Integrator controls (hybrid book leg, indexer route check) live in collapsible **Advanced** via [`SwapAdvancedSettings.tsx`](../frontend-dapp/src/components/swap/SwapAdvancedSettings.tsx); collapsed by default; expand persisted in `localStorage` ([`swapSettingsAdvanced.ts`](../frontend-dapp/src/utils/swapSettingsAdvanced.ts)). `data-testid`s: `swap-advanced-settings`, `swap-advanced-settings-toggle`, `swap-expert-mode-toggle`. Default slippage **5%** + presets: [`AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md`](./AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md) (#497). |
| **Trade market** route row | [`frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) — `data-testid="trade-market-route-summary"` inside `data-testid="trade-market-quote"` |
| **Trade market** quote source | Same file — `simQuery` calls [`quoteDirectHybridSwap`](../frontend-dapp/src/utils/directHybridQuote.ts) when hybrid on (`useHybridBook` + `willSubmitHybrid`); indexer `POST /route/solve` then LCD `hybrid_simulation` — **no pool-only fallback** when submit is hybrid ([#418](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/418)) |
| **Swap direct hybrid quote ([#418](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/418))** | [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) — `useHybridBook` default **on**; manual book leg uses same `quoteDirectHybridSwap` helper; removed `receiveQuoteIsPoolOnlyWithConfiguredBookLeg` mismatch UI |
| **Shared hybrid quote helper** | [`directHybridQuote.ts`](../frontend-dapp/src/utils/directHybridQuote.ts) — `quoteDirectHybridSwap`, `quoteDisclosureForIndexerKind`, `DIRECT_HYBRID_AMOUNT_RECONCILED_COPY` |
| **Quote debounce ([#346](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/346))** | Swap + Trade market sim queries debounce pay amount and hybrid book leg (**350ms**, `useDebouncedValue`) and use `placeholderData: keepPreviousData` — see [`quoteDebounce.ts`](../frontend-dapp/src/utils/quoteDebounce.ts) |
| **Sim refetch / Calculating hang ([#484](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/484))** | Use [`simQuoteRefetchInterval`](../frontend-dapp/src/utils/quoteDebounce.ts) (skip while `fetchStatus === 'fetching'`). Swap receive uses [`shouldShowSimReceiveCalculating`](../frontend-dapp/src/utils/quoteDebounce.ts) so background refetch keeps the prior amount. Indexer `getRouteSolve` uses a longer timeout + React Query `signal`. Full checklist: [`AGENTS_FRONTEND_SWAP_QUOTE_REFETCH.md`](./AGENTS_FRONTEND_SWAP_QUOTE_REFETCH.md). |
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

- **Sim quote refetch / Calculating hang (#484):** [`AGENTS_FRONTEND_SWAP_QUOTE_REFETCH.md`](./AGENTS_FRONTEND_SWAP_QUOTE_REFETCH.md).
- **Hybrid book leg amount input (no raw `BigInt` errors):** [GitLab **#169**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/169), [`AGENTS_FRONTEND_DECIMAL_AMOUNT_INPUT.md`](./AGENTS_FRONTEND_DECIMAL_AMOUNT_INPUT.md).
- [`docs/swap-max-spread-ux.md`](../docs/swap-max-spread-ux.md) — frontend invariant **#6** (route preview) and price-impact context (**#134**).
- [`docs/indexer-invariants.md`](../docs/indexer-invariants.md#frontend-expectations-read-path) — indexer `router_operations` vs LCD spread preflight; GET `/route/solve` hybrid default ([GitLab **#191**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/191)).
- [`docs/security-model.md`](../docs/security-model.md#off-chain-trust-boundaries-frontend) — indexer MITM / compromised route risks ([#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378)).
- [`docs/frontend.md`](../docs/frontend.md#swap-page-integration) — Swap page integration.
- [`docs/frontend.md`](../docs/frontend.md#trade-page-market-context) — Trade market ticket + route preview (**#302**).
- [`docs/frontend.md`](../docs/frontend.md#submit-quote-alignment--calculating-ux) — submit–quote alignment + Calculating UX (#356, #360, #484).

## Related

- Anti-cognitive-overload retail copy + Swap vs **Best Trade** disambiguation: [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) ([#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489)); glossary: [`docs/design-system.md`](../docs/design-system.md#terminology-glossary)
- Sim quote refetch / Calculating hang: [`AGENTS_FRONTEND_SWAP_QUOTE_REFETCH.md`](./AGENTS_FRONTEND_SWAP_QUOTE_REFETCH.md) ([#484](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/484))

## Regression checklist (manual)

### Swap (`/`)

1. Pick a CW20 pair that quotes via **indexer hybrid** with **≥ 2 hops** where the client BFS also finds a path.
2. Confirm **exactly one** **Route** row in the trade summary (`swap-route-summary`; no `Route (indexer)` duplicate, no second route card above the grid).
3. Confirm **no** “Quote source:” strip on the main page (execution/hybrid callouts in `swap-execution-summary` may still appear when relevant).
4. **Indexer multihop:** With indexer `router_operations` (≥2 hops), confirm **no** `swap-route-source-client-fallback` label.
5. **Client BFS fallback:** Stop indexer or force a path with client multihop only → confirm `swap-route-source-client-fallback` appears under the route row; copy mentions client graph / not best execution.
6. **Indexer route reconciliation (#450):** Tamper `intermediate_tokens` in a mocked indexer response (or use a compromised indexer in staging) so they disagree with `router_operations` → confirm route row shows the ops path and `swap-route-intermediate-reconciled` warning appears; submit must **not** fall back to client BFS.
6b. **Direct-hybrid amount reconciliation (#471):** Direct swap with book leg &gt; 0 — tamper indexer `estimated_amount_out` vs wallet `hybrid_simulation` → receive line shows wallet amount and `swap-direct-hybrid-amount-reconciled` warning; Trade market: `trade-market-amount-reconciled`.
7. Submit a small test swap (localnet) and confirm the on-chain path matches the displayed symbols (same hop count / ends).
7. **Hybrid quote = execute (#418):** Direct swap with Settings → **Advanced** → book leg &gt; 0 — receive line must **not** show pool-only mismatch copy; quote uses indexer or LCD `hybrid_simulation` with the same `book_input` as submit.
8. **Settings progressive disclosure (#413):** Open Settings on `/` — retail panel shows slippage, deadline, Expert Mode only (no hop addresses, no indexer BFS). Expand **Advanced** → limit book leg + **Compare indexer route** appear; re-open Settings on refresh when `localStorage` has advanced open.

### Trade market (`/trade/:pairAddr` → Market tab)

8. Open `/trade/<pairAddr>` → **Market** tab → **Use hybrid book + pool routing** is on by default → enter amount `1` → confirm **`trade-market-route-summary`** is visible with a **Route** label inside **`trade-market-quote`**.
8. **Multihop:** On a pair where indexer returns **≥ 2 hops** (e.g. EMBER→COBALT via `POST /route/solve`), confirm the route shows **≥ 3** token symbols and matches a small on-chain market submit hop count.
9. **Pool-only:** Hybrid off, amount set → quote card shows; route line is direct `PAY → RECEIVE` (or row absent only when `marketRouteLine` is null per table above).
10. Confirm **no** duplicate route labels on the market quote card (single **Route** row only; no client BFS fallback label — trade market does not submit via client BFS today).
11. **Hybrid quote = execute (#418):** Hybrid on (default), amount set — **no** “pool-only quote / hybrid execution” warning; `quoteDisclosure` reflects hybrid or pool-only path honestly.

### Submit–quote alignment (GitLab #356)

12. **Swap debounce skew:** Type `1`, wait for quote, append `0` quickly (`10`) → Swap stays **Calculating…** / disabled until quote refreshes for `10`; only then re-enables.
13. **Swap hybrid book skew (#360):** Settings → **Advanced** → enable limit book leg, pay `10`, book `2`, wait for quote, change book to `5` → Swap stays **Calculating…** / disabled until debounced book quote settles.
14. **Swap on-chain match:** With settled quote at amount A, submit → on-chain pay matches displayed quote amount (tx / balance).
15. **Trade market:** Repeat (12–13) on `/trade/:pairAddr` Market tab with hybrid on.
16. **Refetch guard (#356 + #484):** With stable amount, during background sim refetch (`simQuery.isFetching`) → submit disabled until fetch completes. Receive field keeps the prior amount (`shouldShowSimReceiveCalculating`); must **not** pulse Calculating forever. Interval must use `simQuoteRefetchInterval` (no cancel/restart while still fetching).
17. **Max makers (#360):** With stable pay/book, change max maker fills → submit disabled until new sim settles.
18. **Slow multihop (#484):** CW20 path with expensive indexer solve — quote settles or shows **Quote unavailable**; never infinite Calculating from overlapping 10s refetches.

### On-chain route alignment (SEC-E07 / GitLab #428)

19. **Automated:** `bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/swap-route-alignment-tx.spec.ts --project=e2e-tx` — direct dual-CW20 swap (1 wasm hop) and multihop CORAL→IRON (≥2 hops); asserts UI route symbols match tx `offer_asset`/`ask_asset` sequence; no duplicate segments.
20. **Manual (optional):** After a multihop quote on `/`, note `swap-route-summary` tokens, submit, and compare LCD tx wasm `swap` events — same hop count and symbols.

## Related

- Anti-cognitive-overload retail copy (Swap vs charts **Best Trade**): [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) ([#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489)); glossary: [`docs/design-system.md`](../docs/design-system.md#terminology-glossary)
- Sim quote refetch / Calculating hang: [`AGENTS_FRONTEND_SWAP_QUOTE_REFETCH.md`](./AGENTS_FRONTEND_SWAP_QUOTE_REFETCH.md) ([#484](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/484))

## Closed scope (GitLab #302 / #329)

- **Client BFS fallback indicator (Swap — #329):** When submit uses client multihop `findRoute` without indexer `router_operations`, **`swap-route-source-client-fallback`** appears under **`swap-route-summary`**. **Trade market** passes `clientRoute: null` — indexer/pool-only direct line only; no client-BFS fallback label there unless product extends hybrid-off multihop.
- **Explainability (descoped for #302):** Token arrow only; no separate "best route" rationale beyond existing hybrid execution callouts and the client-BFS warning copy above.
