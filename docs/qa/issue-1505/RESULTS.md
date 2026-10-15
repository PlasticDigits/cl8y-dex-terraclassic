# Native quote recovery results

Investigation and verification for
[#1505](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1505).
See [the checklist](README.md) and
[NR1505 invariants](../../../skills/AGENTS_NATIVE_QUOTE_RECOVERY.md).

Results are recorded separately for source, deployment and focused live checks.

## Reproduction on the previous deployment

Both Coolify applications still served `14055610ebe86fd3640ff85e8adc15e16785c6dd`
at investigation start. The [50-second initialized socket baseline](baseline-socket.json)
passed 38 useful quotes; its cold first request remained separately recorded.
The [190-second sustained baseline](baseline-sustained-socket.json),
2026-10-15 06:41:01–06:44:12 UTC, reproduced the issue: 142 executable results
and four `missing_observations` HTTP503 responses, all in CL8Y→USTC. The opposite
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
API probes returned503. Both phone windows reproduced expired display: native
input remained visible 2,000ms past the latest matching original expiry and native
output 1,225ms past it. The input-phone window also retained an original-window
validation failure. Desktop windows had no display flags. This complete baseline
remains FAIL. A separate 35-second timer diagnostic received replacements before
each deadline and could not distinguish timer delay from deferred React commit;
it establishes no acceptance pass.

## Demonstrated source defects

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

## Source verification

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
an absent build identifier. The CI-pinned, checksum-verified gitleaks8.30.0
tracked-file scan passed; the workstation's older8.18.4 scanner falsely matched
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
failed: 142 useful quotes and four reverse `missing_observations`503s, with
10,386ms sampled recovery. Useful initialized responses stayed within307ms.
The [strict v8 browser FIRST](deployment-1510-browser-interrupted.json) separately
caught two native-input `stale_observations`503s during its complete desktop
window. Useful paint was366ms, and no expiry or ownership flag appeared, but
availability remained FAIL. The run was stopped after the socket failure;
reverse browser windows, unchanged retests and controlled expiry were not run.

[Bounded router diagnostics](deployment-1510-router-deadline.json) identified
explicit two-second router cancellations, including the poll preceding all four
reverse failures. The deployed provider-attempt timeout is2.5seconds: the extra
wrapper deadline could cancel an otherwise permitted attempt, including admission
and identity work, before its provider timeout. Starting earlier did not resolve
this mismatch. Native-input stale failures are a separate trace and must also be
resolved before acceptance. These receipts remain FAIL throughout subsequent
implementation and deployment loops.

The [input failure window](deployment-1510-input-failure-window.json) showed
fresh wrapper observations and all22 raw pools refreshed before both failures.
[Adjacent useful response clocks](deployment-1510-input-clocks.json) had only
704/706ms left before their original expiry; each503 arrived about589ms after
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

A real-HTTP regression permits a2.2-second router response under the existing
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
The native verifier runs18 wrapper regressions, the HTTP stale-witness and
request-correlation regressions,36 frontend tests,38 browser QA controls and
nine probe controls per pass. Lifecycle checks passed188 frontend tests per
pass. Provider recovery completed its disposable integration suites twice.
Separate live/funded scope remains explicitly skipped by those source verifiers;
it does not establish deployed acceptance.
