# Neutral native entry (#1489)

This presentation follow-up to [PR #1490](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1490)
addresses [#1489](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1489).
Follow [RD1489-7](../../../skills/AGENTS_ROUTE_DISCOVERY.md),
[RC1483](../../../skills/AGENTS_ROUTE_COST_ATTRIBUTION.md) and
[TQ1473](../../../skills/AGENTS_TRADING_QUOTE_OUTCOMES.md).

The offline wrapper configuration query is intentionally disabled. Before an
amount-specific local quote supplies current wrapper context, its empty cache is
unknown, not proof that wrapping is unavailable. The prior UI treated that absence
as a global failure, including the empty native entry. Independent anonymous
fresh desktop and phone observations on deployed `74e7ad03629832076f59614e6b37ee868fe62c9c`
retained the unavailable notice at initial load and five seconds later, with no
quote amount, signing, actors, provider calls or writes. That failed baseline
remains a failure.

The fix changes presentation and button copy only. Empty and zero amounts remain
neutral. Pay and receive debounce, active quote fetching, pending pricing and an
expired original wrapper quote wait for current data rather than asserting global
wrapper failure. Dust, below-precision receive targets and terminal quote errors
retain explicit refusals. Actual cached pause and treasury mismatch still block a
positive intent. Current wrapper context is neither a new availability promise
nor signing permission: supported execution, original expiry, positive floors,
funding, fee coverage and all existing mutation guards still apply.

## Directed verification

- [x] Actual component red reproduction: empty native entry displays the unavailable notice without a solve or wrapper RPC.
- [x] Fresh native default, cleared and zero entry are neutral; submit remains disabled.
- [x] Positive pay and receive debounce/fetch and missing-pricing waits make no false unavailability claim.
- [x] A positive validated wrapper quote supplies current context; no legacy wrapper RPC is introduced.
- [x] Dust, receive precision, rejected oversized input, expired original context, positive unsupported quote, cached pause and treasury mismatch refuse signing.
- [x] Existing price notice/review pinning and exact 5/30/99 regressions remain passing.
- [ ] Exact new merged frontend/backend/all-root attestation and actual desktop/phone first/retest neutral-entry observations.

Stable byte-equivalent frontend verification passed **370 files / 3,397 tests per
full round**, first and retest. Directed first/retest passed **8 files / 227 tests**;
TypeScript and full ESLint passed (zero errors, five existing warnings). The
frontend content scope hash is
`af83034dac68a9b0b4abb674a950861f3e48f1676d6d735166e80edce85cc4e8`:
from the checkout root, hash every existing file under `frontend-dapp/src` plus
`package.json`, `package-lock.json`, `vite.config.ts`, `vitest.config.ts` and
`eslint.config.js` under `frontend-dapp`, sorted by path, as path + NUL + bytes +
NUL. This is a content scope hash, not a deployed Git revision.

Re-run the directed component scope with the repository Node wrapper:

```sh
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
  src/pages/SwapPage.test.tsx src/pages/SwapPage.queryParams.test.tsx \
  src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx \
  src/utils/localRouteQuote.test.ts src/utils/localWrapperOnlyQuote.test.ts \
  src/utils/localQuoteFreshness.test.ts src/utils/quoteDebounce.test.ts \
  src/hooks/useRouteQuotePromotion.test.tsx
```

This isolated UI publication does not certify deeper-route convergence, deployed
independent reserves, production wallet settlement or the separate indexer
canonical-generation follow-up. All broader acceptance and skip rules remain in
[the main checklist](README.md); #1489 stays open while required scope is incomplete.
