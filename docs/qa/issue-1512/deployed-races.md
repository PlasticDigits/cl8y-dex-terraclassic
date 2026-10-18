# Additional deployed pair/orientation race and reload checks

Issue [#1512](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1512),
[unit contract](../../../skills/AGENTS_TRADE_PRICE_USD.md),
[prior source results](RESULTS.md), [original browser matrix](README.md).
These are additional owned anonymous fixture cases, not repeated earlier price,
campaign or unmocked-market cases. No application source changed.

## Actual new browser evidence — 2026-10-18

The [matrix](deployed-race-matrix.json) records four sequential case processes,
each under `timeout 59s`, with an internal 52-second browser deadline and a
240-request ceiling. Counts include two full script/asset loads: Playwright
routing disables HTTP asset caching. Each corrected case issued only 6 fixture trade
reads, with zero writes or actor-shaped requests.

| Page / viewport | Result | Process time | All requests | Trade reads |
| --- | --- | --- | --- | --- |
| [Trade desktop](deployed-race-trade-desktop.json) | PASS | 15.599s | 192 | 6 |
| [Trade phone](deployed-race-trade-phone.json) | PASS | 23.093s | 197 | 6 |
| [Charts desktop](deployed-race-charts-desktop.json) | PASS | 9.157s | 173 | 6 |
| [Charts phone](deployed-race-charts-phone.json) | PASS | 8.766s | 171 | 6 |

Each case performs these distinct checks against the served frontend:

- Hold a real page-owned trade HTTP response for pair A, invert A while it is
  pending, and use the actual pair picker to select B before completing A.
- Deliver that old A response with HTTP200. B's own indexed price, token label
  and initial orientation must remain selected. The held request is actually
  received by the browser: the exact held Request must return status 200, its
  response body must finish without error, and both request-failure checks must
  remain clear. Interception `route.fulfill()` acknowledgement alone is not
  sufficient; cancellation is not counted as delivered-response evidence.
- Hold an updated B response, invert B before delivery, then require the new B
  indexed price in the current direction exactly once. Independent fixture
  arithmetic uses A factory USD 6 / ratio 2 and B USD 50→75 / ratio 5.
- Remove any Charts `?price=` override and perform an actual document reload.
  B's pair and stored orientation must survive, with the expected indexed USD.
  Check the page's own session key and that the other entry's key stays absent.
- Return through the actual picker to A and require A's own orientation/price.

Browser virtual time is installed before navigation and only advances the
existing refetch timers. No query cache, component state, wallet or execution
permission is injected. The fixture substitutes only exact catalog/pair/trade/
candle resources for two pinned pair identities; auxiliary responses are not
exported. Receipts contain scalars, booleans, counts and served-script hashes.
No screenshot, wallet connection, transaction or raw activity export occurs.

Observed shared served-script hashes all match the prior independently attested
frontend `2df4a471a9ca98a8672d911b67d4bd8134f411fd` (45 shared assets for Trade,
30 for Charts). Per-case asset hashes are retained. This QA task did not deploy
or re-probe the backend; the earlier [mixed identity receipt](deployed-mixed-acceptance.json)
remains separately scoped to frontend M and restored prior backend 0a3.

The [first Trade desktop attempt](deployed-race-first-harness-budget-fail.json)
remains **FAIL**, qualified as a harness request-budget failure. Its four race
checks passed, then the initial 150-total-request ceiling blocked scripts during
reload (167 attempted requests). The corrected harness combines clean-URL and
reload persistence into one reload, removing an unnecessary third navigation,
and bounds the two required document loads at 240. No application code changed.
The original script is preserved privately with a SHA256 matching that receipt's
tool digest; the failed receipt was not modified or replaced by later passes.

The first four 240-budget attempts returned raw **PASS** but lacked that exact
browser-response completion witness. Their raw receipts are retained unchanged
as `deployed-race-*-without-browser-receipt.json`, explicitly qualified
**INCONCLUSIVE_TRANSPORT_WITNESS** in the matrix. Independent review found that
Playwright can silently swallow a Chromium fulfill error after cancellation.
Only the four corrected cases in the table above count as delivered-response
acceptance. Same-pair and old-pair bodies both completed with HTTP200 and no
request failure. This additional guard justified those four corrected runs;
the prior missing-USD/campaign/unmocked-market cases were not repeated.

## Commands

Run only a missing case; do not repeat completed live cases without a relevant
change or failure. Each entry/viewport is a separate process:

```sh
timeout 59s node --test scripts/qa/test-browser-price-races-1512.cjs
timeout 59s node scripts/qa/browser-price-races-1512.cjs --case trade --viewport desktop
timeout 59s node scripts/qa/browser-price-races-1512.cjs --case trade --viewport phone
timeout 59s node scripts/qa/browser-price-races-1512.cjs --case charts --viewport desktop
timeout 59s node scripts/qa/browser-price-races-1512.cjs --case charts --viewport phone
```

The two offline fixture/identity/privacy controls passed in 0.54s. They validate
the helper's declared scope; they do not replace the actual browser assertions.

## Coverage audit

Reviewed actual retained command output rather than adding overlapping counts:

| Requirement family | Existing/new evidence | Limit |
| --- | --- | --- |
| Exact missing-USD reproduction and explicit modern units | Original helper returns `1.214125e-14`; focused final 61 helper/chart tests plus 3 page cases pass | The 3 page cases select only #1512 tests; their 127 skipped neighbors are not additional completed checks |
| Decimal/direction/amount reconstruction and malformed-value guards | Four decimal combinations, both trade/display directions, indexed/missing USD, raw reconstruction, nonpositive/nonfinite, overflow/underflow, foreign pair/future timestamp, metadata bounds | No implicit legacy support remains; this is not the full API identity/chain/extreme-dust attack matrix |
| Orientation, chart and conversion regression | #524 all 10 groups (172 main tests + 2 selected chart tests); #680 all 6 groups (121 main + 7 selected Trade tests); relevant 62 tape/chart/conversion subset | Suites overlap; do not sum them as unique acceptance cases. Source conversion checks are not funded execution |
| Actual page valid→missing→invert→recover→foreign pair | Prior four controlled deployed desktop/phone cases | Virtual refetch clock and synthetic HTTP prices; not current market-source rejection proof |
| Actual indexed market and history | Prior unmodified-HTTP Trade/Charts comparison; 200 historical candles | Actual latest USD was present. Controlled fixtures cover absence |
| Pair/orientation late responses and real reload persistence | Four additional cases above, eight named assertions each | Focused two-pair schedule, not every cache-poisoning/network interleaving |
| API/storage amount→human ratio, asset_0 USD, candle fields/timestamps | Backend source contract unchanged; unmocked browser compared actual scalars | Comprehensive requested backend contract matrix not executed for this fix |
| Empty tape/current-source expiry/rejection, broader chain/metadata attacks, extreme rounding/dust/injection | Some existing source guards; no new live oracle fallback exists | Complete requested abuse/edge matrix remains unverified; historical indexed USD may legitimately be old and must not be relabeled as a current oracle |
| Market/limit place/update/crossing, both directions, LP valuation | Existing conversion regressions; display code does not grant signing | Actual funded execution/LP matrix not performed; no claim of wallet/device acceptance |

The completed focused race/reload criteria can now be checked off. **#1512 stays
open** for the remaining full API, edge/attack and execution acceptance. This
evidence makes no native useful-result latency or new backend deployment claim.
