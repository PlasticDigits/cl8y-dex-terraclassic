# Issue #1464 verification receipt

Source base: `c5cc674f2d32655b1f4ee7521dd5f34aa196a8ab`, plus the reviewed #1464
worktree patch. Final commit/PR is the revision containing this receipt; verifier
logs record the Rust source fingerprint and UTC start/end. Worktree:
`/home/answorld/repos/worktrees/dex-1464`.
Agent instructions and reproduction: [README](README.md).

## Implemented invariants and local acceptance

The implementation separates recoverable provider pressure from integrity failure,
keeps pending economic blocks for retry, supervises serving independently, exposes
bounded indexing readiness and preserves independently qualified pair admission
through transient refresh failure. Typed discovery provider/integrity/database
failures propagate instead of successfully omitting economic events. Supported
schema/authoritative guard observations invalidate affected admissions immediately;
cache/signing/execution constraints remain intact.

Checkboxes below establish only the stated local evidence, not the full deployed
criterion. The final frozen focused verifier is **PASS WITH SKIPS**: 11 stages PASS,
zero FAIL, two explicit source-scope omissions; all selected functional groups run
twice on unchanged Rust fingerprint
`027a2df1756a483be80f1625c60aaf605de7a93578e7c7e98a14a30b336cab02`.
The broader requested verifiers' outcomes are recorded separately below.

| Completed local subcheck | Criterion/scope | Evidence |
|---|---|---|
| [x] | AC1: synthetic reported-direction executable corpus | `route_local_quotes_1452::recovery1464_reported_direction_corpus_is_executable_fast_and_network_free`: 1 passed, 24 useful socket results. All four direction semantics and exact documented ordinary/small raw 6/18-decimal amounts, protection 5%/1%/30%, pool-only, native wrap-enter, positive operations/floors, original generation/expiry and request provider counter zero. Synthetic pinned contracts, deep liquidity, reference marks 1 and supported zero wrapper fee establish an eligible fixture, not production token configuration/economics. |
| [x] | AC2: local checkpoint provider recovery | Lifecycle tests preserve same height/hash through repeated checkpoint/block capacity/outage failure and recovery beyond the permanent-failure budget. Provider failure does not become accounting success or an omitted event. Local fixtures test replay/idempotency and serving continuity; deployed restart stability remains unverified. |
| [x] | AC3: local integrity/lifecycle readiness | Unit/API fixtures distinguish unready provider outage, caught-up readiness and halted integrity. Post-swap position/required raw-fee SQL faults record one durable unsafe marker and halt without automatic replay. Removing the fault and starting fresh still refuses that pending height before RPC; incomplete projections require reviewed repair and explicit marker clearing. Reorg/malformed failure halts, serving remains alive with executable quotes blocked, and cancellation joins owned producers. No atomic crash guarantee or historical completeness is inferred. |
| [x] | AC4: fresh admission retention subcheck | `transient_admission_refresh_preserves_original_times_then_known_pause_invalidates`: 1 passed. A provider outage retains original observation/expiry and useful output; an authoritative pause immediately invalidates affected admission. Supported schemas are distinguished from malformed/unsupported/guard rejection; full deployed coverage/recovery-before-expiry remains open. |
| [x] | AC5: fixed blocker taxonomy | Diagnostics tests distinguish missing/stale prerequisites and missing catalog-corridor configuration from fully observed infeasibility, while retaining a healthy alternative and authoritative guards. Missing/stale and unsupported states use separate 503 blockers; genuine fresh protected-trade failure retains 422. |
| [x] | AC6: initialized loopback corpus subcheck | Frozen first/retest: 12 new-connection/result misses per pass, p50 2.068/2.593 ms, p95/p99 2.328/2.792 ms, max 2.420/2.902 ms; 12 reused-connection/result hits per pass, p50 1.432/1.854 ms, p95/p99 1.530/2.006 ms, max 1.765/2.190 ms. Every individual sample asserts <250 ms. Cold producer initialization is excluded and recorded separately. These figures do not establish regional/browser or production useful latency. |
| [x] | AC7: corpus and scheduled-worker network ownership | Counting/throwing upstream fixtures assert zero request calls after independent producer initialization. API readiness and canceled/halted producers also make zero upstream calls. Full GET/best/POST/progress/reference/refresh/signing matrix remains verified by the existing #1452 corpus, not inferred from health success. |
| [x] | AC8: local cache/confirmation isolation | Final #1452 verifier passed all 14 stages without skips: library/socket/cache isolation, frontend refresh/signing, production-build controlled desktop/phone browser, typing/formatting and unchanged-source checks. Actual deployed transport remains separately scoped. |
| [x] | AC9: focused verifier/docs subcheck | Frozen focused verifier passed all eleven source stages with functional test/retest and unchanged source; invariants/checklist/recovery/crosslinks updated. Broader prerequisite verifiers are separately recorded below; sustained deployed stability/outage acceptance remains open in #1456. |

## Commands, prerequisites and executed outcomes

Use host Cargo with `CARGO_TARGET_DIR=/home/answorld/repos/cl8y-dex-terraclassic/indexer/target`.
Private `indexer/.env.qa` was provisioned through the existing DB1396 script against
an independent disposable Postgres 16 cluster on IPv4 loopback port 55464. Runtime,
migrator and dedicated QA roles differ; the admin-owned `dex_indexer_test` control
DB is marked, and each integration process creates/migrates only its own random child.
No shared/production database, volume or migration history was changed. Private env
files have mode 0600 and no DSNs/passwords belong in this receipt. During concurrent
builds the disposable cluster's fixture creation/truncation encountered disk sync
stalls; fsync/synchronous_commit were disabled only in that dedicated disposable
cluster. These tests do not establish crash durability. Other ACL/socket verifiers
own their independent clusters/configuration.

Node 24 and installed frontend dependencies/browser fixtures are available through
the base checkout's dependency directory. Rust and frontend source remain in the
isolated worktree. Verification runs source/socket/provider/persistence regressions
and the existing requested verifiers against the current patch, then retests.

```sh
make verify-issue-1464
make verify-issue-1396
make verify-issue-1392
make verify-issue-1394
make verify-issue-1453
make verify-issue-1452
```

Frozen focused command: `make verify-issue-1464`, UTC 2026-10-10 08:40:39–08:43:05.
First/retest each: LCD 62 passed/one ignored benchmark, recovery 17 passed,
local quote 74 passed, economic ingestion 21 passed, API health 12 passed,
book snapshot 16 passed, initialized route socket fixture six passed. These are
208 test executions per pass; filtered groups overlap and are not 208 unique tests.
The four-direction corpus contributes 24 useful initialized samples per pass,
with exactly zero request upstream calls. The bounded evidence helper has four
tests, all passing; shell syntax/formatting/crosslinks/whitespace pass. The ignored
provider benchmark is separately executed by the broader verifiers.

Broader verifier outcomes:

| Verifier | Final source outcome | Scope |
|---|---|---|
| #1464 focused | PASS WITH SKIPS, 11 PASS / 0 FAIL / 2 SKIP | Frozen first/retest above; deployed/full current-artifact parity are separate. |
| #1396 database isolation | PASS WITH SKIPS, 28 PASS / 0 FAIL / 3 recorded SKIP | Library/disposable binaries, launcher/CI controls, internal/external namespace, opt-in host IPv4 and fresh role ACLs run twice. Authenticated runtime missing-env skip is independently fulfilled twice. Actual unverified scope: generated bridge published-host path and host-disabled IPv6. |
| #1392 chain attestation | PASS WITH SKIPS, 10 PASS / 0 FAIL / 2 SKIP | Library/check-tests, selected disposable integration, treasury/static/docs first/retest. Local/live read-only attestation command not supplied; later configured-provider point-in-time evidence and latest LCD regressions are separately recorded. |
| #1394 discovery | PASS, 23 PASS / 0 FAIL / 0 SKIP | Eight default plus eight high-parallel rounds, full library, explicit ignored benchmark and disposable discovery twice, after correcting the obsolete successful provider-outage skip expectation. |
| #1453 cached feed | PASS WITH SKIPS, 12 PASS / 0 FAIL / 2 SKIP | Provider/book/valuation/discovery library and six disposable suites twice; frontend 97 tests twice and desktop/phone mocked outage/recovery browser. Sustained deployment/history are separate. |
| #1452 local execution quotes | PASS, 14 PASS / 0 FAIL / 0 SKIP | Library/benchmark, seven disposable socket suites, frontend quote/refresh/signing and production-build controlled browser first/retest, typing/format/docs and unchanged source. Deployed/current-artifact parity remain separate. |
| #1395 event amounts | PASS, no selected stage skipped | Current library 708 passed/one ignored twice; raw event integrity ten passed twice; ingestion 21, fee ledger 31, evidence 15, volume 20, pair volume seven, 18-decimal positions six, rolling numeric 15, fills three and parked lifecycle two all passed. |

Independent supplemental evidence: corrected exact-output fixture 15/15 PASS and
routed-tax funding API 10/10 PASS; full ingestion binary 21/21 PASS twice;
authenticated runtime LOGIN privilege fixture 1/1 PASS twice after its exact-one-DB
prerequisite was satisfied in the disposable cluster. Temporary application CONNECT
revocation for that fixture was restored afterward. No shared runtime role changed.

[Funded LocalTerra settlement guards](localterra-settlement-guards.json): two
isolated passes succeeded with 40 and 35 receipts, including one expected atomic
minimum-return rollback per pass. Taxed initial gross, positive raw quotes/floors
and reverse gross guards were checked using checksum-bound existing deployed
artifacts. The available chain was used without shared resets. Pair/token hashes
differ from the existing provenance receipt and mintable provenance is absent,
so this establishes actual-artifact guards rather than current local-engine parity.
Applicable NE1398 source verifier: seven stages PASS, zero FAIL, zero SKIP.
LG1390 source verifier: 14 PASS, zero FAIL, two explicit full-chain/live omissions,
with functional first/retest. Final clean RT1381 verifier: 12 PASS, zero FAIL, two
explicit full-chain/deployed omissions, after corrected full first/retest; all
15 exact-output, 30 forward route, 10 hybrid and 10 routed-tax API tests pass.
The funded subset above remains independently proven.

Failures encountered and corrected remain part of the record:

- An initial #1396 attempt compiled while AppState/readiness edits were incomplete
  and failed on missing `indexer_health` fields. The broader loop was stopped and
  final verifiers rerun on stable source; the initial failure was not skipped.
- The first four-direction corpus had an incorrect assertion that composed wrapper
  `output_token` meant the mapped intermediate. It correctly means final cUSTC.
  The assertion now checks final cUSTC and first-hop cLUNC explicitly; rerun passed.
- The retention fixture initially imported a private request type; inferred serde
  request construction repaired the compilation failure and the fixture rerun passed.
- A temporary poller tracing annotation collided with the binary's existing global
  subscriber during full-suite execution. Removing the debug-only annotation kept
  health/deadline assertions and the final full suite passed twice. An earlier ten-second
  poller fixture timeout was investigated rather than skipped; webhook isolation,
  immediate halt propagation and targeted/full reruns established recovery behavior.
- Early focused runs executed during additional safety fixes failed their source
  fingerprint check. Both were retained; the final frozen run above passed unchanged
  source and every executed test stage.
- Broader routed-tax tests exposed an incomplete mock factory blacklist schema and
  old generic-422 expectations for malformed/stale/missing prerequisites. Supported
  200 economics assertions were retained, mocks now model the complete schema, and
  exact-output 15/15 and routed-tax API 10/10 reruns passed. The full RT1381 verifier
  was rerun after repair rather than relabeling its original failure as a skip.
- The first optional runtime LOGIN fixture correctly rejected access to its app DB
  in addition to its disposable child. Its private dedicated-cluster prerequisite
  was corrected to an exact single DB for the test, restored afterward, and both
  authenticated privilege runs passed.
- The original #1396 generated bridge fixture had ready internal TCP but its
  published IPv4 loopback host path was unreachable (port 59985, repeated bounded
  probes). The prerequisite wait was stopped and only the owned disposable test
  project removed. Rerun used its explicit supported bridge-host skip; internal
  service/external namespace and host-mode IPv4 confinement/ACL checks passed twice.
  Other QA loopback traffic on port 55464 remained available. Host IPv6 was disabled
  by the kernel. Neither unavailable path is claimed as deployed acceptance.
- The historical-body regression reproduces a warmed tip 1000 followed by valid
  requested height 41: historical block/filtered transaction reads no longer open
  live-lag circuits; substituted height 42 and missing nonzero transaction bodies
  fail closed. This is source causality, not a proven complete production root cause.
- Required positive raw-fee asset lookup SQL failures now propagate rather than
  silently omit the ledger. Two actual disposable post-insert SQL fault fixtures
  establish a pending checkpoint, one failed-row marker, truthful halt and zero
  upstream/replay after fault removal/new process. Their deliberately incomplete
  projections remain incomplete until separately reviewed repair; optional USD
  absence remains distinct from failed raw fee recording.

## Independently executed deployed baseline: FAIL

Public health independently returned `status=ok`, deployed `git_sha`
`c5cc674f2d32655b1f4ee7521dd5f34aa196a8ab` before the corpus. At
2026-10-10 07:04:14–07:04:42 UTC the bounded read-only corpus made 24 requests:
12 HTTP 422, 11 HTTP 503, one HTTP 502; **0/24 useful executable quotes**.
The [redacted baseline](deployed-baseline.json) retains fixed route/size/transport
classes and status/timing only. No bodies, endpoints, actors or credentials.
Command:

```sh
python3 scripts/qa/probe-issue-1464.py \
  --revision c5cc674f2d32655b1f4ee7521dd5f34aa196a8ab \
  --output /tmp/dex1464-live-corpus.json
```

Useful latency sample count is zero; p50/p95/p99/max are null. Error completion
cannot count toward <250 ms useful acceptance. These executed availability failures
remain **FAIL for that deployed revision**; local fixes and PASS WITH SKIPS do not
erase them or establish production deployment of this PR.

At approximately 07:38 UTC, separate read-only
[configured-provider observations](configured-provider-observations.json) established
point-in-time expected-chain attestation and exact requested-height successful pool
reads through all three configured endpoints. Factory blacklist response types are
the full supported schema. Configured ordinary IDs include 6036/8266/10184; routed
tax code 11713 is configured. CL8Y/USTR were observed at code 10184 and ALPHA at
11713 with funding version 1, buy 450 bps and sell 100 bps. These observations do
not imply an outstanding ALPHA migration, nor prove sustained failover or eligible
quotes. New-client provider timing with network path unverified is separate from production
ingestion deadlines and initialized useful quote performance.

## Merged and deployed #1465

[PR #1465](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1465) passed
all six mandatory [hosted CI steps](https://ci.cl8y.com/repos/7/pipeline/385/1)
on `f664f3df4b1010057f38086925306a617cfbf246`, then merged normally as
`79ab27c1563ccb85667be86ff540325cac4e057b`. The indexer rollout finished
2026-10-10 09:31:08 UTC; public health confirmed that merge revision.
The [deployment receipt](deployment-1465.json) distinguishes successful rollout
from **failed trading acceptance**.

The [first deployed socket corpus](deployed-1465-first.json) returned **0/24 useful**.
Fresh disconnected [production desktop/phone browser checks](deployed-1465-browser.json)
returned **0/12 useful per viewport**. No transaction was signed or broadcast.
Useful latency sample counts remain zero; error completion times cannot certify
fast useful quotes. Logs showed checkpoint progress and no repeated indexer exits,
while every quote corridor remained unavailable.

A second rollout of the same revision finished 09:50:07 UTC with bounded runtime
settings: attempt timeout 2,500 ms, request deadline 8,000 ms, circuit cooldown
1,000 ms and maximum backoff 5,000 ms. Shared rate/concurrency limits, original
timestamps, TTLs and execution protection were unchanged. Selective fixed-label
admission diagnostics were enabled. The [cold post-rollout corpus](deployed-1465-tuned-cold.json)
still returned **0/24 useful**. This cold probe does not establish initialized
acceptance. Later diagnostics reported zero admitted pairs and unsupported
pre-pair prerequisites, so deployment success and provider tuning alone did not
resolve the live defect. These executed failures remain FAIL pending a corrected
release and successful deployed retest.
The [warm post-tuning corpus](deployed-1465-tuned-warm.json), 09:58:50–09:59:19 UTC,
also returned **0/24 useful**. The [fixed-label cycle census](deployed-1465-tuned-census.json)
records successful 22/22 reserve cycles with original ages below the unchanged
expiry, alongside zero admitted pairs. This separates reserve recovery from the
remaining token/configuration prerequisite failure. Later public hub reads recovered
bounded USTR/UST1 marks; unavailable reference snapshots were not a permanent
contract or migration limitation.

At 10:09:35 UTC, a bounded [configured code-envelope probe](configured-code-envelopes.json)
confirmed successful code-details responses for ordinary 10184 and routed-tax
11713 exceeded the worker's 65,536 byte body limit: the endpoint also returns
inline WASM. Production configured a hash fallback, causing redundant code-detail
reads even for positive code-ID registrations. That reproducibly rejected all
known tokens before pair validation. The follow-up skips only those redundant
hash reads; unknown code/hash attestations and body limits remain mandatory.

### Serving transport limit

Separate [bounded transport reads](deployed-1465-transport.json) measured reused
HTTP/2 health/oracle/error TTFB 278.759–297.261 ms. These contain **zero useful
quote samples**. [Protocol comparison](deployed-1465-protocol-comparison.json)
measured reused HTTP/1.1 health TTFB 279.175–280.476 ms and HTTP/2
289.481–289.940 ms; one HTTP/1.1 body completed after 1,353 ms and remains recorded.
[Production-origin browser document reads](deployed-1465-browser-transport.json)
measured approximately 283–284 ms request TTFB with JavaScript disabled. Those
document reads do not certify quote performance.

Observed TCP connection phases were about134ms. These timings cannot establish
a physical RTT or geographic minimum; the network path is unverified. Measured
response timing exceeds250ms, and backend microbenchmarks cannot waive it.
The original HTTP/3 reachability omission is resolved by the later guarded repair.
No DNS authority for serving-placement changes is established by the scoped API.

## Follow-up live admission source verification

The [frozen-source receipt](live-ingestion-source.json) records the follow-up
`make verify-issue-1464` first run and retest: **11 passing stages, zero failures,
two explicit source-scope skips**, ending 10:29:38 UTC. It covers the exact oversized
code-envelope regression, denied-provider pacing, bounded actor preparation,
independent ticker scheduling, original wrapper freshness and slow optional output
publication. Real disposable checkpoint/health/book/socket cases pass twice;
four-direction useful socket results remain below 250 ms with zero request RPC.
The initial private-module test compilation error was corrected through public
refresh/solve APIs. An earlier book deadline failure under overlapping disk-heavy
verification passed on retest and again after serialization. The initial
changed-source fingerprint failure was corrected by repeating unchanged final
source; these earlier failures were not relabeled SKIP.

Two fresh, funded [isolated LocalTerra oracle controls](localterra-oracle-controls.json)
and both current Rust receipt replays pass. Each fixture executed 19 transactions
and captured three reserve stages without resetting/restarting the shared chain.
The controls use actual chain reserves plus an explicitly synthetic certified
CEX quorum/bootstrap clock. The driver now accepts current nested factory responses,
keeps its reserve-shift attack below the unchanged 50% protection after integer
rounding, and reconciles an accepted transaction under a finite 120 second deadline
without rebroadcast. Earlier fixture rounding/inclusion failures were corrected
and successfully rerun. Artifact provenance limitations remain explicit; this is
not current-release settlement or production acceptance.

The broader #1452/#1453/#1394/#1392/#1396 and oracle #1391 verifiers are rerun
with bounded build parallelism. Hosted CI, deployment and useful live retest must
be recorded independently before checking their acceptance scope. Current #1465
production failures remain FAIL; this source receipt does not establish rollout.

## Deployed #1466 trial and remaining fairness defect

[PR #1466](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1466) passed
all six mandatory [CI jobs](https://ci.cl8y.com/repos/7/pipeline/387/1) on
`2c222aa83cf463ccb04b0d4d649a60ae60bbdb00`, merged as
`325ad4ec0938bdab68cedc3143d0aa0ef930cdf1` and finished rollout 11:14:54 UTC.
[The receipt](deployment-1466.json) records 712 library tests and 84 Postgres
tests passing, with source-scope omissions explicit. All six local prerequisite
verifiers passed their runnable checks. Supplemental
[identity observations](identity-1392-supplemental.json) and
[authenticated runtime fixtures](authenticated-runtime-fixture.json) pass twice.
The runtime test's earlier unrelated-QA-grant failure was corrected using a
separate fresh cluster; shared/production ACLs were unchanged.

The [first live corpus](deployed-1466-initial-quotes.json), 11:17:46–11:18:14,
returned **16/24 executable quotes**: LUNC 6/6, CL8Y 5/6, USTR 5/6 and ALPHA 0/6.
All useful socket timings exceeded 250 ms. Its legacy connection labels describe
intended groups, including a new connection at the start of the reuse group;
the corrected probe observes actual socket identity and automatic reconnect.
The [next corpus](deployed-1466-unavailable-window.json), 11:19:39–11:20:06,
returned **0/24, all 503**. Coverage was not sustained, so acceptance remains FAIL.

Bounded diagnostics showed 306/338 token discovery outcomes were immediate
provider availability/capacity loss; admitted pairs declined from seven to three
as original 180 second qualification expired. Community discovery succeeded four
times but dependent policy reads failed three times. The opaque optional policy
result incorrectly invalidated fresh tax qualification on provider loss. No
indexer halt/restart or unsafe checkpoint advance occurred in this window.
The next fix uses bounded zero-attempt waiting, pool/book lane headroom and typed,
static-first tax policy admission; it does not extend TTLs or provider quotas.

The original browser probe wrongly rejected the normal disconnected **Quote only**
label; its zero-useful conclusion for 200 responses is invalid. The actual 503s
remain failures. A [corrected diagnostic](deployed-1466-browser-diagnostic.json)
validated LUNC small/30%: DOM 0.904966 matched raw net 904966, final floor 633476
and two positive hop floors, with displayed fair-reference deviation about 3.63%.
ALPHA remained unavailable in that diagnostic. No timing pass is inferred from
the invalid browser timer, and no product change was needed for that label.

## Admission fairness and cancellation repair

The [frozen source receipt](admission-fairness-source.json) records 11 passing
stages, zero failures and two explicit source-scope omissions, with full first run
and retest on the unchanged Rust fingerprint. Each run includes LCD 66+one ignored,
recovery 25, local quote 75, discovery 11, oracle 27, ingestion 21, health 12,
actual worker overlap 17, route sockets seven, tax recovery five and holder two
passing tests. The four-path initialized socket corpus has 48 useful results,
maximum miss 2.861 ms and maximum hit 2.007 ms, with exactly zero request RPC.
Five bounded evidence/real reconnect tests also pass.

Production pool/book lanes now use four/two concurrent tasks under the unchanged
shared provider cap of eight. Only a scheduled full scan with zero actual attempts
may wait 50 ms before retrying within the same original deadline. No permit is
held during the wait and no quota/deadline/attempt budget is reset. Required static
tax admission uses typed provider availability, publishes before optional wallet
work, and retains original expiry on loss. Actual changed policy or a previously
qualified pair's negative preview revokes related qualification immediately.

Completed manager=false or protocol=true flags revoke prior manager exemption
before awaiting balance; the 64 second delayed-balance regression checks immediate
revocation, whole-pass cancellation and fresh nonexempt recovery. Optional actor
work never publishes new positives without fresh flags/balance and unchanged
required-policy recheck. Four recent actors and four uncensused holders share an
eight-observation batch. The tax fixture uses a larger bounded **fixture-only**
burst to avoid exhausting cold setup before its negative-read assertions; deployed
rate/burst limits are unchanged and actual production overlap has separate proof.
Earlier insufficient-fixture-budget, synthetic-address and overlap timing failures
were repaired and rerun, including explicit negative-read witnesses and absence
of capacity deferrals. No executed failure was converted to SKIP.

Broader prerequisite verifiers run serial before merge. Their exact frozen-source
counts and hosted exact-head job receipts are recorded in the PR review evidence.
Production #1466 remains an executed FAIL until the replacement rollout is
independently retested; source speed does not establish normal transport acceptance.

## HTTP/3 reachability and transport evidence

The existing origin advertised HTTP/3 and Traefik published UDP443, but its
direct firewall allowed only TCP443. The [guarded repair](http3-firewall-repair.json)
preserved all seven rules and appended only UDP443 using the same HTTPS source
set, on one exact direct origin attachment. Fourteen mutation/refusal/rollback
guard tests passed twice. Actions completed and the exact resulting rules were
reread; private rollback state is retained separately.

[Forced HTTP/3 health](http3-forced-health.json) succeeded twice.
[Normal browser health](http3-normal-health.json) naturally changed from h2 to h3:
reused total 292.552 ms versus 282.200 ms. This proves reachability and a small
observed transport benefit; health is not useful quote acceptance and remains
above250ms on this measured path. No unrelated infrastructure setting was changed.
Traefik's [HTTP/3 documentation](https://doc.traefik.io/traefik/reference/install-configuration/entrypoints/#http3)
requires UDP access for the configured entry point.

The client network path remains unverified. Operational location metadata is
omitted from receipts and review text. Actual timings and failures are retained;
neither handler-only nor health timing can check AC6. Do not collect or publish
workstation location, network exit location, timezone or identifying routing data.

The [current-source runtime fixture](authenticated-runtime-fixture-82a6f174.json)
passed twice on the frozen Rust fingerprint with two independently owned DBs,
verified zero runtime CONNECT before and after, and full owned-resource cleanup.
Its initial final-initialization readiness failure was repaired before successful
fixture execution. No shared/production privileges changed.

## Deployed #1467 and current 500 ms target

[PR #1467](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1467) passed
all six mandatory [CI jobs](https://ci.cl8y.com/repos/7/pipeline/392/1) on
612f741af5c8ae1be39b9fa9401801dae91bf867, merged as
ef1cf3995f2e725b8c3847ca9a5504b454a8ec89, and finished rollout13:32:16UTC.
Public health attested that exact merge. The [cold trial](deployed-1467-cold.json)
returned22/24 useful quotes, including ALPHA6/6. The later
[warmed trial](deployed-1467-warm-economics.json) fell to3/24. The corrected
[desktop/phone browser](deployed-1467-browser.json) returned22/24 useful scenarios,
all200 shapes valid, but six503 attempts and two unavailable scenarios.
Its first-useful median349.7ms and maximum11956.5ms retain retry tails.
No subsequent successful fast fetch replaces the entire first-useful completion.

The user accepted 500 ms temporarily and deferred tighter latency improvement.
At that current target, the browser has17/24 passing useful scenarios, with
availability and slow tails still FAIL. Older 250 ms verdicts are retained as
historical scope; their thresholds were not silently rewritten. The current
bounded helper defaults to 500 ms, cannot select a looser budget, and tests usefulness
and unrounded timing before acceptance. Operational location metadata is omitted.

## Caller-budget cancellation and original deadline repair

The [frozen source receipt](caller-budget-source.json) records11PASS/0FAIL/2SKIP
with full first/retest and unchanged Rust fingerprint. Per run: LCD73+one ignored,
recovery33, local quote76, discovery11, oracle27, ingestion21, health12,
book18, route7, tax5 and holder2 all pass. The cache benchmark separately passes
twice. Four-path loopback has48 useful samples, maximum miss3.458ms/hit2.193ms
and exactly zero request RPC. Six evidence/transport/budget tests pass.

Real four-second pair fixtures reproduce a failed2.5s first provider followed
by a healthy2s fallback. Caller cancellation previously quarantined both providers;
a trusted scheduled view now releases the closed fallback neutrally, charges
admission, records caller_cancellations and permits the next original-timestamp
observation. Same shared state/caps, no success or refund, no extra attestation
or HTTP, no deadline/TTL increase. Generic cancellation, incomplete half-open
recovery and real failures keep quarantine. Non-success headers preserve default
cancellation quarantine while ambiguous bodies remain unread; completed500
contract/gas/classic outcomes retain their exact previous classification.

A separate publication race could capture a still-fresh pair, cross its expiry
during mirror database reads, then omit its deadline and use a fresh sibling's
global maximum. Per-pair deadlines are now captured from the same pair membership
before any await. Actual score rejects the expired corridor while the fresh
sibling retains a positive quote. Original deadlines and known negatives remain.


The prerequisite reruns record #1453: 12 PASS / 0 FAIL / 2 SKIP;
#1394: 23 PASS / 0 FAIL / 0 SKIP; #1392: 10 PASS / 0 FAIL / 2 SKIP;
and #1396: 28 PASS / 0 FAIL / 3 SKIP.
[Fresh identity observations](identity-renew.json) pass twice independently of
quote admission. [Authenticated runtime isolation](authenticated-runtime-fixture-renew.json)
passes twice on the same frozen source, with zero unrelated CONNECT grants before
and after and complete owned-resource cleanup. Unavailable published-loopback
and IPv6 probes remain explicit skips; internal, host-mode, confinement and ACL
checks executed successfully.

The first #1452 run retained one controlled-browser latency failure (1132.7 ms);
its immediate retest passed. That isolated outlier's cause is unproven. A full
unchanged-source rerun then passed all 14 stages without skips, including both
controlled browser passes; all eight case maxima were below 63 ms. The original
outlier remains recorded and was not waived by the temporary 500 ms target.
Independent review subsequently found a known-negative cancellation race during
provider metadata lock contention. The repair latches known HTTP, transport,
height and identity negatives before failure-recording awaits, preserves typed
integrity evidence in the existing retained error slot, and keeps own-timeout
failure recording armed until completion. Actual held-lock cancellation tests
cover closed and half-open providers and exactly-once failure counts. Fresh
verification on this final source is recorded separately below. The
[held-lock targeted receipt](known-negative-cancellation.json) records 13 passing
tests twice on fingerprint ce877c360a9c4340e84bcb42be317b602d8b6cc2e81f7d60ee040719384efc4d.

## Current-artifact funded controls

[Current DEX build provenance](current-contract-build-provenance.json) and
[locked CW20 provenance](current-cw20-fixture-provenance.json) establish bounded,
locked cold builds of the unchanged contract tree. Thirteen optimized DEX outputs
and the exact locked CW20 fixture are attested; previous manually replaced binaries
are not silently promoted. This does not claim a second independent cold rebuild.

[Funded controls](current-artifact-settlement-controls.json) passed twice on two
fresh owned chains and two owned database clusters: six scenarios each,
110 successful transactions total plus two expected floor rejections, five
on-chain checksum matches each and exact pool/LP rollback across all five pools.
Two actual reserve/oracle controls add38 transactions and current Rust consumer
replays2/2 per process. Source/lock/outputs were checked; owned resources cleaned.
The initial broad cleanup audit included pre-existing volumes, was corrected
to exact owned-resource checks and passed twice without touching those volumes.

These establish current-artifact tax/final-floor guards and actual reserve
consumer behavior. Full cached-engine quote/settlement parity has no reusable
observer and remains explicit SKIP. Hooked/AutoLP artifact execution is not
covered by the hook-free fixtures; source guard regressions remain distinct.
Production reads did not sign or broadcast.

## Remaining acceptance and skip scope

- [ ] AC1 full live four-direction acceptance: #1467 is deployed/attested, but fails sustained useful coverage.
- [ ] AC4 production coverage before expiry across multiple cadence/recovery cycles.
- [ ] AC6 initialized hit/miss useful latency under normal desktop/phone
  browser transport, first/later result and separately measured RTT/cold state.
- [ ] AC9 sustained deployed stability and bounded read-only outage/recovery evidence.
- [x] Current-artifact funded tax/final-floor controls: attested builds and two owned-chain passes are recorded above.
- [ ] Full cached-engine quote/settlement parity and hooked/AutoLP artifact execution remain unverified; the funded controls have narrower scope.
- [ ] #1456 production rollout/history acceptance stays separately scoped/open.

Follow [the explicit skip policy](README.md#skip-policy-and-receipt). Finish available
checks, record missing prerequisite/evidence and unchecked scope; source overall may
be **PASS WITH SKIPS** only when executed source checks pass. A failed executed test
or measured useful latency assertion cannot be waived into a skip.

The first final-source full verifier was interrupted after independent review
found two sequential metadata writes in the wrong-block-chain failure path.
Cancellation after failure recording could count it twice. That run is not a
PASS. The repair combines quarantine and identity metadata under one write guard
with no later await; the regression checks retained typed error and exactly one
failure. Final acceptance requires the restarted frozen-source verification.

Unambiguous nonretryable response headers also enter the retained typed error
slot before a potentially stalled body read. A caller deadline cannot downgrade
known HTTP rejection to availability. HTTP 500 and contract-path 404 still require
the original body classification; their semantics and bounded body limits remain.

The [final targeted receipt](final-cancellation-controls.json) records41/41
recovery tests passing twice on unchanged fingerprint
df4d090e6e043351ccc90d4939dcf5e25313376e3b94027b85b07edf841804c6.
The complete focused and prerequisite checks are rerun on that same final source.

The [final focused first/retest](final-focused-source.json) passed11/0/2 on
the same df4d090e fingerprint. LCD81 plus one ignored benchmark, recovery41,
local quote76, discovery11, oracle27, ingestion21, health12, book18, route7,
tax5 and holder2 pass per run. Four-path useful socket results remain below
500 ms with exactly zero request RPC. Broader prerequisite and hosted verification
must finish successfully before merge; source PASS WITH SKIPS does not establish
production acceptance.

The normal commit secret scanner initially flagged five public artifact/source
SHA-256 values next to token file labels. Their digest shapes and provenance
were verified; the public receipt presentation now separates paths from hashes
and distinguishes original/public document checksums. The unchanged scanner
passed after that restructuring; no hook or scanner rule was bypassed.

## Deployed caller-budget repair #1468

[PR #1468](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1468) passed
[six actual hosted jobs](hosted-394.json) for ed6ddec6af57d3a0d9e20e74c36ef41a10dfad74
([pipeline394](https://ci.cl8y.com/repos/7/pipeline/394/1)): 731 library tests,
84 selected PostgreSQL tests, and matching exact-head completion markers.
[Final local prerequisites](final-prerequisites-1468.json) record98/0/9 stages
on unchanged df4d090e fingerprint. [Fresh identity](final-identity-1468.json)
and [authenticated runtime](authenticated-runtime-fixture-renew-final.json)
controls pass twice independently. Their scope does not certify deployment.

The [rollout](deployment-1468.json) completed15:58:45UTC on
8f62fb3c0125f73785fd7a9af0cd5b44a87dd01c, with3 healthy checks, old-container
removal and no compiler errors. Public health independently attested that merge.
[Initial corpus](deployed-1468-initial.json):23/24 useful, one stale503;
11 actual reused successes all<=417.456ms. New TLS samples remain slower.
[Later corpus](deployed-1468-warm-economics.json):24/24 useful, followed by
[continued trial](deployed-1468-sustained-economics.json):20/24 useful.
All actual reused useful fetches remain below500ms; cold sockets are separate.
These measured failures remain FAIL and are not overridden by later successes.

[Actual browser](deployed-1468-browser.json),16:03:15–16:05:35UTC:
23/24 useful,21/24 meet500ms. Two desktop small30% scenarios waited about10.63s
after503 before recovering; USTR small30% remained unavailable after two503s.
Phone12/12 useful. All useful rows retain positive matching hop/final floors
and numeric DOM output. Whole first-useful time includes every retry.

[Five-minute metadata](deployed-1468-metadata-summary.json) has31 exact-merge
samples, no counter resets and no all-provider-open samples. Provider2 gains
2730 successes and24 caller cancellations with zero failures or circuit deferrals.
Every sampled reference remains fresh below6.5s. This establishes the cancellation
repair's operational effect, while intermittent route availability still fails.
The monitor's stricter HTTPS-only admin-base validation refused log collection;
that helper failure is distinct from the public availability observations.
Separate log attempts preserve broker-expiry and bounded-read failures.

A remaining source gap is the pool cycle's head prelude: it treats zero-attempt
capacity deferral as missing height immediately, while subsequent pool phases
already wait for capacity. Successful later live reads cannot establish a usable
pinned fee/book generation; known-height retained rows cannot be relabelled.
The follow-up uses a head-only wait inside the existing four-second budget,
without retrying actual failures or renewing the original pre-head timestamp.
Queued-cycle aging remains a separate hypothesis until lane-labelled evidence
establishes it. No freshness period or shared provider cap is increased.

## Pool frontier and complete fee generations: follow-up source

Subsequent classified logs counted 110 rejected older pool observations,
155 expired pool generations and 62 missing compatible fees. No head-unavailable
warning was observed in that window; the capacity race is covered by source
regressions, while older successful heads are an independently observed failure.
The scheduled pool consumer now requires the stored live-pair height frontier
inside its original bounded provider scan. A valid lower head is ineligible for
that query and cannot refresh reserves or alter the shared lag policy.

Raw reserves still publish for valuation. An explicit bounded pending phase lets
both quote publishers retain only the exact previously qualified complete tuple,
with original timestamps and expiry. Negative headers, cancellation, changed fees,
maximum fees and obsolete completion remain fail closed. Cold producers also own
a non-retaining guard before raw SQL; newly arriving qualification cannot evade
that guard. Current raw fee validation precedes older hybrid selection.

[Final targeted test/retest](final-fee-generation-controls.json): fee-phase controls 11/11 and actual scheduled-worker
controls 7/7 passed twice. Earlier LCD controls passed 93/93 plus one ignored
benchmark twice; producer/book controls passed 22/22 twice before the final cold
guard extension. The complete verifiers below must pass on the final frozen source
before merge. These source results do not resolve the deployed FAIL observations.

The first broad run failed the existing initialized socket fixture when it
replaced its production-qualified namespace with deliberately backdated synthetic
observations. Monotonic fee provenance correctly rejected that regression. The
controlled-age section now owns a separate synthetic namespace; all production
measurements and stale/future rejection assertions remain intact. The failed run
is retained, and the complete frozen checks restart after the fixture correction.

[Final frozen fee-generation verifier](final-fee-generation-source.json) passed
11 stages with zero failures and two explicit source-only omissions, UTC
2026-10-10 23:44:31–23:49:42, on cadffc5f6cff17905999f06e1afebda272b24cb24ec5cf1a6c6cccc26747eb7f.
Per pass: LCD 93 plus one ignored benchmark, recovery 67, local quote 77,
discovery 11, oracle 27, ingestion 21, health 12, pool/book 22, route 14,
tax 5 and holder 2. These overlapping groups represent 351 test executions per
pass, rather than unique tests. Original observation/expiry, owned cold guards,
positive floors and zero request RPC remain asserted.

The first #1392 broad run reported 9 PASS / 1 FAIL / 2 SKIP. Its first
book-worker fixture aborted an unfinished fee phase after quote qualification,
then incorrectly required a positive cached quote; the second pass happened to
shut down between phases. The fixture now stops the capacity worker before any
quote qualification, releases its leadership, and completes a bounded pool/book
pass before initializing quote workers. Four consecutive targeted runs pass,
retaining original pre-head timing, pinned reads, positive output/floors and
zero request RPC. The complete checks restart on this final test source. An
intermediate attempt to run the full pass while the loop held leadership timed
out and was corrected; no production cancellation or admission rule changed.

## Merged and deployed #1469

[PR #1469](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1469) merged
source `a585a0505dd509263d2f8f78526baae3d4f37dd3` as
`bf59d6db032cbcf9a65a98d1935b2665a8e4fa9a`. All six mandatory
[hosted CI steps](https://ci.cl8y.com/repos/7/pipeline/399/1) passed on that
exact source. The rollout finished and public health independently attested the
merge revision before these read-only browser trials. This confirms deployment,
not sustained trading acceptance.

Public receipts: [local prerequisites](final-prerequisites-1469.json),
[hosted jobs](hosted-399.json), [deployment](deployment-1469.json),
[initial browser](deployed-1469-browser.json) and
[browser retest](deployed-1469-browser-retest.json).

The first initialized desktop/phone corpus, 2026-10-11
01:06:58.498–01:08:46.780 UTC, returned **24/24 useful results within 500 ms**:
p50 334.9 ms, p95 362.2 ms, p99/max 363.7 ms. It made 24 quote GETs with
actual reused browser connections. This trial was **PASS** for its measured scope.

The later retest, 01:44:21.348–01:46:26.251 UTC on the same revision, returned
**24/24 useful results but only 22/24 within 500 ms**. Desktop CL8Y→USTR
small/1% took 10,637 ms, and phone ALPHA→CL8Y ordinary/5% took 10,633.5 ms;
each first received 503 and then recovered on a later request. Full input-to-first-
useful DOM time retains those retries: p50 333.3 ms, p95 10,633.5 ms,
p99/max 10,637 ms. The retest made 26 quote GETs. **Sustained acceptance remains
FAIL**; the initial passing run does not erase these measured failures.

Both trials used disconnected Chromium profiles without signing or broadcasting.
Phone means a phone viewport in desktop Chromium, not a physical-device trial.
Positive executable output and matching hop/final floors were checked; fair-reference
cost/deviation and per-hop market spread remain distinct. Browser initialization
and cold/new socket timing are separate from initialized useful-result timing.
No production outage was induced, and no operational location metadata, private
endpoints, raw response bodies or actor data are included here.

## Independent complete views: current follow-up

Implementation/review: [PR #1472](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1472).

The follow-up keeps complete pool and hybrid views in one immutable quote snapshot
with independent original publication watermarks. A newer pool no longer makes an
otherwise compatible original hybrid regress; expiry of that hybrid cannot erase
a still-fresh pool. Zero-book/pool-only budgets prefer the complete pool. Automatic
optimization may compare the independently eligible original hybrid, while a
declared positive book budget requires it and fails unavailable after expiry.
Every candidate uses one whole compatible tuple. No old book is attached to new
reserves or fee economics.

Both views share the existing hard namespace/marker/mirror/order/byte caps and
known-negative invalidation. The four-second pending phase and original twenty-
second observation TTL are unchanged. Cache and reviewed-generation identity
includes changes to either complete view, including a pool-only update.
See [invariants](../../indexer-invariants.md#quote-ready-fee-provenance-1464-follow-up),
[local engine](../../../indexer/src/api/local_quote.rs),
[whole-view loading](../../../indexer/src/api/db_orderbook_sim.rs) and
[fee registry](../../../indexer/src/indexer/fee_phase.rs).

The [state controls](../../../indexer/src/indexer/fee_phase_tests.rs) exercise
independent watermarks, shared immediate negatives and combined allocation caps.
The [actual-worker controls](../../../indexer/tests/scenarios/fee_generation_recovery_1464.rs)
exercise both publishers, original hybrid/pool expiry without another scheduled
pass, exact-input/receive caches, automatic book exclusion, declared-book refusal,
pending/changed fees, cancellation, known rejection and exactly zero request RPC.
Follow the [agent instructions](README.md#independent-complete-pool-and-hybrid-views)
and preserve the explicit skip policy. Final-source counts and deployed acceptance
for this follow-up require their own completed receipts; none are inferred from
#1469 or from these implementation descriptions.

The [initial source run](independent-view-initial-source-change.json) passed its
executed test assertions but failed the unchanged-source guard after the final
diagnostics correction landed during verification: 10 PASS / 1 FAIL / 2 SKIP.
That failure is retained. The complete focused first/retest must run again on
the frozen source; neither successful individual tests nor a skip supersedes it.

The first frozen run passed #1464 (11/0/2), #1452 (14/0/0) and #1453 (12/0/2)
on 1a048f6c354a56f0ce62493bb03b79d25c146da86a0c302ce3218fcf37f5fc8f.
[Pipeline 401](https://ci.cl8y.com/repos/7/pipeline/401/1) also passed all six
actual jobs for `6ffd2a2b98f8a2c7d02c9a63e43eb75e78a02e0c`:
764 library tests and 84 selected PostgreSQL tests, with exact-head markers.

The [discovery prerequisite failure](independent-view-parallel-source-failure.json)
then reported 22 PASS / 1 FAIL / 0 SKIP: a default-parallel library test assumed
the global registry would remain immediately vacant after topology returned.
Another fixture could legitimately acquire it. Both real topology requests had
completed, including structural-cache reentry; the test now waits for ownership
instead of using immediate `try_lock` success as an ownership guarantee. This
changes only the test assertion, not production locking. The failure remains
recorded, and final acceptance requires fresh parallel first/retest and frozen
verification after this correction.

Both corrected default-parallel library runs passed 764 tests with zero failures
and one ignored benchmark (12.87 s / 13.13 s), on
cc7f622dc7656cd000e4c207008badc1be5bc9634a014b3ba49c40716a13d49b.
The complete focused/prerequisite sequence restarts on this unchanged corrected
source before merge; prior source and deployed failures stay in their receipts.
