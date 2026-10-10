# Provider recovery and useful quote readiness (#1464)

Apply for provider scheduling, checkpoint retries, indexer/API supervision,
local admission publication, quote blocker classification or useful-result latency.
Issue: [#1464](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1464).
First implementation: [merged #1465](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1465).
Registered-token/reference follow-up: [merged #1466](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1466).
Its [deployed trial](../docs/qa/issue-1464/deployment-1466.json) recovered initial
quotes but did not sustain coverage. Capacity waiting and typed static tax admission shipped in
[merged #1467](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1467); its
[deployed follow-up](../docs/qa/issue-1464/verification.md#deployed-1467-and-current-500-ms-target)
still fails sustained availability and first-useful latency. Caller-budget and
original-expiry repairs must retain the remaining deployed acceptance as unproven.
Its [deployed receipt](../docs/qa/issue-1464/deployment-1465.json) records a successful
rollout and failed useful-quote acceptance; use the current verification results,
not merged status or liveness, to assess trading readiness.

| Invariant | Required behavior |
|---|---|
| PR1464-1 | Provider capacity deferral and retryable upstream availability preserve the committed checkpoint and retry that exact height with bounded cancellation-aware scheduling. Never successfully skip, advance past or duplicate a failed economic block. |
| PR1464-2 | API serving and indexing have separate lifecycles. Transient indexing pressure cannot erase qualified in-memory quotes through process restart. Liveness alone never certifies accounting progress or trading readiness. |
| PR1464-3 | Reorg/checkpoint mismatch and nonavailability parser/database/state failures halt unsafe progress visibly. Record processing/state failure once with the durable unsafe marker and halt before replay can skip a partial economic write; marked pending heights refuse replay after restart. Only prewrite LCD errors use the finite permanent-error retry budget. Repair incomplete projections before explicitly clearing the marker; this is not an atomic crash guarantee. |
| PR1464-4 | Cloned clients share finite endpoint rate/concurrency/circuit state. Background pacing must not hold a provider permit while awaiting its pacing turn. Historical block/checkpoint/filtered-tx bodies must match the requested height and are not live-tip lag evidence. No additional client/queue may evade admission, and cancellation/leadership loss releases owned work. |
| PR1464-5 | Independent still-fresh observations remain eligible until their original expiry or authoritative invalidation. Failed refresh cannot renew age, delete healthy unrelated candidates, clear known actor blocks or combine incompatible pool/book generations. |
| PR1464-6 | Internal rejection reasons use fixed low-cardinality classes. Missing/stale/unsupported observations fail closed with an availability blocker; genuine protected execution rejection remains a protection blocker. Never advise arbitrary smaller sizes for infrastructure loss. |
| PR1464-7 | Quote, best, POST, progress, reference, refresh, preflight and reviewed-route preparation trigger zero RPC/LCD/network validation. Only independently scheduled producers acquire observations; traffic cannot accelerate their cadence. |
| PR1464-8 | Exact integer gross debit, route-local tax funding, positive post-hook/hop/final floors, pause/freeze/blacklist and refund ownership remain mandatory. Actor/amount/mode/funding/namespace/generation cache isolation and reviewed signing context remain intact; ≥99% protection is nonwaivable. |
| PR1464-9 | Count useful executable results separately from errors/loading/topology. The currently accepted temporary target is ≤500 ms for representative initialized useful hits/misses; the original <250 ms target is deferred. Report p50/p95/p99/max and sample counts, new/reused socket/browser transport and cold state independently, without operational location metadata. An executed latency failure remains FAIL. |
| PR1464-10 | Finish feasible checks and retest. Unavailable operational/device/fixture evidence may be SKIP with overall PASS WITH SKIPS, exact prerequisite/reason/revision/UTC scope and unchecked criteria. Executed failures stay FAIL until fixed and rerun. Source acceptance never closes missing deployment evidence in #1456. |

Preserve [LQ1452](AGENTS_LOCAL_SWAP_QUOTES.md), [LF1453](AGENTS_LIQUIDITY_FEED.md),
[PD1394](AGENTS_PAIR_DISCOVERY_CACHE.md), [NET1392](AGENTS_CHAIN_ATTESTATION.md),
[DB1396](AGENTS_INDEXER_DATABASE_ISOLATION.md), [NE1398](AGENTS_NET_EXECUTION_FLOORS.md),
[RT1381](AGENTS_ROUTE_LOCAL_TAX.md) and [LG1390](AGENTS_ROUTE_LAUNCH_GUARDS.md).
For reference scheduling preserve [O1391](AGENTS_ORACLE_SAFETY.md): independent
bounded ticker/hub timers, unchanged source quorum and expiry, original poll-start
times, clock-based CoinGecko pacing and owned cancellation. A stale wrapper or
missing reference is availability loss; fresh observed amount limits remain protection.

Run `make verify-issue-1464`, then the existing #1452/#1453/#1394/#1392/#1396
verifiers. Follow the [AC1–AC9 agent checklist](../docs/qa/issue-1464/README.md)
and [recovery/readiness runbook](../docs/runbooks/indexer-provider-recovery.md).
Use workstation `bao-exec` from openbao-tools for scoped secret injection; no
raw bodies, actor identifiers/digests, credentials or private endpoint URLs in receipts.

Implementation: [readiness/supervision](../indexer/src/indexer/lifecycle.rs),
[pending block retries](../indexer/src/indexer/block_indexer.rs),
[checkpoint/worker ownership](../indexer/src/indexer/poller.rs),
[shared provider scheduling](../indexer/src/lcd/mod.rs),
[scheduled qualification](../indexer/src/api/local_quote/admission.rs),
[schema/pins](../indexer/src/api/local_quote/pair_admission.rs),
[fixed blockers](../indexer/src/api/local_quote/diagnostics.rs),
[SQL-only trader accounting](../indexer/src/indexer/trader_tracker.rs) and
[API lifecycle](../indexer/src/main.rs). Durable unsafe-height quarantine uses
[failed-block state](../indexer/src/db/queries/state.rs) without a startup migration.

Static tax admission precedes optional wallet enrichment. Typed provider loss
preserves only original unexpired qualification; observed policy or registered-pair
preview changes revoke it before later awaits. Completed manager=false or
protocol=true flags revoke any existing manager exemption before balance awaits,
including the current pass clone. New wallet positives require fresh flags/balance
and unchanged static-policy recheck. The [tax cancellation regressions](../indexer/tests/tax_admission_recovery_1464.rs)
cover these boundaries and recovery without quote-triggered RPC.


Do not collect or publish operational location metadata. Label the client
network path unverified and retain measured timing failures. See
[HTTP/3 transport evidence](../docs/qa/issue-1464/verification.md#http3-reachability-and-transport-evidence).
Confirm real socket/protocol reuse; health or handler-only timing never establishes
useful quote latency. This rule also applies to agent notes and PR descriptions.


Scheduled caller cancellation is not provider failure: for a trusted independently
scheduled observation, release an incomplete closed-provider attempt without
success, failure reset or rate refund. Generic cancellations and incomplete
half-open probes still quarantine; observed HTTP/transport and the provider's
own attempt timeout still quarantine. Keep the same shared client state, caps,
attestation and deadlines. Record caller_cancellations separately.

Capture published pair membership and each original configuration expiry together
before any database await. A pair crossing expiry during publication must fail
closed independently; it cannot inherit a fresh sibling's global maximum deadline.
The [score regression](../indexer/src/api/local_quote/admission.rs) covers this race.

Latch known provider negatives before any asynchronous metadata/failure recording.
Lock contention followed by caller cancellation must retain quarantine and known
typed integrity evidence; it must not turn a recorded negative into a neutral
caller cancellation. Keep failure recording exactly once, including own attempt
timeouts. See the [LCD regressions](../indexer/src/lcd/mod.rs).
