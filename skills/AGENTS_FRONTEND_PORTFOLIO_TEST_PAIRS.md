# Agent playbook: hide test-gem performance on `/portfolio` (GitLab #674)

Audience: third-party agents touching `/portfolio` Open Positions, header realized P&L, Recent activity, or the #534 gem classifier.

**Issue:** [GitLab **#674**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/674)  
**Invariants:** [`docs/frontend.md` § My Portfolio](../docs/frontend.md#my-portfolio) (**P674-1–P674-8**); [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **Portfolio hide test-gem performance #674**)  
**Related:** [#534](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534) economic/test classifier, [#551](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/551) / [#560](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/560) P&L scale + USD, [#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562) production discovery hide

## Problem class

Soft-launch gems (EMBER, CORAL, JADE, TOPAZ, ONYX, RUBY, PEARL, QUARTZ, plus LocalTerra extras) have no economic meaning. `#534` ranked them last in pickers and `#562` omitted them from production discovery, but `/portfolio` still listed EMBER/CORAL net position, avg entry, cost basis, and realized P&L as if they were real performance. That clutters the page and can be read as live P&L.

## Invariants (P674-1–P674-8)

| ID | Rule |
|----|------|
| **P674-1** | **One classifier.** Reuse `isTestPair` / `isGemTokenId` / `GEM_SYMBOLS` + `COLUMBUS5_GEM_ADDRESSES` from [`pairCatalogRank.ts`](../frontend-dapp/src/utils/pairCatalogRank.ts). Do **not** add a second gem list. Address in the gem set wins over a spoofed `symbol=UST1` (**X1**). |
| **P674-2** | **Default hide.** `/portfolio` Open Positions, header realized P&L USD (sum of the **visible** positions), and Recent activity omit test-gem pairs. Economic assets stay (CL8Y, UST1, USTR, cLUNC, cUSTC, vFDUSD, SpaceUSD, …). |
| **P674-3** | **Toggle.** Offer **Show test pairs** when any hidden test position or recent-activity trade exists. Default **off**. Not `?showGems=` / not `VITE_SHOW_TEST_TOKENS` (that flag is discovery, **P562-1**). |
| **P674-4** | **Reveal.** Toggle on: economic rows first, then a **Test pairs** divider, then gems. Header P&L USD includes gem positions while they are shown. |
| **P674-5** | **Scope.** `/trader` public profile still lists every pair. Open limits and LP overview stay unfiltered (cancel / on-chain balances). Indexer JSON is unchanged. |
| **P674-6** | **Hubs stay economic.** UST1 / CL8Y / wrap / vFDUSD are never gems (**U6** / **P534-8**). Missing position metadata (`symbol=—`, no denom) is **not** a gem — keep the row. |
| **P674-7** | **Volume header.** `traders.total_volume_usd` stays the indexer-wide wallet figure. Do **not** invent a client subtract for gem volume. Fees / best / worst stay **—** (**P551-5**). |
| **P674-8** | **No lecture.** No always-on “test tokens removed” banner. Toggle copy is short. Overlay lives in the dApp ([`portfolioPerformanceFilter.ts`](../frontend-dapp/src/utils/portfolioPerformanceFilter.ts)). |

## Do / don’t

- **Do** filter through [`visiblePortfolioPositions`](../frontend-dapp/src/utils/portfolioPerformanceFilter.ts) / `visiblePortfolioTrades` and pass the same visible list into `TraderSummaryStats`.
- **Do** keep the hatch so QA can still see gem activity (mirrors #534 **Test pairs**, not #562 production omit).
- **Don’t** hide gems on `/trader` to “match portfolio.” Public profiles stay complete (**P674-5**).
- **Don’t** filter LP or open limits. **Don’t** burn, factory-delete, or wipe indexer gem rows.
- **Don’t** treat `#562` `retailExposeTestTokens()` as the portfolio default — LocalTerra must still hide gems on `/portfolio` until the toggle is on.

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/utils/portfolioPerformanceFilter.ts` | Position / trade classify + default hide |
| `frontend-dapp/src/utils/pairCatalogRank.ts` | Shared gem set (`isTestPair`) |
| `frontend-dapp/src/pages/PortfolioPage.tsx` | Toggle state; visible lists into summary / table / tape |
| `frontend-dapp/src/components/portfolio/PortfolioShowTestPairsToggle.tsx` | Short checkbox |
| `frontend-dapp/src/components/trader/TraderPositionsTable.tsx` | Optional header action + **Test pairs** divider |

## Regression

```bash
make verify-issue-674
```

Vitest: `portfolioPerformanceFilter.test.ts`, `PortfolioPage.test.tsx`, `TraderPositionsTable.test.tsx`. Playwright `e2e/portfolio.spec.ts` when LocalTerra is up (toggle absent on a no-gem wallet). Stacked P&L: `make verify-issue-551` · `make verify-issue-560`.

## Related

- [`AGENTS_FRONTEND_PORTFOLIO.md`](./AGENTS_FRONTEND_PORTFOLIO.md) — portfolio shell / APIs
- [`AGENTS_FRONTEND_PORTFOLIO_PNL.md`](./AGENTS_FRONTEND_PORTFOLIO_PNL.md) — human-scale P&L (**P551**)
- [`AGENTS_FRONTEND_HUB_PNL.md`](./AGENTS_FRONTEND_HUB_PNL.md) — header USD from hub_prices (**P560**)
- [`AGENTS_FRONTEND_PAIR_CATALOG_RANK.md`](./AGENTS_FRONTEND_PAIR_CATALOG_RANK.md) — empty-browse rank + **Test pairs**
- [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md) — production discovery hide (**P562**); portfolio performance is this ticket
