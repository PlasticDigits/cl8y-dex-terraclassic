# Trading entry, layout, lifecycle and order metrics release

Issues: [#1497](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1497),
[#1498](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1498),
[#1499](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1499),
[#1500](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1500).
Issue bodies and all comments were read on 2026-10-14 UTC; all four had zero
comments at investigation. Related old quote/source verifiers are retained.

## Implementation

- Bare Trade selects a genuine eligible factory market and stays in the advanced
  workspace. Explicit identities and bounded prefills win. Repeated active Trade
  navigation preserves a deliberate market and browser history. Native Swap's
  exact configured USTC→CL8Y default remains separate.
- Swap keeps one fixed summary/status region below the inputs and one persistent
  DETAILS disclosure. Disconnected valid quotes display their checked minimum;
  anonymous display does not permit signing. Costs and actual operations stay
  aligned, and the LUNC network fee appears once.
- Fresh same-intent output/route/costs remain during recoverable refreshes while
  signing is blocked. Original expiry and genuine negative/context changes revoke
  stale display/payloads. Missing scheduled router/provider observations are distinct
  from authoritative incompatibility. A stable publication anchor avoids observer
  renders postponing a ten-second refresh toward expiry.
- Order slippage independently compares selected original fee-free execution with
  chained original spot output. Pool/book commissions, taxes/hooks/wrappers and
  independent reference divergence remain separate. Signed favorable values,
  canonical amounts, reserve/book replay and original-cohort binding are validated.
  Original fair-reference 5/30/99 safeguards and exact-net ranking remain intact.

## Evidence and retained limits

All 16 required/relevant source verifier commands completed with passing available checks. Explicit unavailable acceptance remains separate; deployed sustained acceptance is recorded separately below. [Source receipt](source-verification.json) records scopes and failed-run retention.

Focused checks already passed: Trade entry/navigation; 164 lifecycle frontend
checks and two backend classification/recovery cases; 102 shared metric/frontend
checks, six independent Rust arithmetic cases and type checks. The lifecycle
fake-clock test forces 99 unrelated renders before the ten-second refresh.

Owned funded pool fixture: 24 successful cases across legacy/route-local-v1,
one through four hops and three exact raw sizes. Actual included gas and signed
fee/messages, independently calculated delivered output and rejected-floor atomic
rollback were checked. Complete current DEX contract input equivalence covered
170 files and tracked artifact hashes. See [pool receipt](funded-offline-pools.json). This does not re-attest the original build
recipe, native/tax/hook/book/physical-wallet worst cases or mainnet settlement.

Read-only Grafana/Loki before deployment returned zero error-pattern lines for
both services, with provider-capacity-pattern activity. No Prometheus datasource
is provisioned; this is explicit unavailable monitoring scope, not a pass.
Aggregates alone cannot establish quote/deployment readiness.

Retained first failures: the initial #1498 verifier had obsolete disclosure
selectors and failed browser fixture initialization/refresh/expiry checks while
source was still changing. A later browser retest passed 32/36 checks and exposed
four real postponed-refresh cases; the stable observer anchor fixes that defect.
The initial Rust metric test incorrectly expected 10 rather than 15 taker-fee bps;
its corrected independent formula passed. Initial LocalTerra provisioning had a
root-owned tmpfs, then the image's default insufficient genesis funding; the owned
fixture was reset with UID1000 and the repository's explicit funded genesis,
after which all 24 cases passed. These failures are retained and never relabeled
as successful acceptance. No production/shared infrastructure was reset.

## Acceptance tracking

Individual checklists remain at [Trade entry](../issue-1497/README.md),
[Swap layout](../issue-1498/README.md), [quote lifecycle](../issue-1499/README.md)
and [order metrics](../issue-1500/README.md). Invariants: [TE1497](../../../skills/AGENTS_TRADE_ENTRY.md),
[SL1498](../../../skills/AGENTS_SWAP_LAYOUT.md), [QL1499](../../../skills/AGENTS_QUOTE_LIFECYCLE.md),
[OS1500](../../../skills/AGENTS_ORDER_SLIPPAGE.md).

## Additional completed owned checks

The [production-build geometry receipt](browser-geometry-source.json) passes all
36 checks across 1440/320/390/430 widths, including keyboard, actual route identity,
disconnected minimum, one fee row, delayed refresh, negative/missing/recovery,
original expiry/new intent and 200% text zoom. It records zero provider requests,
28 local quote requests, no page errors, exact built JS hashes and dirty-source
provenance; it does not claim deployed acceptance.

[Native first/retest summary](native-funded-summary.json) records 24 delivered-net
settlements and 48 actual high-hop/high-final-floor atomic rejections. Both trials
use the same 170 DEX source inputs and 47 wrapper inputs from reviewed companion
revision `519c2766e5be0a3788a4c920ad879b641a50aa74`, independently built with locked
Rust 1.83. All compared economics and artifacts remained equal. Native gas was
2.527–2.531M and wrapped gas 2.121–2.122M under synthetic 12M requests; these are
not production gas calibration. [Independent arithmetic](native-metric-arithmetic-first.json)
compares native/matching-credit/equal-gross predictions against each observed
cohort. A separately executed same-cohort matching-credit control, native-output,
tax/hook/book populated fixtures and physical signing remain distinct incomplete
scope. The existing inspected external main/artifacts lacked the current direct
floor protocol; the reviewed companion source was built in an isolated worktree.
Only the owned native container was cleaned up.

The stable-budget pool first/retest also passed 24+24 cases on new owned contracts;
original contract inputs and the offline-budget source hash did not change across
the runs. Subsequent advisory book binding changes alter no transaction messages,
contract bytes or gas-family budgets; those already completed actual receipts are
preserved separately from the final source verifiers' optional funded skips.

A final advisory consistency regression changed an unused maker row while
preserving original execution receipts, producing an incorrect favorable metric.
The bounded original maker terms now bind the counterfactual, including unused
rows; its failed synthetic input and passing regression are retained. Original
fair-risk/signing controls were unaffected. Manual-plan diagnostic policy is also
kept aligned with its final execution policy; a proven mismatch was fixed at the
classification point. These were working-tree findings fixed before release.

## Final available source verification

Passing available checks: `1497`, `1498`, `1499`, `1500`, `1483`, `1489`, `1473`,
`1480`, `1452`, `1479`, `1398`, `1381`, `1390`, `1396`, `1464`, `1453`.
Source/type/lint/format checks and original signing/fee/floor/tax/launch/reference
controls passed. Explicit optional live/device scopes remain unchecked.

The full [controlled browser matrix summary](browser-matrix-source-summary.json)
is complete, untruncated: 4,250 first/retest rows, 3,948 useful quotes, 302 typed
blockers, zero failures, 1,988 local quote requests and zero provider requests.
Useful original-input-through-render p50/p95/p99/max were 80.8/138.5/157/550.6ms
against the specified 750ms browser budget. The full raw receipt is retained
privately with its published hash; these synthetic fixtures use zero-fee wrappers
and do not claim deployed economics or settlement.

The #1473 run's only failed check was a documentation literal removed while
reconciling Trade defaults. Its exact documentation function passed after restoring
native USTC wording; all unchanged executable checks and the full browser matrix
were already passing, so that documentation-only correction did not erase or
relabel the failed first command. The authenticated #1396 fixture also initially
failed because our disposable cluster retained default PUBLIC connection to its
control/template databases; those owned ACLs were repaired and its full verifier
retested with 28 PASS, zero FAIL. Production databases were untouched.

Tax guidance now lives in DETAILS, keeping route-funding wording out of Pay's
height. All 122 affected page/receive/tax tests and the tax-disclosure placement
assertion passed; the final geometry and first/retest #1498 verifier passed on
those final source bytes. Original quote economics, contract bytes and budgets
were unchanged by this final placement correction.

The #1381 driver also executed funded route-tax migration, funding and atomic
invoice/token/native flows successfully. Its raw receipts include synthetic test
actors/contracts and remain private; no production custody is claimed.

## Merged release and deployment

[PR #1501](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1501)
merged implementation `b8db3066b8881246a713656b3ccc4a4cda5c28e4` into main
`9dfa373f989ca9684c00ce40bc0525265243dfe9`. The
[required CI receipt](ci-release.json) records all eight successful pipeline steps.
Both Coolify deployments finished at that exact main revision. The
[independent deployment attestation](deployment-attestation.json) matches backend
health, finished frontend/backend metadata and complete served root script hashes.
The [build-log summary](deployment-build-summary.json) records zero fatal patterns
across 100 frontend and 372 backend build messages. Runtime logs and live browser
acceptance remain separate from successful builds.

After merge, both main and the feature worktree synchronized with origin/main.
Later QA-helper or evidence-only edits do not attest a new deployed revision;
release receipts continue to name the actual deployed revision above.

The additional `make verify-issue-713` run passed all eight available checks;
its LocalTerra frontend URL-sync smoke remained unavailable because that exact
frontend/deployment environment was not prepared. Its internal skipped-as-PASS
label is explicitly recorded as SKIP here, rather than funded/live acceptance.

Physical-wallet preparation successfully attested the same revision and obtained
native/matched-credit positive-floor controls. It is read-only and cannot establish
physical signed gas or settlement. The remaining physical-wallet action was
submitted using the [inbox-assigner skill](https://git.cl8y.com/PlasticDigits/cl8y-pm/src/branch/main/skills/inbox-assigner/SKILL.md)
to cl8y-pm as a queued card linked to
[the operator tracking issue](https://git.cl8y.com/PlasticDigits/cl8y-pm/issues/143).
The API rejected a direct DEX link; the accepted card includes the DEX issue URL
and an exact secret-consuming preparation command, without credential values.
One fresh reviewed physical-wallet attempt remains human custody. Completing
that card cannot establish unrelated contract/oracle/matrix acceptance.

## Deployed Trade first/retest

[First](live-trade-first.json) and [retest](live-trade-retest.json) each pass
15/15 checks across 1440/900/390 px (30/30 total): fresh direct entry with risk
acknowledgement, selected-market reload, genuine deep link with Market/Limit
tabs, all navigation surfaces, repeated clicks and Back/Forward. Root and Trade
asset hashes match the independently attested deployed revision. No wallet
connect/sign/broadcast is performed. Connected-wallet, full prefill/abuse,
outage/offline and stale-chunk browser acceptance remain separate.

The [original live receipt](live-trade-first-original-failed.json) retains its
13/15 verdict. The compact-header link has ARIA role `menuitem`; the QA helper
incorrectly searched `link`. A helper-only role-aware selector correction
resolved both that check and its dependent history check. The corrected receipt
records the helper patch/hash separately from deployed application provenance.
The corrected #1497 source verifier also passed 159 tests and type/crosslink
checks. No product behavior was changed by this QA correction.

## Retained deployed lifecycle failure and producer follow-up

The [original concurrent-tab summary](live-lifecycle-original-failed-summary.json)
retains FAIL. A foreground, sequential four-window shard per independent browser
corrects background paint timing and accounts for the existing one-Hz advisory
progress traffic: 600 solve/progress reads and 16 minutes per shard, three shards,
12 total windows and 1,800 maximum reads. It does not reinterpret the original
shared-600-cap failure. The [foreground first summary](live-foreground-first-summary.json)
also remains FAIL: 12 completed 190-second windows, 11,032 samples and 1,163 reads,
with zero writes/provider calls. Five useful browser timings exceed 750ms,
including 11.25/23.41-second native-receive recovery. Geometry, matching economics
and unexpired retention passed their available checks, rather than the full run.
One old DOM-versus-later-Node expiry signal remains an executed, unresolved
failure; the observer now captures epoch/eligibility at the same in-page instant
and reports detailed expiry evidence for the next run. This is a QA correction,
not proof that the original signal was harmless.

A bounded independent cLUNC diagnostic measured 374ms from cold original input
and 97ms after a deliberate new warm input. It does not erase the original
1,519ms failure; its slower original sample was already 1,397ms at response body,
well before DOM/paint. Other retained 834/951ms samples were similarly delayed
before body arrival. No input clock is reset to make these timings pass.

Native-output publication had a concrete scheduling defect: four independent
reads ran sequentially inside one two-second optional phase, and any timeout
discarded already completed required fields. The follow-up observes those fields
concurrently under the same absolute deadline and retains individual successes.
Mapper/treasury floor capabilities use the same bounded independence. Unknown
required tax/router/caps still refuse execution; protected wrap requires both
capabilities. Original poll-start freshness, two permits/four RPS, the 7.9-second
publication margin and eight-second outer deadline remain enforced. There are no
new provider queries, detached tasks, stale reuse or request-triggered RPC.

Focused producer first/retest each pass 13 tests, including seven new adversarial
cases: original pacing/call counts, slow unrelated cap, unknown required tax,
genuine unsupported configuration, cancellation/permit release, partial floor
capabilities and zero-budget/no reads. The real LCD ingestion-budget test passes
first/retest too. Each test owns fresh scheduling/budget state. Broader verifier
and new deployment results are recorded separately after their completion.

## Scoped Trade recovery evidence

[Recovery summary](live-trade-recovery-summary.json): injected catalog/panel API
outage, offline lazy import, one-shot stale chunk and repeated stale chunk with
explicit reload, across desktop/phone. The first 8/10 FAIL retains two desktop
30-second readiness timeouts with insufficient earlier diagnostic detail; its
unchanged retest passes 10/10 with matched per-case root/Trade bytes, bounded
1/2/3-document counts and zero provider/mutation/hash mismatches. Interrupted
optional asset-body reads during intentional reloads are reported separately.
Synthetic consumer recovery is not actual provider-outage acceptance. Connected
wallets and the earlier unclassified readiness waits keep AC6 incomplete.
The exact executed helper is archived; later formatting-only changes are
identified separately in the receipt. The original short-wait/selector and
body-cancellation QA failures are also retained privately.

## Final producer source verification

[Producer verification receipt](wrapper-producer-verification.json) records final
runtime input hashes. #1499 passes 10/0/1, #1483 and #1464 pass their available
source/socket/frontend checks with explicit external skips. #1452 passes all
library/socket/frontend/type/format/source groups on first/retest but its full
command retains FAIL (13/1/0): one controlled desktop sample measured 401.2ms
against 250ms. Headers started promptly, body completion was delayed 364.6ms,
and no frontend long task was recorded. No deterministic product/transport cause
was established, and TCP_NODELAY was already enabled.

After resetting the owned HTTP/browser fixtures, the exact failed browser step
passes first/retest, 40/40 samples each, maxima 49.9/48.0ms against unchanged
250ms; both have zero provider requests. All 88 observed fixture socket finishes
were within 0.403ms. These targeted passes supplement the unchanged passing
source groups; they do not relabel the original full command or lost live clocks.
No unrelated already-passing group was repeated merely to erase that failure.
The next merged producer release's exact deployment and live first/retest verdicts
are recorded in the linked issues' comments; earlier deployed receipts above
continue to describe 9dfa only. Physical signing and full acceptance remain open.
