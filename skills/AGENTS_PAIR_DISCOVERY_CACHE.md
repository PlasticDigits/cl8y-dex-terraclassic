# Pair discovery cache and admission — #1394

Apply this playbook when changing indexer pair discovery, LCD rejection handling,
cache ownership, or discovery admission. Issue:
[#1394](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1394).
Review and reproducible verification:
[checklist](../docs/qa/issue-1394/README.md).

## Invariants PD1394-1–PD1394-10

1. **PD1394-1 — ownership.** Each newly constructed `LcdClient` owns isolated
   discovery state; clones share that state. Different client instances cannot
   suppress one another's factory queries, even for the same pair address.
   Construct a fresh client when changing the configured endpoint set or chain.
   This is a local client namespace, not remote chain attestation. Failover among
   a client's configured endpoints assumes those endpoints serve the same chain.
2. **PD1394-2 — factory provenance.** Rejection state and admission are keyed by
   factory inside the client namespace. Accept only a pair whose queried asset
   tuple resolves to its exact contract address in that factory. An empty or
   invalid factory fails closed; it never bypasses provenance.
3. **PD1394-3 — validated inputs.** Before discovery LCD work, validate pair and
   factory contract addresses as canonical lowercase Terra Bech32 (`terra`,
   original Bech32 checksum, 20 or 32 decoded bytes). Reject mixed/upper case,
   invalid checksum/HRP/payload and oversized strings. Synthetic test fixtures
   must use valid generated addresses, not `terra1attackerpair` placeholders.
4. **PD1394-4 — bounded retention.** Retain at most 16 factory namespaces per
   client and 512 negative entries per factory (8,192 total). Entries expire
   after one hour. Lookup checks only its key; FIFO capacity eviction and expiry
   handling must not scan the full rejection map on every access. Keep all
   queues bounded as well as the maps. Factory slots are never evicted or reused
   to reset admission budgets; a seventeenth factory receives a typed capacity
   deferral. Expired entries refresh in their original FIFO slot; duplicate live
   evidence must not extend the fixed TTL.
5. **PD1394-5 — rejected means deterministic.** Cache only proven unlisted pairs
   or contracts whose pair query deterministically rejects its schema. Match
   rejection text only on `LcdError::ContractQueryRejected`, never on aggregate
   `AllEndpointsFailed` diagnostics. Preserve `Unlisted` versus `NotAPair` as
   the stored reason and observable cached result.
6. **PD1394-6 — transient remains retryable.** Timeout, endpoint/failover errors,
   malformed responses, oversized responses, invalid input and admission
   deferral do not create negative entries. Never make a test pass by hiding a
   discovery failure or accepting an unlisted pair. Direct discovery preserves
   typed errors and fixed transient counters. The production getter preserves
   its legacy transient soft-skip policy: warn with the error and fixed outcome,
   return `None`, and allow a later event to retry without poisoned evidence.
   This existing policy can omit the failed event; this fix does not change the
   ingestion policy or promise replay. Invalid factory configuration and permanent
   admission deferrals remain distinct errors rather than transient soft skips.
7. **PD1394-7 — bounded work.** Each factory independently has a token bucket
   with a burst of 32 attempts and refill of eight attempts per second, with at
   most four concurrent discoveries. Coalesce simultaneous same-pair work.
   Typed admission deferrals stay separate from provenance rejections. Release
   permits on completion, error and cancellation. A flood in one retained
   factory cannot consume another retained factory's admission budget. The
   production getter waits 125 ms between retryable admission attempts, checking
   the DB again before every attempt; it performs no LCD call until admitted.
   Shared provider rate/concurrency/half-open admission exhaustion is the typed
   `LcdError::AdmissionDeferred` and follows the same wait/DB-recheck policy
   ([#1453](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1453)). Factory
   budgets remain independent; provider HTTP capacity is intentionally shared.
   Production discovery retains completed identity/pair/provenance/metadata
   phases during capacity waits while holding its factory permit; restarting the
   whole discovery can livelock a small provider burst. Recheck concurrent DB
   persistence at every retry. Direct discovery returns capacity deferral promptly.
   Actual attempted provider failures retain the existing transient policy.
   Already-persisted legitimate pairs bypass discovery admission. Waiting futures
   belong to callers; do not introduce an unbounded internal event queue. A flood
   may slow block progress, but admission pressure must not create an additional
   successful skip of a legitimate event.
8. **PD1394-8 — bounded response and telemetry.** Pair/factory discovery smart
   query bodies are limited to 64 KiB. Use only fixed outcome/reason labels for
   metrics; never introduce per-pair or per-factory metric labels, retained log
   maps, or another unbounded address sink. Expose client-scoped counter snapshots
   for initial outcomes, cache hits and admission deferrals; the production poller
   logs a fixed-field aggregate every 100 processed blocks. Logs are evidence,
   not a cache; do not warn separately for every admission wait.
9. **PD1394-9 — independent tests.** Inject isolated state and a controllable
   monotonic clock where needed. Parallel tests must not clear process-global
   caches or depend on serial library execution. Assert LCD query counts,
   capacity, TTL/eviction, namespace isolation and admission fairness, including
   concurrent access and transient/failover paths.
10. **PD1394-10 — truthful receipts.** Run and retest feasible checks with exact
    revision, UTC time and commands. An executed failure remains FAIL. Agents
    may mark overall **PASS with explicit skips** for unavailable checks only
    after completing runnable checks and recording each reason/evidence; leave
    skipped acceptance criteria unchecked. Source tests, synthetic benchmarks
    and Postgres tests do not establish production deployment or flood safety.

## Agent workflow

Read [pair discovery](../indexer/src/indexer/pair_discovery.rs),
[LCD client](../indexer/src/lcd/mod.rs) and
[discovery state](../indexer/src/lcd/pair_discovery_state.rs) before editing.
Review [parser event handling](../indexer/src/indexer/parser.rs) and
[poller checkpoint/retry handling](../indexer/src/indexer/poller.rs) whenever
changing getter outcomes. Retryable admission waits must not become successfully
skipped events; preserve observability and uncached retryability of legacy
transient soft skips.
Use an isolated git worktree. Run `make verify-issue-1394`, then follow the
[acceptance checklist](../docs/qa/issue-1394/README.md) to review invariants that
test output alone cannot establish. Provision a dedicated Postgres test database
using the [Postgres playbook](AGENTS_LOCAL_POSTGRES_DEV.md); do not reset shared
DB/chain volumes. Keep all published test identities synthetic and omit secrets.

Crosslinks: [testing](../docs/testing.md),
[indexer invariants](../docs/indexer-invariants.md),
[repository agent guide](../AGENTS.md).
