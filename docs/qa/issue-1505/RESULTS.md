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

Exact application deployment and focused live verdicts will be recorded after
merge; source passes do not establish those criteria.
