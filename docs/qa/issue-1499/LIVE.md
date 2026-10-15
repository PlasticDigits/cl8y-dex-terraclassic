# Bounded anonymous live lifecycle and metric receipts

The [native #1505 follow-up](../issue-1505/README.md) uses observer v8. Response
observation comes from the browser network stack's original request start plus
response completion; delayed Node/CDP body delivery is recorded separately and
cannot replace that clock. Two-frame candidates require stable output, fee and
slippage together. Original input and expiry clocks, 750ms useful-paint budget,
native transport guard and cohort bounds remain unchanged. Older executed helper
versions retain their own hashes and verdicts.

For that focused follow-up set `DEX_LIVE_ISSUE=1505` and `DEX_LIVE_SHARD=0` in
the command below. This mode permits only the four native direction/viewport
windows, requires useful initialized API results (typed503 is a failure), and
retains sanitized `x-quote-request-id` values for bounded log correlation. It
also counts every selected sustained native solve response: a known503 or
invalid/non-useful result remains a failure after later recovery. Intentional
window teardown is recorded separately from active response failures. It does
not run the other two shards.

Run only after the exact final frontend/backend deployment is independently
attested by [the existing attester](../../../scripts/qa/attest-trading-quotes-1473.py).
The lifecycle driver checks the attested complete served root/script hashes before
and after, plus public backend health. No prior deployment or repository HEAD is
silently accepted. The driver has no wallet, screenshot or broadcast capability;
non-read methods, blockchain-provider paths and unrelated script/XHR requests are
blocked by pre-app fetch/XHR/beacon/form/worker guards and native CDP URL rules.
Known public GETs use native browser transport without Playwright Fetch routing.
Only the exact public tokenlist image URLs are allowed outside frontend/backend
origins; redirects cannot escape the native deny rules. An owned actual-Chrome
regression verifies native allow/deny support before deployed runs. Only sanitized response classes, public raw quote output, original
observation/expiry clocks, DOM values/metrics and geometry enter its0600 receipt.
No generation/context digest, actor, credentials, upstream text or raw body enters
its report.

```sh
# The coordinator supplies the independently attested final revision and file.
EXPECTED_SOURCE_SHA="$DEPLOYED_FINAL_SHA" \
DEX_LIVE_ATTESTATION=/tmp/dex-final-attestation.json \
DEX_LIVE_SHARD=0 \
DEX_LIVE_OUTPUT=/tmp/dex1499-live-shard0-first.json \
bash scripts/with-node.sh -- node scripts/qa/live-trading-lifecycle-1499.cjs
```

Use `node scripts/qa/live-trading-lifecycle-1499.cjs` through the repository's
Node wrapper if the script is not executable. An optional `DEX_LIVE_CLUNC` may
supply the exact public configured cLUNC execution identity; otherwise the bounded
public token catalog must identify exactly one cLUNC. Ambiguity fails setup.
Explicit native USTC↔CL8Y132, CL8Y↔cLUNC13 and the arbitrary factory UST1↔CL8Y1
controls run unchanged for190seconds each at1280×900 and390×844. Explicit
`DEX_LIVE_SHARD=0`, `1` or `2` selects respectively the native, cLUNC or arbitrary
factory bidirectional group. Each shard has four sequential foreground windows
in its own browser process, a16minute wall budget and600 total quote/progress
reads; all three shards form an explicit12-window,1800-read maximum cohort.
Independent shards may run in separate processes concurrently; a browser never
hosts competing measurement tabs. Useful DOM and two-frame paint clocks are
recorded inside the foreground page from its original input event, independently
of later Node polling. The original four-concurrent-tab shared600-read trial
remains FAIL: its budget accounting included normal1Hz advisory progress and
its two-RAF Node roundtrip inflated some paint clocks. Corrected shard results
never rewrite that receipt. This selected cohort does not claim the full amount,
protection/manual/wallet/Trade matrix.
Settings/DETAILS and both amount fields are prepared before the original input
clock; measurement never resets that clock. Every200ms sample records its browser wall clock before reading DOM fields;
original-expiry eligibility uses that capture clock rather than later CDP/Node
delivery time. Precise failure details retain expiry deltas for investigation.
Every200ms sample checks fixed
Pay/Receive/summary/status geometry, one persistent DETAILS, one network-fee row,
no page overflow, retention of the last actually displayed matching output/metrics
and typed unsupported status. Older unused different-output quotes cannot revive
that display after its original expiry. Identical output/metrics across overlapping
cohorts cannot independently prove which response the client adopted; mixed
expired/unexpired owners are recorded as qualified ownership unavailable rather
than a fabricated retention failure or proven acceptance. The earlier native
transport first receipt and its historical-quote false flag remain archived.
Unchanged output/metrics retain every possible original owner, including expired
ones, until a distinguishable DOM transition. Matching uses output and displayed
metrics together. The owner union is bounded by the600-read shard budget; overflow
explicitly makes ownership unavailable rather than proving a newer adoption.
Bounded browser long-task/frame-gap diagnostics preserve the original input clock
when a useful result exceeds its budget; diagnostics never reset that clock.
`first_observed_dom_ms` records the earliest visible DOM observation independently;
`first_qualified_dom_ms` (legacy alias `first_dom_ms`) precedes the first validated
two-frame paint. An aborted candidate or blank followed by identical output can
retry on the same original input clock. Frozen v4 live receipts predate these final
QA corrections, retain their executed helper hashes and verdicts, and are not
relabelled as v5 live acceptance. Final v5 corrections have isolated first/retest
regression coverage, without another full trial to seek a passing result.
First useful output is confirmed through two render frames against the750ms
browser budget. The independent initialized API first/retest probes use500ms;
loading, transport errors and typed blockers never count as useful output.

The independent [raw-stage validator](../../../scripts/qa/live-quote-economics-1500.cjs)
checks positive floors, quote-relative market ceiling, original metadata/operations,
selected original reserve/book stages, exact bounded original hop.book_terms
(including makers unused by the actual budget), common-unit commissions, fee-free replay,
signed movement, denominator and attribution reconciliation. Missing advisory
metrics are explicitly unavailable, never zero or signing permission. Its isolated
[attack regressions](../../../scripts/qa/test-live-quote-economics-1500.cjs) require no
production access:

```sh
node --test scripts/qa/test-live-quote-economics-1500.cjs \
  scripts/qa/test-live-input-clock-1499.cjs \
  scripts/qa/test-live-trading-lifecycle-1499.cjs \
  scripts/qa/test-live-read-guard-1499.cjs
node --check scripts/qa/live-trading-lifecycle-1499.cjs
```

Retain every first/retest failure separately. A valid typed policy negative is not
an unsupported transport outcome, and a monitoring error count cannot establish
quote readiness. No browser request/RPC observation here independently proves
server-side zero-RPC, reference-oracle quality, contract deployment or actual
settlement; those require the separate existing socket/producer/contract/funded
receipts. Unavailable cases stay unchecked. An incomplete or failed run remains
FAIL and cannot close#1499 or#1500.
