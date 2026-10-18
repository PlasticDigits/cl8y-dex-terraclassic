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
