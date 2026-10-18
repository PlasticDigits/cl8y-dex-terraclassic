# Missing trade USD verification (#1512)

Issue: [#1512](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1512).
Results: [source verification](RESULTS.md).
Contract: [TP1512](../../../skills/AGENTS_TRADE_PRICE_USD.md).
Implementation: [pairPriceUsd.ts](../../../frontend-dapp/src/utils/pairPriceUsd.ts),
[headline](../../../frontend-dapp/src/components/charts/chartHeadlinePrice.ts),
[Trade](../../../frontend-dapp/src/pages/TradePage.tsx),
[Charts](../../../frontend-dapp/src/pages/ChartsPage.tsx), and
[Limits](../../../frontend-dapp/src/pages/LimitOrdersPage.tsx).

- [x] Exact original 6/18 missing-USD fixture fails on original source and returns unavailable on final source.
- [x] Table-driven 6/6, 6/18, 18/6, 18/18, both trade/display directions and indexed USD arithmetic pass.
- [x] Malformed/nonfinite/zero/negative/overflow/underflow, spoofed symbols, foreign pairs, future observations, invalid decimals and malformed raw amounts fail closed.
- [x] Actual Trade phone/desktop and Charts components replace prior valid USD with **—**, retain **—** through inversion, recover indexed USD and reject foreign-pair rows.
- [x] Historical candles stay plotted when latest trade USD is missing; no candle-close current headline fallback.
- [x] T524/C543/C680 orientation, tape/amount and transaction conversion regressions pass; types and focused lint pass.
- [ ] Exact frontend/backend deployment revision and build logs attested separately; live desktop/phone Trade/Charts inspected.

Run `make verify-issue-1512` for the focused source suites. Each process uses a
59-second timeout. Relevant legacy verifiers can be run separately with the same
bound. Backend API/storage units are unchanged. Full original issue browser,
attack, API and execution acceptance must remain explicit if not exercised;
source checks alone do not close the issue. Retained local before/fixed logs are
in `/tmp/dex-issues-1206-1512` during this investigation.

## Bounded deployed frontend browser cases

The helper `scripts/qa/browser-price-campaign-1512.cjs` checks the deployed
frontend using anonymous synthetic HTTP fixtures for the canonical UST1/USTR
catalog/detail, latest trades and historical candles. It records served-script
hashes and acceptance booleans; it never exports actor/transaction rows, takes
screenshots, connects a wallet or signs. These cases verify deployed rendering;
actual market pricing and execution remain separate acceptance.

After independently attesting the frontend deployment, run each process
separately with a 59-second outer ceiling (the browser itself has a 52-second
budget):

```bash
timeout 59s node scripts/qa/browser-price-campaign-1512.cjs --case trade --viewport desktop
timeout 59s node scripts/qa/browser-price-campaign-1512.cjs --case trade --viewport phone
timeout 59s node scripts/qa/browser-price-campaign-1512.cjs --case charts --viewport desktop
timeout 59s node scripts/qa/browser-price-campaign-1512.cjs --case charts --viewport phone
```

Each case verifies valid USD, missing USD replacing the headline with `—`,
inversion while unavailable, indexed recovery in the new direction, rejection
of foreign-pair trade rows, and retained historical candles. Virtual browser time
advances the existing refetch intervals; it does not alter production responses
or the application's quote/signing rules. Only the price rendering fixture
requests are substituted. Default receipts stay under `/tmp/deployed-*-1512.json`.
