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

The [old deployment desktop Trade receipt](deployed-before-trade-desktop.json)
is retained as **inconclusive harness evidence**. It passed initial indexed USD
and two historical candles, then stopped at the coarse missing-USD stage in
10.00 seconds. The first helper installed virtual time after React Query had
registered native intervals, so that stage could not distinguish a missing
refetch from an incorrect headline. It does not establish a deployed product
regression. The corrected helper owns timers before navigation and records
per-phase refetch counts and separate refresh/headline failure stages. Served
script paths/digests and the original helper digest remain in the receipt;
no actors, trade rows or screenshots are exported. Neither helper scope makes
native quote latency claims.

## One actual unmocked market comparison

After the final frontend/backend deployments are independently attested, run:

```bash
timeout 59s node scripts/qa/browser-unmocked-price-1512.cjs
```

This one process visits desktop Trade and Charts for the canonical UST1/USTR
pair. It observes the pages' own actual pair/trade/candle responses, immediately
projects only the required USD/ratio scalars and validity flags, and compares the
visible headline with the corresponding indexed scalar in the selected direction.
It uses unchanged HTTP responses and ordinary browser time. Missing indexed
USD requires `—`; available historical candles must remain plotted. Missing
plottable history is recorded as an explicit historical-check skip, with overall
`PASS_WITH_SKIPS`. Receipts contain booleans, price/count scalars and served-script
digests only; no actor, transaction rows, screenshots or wallet connection.

## Deployed frontend controlled rendering (2026-10-18)

The [four-cell matrix](deployed-price-matrix.json) passed against frontend
`2df4a471a9ca98a8672d911b67d4bd8134f411fd`, Coolify deployment
`me0y48gdxmjv3ibut9yz9grv`, independently attested finished at 06:18:14Z.
Each browser receipt records the actually served JS paths and SHA-256 digests.

| Entry / viewport | Result | Actual process time |
| --- | --- | --- |
| [Trade desktop](deployed-price-trade-desktop.json) | PASS | 7.826s |
| [Trade phone](deployed-price-trade-phone.json) | PASS | 13.202s |
| [Charts desktop](deployed-price-charts-desktop.json) | PASS | 5.544s |
| [Charts phone](deployed-price-charts-phone.json) | PASS | 5.239s |

All cells verify the valid indexed USD, missing-USD dash, retained historical
candles, unavailable inversion, indexed recovery and foreign-pair rejection.
Observed trade refetch counts advance 1→2→3→4. Each process had a 59-second outer
ceiling and 52-second browser budget, with at most two browser processes running
concurrently. Every cell recorded zero writes and actor-shaped requests.

The [first desktop](deployed-first-trade-desktop-harness-fail.json) and
[first phone](deployed-first-trade-phone-harness-fail.json) attempts are retained
as **inconclusive harness failures**. Their clock was installed after native
refetch intervals had already been registered. Moving clock installation before
navigation and recording separate refetch/headline stages fixed the harness;
no application code changed during this browser verification loop. The original
raw receipts remain preserved in `/tmp/dex-issues-1206-1512/`.

This acceptance covers the served frontend with owned anonymous HTTP price
fixtures and virtual refetch time. The backend was deliberately still on its
prior revision. Campaign retry, actual unmocked market behavior, exact backend
revision and native quote useful-result latency are separate checks; this matrix
does not claim they passed.
