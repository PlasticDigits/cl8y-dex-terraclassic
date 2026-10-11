# Pricing data waits and original client expiry

Follow-up to [#1473](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1473)
and [PR #1474](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1474).
The pricing-data and client-expiry follow-up is [PR #1475](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1475).
Use [the checklist](README.md), [results](RESULTS.md), and
[TQ1473](../../../skills/AGENTS_TRADING_QUOTE_OUTCOMES.md).

## Behavior

Missing or stale cached observations keep the current trade in an indeterminate
price-loading state until a validated executable quote arrives. This applies to
all supported identities, including uncommon tokens. There is no manufactured
estimate, completed percentage or finite ETA. Amount/direction changes cancel
the prior wait. Dust, real protection, unsupported semantics, accounting errors
and trading restrictions remain distinct truthful conditions. Waiting never admits
a trade. A disconnected wallet retains its connection action; connected trade
submission stays disabled until a current executable quote is ready.

Only independently scheduled ingestion may supply observations. Waiting watches
the local publication generation; it does not start chain queries or repeatedly
solve unchanged data. An initial missing-data response without a generation must
still observe subsequent publications. Each intent retains only the 64 most recent
attempted generations, so an indefinite wait has bounded memory. Repeated current
publications never restart solves; an older evicted generation can be considered
only after an actual publication change. History resets on trade-intent change. `/progress` being search-idle is not
proof that all pricing prerequisites exist.

KENA buying requests on deployed `7b66402c…` returned `missing_observations` in
bounded read-only probes. Its sole active pair had approximately $25.77 cached
liquidity. The admitted economic reference producer has a $100 default depth
floor; the scoped deployment configuration does not override that setting.
Waiting does not bypass reference admission: a usable reference must actually
become eligible. The loading state makes no promise about when that will happen.

## Expired client cache regression

On deployed `7b66402c…`, revisiting 0.1 UST1 ↔ CL8Y at 1% reused a quote already
expired by about 1.1 seconds. Its validation had occurred less than ten seconds
earlier, so the global React Query freshness window still treated it as fresh.
The app displayed the old value in about 36 ms and fetched again only at the
ten-second refresh. The new response took about 297 ms; no main-thread long
tasks occurred in the bounded diagnosis.

Local quote cache freshness, active display and submit admission must honor the
original `expires_at_ms`, including the nested receive-sizing quote. Revisit
fetches promptly; expiry removes the old value/signing payload and refreshes
local arithmetic with bounded cadence. It never extends producer watermarks,
the four-second fee phase, twenty-second observations or 180-second configuration
expiry. Failed refreshes retain React Query data; their original expiry must not
accelerate retries. Ordinary forward errors retain the ten-second cadence, receive
errors have no interval retries, and typed missing/stale recovery remains driven
by changed publication. Legacy data without local execution metadata retains its
previous policy.

## Implementation links

The [scheduled reference producer](../../../indexer/src/indexer/economic_usd.rs)
retains its [configured admission floor](../../../indexer/src/config.rs).
The frontend uses [original-expiry freshness](../../../frontend-dapp/src/utils/localQuoteFreshness.ts),
[local publication recovery](../../../frontend-dapp/src/hooks/useQuoteErrorRecovery.ts),
[current-intent waiting](../../../frontend-dapp/src/hooks/useQuotePricingDataPending.ts)
and the shared [accessible progress surface](../../../frontend-dapp/src/components/QuoteDataLoadingProgress.tsx).
No token identity enters these presentation or cache rules.

## Agent verification

Run `make verify-issue-1473` for the repeated source/socket/desktop/phone corpus.
For a focused frontend check before the full verifier:

```sh
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
  src/utils/localQuoteFreshness.test.tsx \
  src/hooks/useQuoteErrorRecovery.test.tsx \
  src/hooks/useQuotePricingDataPending.test.tsx \
  src/components/QuoteDataLoadingProgress.test.tsx \
  src/pages/SwapPage.test.tsx \
  src/components/trade/__tests__/TradeMarketOrderPanel.slippagePresets.test.tsx
```

1. Run the expiry/freshness, recovery, Swap pay/receive and Trade market tests.
   Verify a missing-generation 503 waits, observes publication, and replaces the
   bar with an executable quote; unchanged generations do not repeatedly solve.
2. Verify original expiry inside the global ten-second freshness window removes
   display/signing data and prompts a fresh quote on revisit. Expiry while the
   amount remains unchanged must also hide the old value and refresh locally.
3. Check pending → dust/protection/unsupported transitions retain their real
   condition, and pair/direction/amount cancellation does not leave a stale wait.
4. Repeat the controlled matrix and then separately attested live browser cases
   with the temporary 750 ms useful-output budget. Loading is not useful output.
   Confirm quote requests never drive producers or extend original expiry.

Unavailable checks may be explicit SKIP with **PASS WITH SKIPS** for the executed
scope. Executed failures stay FAIL; incomplete acceptance keeps the issue open.

## Windowed browser reproduction

The final full headed trial uses the normal
[controlled fixture](../../../scripts/qa/browser-trading-quotes-1473.cjs) with a
private temporary adapter. Copy it outside tracked source; replace only its
repository-root resolution for that copy, change the one `headless: true` launch
argument to `false`, and add receipt fields naming `browser_headless: false`,
`windowing_mode: owned_xvfb_x11` and source fingerprints. Keep every amount,
fixture response, native output, launch flag, clock and acceptance bound intact.
Record the adapter digest/diff with the receipt; the final summary binds that digest.

Provision a dedicated authenticated Xvfb display with automatic display-number
allocation and TCP listening disabled. Use only its `DISPLAY`/`XAUTHORITY` for
the adapter process; never use the human display. Dependencies may be downloaded
and extracted into an owned temporary directory without global package changes.
Run the complete unfiltered 4,250-case first/retest, compare source fingerprints
before/after and close only verified owned browser/display PIDs and auth files.
Unavailable display prerequisites may be an explicit SKIP; earlier headless
failures remain FAIL and a headed PASS cannot prove their cause. The normal
repository fixture remains headless. Follow [the final source scope](source-warmup-final.json)
and [headed receipt](browser-warmup-headed-pass-summary.json).
