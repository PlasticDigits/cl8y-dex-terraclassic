# Agent skill: retail hybrid always-on (GitLab #596)

Use when changing **Swap** (`/`) or **Trade market** (`/trade/:pairAddr` → **Market**) quote/submit, Advanced book-leg controls, or copy that could re-introduce a pool-only opt-out.

Follow-on to **#501** (GET `/route/solve` as the default quote). **#596** is the product rule: the official dApp **must always** consider resting limit orders. Users must not opt in, and must not opt out, to receive best pricing.

Issue **#596 is implemented**. Indexer `GET /api/v1/route/solve?pool_only=true` remains for **integrators** and custom frontends — not a retail control.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#596**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/596) | Always-on book leg in the official dApp |
| [GitLab **#501**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/501) | Trade market + Swap share GET `/route/solve` |
| [`docs/limit-orders.md` § hybrid vs pool-only](../docs/limit-orders.md#swap-ui-hybrid-vs-pool-only-estimates) | Product invariants |
| [`docs/adr/0002-global-best-execution-route-solver.md`](../docs/adr/0002-global-best-execution-route-solver.md) | Solver contract + #596 amendment |
| [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) | Retail GET + no dApp `pool_only` |
| [`SwapAdvancedSettings.tsx`](../frontend-dapp/src/components/swap/SwapAdvancedSettings.tsx) | Book override only (no hybrid checkbox) |
| [`TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) | Same: Advanced override, no toggle |
| [`quoteCw20ViaRouteSolve`](../frontend-dapp/src/utils/cw20RouteSolveQuote.ts) | Shared GET quote helper |
| [`AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md) | L8 quote = execute |
| [`AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](./AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md) | Route row; no hybrid-off path |
| [`AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md`](./AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md) | Integrator `pool_only=true` |

## Invariants **H596-1–H596-8**

1. **H596-1 — no retail opt-out.** Swap Settings and Trade market Advanced must **not** expose a hybrid on/off checkbox (`useHybridBook` / `trade-market-hybrid-toggle`). Best execution is not a preference.
2. **H596-2 — default quote is GET.** Empty manual book → [`quoteCw20ViaRouteSolve`](../frontend-dapp/src/utils/cw20RouteSolveQuote.ts) (`GET /api/v1/route/solve`). Empty book ≠ skip the book: the solver may still allocate an interior `book_input`.
3. **H596-3 — Advanced is override only.** A typed positive book amount uses [`quoteDirectHybridSwap`](../frontend-dapp/src/utils/directHybridQuote.ts) (`POST` `hybrid_by_hop`). That is a **split** override, not a way to disable the book.
4. **H596-4 — pool-only is integrator-only.** Official dApp must not send `pool_only=true`. Integrators who want v2-LP-only quotes use indexer GET `pool_only=true` or their own frontend.
5. **H596-5 — degraded fallback is not opt-out.** If indexer GET fails, LCD `simulateSwap` / pool-only hybrid may still quote so the ticket is not stuck. That is transport degradation, not a user control.
6. **H596-6 — quote = execute.** Submit uses solver `hybrid` from `indexerOperations` (`hybridFromSingleHopIndexerOps`) or the Advanced override split. Do not show a pool-only receive line while a manual book leg is configured (**#418**).
7. **H596-7 — wrap / native unchanged.** Direct wrap/unwrap and native denom routes are not CW20 Pattern C hybrid. Do not force a book leg there. USTR→USTC gas is the unwrap+≥2hop combo ([#599](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/599)), not hybrid 15M — see [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md). Post-merge E9 / columbus-5: [#600](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/600) / `make verify-issue-600`.
8. **H596-8 — gas assumes hybrid.** Trade market `estimateMarketPairSwapSequenceUlunaFeesTotal(true, …)` / `marketUsesHybrid={true}`. Do not reserve pool-only gas for the retail market ticket.

Community tax buy/sell **does** apply on router hops after the #607 option 2 wasm (**T592-13**). Do **not** turn hybrid off or force pair-only execute — tax is classify-side, not a hybrid opt-out. Live **11611** is Honest hops until migrate. Playbook: [`AGENTS_COMMUNITY_TAX_ROUTER.md`](./AGENTS_COMMUNITY_TAX_ROUTER.md). LocalTerra tax-pair Playwright must keep hybrid on: [`AGENTS_E2E_COMMUNITY_TAX_TX.md`](./AGENTS_E2E_COMMUNITY_TAX_TX.md) (**E622-7**, [#622](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/622)).

## Rules of thumb

- Do **not** restore `useHybridBook` state or a “Best execution” checkbox.
- Empty Advanced book field must stay **silent** (**#492**) — no “add a book leg” copy.
- Metric for the product outcome: indexer `book_leg_volume` vs `pool_leg_volume` on venue swaps (issue notes). That is observational, not a unit test.
- Cognitive load: Advanced copy should be one short line (override split), not a lecture on AMM vs book.

## Verify

```bash
make verify-issue-596
# Optional chain: VERIFY_ISSUE_596_CHAIN=1 make verify-issue-596
```
