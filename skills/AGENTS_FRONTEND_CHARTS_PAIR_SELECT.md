# Agent playbook: Charts Select Pair first change (#1266)

Audience: third-party agents touching `/charts` pair picker (`#chart-pair-select`), hero auto-nav, or `selectPair` vs `useParams().pairAddr`.

**Issue:** [#1266](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1266)  
**Invariants:** [`docs/frontend.md` § Charts Select Pair](../docs/frontend.md#charts-select-pair) (**C1266-1–C1266-8**)  
**Related:** [#680](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/680) idle UST1/cUSTC hero (**C680-1** / **C680-5**), [#666](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/666) pair-scoped stats, Trade auto-pick [#357](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/357) (closed, Trade-only — do not reopen).

## Problem class

Bare `/charts` hero replace-nav treated `selectedPairAddr !== hero` as “force hero”. A first `#chart-pair-select` change queued `navigate(/charts/{B})` while `isBareCharts` was still true; the hero effect then wrote the hero href. A second identical select stuck because the route was no longer bare. Catalog-head fallback could also snap a valid bech32 choice before `getPair` returned.

## Invariants (C1266-1–C1266-8)

| ID | Rule |
|----|------|
| **C1266-1** | First `#chart-pair-select` change on bare `/charts` to listed **B** ≠ hero is authoritative: URL, trigger, candles, 24h stats, leaderboard all use **B**. No second select. |
| **C1266-2** | Idle bare `/charts` (no user select) still auto-picks hero (**C680-1**). |
| **C1266-3** | Reload `/charts/{B}` keeps **B** (**C680-5**). Hero effect does not run. |
| **C1266-4** | After a successful select, Find/sort/page must not snap the workspace to hero or `pairOptions[0]`. |
| **C1266-5** | Carried `?price=` is dropped when it is not a leg of the newly selected pair; hostile price ignored. |
| **C1266-6** | Catalog-head fallback must not replace a valid bech32 `selectedPairAddr` while `getPair` is pending. Never replace `validRoutePair`. |
| **C1266-7** | `selectPair` / hero navigate only via `chartsPairHref` + `replace`. Non-bech32 `onChange` does not navigate or fetch. |
| **C1266-8** | No last-pair `localStorage`. Trade `PairSearchSelect` unchanged. Chrome nesting green. Production still hides gems (**P562**). |

## Do / don’t

- **Do** run #680 hero replace-nav only when idle: `isBareCharts` and no committed pair (`shouldAutoPickChartsHeroPair`). Same pattern as Trade `shouldAutoPickDefaultTradePair`.
- **Do** set a `userCommittedPair` ref inside `selectPair` so a still-bare re-render cannot clobber the first click.
- **Do** wait for `getPair` extra row when the selected bech32 is off the current pager page.
- **Don’t** treat `selectedPairAddr !== hero` on bare `/charts` as force-hero.
- **Don’t** remount `/charts/{B}` in tests to “prove” a pair switch — click `#chart-pair-select`.
- **Don’t** persist last pair. **Don’t** change Trade. **Don’t** concatenate `pairAddr` into `Navigate`.

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/pages/ChartsPage.tsx` | `selectPair` + idle hero + fallback guards |
| `frontend-dapp/src/utils/chartsPairRoute.ts` | `shouldAutoPickChartsHeroPair` / `shouldSnapChartsSelectionToCatalogHead` |
| `frontend-dapp/src/pages/ChartsPage.test.tsx` | MenuSelect click path vs hero |

## Regression

```bash
make verify-issue-1266
make verify-issue-680
python3 scripts/check_chrome_nesting.py
```

## Related

- [`AGENTS_FRONTEND_CHARTS_UST1_HERO.md`](./AGENTS_FRONTEND_CHARTS_UST1_HERO.md) — idle hero only; **C680-5** deep links
- [`AGENTS_FRONTEND_CHARTS_PAIR_SCOPED.md`](./AGENTS_FRONTEND_CHARTS_PAIR_SCOPED.md) — pair-scoped stats/leaderboard follow the selected addr
- [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md) — **P562** gems stay hidden
