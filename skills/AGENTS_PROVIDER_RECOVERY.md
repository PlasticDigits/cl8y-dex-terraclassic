# Provider recovery and useful quote readiness (#1464)

Apply for provider scheduling, checkpoint retries, indexer/API supervision,
local admission publication, quote blocker classification or useful-result latency.
Issue: [#1464](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1464).
First implementation: [merged #1465](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1465).
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
| PR1464-9 | Count useful executable results separately from errors/loading/topology. Every representative initialized hit and result miss must be strictly <250 ms; report p50/p95/p99/max and sample counts, new/reused socket/browser transport, cold state and regional RTT independently. An executed latency failure remains FAIL. |
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
