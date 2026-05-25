# Agent playbook: Trade page invalid / unknown pair deep links

Use when changing **`/trade/:pairAddr`** routing, pair selector sync, or empty-state behavior for malformed or unknown share links ([GitLab **#176**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/176), [**#175**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/175)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § invalid pair deep link](../docs/frontend.md#trade-page-invalid-pair-link) | Charset-invalid `terra1…` invariants |
| [docs/frontend.md § unknown pair deep link](../docs/frontend.md#trade-page-unknown-pair-link) | Valid-format but not on factory ([#175](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/175)) |
| [`frontend-dapp/src/utils/tradePairRoute.ts`](../frontend-dapp/src/utils/tradePairRoute.ts) | `isTradePairRouteParam`, `getInvalidTradePairRouteParam`, `getUnknownTradePairRouteParam`, `isKnownFactoryTradePair` |
| [`InvalidPairLinkNotice.tsx`](../frontend-dapp/src/components/trade/InvalidPairLinkNotice.tsx) | Invalid charset / format |
| [`PairNotFoundLinkNotice.tsx`](../frontend-dapp/src/components/trade/PairNotFoundLinkNotice.tsx) | Pair not on factory |
| [`tradeInvalidPairLinkCopy.ts`](../frontend-dapp/src/utils/tradeInvalidPairLinkCopy.ts) / [`tradeUnknownPairLinkCopy.ts`](../frontend-dapp/src/utils/tradeUnknownPairLinkCopy.ts) | Retail strings |
| [`TradePage.tsx`](../frontend-dapp/src/pages/TradePage.tsx) | Route sync, notices, auto-pick guard |

## Invalid pair (GitLab #176)

1. **Validate with `isValidTerraAddress`** — do not gate only on `startsWith('terra1')`; short garbage like `terra1` must be treated as invalid.
2. **Never keep garbage in URL or selector** — replace to `/trade` and keep `pairAddr` empty until user selection.
3. **Do not auto-redirect to first pair** while the invalid-link notice is visible unless product explicitly changes that invariant.
4. Manual repro: `http://localhost:3000/trade/lilwayne%20babyyy` or `terra1damThat'scrazy` (charset fails regex).

## Unknown pair (GitLab #175)

1. **Factory list is the gate** — after `getAllPairsPaginated` succeeds, if `routePair` is valid-format but not in `pairs[].contract_addr`, show **Pair not found** (`PairNotFoundLinkNotice`). **No bech32 checksum** on URL segments.
2. **Do not set `pairAddr` until known** — avoids indexer 404 loops and ambiguous “not indexed yet” copy for regex-valid garbage (e.g. `terra1` + 38× `x`).
3. **Same URL/selector hygiene** as invalid links: `navigate('/trade', { replace: true })`, empty `MenuSelect` value, block auto-pick while notice is visible.
4. Manual repro: `http://localhost:3000/trade/terra1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx` with LocalTerra + factory pairs loaded.

## Regression tests

Extend [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx) and [`tradePairRoute.test.ts`](../frontend-dapp/src/utils/__tests__/tradePairRoute.test.ts).

## Related

- Trade layout / breakpoints: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- User-facing error funnel: [`AGENTS_FRONTEND_USER_ERRORS.md`](./AGENTS_FRONTEND_USER_ERRORS.md)
- Chart 404 retry (known pair only): [docs/frontend.md § Trade page — chart pair fetch retry](../docs/frontend.md#trade-page-chart-retry) ([#177](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/177))
- Indexer outage banner (distinct from invalid/unknown link): [docs/frontend.md § Trade page — indexer outage banner](../docs/frontend.md#trade-page-indexer-outage-banner)
