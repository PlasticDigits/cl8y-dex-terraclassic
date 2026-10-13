# Route attribution results (#1483)

## Retail warning follow-up

Implementation: [PR #1485](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1485).

The initial deployed implementation used a technical metric label and left
overlapping price, confirmation, funding and speculative refresh messages.
The follow-up uses **Price difference**, moves ordinary <=5% information into
existing Signing details and selects one current operational blocker or consent
notice. Disconnected wallets do not show failed balance reads, and inline
transaction errors are not duplicated in a toast. Preserved included/unknown
transaction status cannot hide the current price during confirmation.
Missing advisory cost data is silent in the primary flow. The required
risk acknowledgement, exact 5/30/99 checks, original review binding and every
independent execution gate remain; tests also cover direct/stale callbacks for
known pause, freeze and blacklist conditions. Whole acceptance remains scoped
to the receipts below, including the outstanding device/contract evidence.

Issue: [#1483](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1483).
Implementation [PR #1484](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1484),
initial implementation `8227ed351a072b69de938dc07d9cfdf0c09d41a7`.
Final merged/deployed revisions are attested separately. Follow [the checklist](README.md) and
[RC1483](../../../skills/AGENTS_ROUTE_COST_ATTRIBUTION.md).

## Supported source scope

Generic signed raw-output attribution retains independent fair-reference quality,
the exact aggregate warning, authorized native gross and original expiry. One
same-cohort synthetic fixture independently calculates 6/18-decimal constant
product/fee outputs for direct and eligible UST1/CL8Y alternatives. It warms a
small quote before large inputs and repeats the small input, testing changing
amount-specific winners rather than scaling a cached route. Native 2% wrapping
is compared with matching wrapped credit and equal wrapped gross separately.
These are owned synthetic inputs, not a historical report reconstruction.

Exact verification counts, UTC/source fingerprints, deployment/tool/artifact
provenance, timing distributions and failures will be appended after final
stable first/retest and deployment. Test logs stay private when they may contain
synthetic actor or local endpoint fields; public summaries retain bounded counts.

## Retained failures and unavailable acceptance

SKIP: original reported amount/protection/cohort/frontend/wallet is unavailable;
no similar percentage establishes historical reproduction. Original atomic
production SQL and physical-wallet/device signing are separate missing evidence.
Complete native/unwrap/tax/hook/book/attack acceptance and deployment are not
established merely by the focused attribution fixture. Gas calibration remains
[#1480](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1480), independent
small-market/oracle integrity remains
[#1479](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1479), and external
DEX/splitting design remains
[#1330](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1330).

## First verification and retained failures

At 2026-10-12 UTC, the first independent amount-cache fixture passed on the
reviewed working-tree patch based on `d65228c2970a1ec7b11d81000d3f83ceb58132c4`.
Its initialized useful socket sample (500ms budget) was n=14, p50=1.420ms,
p95/p99=1.666ms, maximum=11.772ms. Follow-up assertions independently check every
raw attribution component from exact original reserves/marks and actual fee math.

Retained failures: concurrent unfinished source caused source-fingerprint FAILs;
an old frontend copy assertion and two unused imports caused the first #1480
frontend/type FAIL and were corrected. A first broad #1483 run also failed the
existing pending-fee provider fixture's three-second gate on its last case during
concurrent testing. The failure remains recorded; successful fresh #1479 runs of
the same original test are tracked separately and no production TTL was relaxed.
Final stable source first/retest verdicts follow below.

The new verifier's initial wallet-helper command had a filename typo and failed
both rounds; correcting the path produced passing 7/7 helper tests each round.
An owned #1473 funded retest later timed out in the historical 45-second CLI
inclusion acquisition after six positive cases. A bounded owned ledger search
found the latest included code-5 transaction with the matching RAW18 size-100,
1%-spread input, but the historical helper did not retain the original hash, so
this is not exact original-hash attestation. It was never automatically resent.
The ledger was retained privately before resetting only owned chain state.
The corrected driver reuses the existing #1480 bounded REST monitor and private
accepted-hash receipt, with current artifact/source preflight.

The broad #1452 first browser pass succeeded; its retest failed before measurement
while waiting 20 seconds for Trade input initialization. A fresh owned retry later
failed the original 30-second navigation clock. Private diagnostic reruns retained
both original clocks and found separate incomplete receive-output and transport
timing failures. These remain FAIL until their documented corrections/retests;
they do not invalidate passing exact raw socket economics, nor establish an app
or host diagnosis without independent evidence.

A further independent regression reproduced an actual ranking defect: the
bounded deeper worker evaluated a strictly higher-net three-hop route but did
not publish it when its improvement was below the undocumented 0.1% threshold.
The exact fixture's initial direct output is 997,000 raw, with a three-hop output
of 997,498 raw on the same original cohort. The red run retained
the initial inferior output through 100 bounded cache requests and failed. The
fix compares strict positive raw output without changing work caps or reviewed
transaction binding; green/retest receipts follow below. The user's separate
small-amount-cache hypothesis was tested with changing direct/two-hop winners:
exact amount/generation keys already prevent linear scaling or stale winners
from deciding the larger amount in the supported bounded cohort.

## Owned funded pool subset

The [first funded receipt summary](funded-offline-first-summary.json) records
24 supported CW20 pool-only, hook-free, empty-book settlements and 48 intentionally
rejected hop/final floors. Legacy and route-local-v1 each cover 1–4 hops and raw
inputs 10,000/1,000,000/1,000,000,000 in six-decimal assets. Independently
calculated delivered net/floors, original wallet principal, reserves and LP rollback
pass. Requested/signed gas agrees; gas used remains below the unchanged budget
with the fixture's required 20% headroom. Network fees are separate evidence.
Tracked artifacts and complete current contract/package/lock inputs match the
provenance preflight; original build recipe/production artifacts are not re-attested.
This subset does not establish native composition, tax/hooks/books or device
acceptance. Only the explicitly owned isolated chain was created; no shared or
production state was reset.

## Stable final source verification

[Final focused source receipt](source-final-summary.json): **PASS WITH SKIPS**,
17 passing groups, zero failures and two explicit complete-funded/device/deployed
acceptance skips. Both rounds passed 7 public-probe controls, 7 private-wallet
helper controls, 8 independent attribution tests, 1 per-maker book-commission
rounding test, 2 same-cohort socket/ranking fixtures and 225 actual frontend tests.
Types, Rust formatting, helper/verifier syntax, documentation and stable-source
fingerprint checks pass. Exact source fingerprint:
`2f4730de4ca1a1759598986a69765e2c6af4425669750ba996c76dab953cb248`.
The additive API/OpenAPI response schema is separately covered by two passing
regressions; source/backend/served root attestation remains a separate requirement.

Initialized useful socket timing, 500ms budget, n=14 each round: first
p50=1.543ms, p95/p99=1.892ms, max=10.523ms; retest p50=1.492ms,
p95/p99=1.936ms, max=10.745ms. The exact deeper regression improves
997,000 to 997,498 raw units on one unchanged cohort, while retaining the useful
initial direct result and original generation/floors. No request-provider calls
occurred. These are loopback synthetic measurements, not a deployed SLA.

The separate #1480 stable-source retest passed 10 groups, zero failures and three
explicit broader funded/device skips, with 370 frontend tests per round. Additional
normal verifier and funded first/retest summaries are recorded as they finish;
interim source-fingerprint failures remain explicit because source changed during
those runs. Passing economic assertions cannot be labeled stable final receipts
until the final checksum is recorded.

## Owned native composition subset

[Native atomic green receipt](funded-native-green-summary.json): 12 positive
settlements and 24 intentionally rejected hop/final floors passed on fresh owned
state after fixing the local fixture's narrow read-only loopback EOF handling.
The earlier run retained eight passing cases before that pre-transaction read
failed; no accepted transaction was resent. Supported native wrap plus one pool
hop and wrapped one-hop routes cover 1/5/30% floors and independently chosen
1/1,000-token inputs. Native positive cases prove custody increases by authorized
gross, supply increases by exact 98% credit, rate usage increases by gross and
pre-existing wrapped wallet inventory is unchanged. Wrapped positive cases debit
only exact principal and leave wrapper custody/supply/rate unchanged. All output
wallet net/floors reconcile independently; rejected cases restore custody, supply,
rate, wallet principal, reserves and LP shares. Paid transaction ante fees remain
separate. Gas used ranged 2,120,583–2,530,859 with an explicit 12M synthetic budget;
this is not production gas calibration or device inspection. Companion wrapper
source/input and artifact byte hashes are recorded, with the documented Rust1.83
release-library recipe; this does not attest a deployed wrapper release.

## Owned infrastructure correction and clean browser evidence

Host disk I/O pressure caused separate owned fixture failures. The native retest
retained its original accepted hash privately at the unchanged 120-second
deadline; exact RPC and REST reconciliation subsequently confirmed the intended
code-5 floor rejection. Another accepted setup transaction reconciled code 0.
Neither was resent. Original ledgers/failures remain private. Only the owned
LocalTerra data, future disposable Postgres state and browser/build scratch were
reprovisioned into bounded tmpfs; runtime durability, budgets, assertions and
measurement clocks were not relaxed. Active old SQL fixtures kept their cluster
until their processes completed. Tmpfs does not establish machine-crash persistence.

[Clean original browser first/retest](browser-controlled-summary.json) used
fresh owned private tmpfs scratch with the original uninstrumented #1452 helper.
Both rounds passed all 40 cases across desktop/mobile Swap/Trade, maxima 50.6ms
and 52ms at the historical 250ms budget. There were zero quote-triggered provider
calls and page errors. Private controls distinguished immediate server flush/body
delivery from disk-backed renderer stalls. Historical disk-backed failures stay
recorded; the new passing tests do not rewrite them. This is controlled browser
evidence, separate from live artifacts and the issue's named 750ms deployed criterion.

## Related security checks

[Normal related-verifier summary](security-verifiers-summary.json) records
#1391 (6/0/2), #1398 (7/0/0), #1381 (12/0/2), #1390 (14/0/2) and #1396 (28/0/1),
as PASS/FAIL/SKIP groups respectively. Contract source stayed unchanged; the
parent's additive API schema edits are explicitly scoped, with final focused
source evidence recorded separately. Full contract workspace checks covered 1,099
tests; actual ACL/socket/role and authenticated runtime-login checks passed twice.
Two original #1396 missing-runtime-role fixture failures were retained, corrected
only on owned QA clusters, and rerun. Host IPv6-disabled and unavailable broader
chain/live oracle acceptance remain explicit skips, not production privilege proof.

Final stable #1479 verification passed 12 groups, zero failures and one complete
funded/live acceptance skip; the original pending-fee gate passed without extending
its three-second test or four-second production window. Economic reporting remains
at $10 and independently priced hub depth remains $100.

AC7 is complete in the reviewed source scope: API meaning/nullable OpenAPI schema,
integer rounding, exact ranking, retail/fee separation, issue/PR/checklist and
agent playbook links are updated. Protocol/reference/device/verification limits
remain explicit. AC1–AC6 full boxes remain unchecked pending their complete
body/comment coverage; individual passing subsets above do not close the issue.

The clean tmpfs native [first](funded-native-tmpfs-first-summary.json) and
[retest](funded-native-tmpfs-retest-summary.json) each passed 12 positive
native/wrapped settlements and 24 intentional floor rejections. Both retain the
same raw principal/custody/supply/rate/output/LP assertions and original
120-second inclusion deadline. This completes their supported one-hop subset,
not native-output/tax/hook/book/device or production acceptance. Clean current
artifact 6/18/9-decimal protection [first](funded-protection-tmpfs-first-summary.json)
passed all 24 cases at 1/5/30%, with independent wallet net and rejected-floor
reserve/LP rollback. The matching
[retest](funded-protection-tmpfs-retest-summary.json) also passed all 24 cases;
legacy false-spread rejections and poisoned positive floors restored the original
reserves/LP state, followed by exact code-0 delivery. Included-hash monitoring
never resubmitted accepted/uncertain transactions. Native output and complete
tax/hook/book/device/deployed acceptance remain separate.

The first full #1473 controlled browser matrix completed 4,250 scenarios and
3,948 useful timing samples: p50=88.6ms, p95=145.7ms, p99=166.7ms,
maximum=1,629.7ms against the unchanged 750ms temporary budget. Four actual timing
cases failed; its concurrent-source fingerprint also failed. All other source/
socket/frontend groups passed. The original failed inputs/receipt remain private;
a complete fresh owned tmpfs rerun keeps the same input clocks/budget and is
recorded separately once finished. The passing smaller #1452 matrix cannot
substitute for those failed full-matrix cases.

The complete fresh original #1473 tmpfs rerun passed **all 4,250 scenarios with
zero failures**. Its 3,948 initialized useful samples measured p50=88.4ms,
p95=146.2ms, p99=164.1ms and maximum=236.3ms against the same named **750ms**
temporary browser budget. Original input/transport/DOM clocks and all protection/
visibility assertions were unchanged. The four original disk-backed timing
failures and concurrent-source verdict remain retained. This full passing matrix
completes that controlled source/browser scope; it does not attest deployment.

Root's final public probe tool added canonical Uint128 validation before exporting
fair/net raw fields plus required initialized native sentinels. Its updated
**8/8 first and 8/8 retest** tests are separate tool-only evidence; the application
source fingerprint above remains unchanged. The bounded deployed explanation
tool's **11/11 first and 11/11 retest** tests cover actual source-bound validators,
clock/caps, clipping, injection/duplicate/expiry/floor/actor restrictions and the
750ms boundary. Neither helper test suite claims an executed deployed matrix.

## Core deployment and live API

[Core deployment attestation](deployment-core-attestation.json) binds the served
frontend root and backend health to merged `db46e30a5011ee4cbd1998c1c97a34d966b07003`.
PR #1484 passed hosted pipelines 427 and 428 before merge; [both Coolify jobs](deployment-core-jobs.json)
finished with matching revision and zero fatal build-pattern entries.
[Live API receipt](live-core-quotes.json) covers two cycles separated by 185 seconds:
176 bounded reads, 154 executable useful quotes and no failures. Useful quote
p50/p95/p99/max were 282.435/283.348/283.783/284.251ms under the named 500ms budget.
Native 1/1,000/4,000 LUNC solve and best requests all produced executable results;
all returned raw component sums independently reconciled to fair minus net.
These current inputs are independent probes, not the unavailable original report.

[Monitoring aggregates](monitor-core.json) retain two indexer error-pattern entries.
[Source-message classification](monitor-core-classification.json) identifies both
as fee-discount LCD config probe failures; the follow-up public health read shows
recovery and zero consecutive failures. The 15-minute sample had 169 provider
capacity-pattern entries; passing quotes do not establish redundant provider
capacity. No Prometheus datasource is provisioned, so that check remains SKIP.
No raw log lines or private credentials are published.

[Complete controlled browser summary](browser-full-controlled-summary.json)
records 4,250 scenarios, zero failures and 3,948 useful results. Useful output
p50/p95/p99/max were 88.4/146.2/164.1/236.3ms at the unchanged 750ms budget.
This owned-infrastructure result remains separate from deployment and custody.

## Retail follow-up retained failures

The first follow-up regressions exposed missing alert semantics on the existing
Swap and Trade inline transaction error surfaces; both now expose one alert.
The initial fee-only fallback test invoked mutation before quote/precision
initialization, so its setup now waits for the validated quote without changing
production gates or measurement clocks. Earlier cause-copy fixtures were updated
to the final fixed labels. These initial failures remain recorded; stable final
first/retest results are separate receipts.

The first complete frozen-source #1483 follow-up run retained **17 passing groups,
2 failed groups and 2 external skips**: both frontend rounds found the same old
Trade freeze-banner text expectation. The other 285 frontend tests per round,
all Rust/socket/tool groups, types and documentation checks passed. The outdated
copy assertion is corrected while the original disabled-limit-action assertions
remain. A complete fresh first/retest is required for the final verdict.

## Stable retail warning verification

[Final warning source receipt](source-warning-summary.json) records **19 passing
groups, zero failures and two explicit external acceptance skips**. Each round
passed 286 actual frontend tests, 8 raw-attribution tests, 1 book-fee regression,
2 owned same-cohort socket/ranking fixtures, 8 public-probe controls, 7 private
wallet-helper controls and 13 browser-tool controls. Types, Rust formatting,
syntax, documentation and exact stable-source fingerprint pass:
`18f7de57c89921e51ff480a7e87d611ed6325b8e7aefd4a06cdc1e2522132ea2`.
Fresh owned disposable SQL children and React fixtures reset affected test state;
no shared or production infrastructure was reset.

The initialized useful sockets have 14 samples per round. First
p50/p95/p99/max are 1.438/1.683/1.683/9.925ms; retest are
2.317/2.850/2.850/15.165ms at the unchanged 500ms budget. The build with actual
public production configuration also passes without credentials in its child.
The broader #1452 selection exposed one stale SDK MaxSpread error-copy assertion;
the scan also corrected the matching TxResultAlert copy assertion while retaining
rejection/masking checks. Those two files passed all 70 tests before the final
stable receipt. Related frontend counts and hosted/deployed verification are
reported separately; final live follow-up receipts belong in the issue comments.

[Related normal frontend selections](source-warning-related-summary.json) also
pass first/retest on stable frontend source: #1452 has 499 tests in 27 files,
#1473 has 463 in 24 and #1480 has 382 in 20 per round. These overlap and must not
be added as unique tests. Unchanged backend, contract and owned-chain evidence
remains scoped to the earlier receipts. Removing unrelated formatting churn from
the shared browser helper left only its intended price-copy update; its actual
source-bound controls passed 13/13 first and 13/13 retest afterward.

Independent review, TypeScript and lint pass. Five lint warnings remain only in
unchanged files; none was added by this patch. The related receipt documents its
frontend/package/config-only fingerprint recipe, distinct from the focused
five-directory source fingerprint. Every mandatory acknowledgement and execution
condition remains independent of which one notice is presented.

## Full-suite copy correction

Hosted [pipeline 431](https://ci.cl8y.com/repos/7/pipeline/431/1) retained two
obsolete shared-error-wrapper text expectations. The other 3,363 frontend tests
passed; subsequent backend jobs were skipped by the failed dependency and are
not claimed as passing CI. Both expectations now require the exact short price
protection message while preserving nested raw-log masking. Independent review
passed 146 tests across six SDK/error-boundary/wrapper files. No application,
principal, signing or guard code changed.

[Post-CI focused first/retest](source-warning-post-ci-summary.json) passes all
19 runnable groups, zero failures and the same two external skips on fingerprint
`bd5abc96c16b4d2167a1c9dfa6b23030003c366494cad1d630f85352ab6ce31c`.
Both rounds pass 286 frontend tests, with fresh owned disposable SQL/React state.
Each initialized useful socket round has 14 samples: first
p50/p95/p99/max 1.460/1.714/1.714/10.392ms, retest
1.451/1.750/1.750/10.592ms at the unchanged 500ms budget.

The first full local invocation used repository cwd instead of the frontend cwd
and failed WalletModal's static public-file lookup; that executed setup failure
remains retained. Corrected full runs use the same frontend cwd and two workers
as hosted CI, fresh owned scratch and original assertions/clocks. Their final
counts and a new hosted verdict are separate from the failed pipeline.

[Complete frontend first/retest](source-warning-full-frontend-summary.json)
passes all **368 files and 3,365 tests in each round**, with two workers and fresh
owned scratch from the correct frontend cwd. Durations are 225.67s and 225.64s;
original assertions/clocks and all rejection/masking conditions remain intact.
No application or signing code changed for this full-suite correction. Hosted
CI is rerun on the corrected commit before merge; the failed431 receipt remains.

## Deployed root-script verification correction

PR #1485 merged normally after hosted432 passed every mandatory job. The exact
verified tree deployed as `33ef64a99e65f0b0c7116e79bb7c6860b303beda`; both Coolify
jobs finished with matching revision and zero fatal build-log matches. The
post-deployment API probe passed 176 reads over two cycles separated by 185s,
including 148 executable useful results: p50/p95/p99/max
288.940/291.812/292.481/292.752ms at the unchanged 500ms socket budget.

The first live UI checker failed before any user-flow measurement (zero samples,
two reads): it assumed all external root scripts were under `/assets/`, while the
app also serves `/bootstrap/theme.js` and `/bootstrap/trade-shell.js`. The initial
attester filtered those bootstrap paths, so its root coverage was incomplete.
That failed checker run remains recorded. The correction hashes every external
root script and checks full ordered path/basename/hash/byte-length identity;
known bootstrap plus canonical assets remain within the existing cap of eight.
Unknown, duplicate, ambiguous, foreign or traversal paths and base overrides
fail closed. Inline code remains bound by the full HTML hash. Tool hash/HEAD/dirty
provenance is separate from the unchanged deployed product revision.

Meaningful root-parser first/retest tests pass 9/9 each, browser-control tests
18/18 each and the same seven secret/no-sign wallet-helper tests each. Updated
wallet fixtures attest all three roots in 13 bounded reads. No app, API, market,
fee, signing or contract behavior changed in this verification-tool correction.

[Stronger all-root attestation](deployment-warning-all-roots.json) passes for the
same deployed #1485 revision. [Live API receipt](live-warning-quotes.json)
independently validates the 176-read replacement-cycle probe. Actual live UI
[first](browser-warning-first.json) and [retest](browser-warning-retest.json)
both pass **all 18 desktop/phone cases**, with 86 bounded API reads and 26 quote
reads per round. Each round has zero writes, actor requests, measured provider
requests and WebSockets. Fourteen low-risk cases per round verify that Signing
details open; four higher-risk cases have one visible price notice. All three
root script paths/hashes/byte counts match independently before and after.
Tool revision/hash/dirty fields remain explicit; these are pre-commit corrected
verification tools against the unchanged deployed product. Physical wallets and
settlement remain separate. Useful original-input browser timing stays under the
unchanged 750ms budget; exact per-sample timing is retained in both receipts.
