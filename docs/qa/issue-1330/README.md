# Swap route presentation and campaign retention (#1330)

Issue: [#1330](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1330).
Agent guidance: [route display](../../../skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md)
and [Swap query parameters](../../../skills/AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md).

## Invariants

- **R1330-1:** URL canonicalization replaces only Swap token, amount and exact-field
  aliases, case-insensitively. Unrelated query parameters, including repeated campaign
  fields, remain in the current URL through quote/retry. They do not enter quote
  requests, signing metadata, browser storage, or generated Share links.
- **R1330-2:** The selected operations determine the single route preview and signed
  message. Screen readers receive a spoken `to` between every pair of route tokens.
  Retry requests a new quote; it never signs, and the trader reviews that quote.
- **R1330-3:** Fees and order slippage remain distinct; fair-reference deviation is
  separate. Supported local candidate ranking continues to use exact delivered net.
- **R1330-4:** Third-party factory/pair/router operations are not enabled by this
  presentation repair. The issue comments request a flat 2.9% third-party fee and
  architecture/security design. Implementing those adapters remains open: emitter
  attestation, CW20 code admission, wrapper economics, fee settlement and net floors
  require an explicit compatible execution protocol before such paths can sign.

## Focused verification

`make verify-issue-1330` runs the commands below, each with a 55-second ceiling:

```sh
timeout 55s bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/vitest run src/components/swap/SwapQuoteSummary.test.tsx src/utils/__tests__/swapQueryParams.test.ts
timeout 55s bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/vitest run src/pages/SwapPage.queryParams.test.tsx -t 'campaign parameters'
```

- [x] Two route legs, slippage and fee render; hop order is available to screen readers.
- [x] Forced solver transport failure exposes one retry and recovers without dropping campaign values.
- [x] Repeated campaign values survive; stale case-varied Swap aliases are removed.
- [x] Quote/retry never executes a swap.
- [ ] Live columbus-5 quote demonstrates CL8Y middle-hop winner over direct alternative.
- [ ] Live deployment failure/retry and campaign checks.
- [x] [Third-party DEX architecture/security plan](../../architecture/third-party-route-adapters.md), including tier-independent flat 2.9% fee semantics.
- [ ] Third-party adapters, atomic 2.9% treasury settlement and deployed acceptance requested in comments.

Local passing evidence does not close this issue's remaining live/integration criteria.

## Local results (2026-10-18)

`make verify-issue-1330` equivalent script execution passed: 27 summary/query-helper
tests in 1.72s and the campaign solver-failure/retry integration in 12.76s. Each
process had a 55-second ceiling. The initially reproduced campaign loss is fixed.
Live routing and third-party adapter acceptance remain unchecked above.

## Bounded deployed campaign browser cases

After independently attesting the new frontend and confirming useful anonymous
UST1/USTR backend quotes, run each viewport separately:

```bash
timeout 59s node scripts/qa/browser-price-campaign-1512.cjs --case campaign --viewport desktop
timeout 59s node scripts/qa/browser-price-campaign-1512.cjs --case campaign --viewport phone
```

This injects a solver HTTP 503 until one Retry click, then permits the actual
anonymous backend response. Repeated `utm_source` and campaign/ref fields must
survive URL canonicalization, failure and retry; none may enter solver requests.
No wallet connection, signing, raw activity export or screenshot occurs. A
missing useful backend quote remains failed/incomplete acceptance rather than a
synthetic success. Served-script hashes bind each receipt to observed frontend
assets; source/deployment identity is attested separately. This does not prove
middle-hop ranking or third-party adapters.
