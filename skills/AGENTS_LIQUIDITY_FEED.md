# Cached liquidity feed reliability (#1453)

Apply when changing reserve/book ingestion, provider budgets, failover, TVL
publication or historical contribution reconciliation.
Issue: [#1453](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1453).
Agent actions, evidence and skip policy: [QA checklist](../docs/qa/issue-1453/README.md).
Invariants index: [LF1453](../docs/indexer-invariants.md#cached-liquidity-feed-1453).
Related contracts: [Q1380](AGENTS_PROTOCOL_LIQUIDITY_QUALITY.md),
[PD1394](AGENTS_PAIR_DISCOVERY_CACHE.md),
[NET1392](AGENTS_CHAIN_ATTESTATION.md), [DB1396](AGENTS_INDEXER_DATABASE_ISOLATION.md).

## Invariants LF1453-1–LF1453-10

Implementation: [PR #1454](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1454).
Implementation acceptance and scoped results: [verification](../docs/qa/issue-1453/verification.md).

1. **LF1453-1 — independent pool observation.** A validated successful pool read
   commits raw reserves, exact asset identity, original observation time and height
   independently of fee, bid or ask retrieval. Later fee/book failure must not
   discard that usable pool observation. The pool lane refreshes the matching fee
   after committing reserves; the book lane consumes the cached fee. Never renew an old observation's time on
   failure or substitute zero for missing input.
2. **LF1453-2 — compatible hybrid generation.** Hybrid consumers use separately
   committed reserves, fee and both order sides from a compatible successful
   snapshot. Pool queries pin the independently probed cycle height; fee/book
   queries pin that cached pool height and require exact upstream height attestation.
   Unknown height cannot form a new hybrid; fee-cache reuse requires the same height.
   Fresh pool-only reserves cannot make an old book fresh. Readers must
   explicitly choose pool-only versus hybrid data; do not silently join the two.
   Quote snapshots keep complete pool and hybrid views with independent original
   publication watermarks. New pool completion does not revoke an otherwise
   compatible original hybrid; hybrid expiry does not erase a still-fresh pool.
   Declared positive book budgets require the eligible original hybrid.
3. **LF1453-3 — bounded background ownership.** Reserve refresh belongs to the
   nonoverlapping background cadence. Bound jobs, pair/height deadlines, response
   bytes and orders, distribute admission across cycles, and coalesce competing
   refreshes. Background pool/fee/head/order phases wait 50 ms on typed provider
   capacity deferral within the existing four-second pair deadline; preserve
   completed phases and keep waiters caller-owned, with no internal queue or
   request-path retry. Each logical background query also has a four-second
   bound for direct callers lacking the enclosing job deadline. Actual
   upstream/contract failures do not retry here.
   Lifetime advisory leadership coalesces one writer per lane across
   replicas and releases on cancellation; heartbeat detection is bounded to 500 ms.
   A leadership failure can briefly overlap RPC work, so do not claim distributed
   exactly-once delivery. Monotonic writes reject delayed older results.
   No request/cache miss adds refresh, retry
   or validation work. The
   remaining quote-path removal in [#1452](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1452)
   is distinct acceptance: this feed change alone does not prove zero quote RPC.
4. **LF1453-4 — shared provider admission.** Clones share provider health and
   concurrency/rate admission. Retryable 403/429/408/timeouts/5xx open bounded
   backoff; cap Retry-After and jitter. Prefer healthy providers, permit one
   half-open recovery probe and release permits on cancellation. Bound attempts
   and total operation time; never add an unbounded work queue. Typed provider
   capacity deferrals make discovery retry with a persistence recheck; they must
   not become successful event skips. Actual provider failures remain distinct.
5. **LF1453-5 — failure scope.** Deterministic contract rejection and
   malformed/oversized contract bodies remain pair-local, retryable when their
   evidence is transient, and do not cool all healthy providers. Preserve typed
   PD1394 outcomes, factory/client isolation and negative-cache provenance. Do not
   log arbitrary upstream bodies, endpoint credentials or high-cardinality labels.
6. **LF1453-6 — provenance and monotonic observations.** Chain attest configured
   endpoints before use; preserve original height/time and reject delayed older
   writes. Chain ID alone is not cryptographic validation or proof of current
   height/historical support. Record lag/history evidence separately and leave
   unsupported checks unverified; never manufacture a trusted observation.
7. **LF1453-7 — atomic honest valuation.** Use eligible pool observations and
   prices under the existing Q1380 transaction/lock to publish headline, pair
   stamps, source slots and UTC grains together. Unsupported catalog tokens are
   outside supported completeness. Missing supported contributions stay partial
   or unavailable. Complete membership identity and genuine zero/decreases matter;
   do not increase TTLs, freeze TVL or use book escrow/foreign listing edges.
8. **LF1453-8 — stock history and scoped reconciliation.** Keep the terminal
   actual attempt in each source slot/hour/day/month. Never SUM/average TVL or
   choose an earlier healthy close. Preserve monthly history before pruning.
   Reconcile the 2026-10-09 04:00 UTC coverage loss from original observations;
   repair only evidence-supported, fixed, idempotent scopes with originals kept.
   Today's reserves/prices cannot backdate unknown historical contributions.
9. **LF1453-9 — simple retail and private evidence.** Retain existing plain USD
   headline and one normal chart series, original dates and ordinary tooltips.
   No added quality legends, badges, diagnostic controls, fake zero/flat bars or
   reporter/wallet/private transaction evidence. Run QA in disposable databases
   using a separate QA role; runtime never migrates or repairs production.
10. **LF1453-10 — truthful verification.** Run `make verify-issue-1453`, fix every
    executed failure and retest. Complete feasible checks before skipping unavailable
    work. Record exact command/revision/UTC and concrete reasons; overall
    **PASS WITH SKIPS** is permitted, leaving skipped criteria unchecked. Local
    source/mock/Postgres PASS proves neither live failover nor deployment/history
    repair. Unavailable holds/gates do not block implementation review.

## Agent workflow

Use an isolated git worktree. Read
[book snapshot](../indexer/src/indexer/book_snapshot.rs),
[reserve queries](../indexer/src/db/queries/pair_reserves.rs),
[LCD client](../indexer/src/lcd/mod.rs) and
[liquidity publication](../indexer/src/db/queries/liquidity_snapshots.rs).
Trace each reader's chosen reserve generation and timestamp before changing data
ownership. Apply schema changes only through the separate migrator; use the
[database runbook](../docs/runbooks/indexer-database-isolation.md) and disposable
integration fixture. Run failure/recovery tests twice, then follow the
[checklist](../docs/qa/issue-1453/README.md). Production audits are read-only;
record configured versus active endpoint count, chain/height evidence and deployed
revision without secrets. Never equate a configured URL list with healthy failover.

Provider lifecycle/fair scheduling and admission recovery also preserve
[PR1464](AGENTS_PROVIDER_RECOVERY.md); [agent checklist](../docs/qa/issue-1464/README.md).
Fresh pool retention does not certify executable configuration or live trading readiness.

## Quote-ready fee generation follow-up (#1464)

Use [provider recovery invariants](AGENTS_PROVIDER_RECOVERY.md) and
[the current receipts](../docs/qa/issue-1464/verification.md) for pool-head capacity,
monotonic head selection and pending fee publication. Raw valuation observations
and quote-ready complete tuples remain distinct; a pending generation cannot
renew or splice a prior tuple. Same-height cached/new fees share a guarded
transaction. Known negatives, changed fee economics, expiry and capacity failure
remain fail closed. Lane-labelled diagnostics and actual owned-worker tests
establish source behavior; deployed acceptance requires new exact-revision trials.

The [complete-view loader](../indexer/src/api/db_orderbook_sim.rs) and
[local engine](../indexer/src/api/local_quote.rs) keep original pool and hybrid
tuples independently. Zero-book budgets prefer the complete pool; automatic book
optimization uses only its independently eligible hybrid. No new reserve/fee may
inherit old book rows, including after hybrid expiry without a publication pass.
The [fee registry](../indexer/src/indexer/fee_phase.rs) charges both views against
the same hard mirror/order/byte caps; its shared authoritative negatives invalidate
both views. The four-second phase and twenty-second original TTL are unchanged.
Retest [state bounds](../indexer/src/indexer/fee_phase_tests.rs) and
[both scheduled publishers](../indexer/tests/scenarios/fee_generation_recovery_1464.rs)
under pending/changed fees, cancellation, known rejection and independent expiry.
