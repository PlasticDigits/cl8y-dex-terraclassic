# Agent playbook: Trade page invalid pair deep link

Use when changing **`/trade/:pairAddr`** routing, pair selector sync, or empty-state behavior for malformed share links ([GitLab **#176**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/176)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Trade page — invalid pair deep link](../docs/frontend.md#trade-page-invalid-pair-link) | Invariant table (URL cleanup, notice, queries, auto-pick) |
| [`frontend-dapp/src/utils/tradePairRoute.ts`](../frontend-dapp/src/utils/tradePairRoute.ts) | `isTradePairRouteParam`, `getInvalidTradePairRouteParam` |
| [`frontend-dapp/src/components/trade/InvalidPairLinkNotice.tsx`](../frontend-dapp/src/components/trade/InvalidPairLinkNotice.tsx) | Alert + CTA to `#trade-pair-select` |
| [`frontend-dapp/src/utils/tradeInvalidPairLinkCopy.ts`](../frontend-dapp/src/utils/tradeInvalidPairLinkCopy.ts) | Retail strings |
| [`frontend-dapp/src/pages/TradePage.tsx`](../frontend-dapp/src/pages/TradePage.tsx) | Route sync, `invalidLinkNotice` state, auto-pick guard |

## Rules of thumb

1. **Validate with `isValidTerraAddress`** — do not gate only on `startsWith('terra1')`; short garbage like `terra1` must be treated as invalid.
2. **Never keep garbage in URL or selector** — replace to `/trade` and keep `pairAddr` empty until user selection.
3. **Do not auto-redirect to first pair** while the invalid-link notice is visible unless product explicitly changes that invariant.
4. **Regression tests** — extend [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx) and [`tradePairRoute.test.ts`](../frontend-dapp/src/utils/__tests__/tradePairRoute.test.ts); manual repro: `http://localhost:3000/trade/lilwayne%20babyyy`.

## Related

- Trade layout / breakpoints: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- User-facing error funnel: [`AGENTS_FRONTEND_USER_ERRORS.md`](./AGENTS_FRONTEND_USER_ERRORS.md)
- Indexer outage banner (distinct from invalid link): [docs/frontend.md § Trade page — indexer outage banner](../docs/frontend.md#trade-page-indexer-outage-banner)
