# Agent playbook: Charts UST1/USD hero + `?price=` (GitLab #680)

Audience: third-party agents touching `/charts` default pair, Price (USD) orientation, 24h USD OHLC / %, TWAP headings, or `?price=` query parsing.

**Issue:** [GitLab **#680**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/680)  
**Invariants:** [`docs/frontend.md` § Charts UST1/USD hero](../docs/frontend.md#charts-ust1-usd-hero) (**C680-1–C680-9**; issue draft labels **C674-1–C674-9** map 1:1)  
**Related:** [#524](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/524) Trade other-side default, [#543](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/543) `invertUsd`, [#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547) deep links, [#564](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/564) / [#565](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/565) pair stats, [#666](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/666) pair-scoped layout, [#508](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/508) **U1**.

## Problem class

Charts is a **UST1/USD** product surface. Bare `/charts` must open UST1/cUSTC (retail “UST1/USTC”) priced in **USD of 1 UST1**, not catalog-volume rank and not Trade’s T524-3 other-side default. `?price=` names the priced token; **every price-facing tile** follows it. Volume is not a price.

## Invariants (C680-1–C680-9)

| ID | Rule |
|----|------|
| **C680-1** | Bare `/charts` (no pair segment) selects UST1/cUSTC when listed and replace-navigates to `/charts/{addr}?price=UST1`. Last + candles are USD of 1 UST1 (`~$1` class). |
| **C680-2** | `/charts/{ust1custc}` without `price` uses the **Charts** product default (UST1 USD / not inverted), not T524-3. |
| **C680-3** | `?price=cUSTC` / `USTC` / quote-leg contract on that pair shows cUSTC USD on Last, candles, 24h USD OHLC, Price Change, TWAP, headings, tape Price. Pill matches. |
| **C680-4** | Invert pill updates the URL (`replace`) and all C680-3 rows. A second click / `?price=UST1` restores UST1 USD. |
| **C680-5** | Deep link to another listed pair is kept. `?price=` that is not a leg of **that** pair is ignored. Invalid `pairAddr` still shows the existing notice; no stats/leaderboard fetch with the hostile string (**CS-11**). |
| **C680-6** | Volume USD and factory token volumes do **not** change with invert (values or decimals). |
| **C680-7** | `/trade` UST1/cUSTC first visit still defaults other-side (**T524-3** / **C543-3**). Charts uses `cl8y-dex-charts-pair-invert:` — never the Trade key. Convert-on-submit unchanged. |
| **C680-8** | Missing hero pair → first economic catalog pair (or mainnet columbus-5 pin). No spinner lock. Production still hides gems (**P562**). |
| **C680-9** | No `1/x` of factory USD; no CoinGecko stitch; no U1 mint copy; chrome nesting still green. |

## Do / don’t

- **Do** invert USD with `invertUsd` (`price_usd / human`) per value. Recompute display % from inverted open/close. Swap High/Low after invert so high ≥ low.
- **Do** reciprocal **human** TWAP when inverted (`invertFinitePositive`). TWAP is still not USD.
- **Do** allowlist `?price=` to the two legs of the **selected** pair (symbol, casefold, or leg contract). Aliases `USTC`→cUSTC and `LUNC`→cLUNC only when that wrap leg is on the pair. Repeated `price` keys: **last** wins. Hostile / overlong → ignore.
- **Do** keep Trade on `usePairDisplayOrientation` + `cl8y-dex-trade-pair-invert:`.
- **Don’t** write Charts orientation into the Trade storage key.
- **Don’t** snap a valid `/charts/:pairAddr` back to the hero.
- **Don’t** treat invert or the hero chart as mint/redeem (**U1**).
- **Don’t** change indexer factory USD, CG/CMC, or `/limits` standalone (**T524-10**).

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/pages/ChartsPage.tsx` | Hero replace-nav; `?price=` sync; display OHLC / % / TWAP / headings |
| `frontend-dapp/src/utils/chartsPairRoute.ts` | Parse/serialize `price`; bech32 href hardening |
| `frontend-dapp/src/utils/tradePairDisplayOrientation.ts` | Charts default + Charts session + param↔invert |
| `frontend-dapp/src/hooks/usePairDisplayOrientation.ts` | `useChartsPairDisplayOrientation` (Trade hook unchanged) |
| `frontend-dapp/src/utils/pairPriceUsd.ts` | `resolveDisplayPairStatsUsdOhlc` |
| `frontend-dapp/src/utils/pairCatalogRank.ts` | `firstUst1CustcPairAddress` / `resolveChartsHeroPairAddress` |
| `frontend-dapp/src/utils/ust1SecondaryMarket.ts` | `MAINNET_UST1_CUSTC_PAIR_ADDRESS` |

## Regression

```bash
make verify-issue-680
make verify-issue-524
make verify-issue-543
```

Vitest: `chartsPairRoute.test.ts`, `tradePairDisplayOrientation.test.ts` (#680 isolation), `pairPriceUsd.test.ts`, `chartsPairStats.test.ts`, `pairCatalogRank.test.ts`, `ChartsPage.test.tsx` (#680 describe).  
Chrome: `python3 scripts/check_chrome_nesting.py`.

## Related

- [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) — Trade T524-3 stays other-side
- [`AGENTS_FRONTEND_USD_CANDLE_INVERT.md`](./AGENTS_FRONTEND_USD_CANDLE_INVERT.md) — `invertUsd` not `1/x`
- [`AGENTS_FRONTEND_CHARTS_PAIR_SCOPED.md`](./AGENTS_FRONTEND_CHARTS_PAIR_SCOPED.md) — layout / CS-11
- [`AGENTS_FRONTEND_CHARTS_PAIR_STATS.md`](./AGENTS_FRONTEND_CHARTS_PAIR_STATS.md) — volume unchanged; price tiles follow `?price=`
- [`AGENTS_UST1_SECONDARY_AMM.md`](./AGENTS_UST1_SECONDARY_AMM.md) — **U1**
