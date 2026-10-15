# Native quote recovery #1505

[Issue](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1505) ·
[agent invariants](../../../skills/AGENTS_NATIVE_QUOTE_RECOVERY.md) ·
[results](RESULTS.md) ·
[original retained failures](../trading-1497-1500/RESULTS.md#producer-deployment-and-final-acceptance-scope)

Use `make verify-issue-1505`. Source checks run first and retest against unchanged
source. On an executed failure, fix the cause, reset only owned disposable test
infrastructure when relevant, and repeat. Never reset shared or production data.
Unavailable checks can be explicit SKIP with PASS WITH SKIPS. Executed failures
remain FAIL. Skipping a criterion does not establish issue closure.

| Criterion | Evidence required |
| --- | --- |
| AC1 | Trace affected native-input/output paths from bounded scheduled observations through publication, local API and browser. Reproduce and fix a demonstrated availability/recovery cause; retain bounded typed reason/request correlation. |
| AC2 | Correct cohort/clock observer and reproduce or resolve retained display-expiry flags. Test original expiry without stale executable DOM or signing payload. |
| AC3 | Focused original-input initialized socket measurements meet 500ms, with real transport reuse, useful-result validation and error counts. |
| AC4 | Focused native forward/reverse desktop/phone windows cross 20s/180s original expiry and meet 750ms useful-browser budget. Preserve exact input clocks, failures and observer revision. |
| AC5 | Offline requests, original expiry, provider/work limits, actual economics and fail-closed signing remain enforced. |
| AC6 | Attest exact deployed frontend/backend revision, inspect deployment logs and Grafana/Loki; query Prometheus if provisioned. Repeat focused live checks after deployment. |

No broad verification-only sweep, physical device or wallet signing is required
by #1505. Historical failures remain evidence and are not relabeled by new runs.
The issue closes only when its body and comments' actual required checks pass.

Attest both exact deployment revisions and independently wait at least 180 seconds
from backend deployment completion before useful measurements. Run one browser
at a time without concurrent builds or integration timing jobs. Native browser
FIRST must complete four sequential 190-second direction/viewport windows; run
unchanged RETEST only after the entire FIRST passes. Run separate 190-second
initialized socket FIRST/RETEST. Stop on an actual failure, preserve partial
windows/attempts and diagnose before another implementation/deployment cycle.

Use new receipt paths for each pass and retain helper hashes, useful/error sample
counts, original input/observation clocks, ownership qualifications and bounded
teardown counts. Native observer v9 allows at most 800 solve/progress calls
globally, including four initialized API solves and all browser setup calls.
The shared read guard retains its 600-call context maximum; each context receives
`min(600, global_remaining)`, and either limit stops the cohort as incomplete/FAIL.
Never reset the global counter between windows or exclude progress/setup traffic.
Generic cohorts retain 600 calls per shard and 1,800 across three shards. The
16-minute wall bound, four 190-second windows and all latency, original-clock,
ownership and availability rules remain unchanged. This separate resource-budget
correction follows the retained [v8 cutoff](deployment-1516-browser-fail.json);
it cannot excuse that run's earlier application 503. Run the no-network budget
controls in `scripts/qa/test-live-quote-budget-1505.cjs` before live use.

If natural windows never exercise expiry, record a separate
bounded anonymous client-delivery-hold control: retain original response metadata,
honor AbortSignal, observe synchronous amount/minimum revocation at original
expiry and fresh same-intent recovery. This functional control cannot replace
natural availability/latency acceptance or establish funded signing.

After attestation and warm-up, run the focused probes:

```sh
python3 scripts/qa/probe-native-recovery-1505.py \
  --revision "$DEPLOYED_SHA" --require-correlation --duration 190 --interval 2 \
  --output "$SOCKET_RECEIPT"
EXPECTED_SOURCE_SHA="$DEPLOYED_SHA" DEX_LIVE_ATTESTATION="$ATTESTATION" \
  DEX_LIVE_ISSUE=1505 DEX_LIVE_SHARD=0 DEX_LIVE_OUTPUT="$BROWSER_RECEIPT" \
  bash scripts/with-node.sh -- node scripts/qa/live-trading-lifecycle-1499.cjs
bao-exec --only COOLIFY_URL,COOLIFY_WIRE_TOKEN -- \
  python3 scripts/qa/probe-native-monitoring-1505.py --output "$MONITOR_RECEIPT"
```

Receipts use private 0600 files. The monitoring helper accepts up to eight
`--request-id` values from a response; it exports fixed enum counts and matching
counts, never raw lines. Monitor collection success is separate from useful-quote
acceptance. Keep failed original runs and exact helper hashes.
The native monitor explicitly allows at most 18 reads including two credential
bootstrap reads: datasource discovery, twelve fixed aggregates, one bounded
sample, optional requested-ID sample and optional Prometheus. Existing shared
monitoring keeps its 16-read default. Test all optional branches through the real
reader cap with mocked transport; the nineteenth read must stop before transport.

Slot-bearing producer failure records distinguish per-denom dispatch, global
dispatch and logical LCD-future polling. Export only the fourteen fixed slots,
allowlisted phase/reason/stage, capped 0–60,000ms timings and bounded optional poll
identity/sample timestamp. LCD activity includes internal admission and identity
waits; it is not wire-attempt evidence. Aggregate deadlines without a slot are
not physical-read records. Reject missing/inconsistent stage/clock fields rather
than inventing values, and retain the existing 300-line/eight-ID bounds.

Final503 diagnostics retain at most one stale witness for each of eight fixed
components. The monitor exports only allowlisted component names, bounded
age/expiry scalars and explicitly requested server IDs. A failed alternative can
contribute a candidate witness; these events do not alone prove the sole cause.
Successful responses do not emit stale candidate witnesses.

Reference regressions must reproduce the late-pool/timer phase with original
twenty-second expiry, verify coalesced nonoverlapping work and cancellation, and
block unrelated accounting while a whole qualified replacement still publishes.
Use a disposable database to test absent/mismatched supplied oracle qualification;
the scheduling fix cannot promote a new price class or renew a source clock.

Native browser mode requires useful results during the sustained windows as well
as the initialized probes. Recognized503s remain failures after recovery; ordinary
same-intent display retention does not certify backend availability. Teardown
cancellation is reported separately so closing a completed window cannot create
a false failure or hide an already observed503.

Canonical hash reuse must match a fresh full hash while reducing actual hash
calls only for exact live immutable bindings. Detached/changed views and wrong
address/kind bindings must rehash. Preserve base/reference/policy/final identity,
negative actor/fee authority, original expiry and the two owned CPU slots. The
maximum-publication profile keeps its complete workload, pacing and sample cap.

Native observation regressions cover healthy tax/cap reads longer than the old
two-second subdeadline while preserving the original complete-poll deadline and
read count. Separate controls exercise short-circuit recovery through ordinary
admission within the existing total deadline, long-circuit exhaustion without
HTTP work, request fast failure, attempt caps, integrity precedence and owned
cancellation. A validated pause must revoke a previously published wrapper without waiting
behind unrelated bindings/router reads;
transient unavailability cannot fabricate fields or renew their original clock.
Per-denom contention regressions must retain the exact old failing slow-read
fixture, then prove independently valid sibling output within the same original
margin. Check symmetric gate ordering and release on timeout/cancellation. The
controlled fixture proves that scheduling defect; do not label it the historical
cause when retained production logs lack per-read timing/denom evidence.

Database ownership isolates data and migrations, not CPU/I/O latency. Run the
deadline-sensitive integration/verifier stages one at a time in the QA instance,
without concurrent migrations, builds or browsers during timing measurements.
Keep original provider/head/pair budgets; preserve failures from earlier runs and
record the execution envelope. See the retained
[head diagnostic](head-failure-diagnostic.json): a later quiet pass cannot establish
the cause of an earlier uninstrumented failure or erase it.

Snapshot cleanup regressions use the SQLx pool's public release hook to delay only
queued rollback. Verify normal completion, partial advisory denial and original
SQL-error returns release every acquired key before the next refresh. Preserve
provider/head/pair deadlines and original errors, and use only owned disposable
QA pools. A connection-local missing search path tests 42P01 without schema
mutation.

Grouped commands must return any nested failure explicitly: Bash disables
`errexit` when a function runs inside a conditional. The focused verifier now
short-circuits failed Rust/DB/QA commands;
[status-injection proof](verifier-status-proof.json) uses fake executables and no
network/private environment. The DB group also runs the three Combined cleanup
controls on both source passes.

Capture bounded 15-minute monitoring snapshots after each complete browser pass
so the two snapshots cover the full FIRST/RETEST interval; preserve overlap rather
than adding duplicate counts. Prometheus is queried only if provisioned. Pattern
counts are monitoring evidence, not cohort availability or sole-cause proof.

Scheduled fallback regressions must use the pool lane's actual scheduled view,
including the path without ingestion pacing. Reproduce a real invalid historical
primary response and a qualified alternative briefly denied shared capacity.
Keep one original deadline/cursor, assert actual per-endpoint/global attempts and
known-negative safety callbacks, and show that cancellation releases permits.
Retain the OLD FAIL; fixed FIRST/RETEST must also cover spent attempts, original
expiry with retained integrity, no repeated attempted endpoint, and prompt direct
callers. The narrow [NR1505-10](../../../skills/AGENTS_NATIVE_QUOTE_RECOVERY.md)
continuation is not a restart of a query or its budget. Run its Rust suite through
`make verify-issue-1505` plus applicable chain-attestation, discovery, feed and
provider checks; the verifier's failure propagation includes every new command.
Source controls do not establish historical provider contention or deployed recovery.

Scheduled alternative evidence exports only fixed `data`/`capacity`, actual prior
attempt count (1–8), unattempted capacity-denied count (1–31), their sum within the
existing 32-endpoint census, original remaining milliseconds (0–30,000), and an
optional numeric sample timestamp. The producer emits at most one record per
logical query; repeated ordinary admission ticks emit none. Attempt counts are
admission-charged endpoint-attempt counters, including identity work; they do not
independently count wire/data requests. This witness records local budget and
pending admission, not a selected reference source or an unobserved historical
sequence. The existing bounded sample
query includes these records without another read. Tests refuse unknown, missing,
duplicate, out-of-range and inconsistent evidence and omit raw labels/lines.

Producer deadline controls also exercise an enclosing owner that expires before
the LCD operation's own clock. Use real producer paths and observe first transport
polling after a held metadata completion; wire counts alone can miss immediate
cancellation. Check wrapper/floor and head/pair/nested helper boundaries, including
late identity-only success, closed versus half-open/generic health, known negative
precedence, clone isolation and nonrenewing nested minima. The final verifier
selects producer cases through issue1505_parent_deadline as well as the full LCD
control module; failure propagation covers all eleven Rust commands, including the full parent-deadline LCD module.

Distinguish timely actual height from the requested pool floor. A valid actual
observation may update normal global lag tracking even when ineligible for that
query; never invent the database minimum. Test expired buffered positives before
height/health publication, retain typed negatives first, and keep the existing
minimum-head work/rate/health regression unchanged. A new control that contradicts
this observed-versus-requested contract must be corrected and retained as a
mis-specified failure, not used to justify changing policy.

Final seventh-candidate source evidence: [frozen verification receipt](source-verification-7-final.json), [scheduled continuation controls](scheduled-provider-continuation-proof.json), and [enclosing deadline controls](parent-deadline-proof.json). Deployment acceptance remains separate in [RESULTS](RESULTS.md).
