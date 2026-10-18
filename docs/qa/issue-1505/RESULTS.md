# Native quote recovery results

Investigation and verification for
[#1505](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1505).
See [the checklist](README.md) and
[NR1505 invariants](../../../skills/AGENTS_NATIVE_QUOTE_RECOVERY.md).

Results are recorded separately for source, deployment and focused live checks.

## Current status

Issue #1505 remains **OPEN**. The ninth deployed revision
`4c21b49025f29c2862dedc26c2308ce3295e332a` passes source checks, genuine
CI492/493, exact frontend/backend deployment and logs, independent warmup and
socket FIRST. Natural browser FIRST completed all four 190-second windows with
120/120 useful selected responses. Unchanged RETEST **failed** with three actual
native-output `503 missing_observations` responses; sampled recovery took
12.067 seconds. Its final phone window stopped deliberately. No unchanged rerun,
socket RETEST or positive expiry controls followed that failure.

Bounded logs link the rejections to a fresh partial wrapper poll whose required
USTC rate-limit read exhausted the original publication margin. Keeping the
preceding complete view until its original expiry would still leave a
5.644-second publication gap. The historical provider/peer ordering remains
unproven. Investigation has returned to implementation; source controls must
establish any proposed cause before the next deployment.

The ninth change gives both common treasury reads actual permit admission
priority before router work. Its focused 8/8 and full wrapper 51/51 FIRST/RETEST,
three independent reviews and final six-stage source wave pass: 61 passing groups,
zero failures, six explicit separate-scope skips and successful Clippy. The
observer's actual initialization checkpoint and validated supplemental timing
metadata pass 59 public QA tests. These source results remain separate from the
failed live RETEST.

CI491's incoming-evidence scanner failure is retained. Five exact public literal
exceptions were independently verified without broadening other scanner policy.
The immutable trusted-policy rollout, local scanner/build/runtime controls and
actual PR/main CI492/493 pass. Neither policy success nor the preceding natural
FIRST pass establishes deployed recovery. The [ninth deployment evidence](#ninth-deployment-first-pass-and-retest-failure)
retains the completed and unfinished checks.

The tenth candidate removes only the wrapper's redundant serial identity
preflight. Six focused controls and the full 57-test wrapper module pass twice.
Its final serialized source wave passes 61 groups with zero failures and six
explicit separate-scope skips; Clippy succeeds. All 1,211 application files and
2,013 verification inputs match before/after. Hosted CI, deployment and new live
acceptance remain pending; the ninth RETEST failure remains FAIL.

The [fifth candidate's source wave](source-verification-5-final.json) passes the native,
identity, offline quote, lifecycle, liquidity-feed and provider-recovery
verifiers, including first/retest, with zero executed failures and explicit
separate-scope skips. Clippy succeeds with 155 existing warnings. All 1,205
application files and 2,005 verification inputs retain identical before/after
fingerprints. The final LCD module passes 108 tests twice, including reproduced
deadline/cancellation controls. Hosted CI and exact deployment passed. Socket
FIRST passed, but browser FIRST retained one actual 503; RETEST did not run.

The records below preserve each earlier revision/run, including failures and
interim pending work. Later receipts establish later outcomes; they do not erase
failed runs or prove an unavailable historical cause.

## Reproduction on the previous deployment

Both Coolify applications still served `14055610ebe86fd3640ff85e8adc15e16785c6dd`
at investigation start. The [50-second initialized socket baseline](baseline-socket.json)
passed 38 useful quotes; its cold first request remained separately recorded.
The [190-second sustained baseline](baseline-sustained-socket.json),
2026-10-15 06:41:01–06:44:12 UTC, reproduced the issue: 142 executable results
and four `missing_observations` HTTP 503 responses, all in CL8Y→USTC. The opposite
native-input direction stayed useful. The next sampled reverse useful result
arrived 10,383ms after the first unavailable sample. This polling interval is
recovery evidence, not a replacement for original browser-input latency.

Useful initialized responses stayed within 308ms, so fast error completion and
healthy intervals do not establish sustained availability. These failures remain
FAIL. [Grafana/Loki baseline](baseline-monitoring.json) also matched provider
capacity words, historical-height messages and admission-budget exhaustion, but
lacked native phase/request linkage. The broad capacity pattern also matches
healthy `head_capacity_waits` fields; its count does not establish provider
pressure. Those text patterns alone do not establish cause.
No Prometheus datasource was provisioned.

The [corrected v6 browser baseline](baseline-browser.json) completed all four
190-second native direction/viewport windows with 490 read-only quote/progress
requests. Initial useful paints were 361–411ms, while both initialized reverse
API probes returned 503. Both phone windows reproduced expired display: native
input remained visible 2,000ms past the latest matching original expiry and native
output 1,225ms past it. The input phone window also retained an original-window
validation failure. Desktop windows had no display flags. This complete baseline
remains FAIL. A separate 35-second timer diagnostic received replacements before
each deadline and could not distinguish timer delay from deferred React commit;
it establishes no acceptance pass.

## Historical first implementation: demonstrated source defects

The wrapper producer serialized independent treasury and native-denom reads before
its bounded output and capability phases. Ordinary provider latency could consume
the original publication budget while native-input arithmetic remained available.
The fix overlaps only independent prerequisites through the existing shared
two-permit/four-RPS lane. Both optional phases retain separate two-second bounds,
the 7.9-second margin and eight-second outer deadline. No old field is reused and
all original freshness and fail-closed checks remain.

The shared React expiry hook queued a concurrent state update at the deadline,
leaving executable display and payload in the DOM until a later scheduler task.
A real React-root regression reproduced this failure; expiry now synchronously
commits revocation in the original deadline callback. The observer separately
uses browser network completion time and requires stable output/fee/slippage
through both render frames, preserving original input clocks and historical
helper-version verdicts.

The source regressions prove these mechanisms without establishing the exact
provider or sole cause of every historical 503/expiry flag. The focused deployed
receipts below determine whether the reported native behavior is resolved.

Server-generated, process-randomized request IDs and fixed direction/reason/poll
phase diagnostics provide bounded correlation without quote query strings,
wallets, bodies or private provider endpoints.

## Historical first implementation: source verification

`make verify-issue-1499` passed all selected groups twice: nine wrapper regressions
and 188 frontend tests per pass, plus types, formatting and documentation.
`make verify-issue-1464` completed first/retest provider, admission, oracle and all
six disposable integration suites: ingestion, health, book snapshots, local
routes, tax admission and wallet observations. No executed group failed; its
separate deployed/funded checks remain explicitly outside that source verifier.
Each integration process owned its own migrated disposable database.

The new scheduling regression was also executed with only the former serial
prerequisite ordering restored: it failed at the original publication deadline.
Restoring parallel ordering passed at about 6.7s, including both native denoms,
both floor capabilities, the same thirteen data reads, required independent
identity attestations and original expiry. The generic-cancellation integration
fixture still refuses signing when optional output/floors are unavailable.

Frontend production build passed with the repository's CI test WalletConnect
identifier and development mode disabled. The first attempt correctly refused
an absent build identifier. The CI-pinned, checksum-verified gitleaks 8.30.0
tracked-file scan passed; the workstation's older 8.18.4 scanner falsely matched
a public token address in a historical QA receipt. No scanner policy was relaxed.

## First deployment: verified source, retained live FAIL

[PR #1509](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1509) merged as
`c3e216842fe3bac00e2c71bb967950e004b3a6cf`. Its runtime inputs match the tested
source head exactly. [CI464](ci-464.json) passed contracts (999), frontend (3,470),
library (841, one ignored) and isolated PostgreSQL (84, one ignored), with all
required stages green. Both Coolify deployments finished on that merge; build
logs contained no failure patterns and [independent attestation](deployment-1509-attestation.json)
matched public backend health and served complete frontend root assets.

The [first deployed 190-second socket run](deployment-1509-socket-fail.json)
remained FAIL: 143 executable responses and three reverse `missing_observations`
503s, with 7,800ms sampled recovery. Useful initialized responses stayed within
316ms. No unchanged retest was run to seek a pass. The
[browser run was interrupted](deployment-1509-browser-interrupted.json) after the
socket failure: its one complete desktop window and partial phone window had no
expiry/validation/ownership flags, but incomplete windows remain unverified.

The [correlated failure window](deployment-1509-failure-window.json) identified
the missing prerequisite. Poll44 published at 07:39:45.087 UTC with tax and both
caps complete but router unknown; all three request IDs matched native-output
availability rejections before poll45 restored the router at 07:39:53.570.
Contemporaneous pool observations were healthy. Direct-wrapper capability
rejections were consistent before, during and after the gap; those separate
protections were not relaxed.

The prior complete observation expired at its original deadline during this gap.
Retaining it alone cannot bridge the failure safely, so no expiry extension or
snapshot splicing was added. The follow-up starts the independent router read
beside treasury/bindings, preserving its two-second timeout and the existing
tax/cap and floor phases, shared two-permit/four-RPS lane, eight-second outer
deadline, thirteen data reads and original poll-start age. An explicit typed
router timeout log replaces the formerly silent cancellation.

The new real-HTTP regression reproduces the router-only timeout with the former
four-way output phase, then confirms early placement completes all routed
prerequisites under the same bounds. A separate timeout control confirms native
input remains usable, native output stays blocked and cancellation releases
provider resources. Follow-up verification/deployment is recorded separately.

## Second deployment: retained live FAIL

[PR #1510](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1510) merged as
`1c21064a91dcd51943a96f03108bf47ae8b31c22`. [CI467](ci-467.json) passed every
required stage, including 843 library and 84 isolated PostgreSQL tests; one
existing ignored case in each group remained recorded separately. Both Coolify
deployments finished at that revision, with zero deployment-log failure markers
and [exact attestation](deployment-1510-attestation.json).

After independent producer startup, the [190-second socket FIRST](deployment-1510-socket-fail.json)
failed: 142 useful quotes and four reverse `missing_observations` 503s, with
10,386ms sampled recovery. Useful initialized responses stayed within 307ms.
The [strict v8 browser FIRST](deployment-1510-browser-interrupted.json) separately
caught two native-input `stale_observations` 503s during its complete desktop
window. Useful paint was 366ms, and no expiry or ownership flag appeared, but
availability remained FAIL. The run was stopped after the socket failure;
reverse browser windows, unchanged retests and controlled expiry were not run.

[Bounded router diagnostics](deployment-1510-router-deadline.json) identified
explicit two-second router cancellations, including the poll preceding all four
reverse failures. The deployed provider-attempt timeout is 2.5 seconds: the extra
wrapper deadline could cancel an otherwise permitted attempt, including admission
and identity work, before its provider timeout. Starting earlier did not resolve
this mismatch. Native-input stale failures are a separate trace and must also be
resolved before acceptance. These receipts remain FAIL throughout subsequent
implementation and deployment loops.

The [input failure window](deployment-1510-input-failure-window.json) showed
fresh wrapper observations and all 22 raw pools refreshed before both failures.
[Adjacent useful response clocks](deployment-1510-input-clocks.json) had only
704/706ms left before their original expiry; each503 arrived about 589ms after
that expiry. [Reference-window logs](deployment-1510-reference-window.json)
showed no hub/liquidity refresh failure. These exclusions do not establish
whether the selected complete mirror or fair-reference component was stale;
newer raw reserves alone do not prove either component was published.

## Third implementation: provider bounds and component evidence

The independent router read now runs alongside all remaining settlement
observations, bounded by the existing provider attempt/total limits and original
7.9-second publication margin/eight-second outer poll. It has no extra shorter
wrapper timeout. Tax/cap and floor phases keep their original two-second bounds;
the same thirteen data reads, shared two-permit/four-RPS lane and original
poll-start freshness remain. Settlement observation is a focused helper.

A real-HTTP regression permits a 2.2-second router response under the existing
2.5-second attempt policy: the former wrapper cap cancels it, while the corrected
schedule publishes both native directions. Separate controls prove original
margin cancellation releases resources and router rejection preserves native
input while refusing output. No snapshot retention or expiry extension was added.

For the unresolved native-input trace, request-owned diagnostics record at most
one candidate witness per fixed prerequisite component and emit only for a final
503. Ages/expiry deltas are bounded; actors, assets, queries and provider bodies
are absent. A real HTTP regression covers repeated stale-reference candidates,
correlation, privacy and suppression when an alternative succeeds. A witness is
not a sole-cause assertion; deployed measurements still determine acceptance.

[Third-source verification](source-verification-3.json) passed
`make verify-issue-1505`, `1499` and `1464` first/retest with stable source.
The native verifier runs 18 wrapper regressions, the HTTP stale-witness and
request-correlation regressions, 36 frontend tests, 38 browser QA controls and
nine probe controls per pass. Lifecycle checks passed 188 frontend tests per
pass. Provider recovery completed its disposable integration suites twice.
Separate live/funded scope remains explicitly skipped by those source verifiers;
it does not establish deployed acceptance.

Review also reproduced a zero-time boundary defect: Tokio's timeout polls the
inner future before its expired delay. On the warmed production lane, an already
expired router margin increased provider attempts from 1 to 2. An explicit guard
now refuses the network future before polling; configured router absence remains
authoritative `Some(false)` without I/O. The [boundary proof and verification](source-verification-3-boundary.json)
retain the old FAIL and fixed PASS separately. All eight focused tests passed
twice, then native and lifecycle verifiers passed first/retest on the final source
(19 wrapper tests per native pass). The full provider-recovery first/retest above
belongs to its immediately preceding application revision; full hosted CI and
live acceptance are independently recorded for the final deployment.

## Third deployment: router complete, reference freshness FAIL

[PR #1513](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1513) merged as
`cfc1ba4b01f68f4aa804aa33d86c9048c8ce378a`; its entire tree matches the tested
final head. [CI470](ci-470.json) and [main CI471](ci-471.json) passed all required
jobs: 999 contract, 3,470 frontend, 847 library and 84 PostgreSQL tests, with
zero failures and the same two ignored cases. Both Coolify deployments finished
at that revision with no deployment-log failure markers and
[exact attestation](deployment-1513-attestation.json).

After independent startup, [all 18 sampled wrapper polls](deployment-1513-wrapper-census.json)
had complete routed prerequisites, within 7,259ms. Direct-wrapper floor support
remained separately unavailable and protected. The
[190-second socket FIRST](deployment-1513-socket-fail.json) still failed: 145
useful results and one reverse `stale_observations` 503, with 2,602ms sampled
recovery. Useful initialized responses stayed within 309.04ms.

The [v8 browser FIRST](deployment-1513-browser-interrupted.json) independently
caught two native-input stale 503s during its complete 190-second desktop window:
38 useful responses and two failures, with a 387ms useful paint. Its partial
phone window had 13 useful responses and no unavailable result before shutdown.
No expiry, original-window or ownership flags were observed. These observations
do not make the incomplete run pass; unchanged retests and functional expiry
control were not run.

[Exact output](deployment-1513-output-reference-window.json) and
[input](deployment-1513-input-reference-window.json) request IDs correlate all
three selected failures with reference-only stale witnesses. Their original
reference timestamp was 10:10:55.323 UTC; the failures arrived 119, 446 and
1,039ms after its original expiry. The next raw-pool cycle had already completed
all 22 pools at 10:11:10.934, while the consumer still held the older reference.
The browser adopted a newer original reference clock at 10:11:17.097.

The retained logs distinguish reference expiry from the earlier router loss.
They do not reveal the exact database mark-commit instant. A controlled
scheduled-clock regression reproduces the phase mechanism: a late pool result
misses the five-second mark timer, and the next publication plus local ingestion
falls after the prior reference's original deadline. Subsequent implementation
must close that publication gap without timestamp renewal, quality substitution
or retention of expired references. This deployment remains FAIL.

## Fourth implementation: completed-pool reference publication

One coalesced completion wake now connects the owned pool lane to the critical
reference worker, retaining the original timer as fallback. The worker publishes
the same qualified hub/economic batch atomically with original source clocks.
An independently owned accounting lane captures certified supplied-oracle and
hub/economic prices in one repeatable-read snapshot. Missing supplied values,
mismatches, lost quorum and original age bounds remain unavailable. Reference
production cannot wait behind fee backfill, rollups, idle candles or TVL work.

Both controlled predecessor schedules fail: the original timer phase leaves an
expired cohort at the consumer, and the serial accounting composition cannot
publish a completed replacement while accounting is blocked. The corrected
schedule passes both without renewing timestamps. The new SQL regressions are
shared by the focused native target and the required ingestion CI target.

Initial compatibility verification also exposed a fixture-phase error: its
200ms cutoff canceled before the pool commit it intended to preserve. The
fixture now has a finite two-second setup barrier for the actual committed
reserve/height and delayed book request, followed by its unchanged 200ms book
cancellation window and all original assertions. Production deadlines and code
were untouched by that fixture correction. The original failed/interrupted logs
are retained separately; owned QA infrastructure was reset before repeating
verification on frozen source.

Two further compatibility failures were confined to test setup. An unused global
tracing initializer in the new shared SQL case conflicted with the integration
suite's existing initializer; it was removed. A deliberately backdated recovery
fixture started its original clocks before cold configuration setup, which could
consume the remaining freshness window. Configuration is now primed with an
incomplete, non-executable observation before installing the controlled original
clocks. Original expiry, negative-policy and zero-RPC assertions are unchanged.
Both failures remain archived as FAIL; all affected targets were rerun after a
fresh reset of owned disposable infrastructure. The production fingerprint stayed
identical across these test-only corrections.

[Fourth-source verification](source-verification-4.json) completed all selected
groups with no executed failures: native recovery (14), local quotes (14),
provider recovery (11), liquidity feed (12), small-market references (12) and
quote outcomes (17). Each verifier ran first and repeat groups against its
unchanged fingerprint. The local quote verifier had no skips; the others declare
separate live or funded scope, which does not establish deployed acceptance.

The full ingestion binary passed 23 tests twice, the standalone SQL target
passed three tests, and both dual-view recovery cases passed twice. The local
quote verifier executed 1,700 library, 196 integration and 1,072 frontend tests
across both passes, plus 80 controlled browser samples without provider requests.
The quote-outcome controlled corpus completed 96 windows and 4,250 rows with
zero failures. These are source fixtures, not production availability evidence.

[Initial fixture failure](source-verification-4-initial.json) and
[integration fixture failures](source-verification-4-fixture-integration.json)
remain distinct from corrected passes. Unaffected liquidity/outcome checks used
the identical production fingerprint before the last test-only corrections;
affected full targets and native/local/provider/reference verifiers completed
afterward. Deployment and live acceptance for this implementation remain pending.

## Fourth-source CI failure and fixture coordination

[CI472](ci-472.json) on `f331811f0634a94aae9cd021d86622b610440cf8`
passed 999 contract and 3,470 frontend tests plus the production build. Its library
suite failed the existing maximum-publication profile: the delayed publisher
remained unfinished at reader sample 101, exceeding the unchanged 100-sample cap.
The 500ms useful-quote assertion did not fail. The outer suite recorded 848 passes,
one failure and one ignored test; its nested child reports the same failure.
The required PostgreSQL stage did not execute. This CI result remains FAIL.

[Command and timing evidence](ci-472-concurrency.json) confirms that hosted CI
uses the default parallel Rust harness, while the earlier local full verifier
used one test thread. Other outer tests completed during the child profile's
approximate execution interval. CPU utilization was not sampled, so contention
as the specific cause remains an inference. The maximum profile's parent had
not joined the existing serial group used by heavy publication, discovery and
fee-registry fixtures.

The narrow correction joins that existing fixture group before spawning the
child. Unmarked tests remain parallel: this coordinates shared heavy fixtures
without claiming complete host CPU isolation. The child still owns its private
registry and one Tokio worker. The 512-pair × 2,048-order workload, 100-sample cap,
40ms pacing, original twenty-second expiry and 500ms useful-quote budget are
unchanged. No runtime code, CI runner, policy, gate or service is changed.
Full default-parallel library first/retest and genuine hosted CI are required
after this correction; earlier local passes do not erase the hosted failure.

[Corrected fixture verification](source-verification-4-ci-fixture.json) passed the
full default-parallel library twice (849 passed, zero failed, one existing ignored
per run). Discovery and native verifiers then passed first/retest on fresh owned
QA infrastructure: 20 and 14 groups respectively, no executed failures. Their
separate live/funded scope remains explicitly unverified. The only Rust change
after the previous source commit is in the `cfg(test)` publication test module;
new fingerprints describe this corrected fixture snapshot. Hosted CI and live
acceptance for this final head remain pending.

[CI474](ci-474.json) on `aa730a361ee35b81638c339d57f8e66c3f4980fb`
again passed 999 contract tests, 3,470 frontend tests and the production build,
then failed the same maximum-publication 100-sample cap. Its child took 16.56s
and the outer library suite recorded 848 passes, one failure and one ignored
test. The 500ms assertion did not fail; PostgreSQL tests did not execute.
The serial fixture guard was insufficient. This result remains FAIL, and no
deployment occurred. Subsequent work must resolve the bounded preparation
problem without relaxing the profile or treating local passes as hosted success.

## Canonical immutable-view preparation

Preparation previously formatted and hashed every resting order again even when
the incoming snapshot reused the same immutable view objects. It now captures
only the current namespace's existing digest-map Arc under the short Store lock.
Off-lock, a digest may be reused only for matching address, pool/hybrid kind and
a live Weak binding to the exact incoming Arc. Changed, detached, dead or unknown
bindings receive a fresh full hash. There is no new global cache or retained
strong mirror ownership, and the existing two CPU slots remain unchanged.

Base namespace/reference/policy identity and final actor/admission identity are
recomputed normally. Commit-time fee, configuration epoch, negative actor and
newer-view guards still apply. Reuse changes preparation work, not canonical
bytes, source clocks, quality, quote expiry or signing permission.

[Deterministic work and identity evidence](immutable-view-hash-proof.json) keeps
the former full-hash control FAIL: 17 hashes rather than the one changed view.
Fixed tests compare cached preparation against fresh full hashing, reject all
invalid binding classes and retain fresh authority fields and original clocks.
Six generation and two publication controls pass. Independent final source
review found no correctness findings.

The unchanged 512-pair × 2,048-order maximum profile passes twice locally, each
with two useful HTTP quotes and a maximum of 3ms, within the same 100-sample cap
and 500ms budget. Full default-parallel library runs pass twice: 853 passed, zero
failed and one existing ignored case per run. These are controlled source
measurements; new hosted CI and focused deployed checks remain required.

A later [provider-verifier book failure](source-verification-4-hash-book-failure.json)
remains FAIL: 21 book cases passed and the healthy-reserve predicate in
`slow_book_is_bounded_healthy_pair_refreshes_and_timeout_recovers` exceeded its
400ms deadline. The first ingestion and health binaries had passed. The retained
run does not identify CPU, database or provider delay as its cause. Source review
shows that this outer timer starts before cycle setup SQL and the separate
500ms per-pair guards. The test calls the Combined snapshot helper directly;
it does not use local quote publication or the new hash-reuse path.

Other frozen-source verifiers completed before edits: local quotes passed all
14 groups without skips, including 1,708 library, 196 integration and 1,072
frontend test executions. Attribution and reference checks passed 21 and 12
groups respectively with zero executed failures. Controlled local browser
first/retest passed 40 samples each, at most 80.5ms/49.2ms, with zero provider
requests. These results do not erase the separate book failure. Owned QA was
reset before a test-only phase correction and controlled prelude checks.

[Controlled book-phase proof](book-phase-proof.json) demonstrates the clock
distinction with a confirmed 450ms blocked inventory SELECT: the old cycle-start
400ms predicate fails, while the initialized-phase version passes. The fixture
now waits within a finite two-second setup budget for the delayed-book request,
then applies its unchanged 400ms healthy-reserve condition. The original 500ms
per-pair deadline continues running; at the healthy observation, provider
cancellations and failures must be zero and a request must remain in flight.
The original 50ms height, 5,000ms LCD and 3,000ms slow-book settings and all
pool/hybrid/recovery assertions remain unchanged.

No controlled lock remains in the final fixture. Its complete book binary
passes 22 tests twice. All 1,162 application source files match before/after
this test-only correction, so the completed quote, attribution, reference and
discovery coverage still describes the same application bytes. Their global
test fingerprints remain recorded separately. Full provider/native verifiers
are being repeated after another fresh reset of owned QA; the earlier
provider-verifier FAIL remains retained.

[Final hash-source verification](source-verification-4-hash.json) passes all
selected first/retest groups: native recovery 14, provider recovery 11, discovery
20, local quotes 14, attribution 21 and references 12, with zero final executed
failures. Local quotes has no skips; the other verifiers declare separate
live/funded scope. The final provider run includes 23 ingestion, 22 book and
19 route tests per pass. Indexer Clippy exits successfully with existing warnings
outside the changed modules. All earlier failed runs remain distinct.

Main advanced during verification with an incident archive and a reviewed probe
cadence change from 40ms to 100ms, retaining the 500ms useful-response budget,
100-request cap and full workload. The stricter 40ms hash-reuse proof above is
retained separately. The integrated tree must preserve the new main commits and
repeat its affected maximum-profile/default-parallel checks before hosted CI
and deployment; this source receipt does not claim those future results.

[Main integration verification](source-verification-4-main-integration.json)
preserves main `6a3ab090af45ba1192791b455dcc6ab69078c65a` in tested merge
`b070b08f1a85918596ef92e4bbf0f2a4b3a29c6d`. The only delta under application
source directories is the reviewed `cfg(test)` probe cadence and elapsed output;
production hash/publication modules match the reviewed feature exactly.
Full default-parallel library checks pass twice (853 passed, zero failed, one
existing ignored each). The full maximum profile passes twice at main's 100ms
reader spacing: two useful quotes and at most 2ms each, with the original
500ms/100-request/full-workload constraints. Its reported publication elapsed
value includes the profile's mandatory reads and sleeps; it is not isolated
CPU preparation time. Formatting passes. Genuine hosted and deployed evidence
for the final PR head remains pending.


The preceding application-byte equivalence covers only its named M4 fixture
revision. M5 subsequently changes wrapper, local quote, ingestion, LCD and book
runtime bytes; none of that earlier whole-application equivalence is carried
forward as M5 coverage. M5 records its own source fingerprints and checks.

## Fourth deployment: source PASS, live FIRST FAIL

[PR #1514](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1514) merged
normally as `358475f4cf71a3089f88259ee27339850512bacd`; its entire tree matches
CI479 head `757f2217df5ccfa5d697e23dcbec8f4ac2ed510f`. Both checkouts were
synchronized. [CI479](ci-479.json) passed 999 contract, 3,470 frontend, 853 library
and 86 PostgreSQL tests, zero failures and two existing ignored tests. Required
aggregate checks passed without retries or policy changes.

[Exact deployment attestation](deployment-1514-attestation.json) passed for both
Coolify services. Frontend finished at 14:18:56 UTC and backend at 14:22:46 UTC
on 2026-10-15. Their 59/291 deployment log records contained no build-failure
markers. A separate invalid-input API control passed HTTP 400 typed rejection,
correlation, CORS and no-store checks. Useful probes began after the three-minute
backend warm-up.

[Main CI480](ci-480.json) also passed all required stages with the same counts,
separately from deployed availability.

The [190-second socket FIRST](deployment-1514-socket-fail.json) remains **FAIL**:
135 useful responses and 11 missing-observation HTTP 503s. Its 134 initialized
useful samples stayed within 306.331ms, but sampled recovery reached 10.382s.
No unchanged RETEST ran. The [interrupted browser FIRST](deployment-1514-browser-interrupted.json)
completed one desktop window: 24 useful responses, one native-input 503 and
372ms initial useful paint. Partial phone output was 2/2 useful at 377.6ms.
No expiry or ownership flags were observed. The run stopped on the independent
socket failure; remaining windows, RETEST and separate forced-expiry control did
not run. This is incomplete failed acceptance, not a live pass.

[Bounded correlation](deployment-1514-diagnosis.json) distinguishes separate patterns. Poll 30 returned no tax
rate after a typed `tax_rate/provider_unavailable`; poll 35 lacked the USTC cap
without an explicit typed cap failure. The source's two-second output subdeadline
may clip a permitted provider attempt, but the retained cap log does not prove
that happened in this poll. In the third cluster, selected native-input/output
IDs and browser ID `b812ccb7b7d0ccd5` show wrapper-only stale witnesses retaining
the original 14:28:05.700 UTC observation. The next scheduled poll exited at
14:28:15.700 with chain/provider-unavailable and zero elapsed milliseconds; the
following poll recovered at 14:28:30.962 with original 14:28:25.700 UTC.
Reference staleness is not the witness in those correlated requests.

The [early producer census](deployment-1514-wrapper-census.json) has 16/18
complete routed-native observations. Incomplete observations are preserved as
evidence; direct floor support remains unavailable and cannot authorize signing.
Implementation continues on bounded producer scheduling and zero-attempt
provider recovery. No old fields are spliced into new observations and no source
clock, provider/work limit or execution permission is extended. Issue #1505 stays
open; all earlier failed receipts remain retained.

The bounded historical census records identity quarantine and 25 all-circuit-open
events immediately before the zero-millisecond failed wrapper poll. This supports
an admission-gap mechanism without claiming a per-poll provider-state snapshot.
Across the live interval, the reference publisher records 19 pool-completion
wakes and 46 atomic mark commits (five economic marks each, at most 13ms), with
no hub-refresh failure event. Current provider counters taken afterward are
explicitly separate from historical state.

## Bounded native producer recovery follow-up

Two controlled old-behavior checks reproduce immediate identity/data failure
while a provider cooldown expires inside the unchanged total deadline. The
follow-up carries typed private scan deferral and waits only after a complete
zero-attempt scheduled scan. Ordinary backoff and half-open admission remain
authoritative; actual attempts never restart their budget. Every rescan, paced
admission and identity metadata/attempt wait stays within its original absolute
deadline. A once-per-call fixed identity/data circuit diagnostic adds no private
path or provider data.

Independent tax/cap reads start after validated treasury bindings alongside
mapping/rate work. Caps attach only to mappings validated in that same poll.
The artificial two-second output subdeadline is removed in favor of the original
7.9-second publication margin; the eight-second outer poll and existing read,
provider and cadence limits remain unchanged. Capability reads retain their
bounded two-second phase after critical mapping and output reads complete.

Review also found a known-negative bug: positively observed mapper/treasury
pauses previously returned the same absence as a transient failure, leaving the
old wrapper until original expiry. Typed pause outcomes now cancel unrelated
bindings/router work and revoke the prior wrapper. A negative record shares the
existing 32-namespace cap with positive views and persists until a validated
positive. Solve/progress return existing HTTP 422 `unsupported_semantics` so the
client cannot mistake this known negative for recoverable provider loss and keep
an executable display. Cached/deeper quote publication still requires the current
wrapper. Unavailable reads cannot restore permission or renew a clock.

The initial focused LCD run remains FAIL: ten tests passed and the mixed-capacity
fixture failed because two identical URLs were correctly deduplicated before it
indexed a second endpoint. The fixture is corrected to use distinct servers.
The controlled failures and first-run failure remain separate from final source
verification, which is pending. No live acceptance is claimed for this follow-up.

A later focused LCD run remains FAIL with 12 passes and one fixture failure:
the near-deadline case selected a preexisting immediate identity mock instead of
the newly mounted delayed response. Its expected timeout therefore did not occur.
Only the owned mock server setup is reset before mounting the delayed identity;
the original deadline and assertions remain unchanged.

The first wrapper run also remains FAIL: ten controls passed before a real-HTTP
case overflowed its ordinary test-worker stack. Concurrent phases had inlined
large transport futures into the surrounding future state. The observer now
boxes the read before creating its returned async state, retaining owned
cancellation and the same bounded reads. No task is detached and no stack limit
is increased. A deliberately large-future control checks that observer state
remains small; the real-HTTP suite still runs with the ordinary stack. Final
checks are pending.

The first FIFO-dispatch run fixed the healthy 2.2-second output case but retained
one failure among 27 controls: the ordinary 800ms provider case lost treasury
floor capability because its two-second phase included a wait behind queued bank
work. Critical mappings, tax, router and caps were complete. The floor phase now
starts after critical work finishes, retaining its original two-second maximum
and the whole-poll margin. Existing positive-capability assertions remain intact;
this failed run is not relabeled. Final full focused/source checks are pending.

The final wrapper focused FIRST passes all 29 controls on the ordinary stack:
healthy 2.2-second tax/cap observations preserve both native directions, both
direct-floor capabilities and exactly 13 data reads within the original margin.
Slow unrelated-denom isolation, queued-expired-floor refusal, production future
state size, malformed-response diagnostics and immediate treasury-pause
cancellation passed. The following final receipt records the subsequent
unchanged-source RETEST; no deployment acceptance is inferred.

[Final native observation proof](native-observation-proof.json) records 29 passes
on both FIRST and unchanged RETEST (61.66s/61.56s), preserving all prior failures.
[Scheduled circuit proof](circuit-recovery-proof.json) records 13 passes twice
with original identity/data deadline, cancellation and attempt-limit controls.
[Cached native pause proof](native-pause-proof.json) retains the former
pause-as-unavailable control FAIL (zero passed, one failed), then fixed FIRST
and RETEST PASS. The source file was restored byte-for-byte before fixed checks.
Known pauses reject cached/native solve and progress with 422, including the
interleaving where revocation arrives after cloning the old wrapper. Independent
final review found no remaining LCD, dispatcher, floor or pause-consumer issue.
These are source controls; full compatibility verifiers and the next deployment
remain separate. Owned disposable QA was reset again after the final fixes.

The broader wave stopped on an [executed head-recovery failure](source-verification-5-head-failure.json)
in the chain-attestation verifier: 21 book tests passed and
`issue_1453_third_provider_recovers_known_height_within_head_budget` retained
height 699 instead of 700. Its 100ms attempts, 400ms logical/head/pair budgets
and two delayed providers remain unchanged. The retained height alone does not
identify the failed phase. Separate book passes do not waive this failure.
The [liquidity verifier](source-verification-5-liquidity-interrupted.json) and
[local-quote verifier](source-verification-5-quotes-interrupted.json) were
interrupted without acceptance claims. All owned processes stopped before the
disposable QA database was reset for bounded phase diagnosis; no shared
infrastructure was reset.

[Bounded head diagnosis](head-failure-diagnostic.json) leaves the original phase
unknown. On fresh quiet QA, the unchanged limits allow the third head at 244ms
and pool/fee/book work through 272ms, publishing raw and hybrid height 700 by
283ms. A controlled 140ms healthy-head handler stall instead exhausts its 100ms
attempt: all three providers fail, no pool/fee/book request starts and height
699 remains. This proves a diagnostic signature, not the original cause. The
instrumented spy setup failure is also retained. The original book file was
restored exactly; no runtime change or deadline relaxation was made. Full book
first/retest and remaining verifiers will run without overlapping timed stages.

The restored full book FIRST also fails quietly: 21 passed and the same height
assertion failed in 27.72s; RETEST did not run. This rules out treating shared
load as a sufficient explanation. The source and 100/400ms constraints were
unchanged. Diagnosis now instruments the full sequence to distinguish preceding
fixture/state effects from head or pair failure; no source fix is claimed yet.

A separate deterministic cleanup control identifies a real ownership defect:
SQLx transaction drop queues rollback, while the pool's public `after_release`
hook runs before that queue is flushed. Holding that release exposes three old
failures without changing 100/400ms limits: normal completion returns with both
1453 locks held and the next pass makes zero provider attempts; partial lane
denial retains the first acquired key; an original 42P01 SQL error is preserved
but both locks remain held. All three old controls fail. This proves the lifecycle
defect, while the exact phase of the historical height failure remains unproven.
The fix awaits owned transaction cleanup before normal/error returns, with the
original error taking precedence. Cancellation retains the existing RAII fallback.
The negative SQL control uses connection-local `search_path=pg_catalog` in an
owned disposable database; it never drops or renames shared fixture tables.
The following final receipt records fixed controls and full-sequence verification.

[Final cleanup proof](combined-snapshot-cleanup-proof.json) records all three
controls passing twice, followed by full book FIRST/RETEST (25 passed each) and
default-parallel library FIRST/RETEST (877 passed, one existing ignored each).
Both library runs include the maximum-publication profile. Independent final
review found no ownership, cancellation, source-clock or budget issue.

[Verifier status controls](verifier-status-proof.json) also reproduce a separate
agent-facing defect: the old grouped Rust/QA function could report PASS after an
earlier command failed. Explicit failure returns now stop those groups. The
no-network regression checks every failure position and success for Rust, DB
and QA. Prior recorded passes were supported by individual passing results;
this defect does not turn any retained failure into a pass. The focused verifier
now includes the three cleanup DB controls and ten Python control tests.
Final serialized compatibility verifiers and deployed acceptance remain pending.

The first database-isolation run correctly rejected an incompletely provisioned
authenticated runtime QA fixture: its default `postgres` maintenance database
still granted CONNECT through PUBLIC. The [fixture failure](source-verification-5-runtime-fixture-failure.json)
remains FAIL (library 878 passed; isolation five passed, one failed). The private
provisioner now revokes PUBLIC CONNECT on maintenance databases before enabling
the disposable runtime login. After resetting only the owned cluster, a direct
catalog check confirms zero accessible unrelated databases before any temporary
fixture grant. No production permission or application source changed. The full
affected verifier and subsequent stages are being repeated on that corrected
fixture; the already-passing native/identity/discovery checks retain their
unchanged application-source scope.

The corrected database-isolation verifier subsequently passed 28 groups with
zero failures. Its unavailable default Docker bridge host-forwarding prerequisite
and host-disabled loopback IPv6 remain explicit network scope skips; internal,
external-namespace rejection, opt-in IPv4 host mode and fresh role ACL checks
passed. The [host prerequisite record](source-verification-5-host-network-unavailable.json)
preserves the earlier interrupted probe and does not establish its root cause.

Independent review then identified an unbounded metadata cleanup await after an
identity attempt timed out. The remaining compatibility wave was
[intentionally interrupted](source-verification-5-identity-review-interrupted.json)
before changing source. No assertion failed in that interrupted wave. Provider
quarantine and metadata invalidation must both preserve their semantics while
completion respects the original total deadline. This is a separate boundary
defect; it does not establish the cause of a historical live failure. Subsequent
source verification must record its own fingerprint and cannot inherit the
preceding run's unchanged-source claim.

[Identity cleanup proof](identity-deadline-proof.json) records the old regression
as FAIL: the 400ms total-budget call remained pending at 500ms while metadata was
locked, after exactly one HTTP read. The fix records failure synchronously on the
owned provider and disarms duplicate permit accounting before deadline-bounded
metadata invalidation. FIRST/RETEST of the complete LCD module pass 108 tests
with one existing ignored benchmark (12.42s/12.55s), using default stack and
parallel harness settings. Closed and half-open cancellation/deadline controls
each record exactly one attempt and one failure; no stale verified metadata can
authorize a new read. Independent read-only review found no remaining concern.
The owned QA cluster was reset before restarting applicable source verifiers on
the final frozen application fingerprint
`36587444f139b0fbe5b15e36116af9f277efd772b65e740dc1280669dc603f07`.

## Fifth candidate: final source verification

[The final source receipt](source-verification-5-final.json) records a serialized
2026-10-15 17:09:54–17:49:47 UTC wave on the frozen implementation. Every selected
runnable group passed; no source changed between first and retest.

| Verifier | PASS | FAIL | Explicit scope SKIP |
| --- | ---: | ---: | ---: |
| Native #1505 | 14 | 0 | 1 |
| Chain identity #1392 | 11 | 0 | 1 |
| Offline quotes #1452 | 14 | 0 | 0 |
| Quote lifecycle #1499 | 10 | 0 | 1 |
| Liquidity feed #1453 | 12 | 0 | 2 |
| Provider recovery #1464 | 11 | 0 | 2 |

Clippy exits zero with unchanged existing warnings. The full library passes
879 tests with one existing ignored benchmark; applicable verifiers separately
run that benchmark. Controlled production-build browser FIRST/RETEST each record
44 quote requests, zero provider requests and four scenario receipts. They prove
the controlled offline boundary, not deployed latency or availability.

The application fingerprint covers actual `smartcontracts` workspace roots as
well as indexer/frontend/package source, including test-only source. Its final
SHA-256 is
`36587444f139b0fbe5b15e36116af9f277efd772b65e740dc1280669dc603f07`;
the verification-input fingerprint adds tests, scripts and explicit package/
toolchain inputs and is
`e6d406cc92e61d076ec61af6c39d88b0609f08c63b1eb04b34cc32a2c5882f3b`.
Documentation, receipts, private env and generated build/cache files are excluded.

Discovery #1394 (23/0/0) and database isolation #1396 (28/0/2) completed before
the isolated identity-cleanup delta and retain that preceding-source scope in
the receipt. Database infrastructure and discovery code did not change afterward;
the final identity/native/quote/feed verifiers cover the changed boundary.
Authenticated role isolation passed twice. The host bridge positive-query and
host-disabled IPv6 prerequisites remain unavailable, with failed/interrupted
diagnostics retained. Separate deployed/history/funded/treasury scopes are not
claimed by source passes. Exact-revision hosted CI and live acceptance remain
pending.

## Fifth deployment: source and CI pass; browser FIRST fails

[PR #1516](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1516) merged as
`bcfaa77f7940fa4385436abb1b68c54cdf623402`; its entire tree matches the tested
`0525b001` head. Genuine [PR CI 481](ci-481.json) and [main CI 482](ci-482.json)
both pass: contracts 999, frontend 3,470 tests across 373 files, library 879 plus
one ignored, and PostgreSQL 86 plus one ignored, with zero failures.

Both Coolify services are [exactly attested](deployment-1516-attestation.json).
Frontend finished 18:16:03 UTC and backend 18:19:51 UTC on 2026-10-15. Final logs
contain 58/305 lines and zero build-error markers. Independent backend warmup
completed at 18:22:51 UTC before any selected useful measurement.

The [190-second socket FIRST](deployment-1516-socket-first.json) passes all 146
selected useful native results. Its 145 initialized results have p95 307.346ms,
p99 314.933ms and maximum 316.041ms; both revision-health reads match the deployed
revision. This pass does not override the later browser failure.

[Browser FIRST](deployment-1516-browser-fail.json) retains an active CL8Y→USTC
phone HTTP 503 `missing_observations`, request `c659a12239334730`, received at
18:35:10.499 UTC with 7ms CDP delivery lag. Selected totals are 133 useful and
one unavailable response, with zero teardown cancellations. The three complete
190-second windows each pass 35/35 useful responses; useful paints are
382.2/374.4/381.2ms. The partial fourth window starts with a 376.1ms useful paint
and records 28 useful plus the failed response. No expiry, original-window,
geometry, response-probe, long-task or frame-gap flag is observed. Ownership
qualifications do not establish unique adoption or positive callback evidence.

The prior useful response's original expiry was 18:35:11.205 UTC, 706ms after
the unavailable response. Display was first sampled blank 41ms after that
expiry, then recovered with a fresh response at 18:35:21.394 UTC; sampled
recovery is 10.895 seconds. These are retained observations, not a timer-callback
acceptance pass. RETEST and separate expiry controls did not run; all owned
browser/Node processes exited.

The [diagnosis](deployment-1516-diagnosis.json) correlates poll 96's original
18:35:01.204 UTC start and 18:35:09.106 completion at the 7.9-second publication
margin. It publishes one mapping, positive tax and router, but both caps absent,
with mapping/cap deadline events. No stale-component witness accompanies the
selected rejection. Poll 95's complete original observation remains fresh until
18:35:11.205; poll 97 completes at 18:35:17.443. Retaining the preceding complete
view alone cannot bridge that original-expiry gap. The inspected ten-second
slice has no correlated admission/circuit/429/parse event. Mapping denom,
successful per-read phase times, and queued-versus-active delay remain unknown.

Separately, the unchanged v8 global 600-read cap stops the last phone window at
139.626 seconds, 19.790 seconds after the actual 503. Counts are 139 solves and
461 progress calls. The cap does not cause or excuse the earlier application
failure, and the last window remains incomplete. Its bounded observation
protocol needs separate correction before another complete four-window cohort.
No latency, original-input/expiry clock, provider budget or signing rule is
relaxed. The 15-minute Loki receipt is collected within the diagnosis; broader
uncorrelated patterns do not establish cause. Prometheus remains explicitly
unavailable because no datasource is provisioned.

## Sixth candidate: native isolation and bounded observer corrections

The [controlled per-denom proof](native-denom-isolation-proof.json) retains an
old-code FAIL under the original provider and poll limits: both mappings were
present but both caps absent at 7.901 seconds, after eleven data reads and zero
provider failures. Two unrelated same-denom reads could occupy both local slots.
The unchanged fixture passes after adding one poll-local mapping/rate/cap slot
per known native denom, acquired before the global two-read dispatcher. USTC's
mapping, cap, tax and router complete; the slow sibling stays incomplete without
retained fields or renewed clocks. Fixed first/retest use ten reads and preserve
the original publication margin. This control's two-mapping failure differs from
historical poll 96's one mapping and does not establish its exact queue state.

Symmetric paused-clock controls keep one blocked-denom read active while the
other denom's three reads complete in 60ms; all permits release on cancellation.
Full wrapper FIRST/RETEST pass 32 tests each using default stack and parallel
harness settings, in 18.62s/18.18s. Intermediate controls briefly overlapped an
unrelated local Chrome QA group; final full runs began after it exited. Thirteen
healthy data reads, two global/provider slots, four RPS, original attempt/total
limits, ten-second cadence, 7.9/8-second bounds, original twenty-second expiry
and fail-closed negative semantics remain unchanged.

Failed physical reads now record fixed slot/phase/reason and per-denom dispatch,
global dispatch or logical LCD stage under their original poll span. Capped
queue/active/poll durations and `lcd_polled` distinguish local waiting from LCD
future polling, which can still include internal admission/identity waits and
does not prove a wire attempt. Successful reads remain silent. Existing aggregate
phase deadlines have no slot and are separate evidence; no body, address, actor,
private endpoint or arbitrary label is exported. Parser privacy and consistency
controls exercise all stage/boundary cases without adding monitoring queries.

The [v9 budget controls](observer-v9-budget-proof.json) preserve the actual v8
503 and incomplete fourth window. The complete browser windows used
152/162/151 reads; four at the largest observed count plus initialized probes
would need 652, while projection of the partial phone gives 648. Native v9's
finite 800-call global allowance includes initialization and all setup, solve
and progress traffic. Each context retains the shared 600 maximum and receives
only remaining allowance; local or global exhaustion remains incomplete/FAIL.
Generic cohorts retain 600 per shard and 1,800 total. All four 190-second windows,
the sixteen-minute bound, original clocks, ownership, availability and 500/750ms
acceptance remain unchanged. The synthetic fixture failure is retained.

[Completion controls](observer-v9-completion-proof.json) also reproduce two
actual-driver classification failures: late budget exhaustion or earlier cutoff
plus response drain could report a complete window. Overall was already FAIL,
so this was not a false acceptance pass. Explicit natural active completion now
keeps budget exits/setup/active exceptions incomplete, independently of drain
time, while a full-duration window with an application failure remains complete
and FAIL. Focused FIRST/RETEST each pass fourteen controls. Prior budget proof
hashes describe the earlier control snapshot; this completion proof records the
final driver hash and supersedes its pending follow-up note.

The [monitoring cap proof](monitoring-read-budget-proof.json) retains another
old-profile failure using the real Reader/Monitor/CLI with an in-memory opener:
the former sixteen-read allowance cannot perform requested correlation plus
optional Prometheus. Native monitoring now explicitly opts into a fixed eighteen
reads, including credential bootstrap. Shared monitoring and deployment
attestation retain their sixteen-read defaults. All optional combinations use
the actual cap, and call nineteen is blocked before transport. Queries,
300-line/eight-ID bounds and raw-data exclusions remain unchanged. The fixed
CLI passes first/retest without network requests; synthetic failure receipts
and source hashes remain in the proof.

Independent read-only review reports no remaining findings in the runtime gates,
captured failure witness, explicit observer completion or real monitoring cap.
The [final sixth-source wave](source-verification-6-final.json) runs serially on
2026-10-15 19:27:47–19:52:47 UTC and passes every selected runnable group:

| Verifier | PASS | FAIL | Separate scope SKIP |
| --- | ---: | ---: | ---: |
| Native #1505 | 14 | 0 | 1 |
| Offline quotes #1452 | 14 | 0 | 0 |
| Quote lifecycle #1499 | 10 | 0 | 1 |
| Provider recovery #1464 | 11 | 0 | 2 |

Clippy exits zero with the same 155 existing warnings. Full library FIRST/RETEST
pass 882 tests with one existing ignored benchmark in 110.42s/110.28s; applicable
checks run the benchmark separately. Controlled production-build browser runs
each record 44 quote requests and zero provider requests. These controlled
receipts do not establish production latency or availability.

The final 1,205-file application fingerprint is
`05c319fdd857b6045e694a6b17dece7f21f219c612445a7f7dee1f9d8a2bfef6`;
the 2,007-file verification-input fingerprint is
`99880e0cb17e926be43cd18280138dca68d2814375db6576ece5c095890db4a0`.
Both are identical before/after the entire wave, using the actual smartcontracts
workspace roots and excluding docs, receipts, private env and generated files.
Only wrapper producer source changes at runtime relative to the fifth deployed
revision. Unchanged infrastructure checks retain their prior scopes rather than
being relabeled as final-head evidence. At this source checkpoint, hosted CI and
deployed acceptance were pending. The sixth-deployment section below records
the subsequent CI/deployment PASS and actual live FAIL.

## Sixth deployment: complete wrapper, expired reference, retained live FAIL

[PR #1517](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1517) merged normally
as `0b002042fbf6dd37f1e8577acbd995b367fe2b30`. Main and the owned worktree were
fast-forwarded; the entire merge tree equals tested head `ffab872e`.
[PR CI 483](ci-483.json) and [main CI 484](ci-484.json) pass all required stages:
999 contract, 3,470 frontend, 882 library and 86 PostgreSQL tests, with one existing
ignored library case and one existing ignored database case recorded separately.
No retry or policy change was used.

Both Coolify deployments finished at the exact merged revision, frontend at
20:18:42 UTC and backend at 20:22:29 UTC. Their final 58/296 log entries contain
zero build-error markers. [Exact attestation](deployment-1517-attestation.json)
passes, and the independently measured backend warmup ended at 20:25:29 UTC,
before the acceptance probes. [Socket FIRST](deployment-1517-socket-first.json)
passes 146 useful native results over 190 seconds with no unavailable response;
its 145 initialized useful results have a 306.747ms maximum, 305.052ms p95 and
305.672ms p99. Two revision/health reads are recorded separately.

The browser process started at 20:27:24.854 UTC; the
[Browser FIRST receipt](deployment-1517-browser-fail.json) starts at 20:27:27.838 UTC,
using unchanged final native v9 helpers.
Native-input desktop and phone each complete their 190-second window with 37/37
useful selected responses. Original-input useful paints are 614.3ms and 410.2ms.
Reverse desktop completes 190.123 seconds with 37 useful responses and one actual
`missing_observations` HTTP 503, so the run remains **FAIL** despite its 378.2ms
initial useful paint and later recovery. No expiry/validation flags were found
in those three completed windows. The fourth window is deliberately stopped and
remains incomplete; no unchanged RETEST or expiry control is performed. The
failed-window checkpoint used 435/800 reads, so no observer budget was exhausted.
All owned Node/Chrome processes exited.

The [bounded diagnosis](deployment-1517-diagnosis.json) preserves the selected
failure ID `016dc250827b0354`, arriving at 20:36:45.621 UTC; server
rejection is 20:36:45.480880 UTC. Its only stale candidate witness is reference,
age 20,448ms and 448ms past original expiry. Wrapper poll 90 completed at
20:36:44.463 with both mappings/caps, tax and router, using its original
20:36:39.363 observation time. Nearby wrapper polls 86–96 are complete; their
floor-contract rejections concern the optional capability only.

Reference commits at 20:36:39.741 and 20:36:44.745 still carry an oldest economic
clock of 20:36:25.032, with newest 20:36:35.031. The preceding pool cycle refreshes
21/22 and records one historical-height failure after 794ms. Its failed pair is
a documented official CL8Y/cUSTC reference candidate, verified by exact token
identity in memory. This establishes candidate identity, not its current winning
source or the historical admission/attempt sequence. All reference clocks advance
to 20:36:45.031 at 20:36:49.744. Original clocks, qualification and expiry are
preserved; a successful publication does not renew an old source.

The last useful browser response before failure retains original expiry
20:36:45.032. The first sampled blank is 20:36:45.098, 66ms after that deadline and
before the 503. The next useful response arrives at 20:36:56.503 with a new original
clock; sampled recovery is 10,882ms. This is positive sampled display revocation,
not instrumented callback evidence, and does not excuse availability failure.
[Bounded monitoring](deployment-1517-monitoring-first.json) uses 17/18 reads;
Prometheus remains explicitly unavailable because no datasource is provisioned.
The private raw logs and any operational identifiers remain unpublished.

## Seventh candidate: preserve unused scheduled provider allowance

Source inspection identifies a separate bounded fallback gap. After one real
historical integrity failure, the provider scan can encounter an otherwise
qualified alternative whose shared capacity is briefly busy. It returns the
retained integrity failure immediately even though the original deadline and
configured actual-attempt allowance are not exhausted. The prior zero-attempt
scheduled-ingestion recovery loop cannot help: this query already attempted a
provider, and the actual pool lane uses the scheduled view without ingestion
pacing. Historical busy/attempt counts are unavailable, so this controlled defect
is not asserted as the unique cause of the selected production failure.

The old-code real-HTTP control retains an actual FAIL: one invalid historical
primary attempt, one latched safety callback, a qualified alternative capacity-held
for 300ms, and terminal historical-height failure after 2ms with zero alternative
data reads despite the original four-second/three-attempt allowance. The fixed
scheduled-only continuation keeps one cursor,
attempted-endpoint identity, one global actual-attempt count, original absolute
deadline, shared admission and integrity precedence; it may wait only for
capacity-denied alternatives that have not been attempted. Direct/request behavior,
provider quarantine, exact-height/chain checks and known-negative safety remain
unchanged. Focused FIRST/RETEST pass; broader source and deployed acceptance
remain pending.

Independent review also reproduces a post-metadata transport boundary defect.
The endpoint future awaits fresh chain attestation and its metadata write, then
can first-poll data transport after an elapsed deadline because Tokio timeouts
poll their inner future before checking elapsed time. Client-owned test counters
observe that first transport poll; server wire counts alone can miss an immediately
cancelled send. The three old-code controls all fail: the original 2.5-second
attempt edge polls one forbidden data future at 2,551ms; scheduled and direct
four-second total edges poll a second data future at 4,051ms/4,052ms, when only
the completed historical primary's first data poll is permitted.

Those closed-provider old controls separately retain the original health signatures: own-attempt
failure one/caller cancellations zero, scheduled shorter-caller failure zero/
cancellation one, and direct failure one/cancellation zero. The prior historical
safety callback remains latched in both total cases. Explicit pre/post-attestation
transport guards now preserve those signatures,
typed original-deadline origin and integrity precedence, without a new time budget
or an early error that accidentally disarms quarantine. Incomplete half-open
probes retain quarantine even for scheduled caller expiry. This source
boundary proof does not establish its historical production occurrence.

The [final scheduled-provider proof](scheduled-provider-continuation-proof.json)
retains each actual OLD failure, intermediate fixture/compile failure and final
source hash. The unchanged continuation control passes in 311–312ms with one
actual primary and one actual alternative attempt. Final nine-control
FIRST/RETEST pass in 4.40s each. Full LCD FIRST/RETEST pass 117 tests with one
existing ignored case in 12.35s/12.29s, using default stack/parallel settings and
fresh serialized Cargo processes. Three deadline controls now first-poll no
forbidden transport and preserve their original health/callback counts. The
source uses test-build-only client-owned counters for phase proof; production
adds no such counters or registry. Independent final review reports no runtime
findings. No owned jobs remain. The earlier 114-test LCD run precedes the deadline
guard and is explicitly separate from final-source evidence.

The monitoring parser adds only fixed scheduled-alternative diagnostics to the
existing bounded sample, with no new query. It accepts actual attempt/pending
counts within the existing 32-endpoint census and original remaining milliseconds
within the policy's 30,000ms maximum; unknown/missing/duplicate/inconsistent fields
are refused. Eighteen privacy/resource tests pass in preflight. The native source
verifier includes the new Rust suite and tests failure propagation at all nine
Rust commands. NR1505-10, NET1392, PR1464 and LF1453 document the narrow continuation
and deadline-origin rules. Broader frozen-source verification, hosted CI and new
live acceptance remain pending.

A separate [cached health census](deployment-1517-cached-provider-census.json)
at 21:04:23 UTC reads exactly one M6 health response, with zero quote requests or
provider probes. It lists three configured endpoints, two observed verified and
all three last observed on the expected chain. Cumulative failure/admission
counters show heterogeneous health and shared-capacity deferrals. This later
snapshot does not establish the selected failure's historical attempt sequence,
current active redundancy or provider ownership; no configuration is changed.
The diagnostic `circuit_open` field records retained backoff state, not a fresh
ordinary-admission test or proof that its deadline has not elapsed.

## Seventh source checkpoint: all existing checks pass, parent gap remains

The [nine-stage source checkpoint](source-verification-7-checkpoint.json) runs
21:02:24–21:53:18 UTC on unchanged application/verification fingerprints. Native,
Clippy, chain attestation, discovery, database isolation, offline quotes,
lifecycle, liquidity feed and provider recovery all pass their selected checks.
The authenticated disposable runtime fixture runs and passes twice. Database
isolation runs all 892 library cases, including the existing benchmark, twice
(125.48s/124.59s). Normal library runs retain their one separately exercised
ignored benchmark. Controlled browser FIRST/RETEST each record 44 quote requests
and zero provider requests; these are not deployed availability measurements.

Its application fingerprint is
2cc39c11237e0ef36321d4765103973a1815fb46d59659af94fce27385574633
across 1,206 files; its 2,008 verification inputs hash to
9f42df256b0d9041bbf8ef4526ee452d2681ef8a87cde650577befbc26701682.
Before/after match. Explicit host IPv6 and published-bridge prerequisites remain
unavailable; authenticated runtime verification is completed. Prior separate
treasury/deployed/funded scope skips remain explicit.

This is a checkpoint, not final release readiness. Further read-only review
identifies an uncovered enclosing deadline. A late wrapper read starting at poll
+7s gets LCD own/total limits of +9.5s/+11s. If successful identity metadata resumes
at +8s, both LCD guards still permit data polling despite the original +7.9s poll
margin having elapsed. The optional two-second floor and later pair/snapshot
phases have the same issue: their enclosing budget can expire before the LCD
operation's own clock. Identity-only probes can also return late positive
success after metadata waiting because their timeout polls the inner future first.

New OLD real-HTTP controls confirm the gaps. The LCD group retains three
passing safety/state controls and five actual failures: after a 300ms enclosing
deadline it first-polls forbidden data at 351ms; identity-only closed, direct and
half-open cases accept late positive metadata; the original 2.5s identity attempt
also accepts positive metadata at 2,551ms. Known typed-negative and minimum/shared
clone controls pass. Four actual producer controls all fail: wrapper dispatch
and head first-poll one data transport instead of zero, snapshot retry polls two
instead of the permitted completed prefix of one, and full wrapper floors poll
twelve instead of the eleven completed critical reads after the unchanged
two-second floor phase. These are first-poll counts, not inferred wire attempts.

The failures do not establish historical production occurrence. Implementation
returns to explicit scoped-deadline propagation before commit. The final fix must carry an owned original absolute deadline into LCD
before constructing futures, use the minimum at nested boundaries, and preserve
shared provider/pacing state, original own-attempt accounting and typed negatives.
Head scope cannot expire later pair jobs; pair scope starts inside each spawned
owner; snapshot retries scope once; floors retain their original poll clock and
dispatcher. No new global/task-local context, work allowance, cadence or expiry
renewal is permitted. The previous full source checkpoint and its logs remain
retained independently of the next final-source wave.

The same positive-result boundary also has three genuine OLD buffered-body
failures: parent data is accepted at 351ms after its 300ms deadline; an original
2.5s whole attempt accepts data at 2,552ms when a permitted 200ms identity read
makes its data-wire timer end later; and a valid but expired lower-head response
publishes height 800 and health success after the parent deadline. These controls
retain the original per-wire/whole-attempt limits and separate logical first-poll
evidence from wire counts.

One proposed control was mis-specified. It expected a timely actual head of 900
to leave the global watermark zero when the pool query needed 1000. The existing
minimum-head regression instead correctly distinguishes observed 100 from an
invented requested 101. Global lag tracking and pool-query eligibility are separate:
the timely actual observation remains valid global evidence. Suppressing it would
weaken rejection of a later genuinely lagging provider. The mistaken zero-expectation
OLD FAIL is retained as a control/contract-reading error, not an application defect.

The first broader LCD run exposed our corresponding runtime regression
(128 passed, one failed, one ignored). Its legacy assertion was left byte-identical;
the runtime restores actual-height publication after the new expiry guard and before
the existing query-local minimum rejection. The new timely-head control now requires
actual 900, never requested 1000, and rejects a subsequent 860 under the unchanged
twenty-block lag rule. Expired buffered heads still publish nothing. All earlier
failures remain retained; corrected focused/full FIRST/RETEST are in progress.

The [final parent-deadline proof](parent-deadline-proof.json) records the corrected
implementation, every earlier control/fixture failure and the unchanged legacy
minimum-head test. Focused LCD boundary FIRST/RETEST each pass 12 cases in 2.89s;
producer FIRST/RETEST each pass five cases in 4.97s/4.95s. Full LCD passes 129
cases plus one existing ignored case in 12.80s/12.82s. Full wrapper passes 35 cases
in 18.40s/18.83s. Fresh processes, default stack/parallel settings and one Cargo
owner are retained. Three independent read-only reviewers report no further
findings at the corrected hashes. Expired positives cannot publish; timely actual
heights retain the original lag policy and never invent a requested floor.

All public source/scripts are frozen for the new serialized verification wave.
Its 1,207 application files hash to
ed85965ee59cc0bb1db8882689e40df43ab6adf15f4b4b0d5d5933d727479d55;
its 2,009 verification inputs hash to
27d4556168a04b68534b82873552b567377ccd29a9e181b32683bd9dc842af1f.
Only the owned QA database was reset, with fresh private least-privilege QA and
authenticated-runtime fixtures. A fresh bounded bridge prerequisite again passes
internal TCP but cannot query host loopback; the other network checks remain
enabled. LocalTerra is running. Final source verification, genuine hosted CI and
new exact deployment/live acceptance are not yet claimed.

The final wave passes native recovery, Clippy, chain attestation and discovery
before database isolation stops on PostgreSQL 28P01. The private provisioning
helper had created the disposable runtime role without assigning its generated
password. This is a fixture failure, not an application failure, and remains
recorded. The helper now assigns the password and proves correct-password
authentication succeeds while a random wrong password fails, then revokes its
temporary control-database grant. Only the labeled, owned QA container and volume
are reset. No credentials are included in evidence.

Application and verification fingerprints match before the failed wave, after it,
and before the resumed wave. The unchanged-source passing prefix is retained;
database isolation and the four remaining stages resume serially on the repaired
fixture. Their final outcomes remain pending.

## Seventh candidate: final frozen-source verification

The [final source receipt](source-verification-7-final.json) combines the unchanged
passing prefix from 22:19:13 UTC with the repaired-fixture continuation completed
at 23:20:32 UTC. All four application/verification snapshots match exactly.
The original database FAIL remains in the receipt with its log digest and
remediation; no application code or assertion changed to repair that fixture.

| Stage | PASS | FAIL | Explicit SKIP |
| --- | ---: | ---: | ---: |
| Native recovery #1505 | 14 | 0 | 1 |
| Chain attestation #1392 | 11 | 0 | 1 |
| Pair discovery #1394 | 23 | 0 | 0 |
| Database isolation #1396 | 28 | 0 | 2 |
| Offline quotes #1452 | 14 | 0 | 0 |
| Quote lifecycle #1499 | 10 | 0 | 1 |
| Liquidity feed #1453 | 12 | 0 | 2 |
| Provider recovery #1464 | 11 | 0 | 2 |

Clippy succeeds with 155 existing warnings. Database isolation runs all 909
library cases, including the existing benchmark, twice in 143.67s/143.53s.
Authenticated runtime privilege checks pass twice. Controlled browser FIRST and
RETEST each record 44 quote requests and zero provider requests. Those controlled
fixtures do not establish deployed latency or availability. Host IPv6 and
published-loopback prerequisites, separate treasury identity, deployed/funded
acceptance and other verifier scope skips remain explicitly described in the
receipt; they do not substitute for the focused live #1505 criteria.

The final 1,207-file application fingerprint remains
ed85965ee59cc0bb1db8882689e40df43ab6adf15f4b4b0d5d5933d727479d55;
all 2,009 verification inputs remain
27d4556168a04b68534b82873552b567377ccd29a9e181b32683bd9dc842af1f.
Genuine hosted CI, exact merged deployment and fresh live acceptance are next.

## Seventh deployment: all windows complete, retained native-output FAIL

[PR #1518](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1518) merges
as `5f486532a73e6effb93a053a8d02029f78c07d4e`; its entire tree equals tested
head `687ece3466f049ba7dd589ece386443a5ba9307e`. [PR CI485](ci-485.json) and
[main CI486](ci-486.json) both pass every required stage: 999 contracts,
3,470 frontend tests, 908 library tests plus one ignored case, and 86 PostgreSQL
tests plus one ignored. The original maximum-publication profile also passes.

Both Coolify applications deploy the exact merge. Frontend finishes at
23:43:58 UTC and backend at 23:47:01 UTC. [Build logs](deployment-1518-build-logs.json)
contain 59/309 entries and zero failure markers. [Independent attestation](deployment-1518-attestation.json)
passes. Independent backend warmup completes at 23:50:01 UTC before native probes.
[Socket FIRST](deployment-1518-socket-first.json) completes 190 seconds with
148 reads including two health reads, 146 executable selected responses and
zero unavailable outcomes. Its 145 initialized useful samples have p50 295.365ms,
p95 305.272ms, p99 305.839ms and maximum 306.770ms, below 500ms.

[Browser FIRST](deployment-1518-browser-fail.json) starts at 23:55:40.606 UTC
and completes every 190-second window with unchanged v9 helpers:

| Direction | Viewport | Useful selected | Unavailable | First useful paint |
| --- | --- | ---: | ---: | ---: |
| USTC → CL8Y | Desktop | 21 | 0 | 366.6ms |
| USTC → CL8Y | Phone | 20 | 0 | 385.9ms |
| CL8Y → USTC | Desktop | 20 | 0 | 381.2ms |
| CL8Y → USTC | Phone | 19 | 1 | 368.7ms |

Overall remains **FAIL** despite 80 useful responses. All 304 of the 800-call
budget are accounted for, with zero blocked/rejected calls. There are no expiry,
ownership-qualification, probe, long-task or frame-gap flags. Sampled blank
27ms after the latest possible original expiry does not instrument adoption or
callback execution. All owned Node/Chrome processes exit. RETEST and positive
expiry controls do not run.

The [selected diagnosis](deployment-1518-diagnosis.json) preserves failure
`4463924d8b8a2c6d`, `missing_observations`, at 2026-10-16 00:06:09.907 UTC,
26,233ms after original input, with 9ms observed delivery lag. Previous useful
`4463924d8b8a2c6a` arrives at 00:05:59.907 with original observation
00:05:50.614 and expiry 00:06:10.614. Next useful `4463924d8b8a2c71` arrives
at 00:06:20.795 with observation 00:06:10.613 and expiry 00:06:30.613.
Sampled recovery of 10,888ms does not remove the failure.

[Bounded exact-ID monitoring](deployment-1518-monitoring-first.json) uses
17/18 reads; Prometheus is not provisioned. The expired private Bao session
initially blocks the CLI before any collector request; refreshed certificate
authentication permits the single collection. The selected ID has no stale
component witness, so it is not classified as reference or wrapper expiry.
Six scheduled-alternative witnesses occur elsewhere; none is within 60 seconds
of the failure.

A separate [fixed producer context](deployment-1518-producer-context.json)
uses exactly four bounded reads and 53 untruncated lines for 00:05:40–00:06:40.
Poll 119 completes at 00:06:08.515 with both mappings, tax, router and uluna cap,
but no uusd cap. Its cap deadline witness records poll elapsed 7,901ms,
denomination wait 4,218ms, global wait zero and logical LCD activity 646ms.
These are client-stage timings, not wire proof. Prior complete poll 118 expires
at 00:06:10.614; next complete poll 120 publishes at 00:06:16.764 with original
observation 00:06:10.613. The selected server rejection is at 00:06:09.765,
consistent with a fresh partial prerequisite. Reference commits and 22/22 pool
cycles are fresh nearby. Keeping the old view only until its original expiry
cannot bridge the next completion.

The live record does not prove the underlying provider delay or that a global
slot was idle. The next source control tests whether the fixed one-read-per-denom
dispatcher unnecessarily serializes remaining work after the other denom finishes.
Any correction must keep the two global permits, existing pacing, thirteen data
reads and original deadlines.

## Eighth candidate: work-conserving native dispatch

The [dispatch proof](dispatch-work-conservation-proof.json) uses the actual
producer and real HTTP responses under the original 7.9/8-second deadline,
provider attempt/total bounds, two-read global gate and shared pacing. The OLD
control fails at 7,900ms: both mappings exist but the required USTC cap is missing.
It measures 3,382ms with a global slot idle after all three ULUNA reads finish,
while USTC predecessors retain their own denomination gate. Only 11 data wire
requests start, with zero provider failures and one neutral caller cancellation.
This proves a source scheduling defect, without proving that production poll 119
had the same underlying provider sequence or idle interval.

The fix extracts a private poll-owned dispatcher. After validated bindings, it
plans each denomination's three required reads before future construction.
Construction-time completion tokens retire exactly once on completion, error or
drop, including unpolled futures. Duplicate/unbound registrations are refused.
While peer work remains, denomination isolation stays at one read. Only after
that peer finishes may queued remaining work bypass its busy denomination gate
and acquire the existing global gate. Notifications register before the monotonic
completion check. No-plan callers retain strict isolation. There are no new
requests, provider permits, tasks, queued global work or time/freshness allowances.

The original fixture passes standalone FIRST/RETEST in 7,022ms/6,671ms of producer
work, with all 13 healthy data wire requests, both floor protocols, no measured
idle interval, no failures and no cancellations. Final focused borrowing tests
pass five cases twice; dispatcher primitives pass three cases twice. The full
wrapper module passes all 43 cases in 18.93s/18.36s. Its instrumented real control
finishes in 7,522ms/7,520ms, with global logical peak two and slow-denomination
peak two only after peer completion, no early borrowing and two admitted
bypasses. Healthy fixture wire counts do not replace the production logical-read
and provider-attempt budgets.

The controls cover both directions, both active denominations, unconstructed peer
work, duplicate slots, unbound work, errors, unpolled drop, cancellation and
post-deadline queued reads. Fresh processes and default stack/parallel settings
are retained. No assertion failures occur after the OLD control, and its clock,
fixture and isolation are not relaxed. Three independent read-only reviews are
clean at the final source hashes.

NR1505-12 and the linked provider, feed, solver and agent guides document the
conditional borrowing rule. Existing completion diagnostics add only
native_borrowed_reads, a bounded 0..6 logical admission count. Twenty Python
privacy/resource tests pass; it uses the existing sample with no additional
query and retains the 300-line/18-read bounds. Formatting, shell syntax and
whitespace checks pass. Only the label/name/image/profile/marker-verified owned
QA container and anonymous volume are reset. Fresh runtime authentication accepts
the correct private password and rejects a wrong one, then revokes its temporary
grant. Shared and production databases are untouched.

The final applicable source wave is serialized: native recovery, Clippy, offline
quotes, lifecycle, liquidity feed and provider recovery. Their unchanged-source
FIRST/RETEST outcomes remain pending. Identity, discovery and database
configuration source are unchanged from the seventh candidate; their prior
separate results are retained, without claiming they were rerun for this one.
The frozen 1,209 application files hash to
a7a8df9ecec322c700cb432534382536e11109aa9d185b7084ec567b4a94f0d1;
2,011 verification inputs hash to
625da21e0eb763387a2c80ab6b4f6b13992e00bcfa683d673873a36fe28d5620.
Hosted CI, exact deployment and fresh live acceptance remain pending.

## Eighth candidate: final frozen-source verification

The [final source receipt](source-verification-8-final.json) covers the applicable
six-stage wave from 00:37:32 to 01:11:02 UTC on 2026-10-16. All log digests,
focused-proof source hashes and before/after fingerprints match. No executed
failure remains in this wave; the actual OLD and prior live failures stay retained.

| Stage | PASS | FAIL | Explicit SKIP |
| --- | ---: | ---: | ---: |
| Native recovery #1505 | 14 | 0 | 1 |
| Offline quotes #1452 | 14 | 0 | 0 |
| Quote lifecycle #1499 | 10 | 0 | 1 |
| Liquidity feed #1453 | 12 | 0 | 2 |
| Provider recovery #1464 | 11 | 0 | 2 |

Clippy succeeds with the same 155 warnings, with no diagnostic in the changed
integration or dispatcher files. The full library passes 916 tests with one
existing ignored benchmark twice, in 149.55s/148.88s. Controlled browser FIRST
and RETEST each record 44 quotes and zero provider requests. Their production
build fixtures do not establish deployed latency or availability.

The unchanged identity, discovery and database-configuration checks remain
separate seventh-candidate results, without a new rerun claim. The applicable
source verifiers retain six explicit separate-scope skips. All 1,209 application
files remain a7a8df9ecec322c700cb432534382536e11109aa9d185b7084ec567b4a94f0d1;
all 2,011 verification inputs remain
625da21e0eb763387a2c80ab6b4f6b13992e00bcfa683d673873a36fe28d5620.
Hosted CI, exact merged deployment and every focused live acceptance criterion
still require new evidence for this candidate.

## Eighth deployment: initialized native-output FAIL, incomplete windows

[PR #1521](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1521) merges
as `b34187f33f1e167ab9741c7135081496ba1daaf6`, with an entire tree identical
to tested head `ce52a0f9554091ba97b6518719b296d73df5c52b`. [CI489](ci-489.json)
and [main CI490](ci-490.json) pass all required stages: contracts 999, frontend
3,470, library 916 plus one ignored case, and PostgreSQL 86 plus one ignored.
The original maximum-publication profile passes. Forgejo's combined and required
PR context independently report success; protection rules are unchanged.

Both Coolify services finish on the exact merge: frontend at 01:38:15 UTC and
backend at 01:41:03 UTC on 2026-10-16. [Build logs](deployment-1521-build-logs.json)
have 101/367 entries with zero failure markers. [Exact attestation](deployment-1521-attestation.json)
passes; independent backend warmup ends at 01:44:03 UTC before quote probes.
[Socket FIRST](deployment-1521-socket-first.json) passes 190 seconds with
148 reads including two health reads, 146 executable selected results and zero
unavailable responses. Its 145 initialized samples have p50 295.468ms,
p95 305.055ms, p99 311.353ms and maximum 311.440ms.

The Browser FIRST process launches at 01:49:07.412 UTC; its
[receipt](deployment-1521-browser-fail.json) starts at 01:49:10.373 UTC.
Two initialized forward API calls are useful; both reverse initialization calls
return actual 503 missing_observations: `27b594f64a04ae0c` and
`27b594f64a04ae0d`, in 290.27ms/290.99ms. Fast error completion is not useful
success. Source v9 records these failures but does not persist their results
until a later window checkpoint or serialize their absolute clocks. Their
original client interval is bounded by receipt start 01:49:10.373 and first
input 01:49:21.387; exact request/response epochs and server Date remain unknown.

The observer agent incorrectly inferred four API passes from entry into browser
windows, and that inference was relayed in progress. Both statements are
explicitly corrected. The final verdict remains **FAIL**. Forward desktop
naturally completes 190.165 seconds with 34/34 useful selected responses; phone
stops deliberately at 26.3 seconds with 6/6 useful responses, and both reverse
windows are unstarted. A partial phone guard is unavailable, not an invented
zero. All 176/800 calls are accounted for: 44 solves and 132 progress calls, with
zero rejected/blocked calls. All owned Node/Chrome processes exit at 01:53:04.443.
No natural RETEST or positive expiry control runs.

[Bounded monitoring](deployment-1521-monitoring-first.json) uses 17/18 reads
with both selected IDs; Prometheus is not provisioned. A separate
[fixed context](deployment-1521-producer-context.json) uses exactly four reads
and 54 untruncated lines for 01:48:45–01:49:45. The
[diagnosis](deployment-1521-diagnosis.json) retains exact server rejections at
01:49:15.270541 and 01:49:15.561715, both native_output/missing_observations,
with no stale-component witness.

Poll 53 completes at 01:49:11.049675, original observation 01:49:03.147, with
both mappings, tax, router and ULUNA cap, but no USTC cap. The preceding forward
successes carry exactly that poll's original clock, directly linking a fresh
partial input view to the absent output prerequisite. Its cap deadline witness
records 7,901ms poll time, 2,367ms denomination wait, zero global wait and 1,158ms
logical LCD activity. That implies roughly 4,376ms before first physical-read polling enters
denomination dispatch; it does not establish when the future was constructed,
or identify a specific predecessor or provider cause.

Previous complete poll 52 expires at 01:49:13.147; next complete poll 54 publishes
at 01:49:21.049233, leaving about 7.902 seconds after original expiry. Retaining
the prior whole view cannot bridge that gap. Nearby pool cycles are 22/22 and
reference commits progress. The bounded sample contains 46 completed polls:
36 zero-borrow, nine one-borrow, one two-borrow; polls 52–56 have zero admitted
bypasses. Zero does not prove that a peer never finished, an idle slot existed,
or the dispatcher malfunctioned. Raw production logs remain private.

## Ninth candidate: initialized evidence persistence

The [initial observer proof](observer-initialized-checkpoint-proof-initial.json)
reproduces three metadata omissions against the old helper. The
[final observer proof](observer-initialized-checkpoint-proof.json) records the
final source hashes and strict HTTP Date controls. Its prior-proof filename is
the original private filename; the immutable public archive has the `-initial`
suffix above and retains the recorded hash.

The separate initialized_api_checkpoint_v1 profile saves and emits actual
initialized rows and sticky failures immediately after the same four existing
API calls, before Chrome launch. Existing fetches record numeric request and
body-complete response epochs plus a validated numeric server Date or null.
Date accepts canonical HTTP-date syntax with a safe numeric epoch and exact
calendar/weekday roundtrip. ISO strings, numeric strings, impossible calendar
dates and wrong weekdays remain unknown. HTTP Date is coarse supplemental
evidence; it does not replace original monotonic or browser clocks and does not
change quote scoring.

Native v9 and generic v8 measurement versions, request counts, 500/750ms limits,
800/global and 600/context caps, wall limit, scoring and window sequence remain
unchanged. No fail-fast or new request is added. The initial explicit offline
allowlist FIRST/RETEST each passed nine tests. Review then reproduced four Date
validation failures against that intermediate source. Final explicit offline
FIRST/RETEST each pass thirteen tests, with no HTTP requests or Chrome process.
An initial negative name filter accidentally admitted two existing loopback
Chrome cases: that eleven-test PASS is retained as a browser run, not relabeled
offline. No production request occurs, and all owned browser/Node processes exit
before the next Rust timing control. An old private expiry helper correctly
refuses the new driver hash and must receive a new separately pinned copy before
future use. Future launchers must forward or inspect the
initialized_api_checkpoint event; window entry does not establish initialized
success.

## Ninth candidate: common prerequisite admission priority

The [priority proof](router-prerequisite-priority-proof.json) retains original
controls, intermediate failures, final hashes and all thirteen run receipts.
Router-first polling can occupy a shared slot before both treasury reads are
admitted. The first actual pre-fix real-HTTP control confirms that order and
a roughly 4,457ms binding prefix, derived from its 2,857ms admission and the
configured 1,600ms fixture response delay rather than a separately logged
completion. It still publishes a useful view in 7,523ms with thirteen healthy
data requests. A second variant remains useful at 7,902ms with optional
work clipped. Those assertion failures prove ordering only. A third permitted
latency variant loses native availability at 7,901ms, with one mapping, eleven
data requests, zero provider failures and one neutral cancellation. Its missing
mapping/rate shape differs from production poll 53's absent-cap shape; it does
not establish the historical provider sequence.

An intermediate first-poll admission gate produces a useful view in 7,874ms,
with only 26ms publication margin. Review finds an independent correctness gap:
Tokio can yield for cooperative budget before a semaphore request reaches its
FIFO. A focused regression reproduces the router gate opening while both
semaphore requests remain unadmitted and both permits remain free. This gate is
an unreleased candidate, not a defect claimed against deployed M8. Additional
fixture-precondition and owned-cleanup failures remain retained separately from
runtime findings.

The correction signals admission only after acquiring the existing global
permit and passing the original read deadline guard. Router waiting occurs
outside its physical read, owns no permit, and shares its original absolute
timeout. No-plan helpers and configured router absence retain their prompt
behavior. Both validated common results are still required before native work;
admission does not substitute for successful data.

An isolated control preserves the same slow mapping/rate and common-read delays
while reducing the independent cap fixture delay from 1,700ms to 1,200ms. Its
explicit no-plan legacy-order control still loses availability at 7,901ms:
eleven data requests, twelve provider attempts, zero failures and one neutral
cancellation. This current-runtime counterfactual is separate from the actual
pre-fix-source controls above. Fixed focused FIRST/RETEST each pass eight tests in 8.17/8.13 seconds. The
planned native view is useful at 7,376/7,373ms with thirteen data requests,
fourteen provider attempts, zero provider failures/cancellations and global peak
two. The unchanged legacy-order control remains unavailable at 7,901ms. Safety
cases cover cooperative exhaustion, one-permit/duplicate/no-plan admission,
expiry/drop/absence without transport, late queued admission, production wiring,
and pause/malformed-binding cancellation.

The complete wrapper module passes 51/51 twice in 18.15/18.11 seconds on unchanged
source; planned useful views complete in 7,373/7,368ms. Three independent reviews
of the final parent, dispatcher and focused-test hashes are clean. These remain
source results, separate from the required next deployment and live cohorts. Original provider, two-read, pacing, logical work, 7.9/8-second
and freshness limits remain enforced.


## Ninth final source verification

The [frozen-source receipt](source-verification-9-final.json) records the complete
serialized wave, 02:41:56–03:17:26 UTC on 2026-10-16, after resetting only the
owned, marked QA database and verifying actual runtime-password authentication.
No other builds or browser jobs overlap this verification.

| Verifier | PASS | FAIL | Separate-scope SKIP |
| --- | ---: | ---: | ---: |
| Native recovery #1505 | 14 | 0 | 1 |
| Offline quotes #1452 | 14 | 0 | 0 |
| Quote lifecycle #1499 | 10 | 0 | 1 |
| Liquidity feed #1453 | 12 | 0 | 2 |
| Provider recovery #1464 | 11 | 0 | 2 |
| Total | 61 | 0 | 6 |

Clippy succeeds with the same 155 existing warnings and no diagnostic in either
changed runtime file. The full indexer library passes 924 tests twice in
168.46/167.90 seconds, with one existing ignored test. The 19-test route binary
passes in 90.98/88.70 seconds and the frontend quote suite passes 536 tests twice.
Controlled production-build browser FIRST/RETEST each make 44 quote requests and
zero provider requests. These local fixtures do not establish deployed latency
or availability.

All 1,210 application files retain fingerprint
`8806694267fe906831cc384a13b686af17cd340dddd22cd2a9f11907f99dba59`;
all 2,012 verification inputs retain
`c28ca761d7861ed13e4a1536ed6af5d37ba2b95041febce81019a9d1ad27f2d9`.
The before/after scope includes tracked and unignored actual application roots,
including cfg(test), tests, scripts and toolchain inputs. Documentation, private
environment files and generated artifacts are excluded. The receipt guards exact
changed application/verifier paths and all final focused/observer/expiry-helper
pins. Prior identity, discovery and database-isolation results remain separately
recorded; they are not claimed as rerun stages of this six-stage wave.

Hosted CI, exact merged frontend/backend deployment/log attestation, independent
180-second backend warmup, native socket FIRST/RETEST, all four native browser
windows FIRST/RETEST, positive original-expiry callback controls and bounded
monitoring remain pending. Historical M8 failure remains FAIL. Source skips for
broader historical/deployment work do not establish acceptance or close other
issues.


The [fresh-main integration receipt](source-verification-9-main-integration.json)
records five incoming commits through `a21e697453564acb0f0af77dc9022a1ec857096e`.
Their 57 changed paths are confined to incidents/. Both local checkouts are
fast-forwarded, with identical tested application and verification fingerprints.
No application/verifier retest is claimed for those unrelated documentation
changes; genuine CI still runs on the submitted head.


## Ninth CI failure and public-fixture policy correction

[Genuine CI491](ci-491.json), exact PR #1524 head
`8b1ca65e2a969d6be84d9af8d89ae138b7258983`, fails secret scanning and completeness.
Contracts, frontend, indexer library and PostgreSQL jobs are skipped and provide
no executed test result. This failure remains retained; there is no unchanged
pipeline retry or fabricated status.

The scanner finds fourteen occurrences of five distinct values in two incident
JSON files inherited byte-for-byte from fresh main. The staged local hook does
not scan those unchanged files; required CI scans every tracked file with the
same pinned gitleaks 8.30.0. The actual worker uses the independently bundled
config, not PR-provided TOML. Its config/runner/scanner bytes match the repository
mirrors before this repair.

[Independent public provenance](ci-491-public-value-provenance.json) establishes
that three values are Solana mint encodings, matched in base64, hex and base58
against committed bridge deployment documentation. Exactly one bounded,
unauthenticated public Relay GET confirms both remaining execution identifiers
and both transaction links. No raw response, literal values, actors, credentials
or endpoint URL is exported in that receipt.

The correction adds exactly five escaped, fully anchored literal exceptions to
the existing secret-target allowlist. All 46 prior entries and every other
parsed TOML field remain unchanged; no directory, field-class, keyword or rule
exception is added. Config SHA256 changes from
`39ae76d2f5833b474fc62dd97d34c90836354335ce6217dde0283196ded408db` to
`a7f619790ab795838344e3bf33d602680a49b19bf8af1fa1743ead9a8cf98024`.
Three independent reviews confirm the exact five-value delta and provenance.

The [local policy proof and asset manifest](public-literal-policy-proof.json)
records 82 successful scanner processes across focused FIRST/RETEST. Exact public
literals fail the baseline generic-key detector and pass the corrected config;
mutated, prefixed and padding-aware suffixed values first prove baseline
detection and remain rejected. Unrelated synthetic secrets remain rejected.
Canonical scanner FIRST/RETEST each pass all four guards in 24.770/23.916 seconds,
including the tracked tree and both forced dependency-tree secret fixtures.

`make verify-issue-1400` passes nine checks twice with zero skips in 57.080/4.724
seconds. The first image build executes signed Go tests. The second build uses
cached layers and is not counted as fresh Go execution. A further canonical
no-cache build passes in 45.104 seconds and actually executes the unchanged Go
suite. Fresh nonroot runtime containers then pass FIRST/RETEST: health 200,
unsigned request 401 and eighteen decoded embedded-asset comparisons per run.
Their 128MiB/one-CPU, read-only, no-network/no-host-mount configuration is verified;
both containers are removed. Expected policy bundle is
`e189b2102b9a93b574b29fcb9520bb7ef6400ce01ff9d257d27a7f60af4d1de1`.

The proof retains the original detector-precondition, reporting-wrapper and
serialized-YAML comparison failures separately. None establishes a policy
broadening defect. Pure suffixes after Base64 padding are baseline-undetected
and are not counted as successful security controls; the corrected vectors
preserve the detector's grammar and compare exact captured hashes. The runtime
comparison parses YAML and the documented Woodpecker dollar-escape layer rather
than changing production escaping.

Application and existing verification-source fingerprints remain unchanged;
scanner configuration and twelve reviewed policy assets are recorded separately
because that config is outside the explicit application/toolchain fingerprint
scope. Local tests do not establish remote policy deployment or hosted CI.

At the pre-deployment checkpoint, the trusted policy service runs immutable commit
`60b6aa7418e4e6a4ca6b09e3193fd7b650a8e148`, bundle
`9f9865defd9ad0287fb90400fe408c942020e1c431275902a35c18555d8775ec`.
Its rollback pin/hash, nonroot 128MiB/one-CPU configuration, signing key,
exclusive extension and netrc=false are preserved. A reviewed immutable policy
release, exact bundle/health verification and genuine CI on the updated PR head
remain required. No application deployment or native live acceptance is claimed
for this policy work.

## Ninth deployment: FIRST pass and RETEST failure

[PR #1524](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1524) merged as
`4c21b49025f29c2862dedc26c2308ce3295e332a`, with the entire merged tree equal to
tested head `1b43914eb74c42b4fb37751ebacb4e7f71a06b17`. Both local checkouts were
fast-forwarded normally. Application and verification input fingerprints match
the final source wave; no forced merge or hook bypass was used.

The reviewed [independent policy rollout](deployment-1524-ci-policy-attestation.json)
pins that tested head and matches bundle `e189b210…`. Its
[actual build logs](deployment-1524-ci-policy-build-logs.json), unsigned401,
exclusive extension and unchanged resources pass. This deployment is separate
from the DEX services. Genuine [PR CI492](deployment-1524-ci-492.json) and
[main CI493](deployment-1524-ci-493.json) pass all required stages: 999 contract,
3,470 frontend, 924 library and 86 PostgreSQL tests, with two existing ignored
cases recorded separately. CI491 remains FAIL. The maximum-publication profile
is the actual named test with 512×2,048 = 1,048,576 orders; successful captured
stdout does not expose hidden per-profile timing. Earlier private collector
annotations using a different name/count were corrected from source and actual
PASS records, without changing any historical executed verdict.

Both DEX services [attest exactly](deployment-1524-attestation.json) to the merge,
and [build-log summaries](deployment-1524-build-logs.json) retain zero failure
markers. Backend deployment finished 04:28:05 UTC; independent 180-second warmup
ended 04:31:05 before measurements. The reusable attestation helper retains its
original issue1473 schema field; the actual expected revision, both services’ revisions and served frontend root
artifacts identify the #1505 deployment.

| Focused check | Actual outcome |
| --- | --- |
| [Socket FIRST](deployment-1524-socket-first.json) | PASS: 146 selected executable results, zero unavailable; 145 initialized results maximum304.523ms. The distinct cold/new-transport result is880.327ms with `socket_budget_pass:false`, outside the initialized criterion. |
| [Natural browser FIRST](deployment-1524-browser-first.json) / [audit](deployment-1524-browser-first-audit.json) | PASS: four naturally completed190-second windows;120/120 useful selected responses; initialized4/4 useful; first useful paint369.6–439.8ms;454/800 reads accounted. |
| [Natural browser RETEST](deployment-1524-browser-retest-fail.json) / [audit](deployment-1524-browser-retest-audit.json) | **FAIL**:87 useful selected window responses plus3 actual503s; initialized4/4 useful;323/800 reads accounted. Two full forward windows pass; reverse desktop completes and fails; final phone stops at19.928seconds. Its unavailable final guard remains null. |
| Socket RETEST | Not run after the natural RETEST failure. |
| Positive original-expiry callback FIRST/RETEST | Not run. Natural sampling does not directly instrument callback execution/adoption and cannot establish this criterion. |

RETEST's exact rejected IDs are `e4d6b2b79601545e`, `e4d6b2b796015461` and
`e4d6b2b796015464`. Browser body completion is05:00:34.114/.707/35.296 UTC;
the next sampled useful response is05:00:46.181, a12,067ms recovery interval.
The first two completions precede the preceding quote's latest possible
05:00:34.819 original expiry, but client timing alone does not establish server
eligibility or a missed safe fallback. Sampled blanking45ms after that expiry
is retained without claiming exact timer execution or adoption. Two diagnostic
frame gaps (101.9ms at input+13,220.5ms and149.1ms at input+523.6ms) occur after
their first useful paints; they are preserved without assigning them as503
causes. No expiry/ownership/probe acceptance flags preceded owned shutdown.
All owned Node/Chrome processes exited05:03:26.801 UTC.

[Bounded FIRST monitoring](deployment-1524-monitoring-first.json) uses16/18 reads;
[RETEST failure correlation](deployment-1524-monitoring-retest.json) uses17/18.
Their [FIRST](deployment-1524-monitoring-first-coverage.json) and
[RETEST coverage](deployment-1524-monitoring-retest-coverage.json) include the
actual initialized calls through phase completion/shutdown. Each sample retains
the300-line cap. Both collection receipts pass with a conditional Prometheus
skip because no datasource is provisioned; that is not an application PASS.

One separately approved [producer-context collection](deployment-1524-producer-context.json)
uses exactly4 reads and36 of300 lines over05:00:10–05:00:50 UTC. Poll198 completed
at05:00:21.790 with original05:00:14.819; poll199 completed partially at32.721
with original24.819; poll200 completed fully at40.463 with original34.820.
Poll199's USTC rate read expires at7,901ms: denomination wait1,231ms, global
wait846ms, logical LCD activity2,382ms and derived earlier prefix about3,442ms.
Logical LCD activity includes admission/identity waits and is not wire evidence.
The USTC mapping is absent while tax, router and the ULUNA cap remain present.
All three exact server rejections are native-output/missing-observations with no
stale-component witness. Nearby reference commits remain present; successful
provider/peer timing/order is unobservable and is not inferred.

Keeping poll198 unchanged until its original34.819 expiry could cover only the
first two failures, leaving5.644seconds until poll200's completion. Extending
expiry or splicing old fields would not be an acceptable fix. The actual RETEST
failure stays sticky while implementation resumes on a new isolated branch.
Earlier failed deployments remain FAIL. Wallet/device, funded signing and broad
acceptance sweeps remain excluded; #1505 stays open.

## Tenth candidate: redundant wrapper identity preflight

The [source proof](identity-preflight-proof.json) traces the mapper query through
`query_endpoint` and its mandatory fresh endpoint attestation before data I/O.
The wrapper producer previously performed a separate serial identity-only query
first. Removing only that redundant preflight preserves every actual data
attempt's fresh expected-chain check and the original caller/attempt guards.
LCD/provider, dispatcher and earlier ordering-control sources are unchanged.

The OLD control gives **every** identity response the same permitted1,100ms
delay, with100ms data responses and unchanged2.5/4/7.9/8-second limits. It loses
both required caps at7,903ms and is actually unavailable:12 identity requests,
9 data requests,12 provider attempts, zero failures and two neutral caller
cancellations. This differs from production poll199's missing rate/mapping;
it proves redundant source work, without retrospectively assigning production
identity latency. A separate healthy OLD control is useful with13 data reads
and14 identity/provider attempts; its FAIL is solely the desired13-count
assertion, not an unavailable response.

The identical delayed fixture becomes useful after the minimal removal, with
both mappings/caps, tax and final router support present. Settlement-required
phase completion is7,482–7,494ms, before the unchanged7,900ms margin. This clock
is the phase5 transition before optional floors, not a separately instrumented
router-completion, publication or browser-input clock. Publication still waits
until7,901–7,902ms for optional capability work; both floor capabilities remain
unknown. Its13 admitted identity attempts yield11 data requests, zero provider
failures and two neutral cancellations. It does not claim successful optional
reads, direct-floor support or a universal13-wire cap under failover.

The separate healthy fixed control has13 data and13 identity/provider attempts,
zero failures/cancellations and both validated capabilities. Six focused controls
pass twice (8.16/8.11seconds); the full wrapper module passes57/57 twice
(18.51/18.64seconds), using the unchanged default Rust stack and test threads.
Controls also cover old verified metadata followed by wrong/malformed/unavailable
identity, fresh qualified alternatives, late positive identity and explicit drop
without data polling, and both authoritative pauses. Three independent source
reviews are clean.

The existing optional-floor parent-deadline fixture changes its first-floor
identity ordinal13→12 after the removed preflight; the50ms delay, two-second
phase and all safety assertions remain. Legacy `Chain` is test-only for the
unchanged historical router controls, and older bounded diagnostics remain
readable. [NR1505-14](../../../skills/AGENTS_NATIVE_QUOTE_RECOVERY.md), the
[indexer invariants](../../indexer-invariants.md),
[solver guide](../../route-solver.md), provider/feed/local-quote agent guidance
and [focused checklist](README.md) document the narrow authority boundary.

The [final serialized source wave](source-verification-10-final.json),
05:24:00–06:00:27 UTC on 2026-10-16, passes after an
[owned disposable QA reset](source-verification-10-owned-reset.json). The first
[reset preflight rejection](source-verification-10-reset-preflight-fail.json)
occurred before any mutation because a strict guard rejected Docker's standard
anonymous-volume label. That FAIL remains recorded; the corrected guard permits
only that exact marker or empty labels, retains all ownership checks, removes
only the exclusive owned volume, and verifies fresh runtime authentication.
No shared or production database was reset.

| Source verifier | Passing groups | Failures | Explicit separate-scope skips |
| --- | --- | --- | --- |
| Native recovery 1505 | 14 | 0 | 1 |
| Offline quotes 1452 | 14 | 0 | 0 |
| Lifecycle 1499 | 10 | 0 | 1 |
| Liquidity feed 1453 | 12 | 0 | 2 |
| Provider recovery 1464 | 11 | 0 | 2 |
| Total | 61 | 0 | 6 |

Clippy passes with the same 155 existing warnings. Actual full-library totals
are 930 passed plus one existing ignored case twice (188.05/187.60seconds),
with the 19-test route suite twice (91.83/89.11seconds), 536 selected frontend
tests twice, and two controlled production-build browser receipts recording
44 quote requests and zero provider requests each. These local fixture results
do not establish deployed latency or availability. Original skips retain their
source/deployed/funded/historical scope; they cannot establish issue closure.

All 1,211 application files and 2,013 verification inputs have identical sorted
path/content fingerprints before and after. A
[fresh main synchronization check](source-verification-10-main-integration.json)
finds no incoming commits beyond baseline `4c21b490…`; both local main and
origin/main remain that revision. Genuine hosted CI, normal merge, exact
deployment and all new mandatory live criteria remain pending. The ninth
deployment's actual RETEST remains FAIL and #1505 stays open.

## Observer signing-window correction (2026-10-18)

The QA observers incorrectly bounded executable metadata to twenty seconds even
though the accessibility signing contract permits thirty minutes. This is a QA-only
correction; runtime producer, reference, wrapper and signing guards are unchanged.
The [original old-deployment receipt](investigation-20261018-original-observer-fail.json)
remains FAIL: eight HTTP200 responses failed `observation_lifetime`. New Python and
shared JavaScript regressions first reproduced that rejection for a valid
thirty-minute quote. The fix accepts the original signing window while separately
checking published snapshot age at quote construction (0–20,000ms). It still
rejects original expiry, overlong windows, malformed/stale ages and invalid
execution/economics. Actual in-browser validation and retained input clocks are
covered. Indicative 422 protection responses use the same actual original expiry
from `final_response`/the public error sanitizer, and remain non-signable.

The [new corrected baseline](investigation-20261018-corrected-observer-pass.json)
against unchanged deployment `0a3c251755a88ee9cbbbbc222fc747811c9da08c`
passes its bounded 21-second scope: eight executable responses, seven initialized
reused-connection samples, maximum 287.797ms, valid unique correlation IDs and no
503s. It does not establish the four 190-second browser windows, unchanged RETEST,
or new release deployment acceptance. Earlier deployed 503 failures remain FAIL.

[Regression commands, timings and observer hashes](investigation-20261018-observer-correction.json)
record 17 Python outcome tests, 39 JavaScript economics/browser/input-clock tests,
and 20 native-probe tests passing. All locally invoked focused test processes were
below 60s; compilation/provisioning is separate. Repository-mandated hosted full
matrix execution is separate. No production mutation or wallet signing was used.
