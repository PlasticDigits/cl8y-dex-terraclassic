# Indexer provider recovery and readiness (#1464)

Contract: [PR1464](../../skills/AGENTS_PROVIDER_RECOVERY.md).
Verification: [AC1–AC9 and skip policy](../qa/issue-1464/README.md).
Related: [database isolation](indexer-database-isolation.md),
[reorg/replay deduplication](indexer-reorg-replay-dedup.md),
[local quote contract](../route-solver.md#local-quote-contract-1452),
[cached feed reliability](../qa/issue-1453/README.md).

## Assess availability without restarting serving

`/health` establishes API liveness. `/ready` separately reports the process-owned
indexer state (`starting`, `running`, `provider_unavailable`, `halted`, `stopped`),
checkpoint and last observed latest head. It is ready only when indexing is running
and its checkpoint has reached that observed head. Fixed reasons distinguish provider
unavailability from halted integrity work. Neither endpoint establishes fresh eligible
routes, complete economic history or trading readiness. Head evidence itself can age;
never use a remembered head as a live chain claim.

Provider admission deferral and retryable availability pause the same pending height
using a cancellation-aware interval clamped to 50 ms–30 s. They do not consume the
finite prewrite-LCD error attempt budget. Nonavailability processing/database/state
errors halt immediately rather than replaying partial writes. No failed height is successfully skipped.
Reorg, checkpoint/hash mismatch, block-integrity/accounting/database failure and
leadership loss halt unsafe work visibly. The API remains available; halted integrity
state blocks executable quotes, while transient provider unavailability permits only
still-qualified cached quotes. Expired or missing prerequisites always fail closed.
Retrying availability never manufactures accounting success or renews timestamps.

The dedicated leadership lock connection is monitored with a 500 ms heartbeat and
500 ms bounded probe. Detected loss cancels child workers; owned tasks join when the
poller exits. Brief detection overlap is possible; this is not distributed exactly-once
processing. Checkpoint/idempotency boundaries still determine economic replay safety.
Oracle observation expiry belongs to the oracle owner's task set, so dropping its
owner cancels the timer. Already-started blocking CPU work for bounded local deeper
quote search can finish naturally; cancellation does not preempt that computation.
It acquires no network observations, and generation/signing guards still fence results.

## Provider and admission recovery

Use shared client budgets and finite attempt, circuit, capacity and producer counts
without logging private endpoints/raw bodies. Background pacing waits before acquiring
a provider permit so dormant paced work cannot monopolize the in-flight limit.
Cancellation releases owned work. Independent reserve, book, configuration, actor and
reference producers own chain reads; retail requests never enqueue network observations
or accelerate cadence.

Scheduled configuration refresh retains a still-fresh validated pair observation with
its original expiry when a remote read fails transiently. Successful malformed/unsupported
schema or authoritative pause/blacklist/code/tax guard changes invalidate the affected
pair immediately. Healthy unrelated pairs and known actor blocks remain intact;
original expiry and compatible pool/hybrid generations are mandatory.
Shared-token discovery retains prior qualified state only for typed provider
availability/capacity; wrong-chain, malformed observations or changed code invalidate
all affected admissions before later remote awaits. Token code identity remains pinned.
An incomplete outer discovery deadline fails closed because cancellation can hide
an inner integrity result; only a completed typed outage may retain qualification.

Missing/stale required observations use the existing 503 recovery blocker; unsupported
route semantics use 503 `Local execution quote unavailable for this route`; a complete
fresh candidate that cannot satisfy execution/protection uses the existing 422
selected-protection blocker. Internal reasons are fixed low-cardinality values.
No smaller-size advice or relaxed taxes/hooks/floors substitutes for unavailable data.

Pair discovery propagates typed provider availability/integrity and database
persistence errors to pending-block classification rather than successfully skipping
the economic event. Ambiguous foreign/non-pair contract-local responses retain their
legacy policy. [PD1394-6/7](../../skills/AGENTS_PAIR_DISCOVERY_CACHE.md) distinguish
this boundary; no transient failure becomes negative provenance evidence.
Trader accounting writes use local SQL without an optional discount-registry LCD
await after a swap insert. Independently scheduled tier reconciliation fills the
conservative default tier later; a registry outage cannot omit position/accounting
updates or cause a fee-only replay. Permanent SQL failures still halt unsafe work.
Nonavailability parser/database/state failures can occur after a partial economic
write. Record and halt immediately instead of automatically replaying into an
existing-swap shortcut that could falsely complete the block. Keep the checkpoint
pending and API liveness separate from halted trading. Before a manual restart,
inspect the failed height and perform a separately reviewed, evidence-supported
accounting repair. The existing failed-block message carries the fixed
`unsafe_integrity_failure:` marker; marked pending heights refuse replay before
upstream work in both indexing entrypoints, including after process restart. Repair
the incomplete projections, then explicitly clear that marker before replay.
Removing the original fault or restarting alone does not establish completeness.
Legacy generic provider-failure rows remain recoverable. This policy is not an
atomic crash-durability guarantee for every parser write.

Historical checkpoint/block paths and `tx.height`-filtered transaction reads are
distinct from live-tip observations. Validate the requested block header height
and returned transaction heights before accepting historical bodies; a genuine
older checkpoint must not open live-provider circuits simply because its height
is more than the live lag allowance behind a separately observed tip. Substituted
historical bodies remain integrity failures and fail over only under bounded policy.
The deterministic regression warms tip 1000, accepts requested historical height 41
without a false circuit penalty, and rejects a substituted height 42. This is a
reproduced source defect; it does not independently prove the deployed restart root cause.

## Read-only deployed acceptance

1. Record the deployed commit from public health and inspect readiness separately.
   Use scoped openbao-tools `bao-exec` for authenticated read-only configuration/logs;
   retain only revision/state/fixed reason/count fields.
2. Run the [bounded socket corpus](../qa/issue-1464/README.md#bounded-public-useful-result-corpus)
   at ordinary/small sizes, new/reused connections and multiple freshness cycles.
   Keep error completion separate from positive executable latency.
3. Confirm all four directions' operations, integer input/output, original ages/generations,
   expiry and positive floors. Measure initialized hits/misses, desktop/phone browser,
   cold initialization and regional RTT separately. Every representative initialized
   useful sample must be strictly <250 ms.
4. Review bounded outage/recovery evidence with checkpoint/hash/idempotency and coverage
   before/after. Source tests or natural outages do not prove a controlled deployed trial.
   Production writes, disruption, migration and rollout are separately authorized actions.
5. Record executed failure as FAIL, unavailable evidence as explicit SKIP, and missing
   deployment/history acceptance separately in [#1456](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1456).
   PASS WITH SKIPS permits source review while live criteria remain unchecked.

Runtime stays least privilege and never migrates. QA uses per-process disposable DBs.
Never reset shared volumes, seed production, repair history from current reserves or
relax provider attestation.

The [LocalTerra guard receipt](../qa/issue-1464/localterra-settlement-guards.json)
records funded isolated tax/floor tests against existing checksum-bound artifacts.
Its explicit provenance limitations prevent treating those passes as parity with
the current local execution engine or as mainnet deployment acceptance.
