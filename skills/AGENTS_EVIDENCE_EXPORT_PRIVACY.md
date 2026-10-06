# Agent playbook: public-chain-linked evidence export (#1397)

Use for evidence schema, query, pagination, middleware, privacy documentation or
external sharing changes. Issue: [#1397](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1397).
Verify: `make verify-issue-1397`. Review and skip policy:
[agent checklist](../docs/qa/issue-1397/README.md). Results:
[verification receipt](../docs/qa/issue-1397/verification.md).

## Invariants (E1397-1–E1397-10)

| ID | Rule |
|----|------|
| **E1397-1** | The public `GET /api/v1/evidence/daily` response is schema version **2**, with `privacy_policy=public-chain-linked-no-actor-identifiers`. It omits actors and actor digests entirely: no sender/maker/owner/provider or `actor_hash`. SQL must not select actor columns for this export. |
| **E1397-2** | This is **not anonymity**. Public transaction/block/order/pair identifiers and exact accounting remain; a chain lookup can reconstruct actors and link their activity across days. Never describe the export as anonymous, unlinkable, wallet-private or safe to publish based on actor omission alone. |
| **E1397-3** | There is no actor grouping, actor HMAC, key, rotation epoch or actor cache. Do not restore globally stable digests, introduce a secret fallback, or derive identity from retained public identifiers. HMAC-rotation acceptance is inapplicable to this deliberately identifier-free actor policy. |
| **E1397-4** | One UTC calendar day, surfaces `swap`/`wrap`/`limit`/`lp`, default page 500 and cap 1000. Exact raw quantities, fee sources and L10 survive. Wrap rows are treasury wrap/unwrap fees, not principal; limit fills are not swaps. Gems remain included, so this is not a listing-safe export. |
| **E1397-5** | `e2.` cursors bind the UTC day and normalized selected surfaces to the complete six-part sort key `(block_height, tx_hash, surface, kind, ordinal, row_id)`. Maximum encoded cursor length is 2048 bytes. Reject malformed, oversized, cross-day, cross-surface and legacy `e1.` cursors with **400**; restart legacy pagination without a cursor. Same-scope replay is intentionally stateless and allowed. Cursors are pagination data, not authentication or snapshot certificates. |
| **E1397-6** | Evidence has a dedicated admission budget of **2** in-flight requests per process, independent of caller IP or forwarded headers. Saturation returns **429**. Do not rely on per-IP governor limits to bound database concurrency, and do not put this DB-only route on `lcd_heavy_router`. |
| **E1397-7** | Pool acquisition is capped at **250 ms**, PostgreSQL transaction-local `statement_timeout` at **1500 ms** and `lock_timeout` at **500 ms**. HTTP execution is capped at **3 s** and availability/time-budget failures return **503** with no partial page. SQL limits each of six source branches to at most `limit + 1` (maximum **1001**) candidates before the final merge. These are candidate/result and runtime bounds, not a claim that PostgreSQL scans at most 1001 heap/index rows. |
| **E1397-8** | A disconnected caller does not immediately free admission while its PostgreSQL query is still active: a bounded worker retains the permit until completion/deadline, then releases it and completes or rolls back its transaction. Local SQL limits must not leak onto reused pool sessions. Evidence load must leave ordinary routes usable. Test socket-backed requests, cancellation and pool recovery, not only in-process responses. |
| **E1397-9** | Cursor values, day bounds and limits remain parameterized SQL; enum/table arms are internal allowlists. Do not add a CSV/unredacted twin route, change trader tapes or alter DeFiLlama aggregates as part of this policy. |
| **E1397-10** | Update OpenAPI, [HTTP docs](../docs/indexer-http.md), [invariants](../docs/indexer-invariants.md), this playbook and the [QA receipt](../docs/qa/issue-1397/verification.md) together. Prior #1205 actor-hash/privacy descriptions are superseded. Executed failures stay FAIL; unavailable checks may be explicit SKIP with overall PASS WITH SKIPS. Local verification never establishes production deployment. |

## Verification guidance for other agents

Read [the checklist](../docs/qa/issue-1397/README.md), record the tested revision,
and run `make verify-issue-1397` against an isolated synthetic Postgres fixture.
Use [local Postgres setup](AGENTS_LOCAL_POSTGRES_DEV.md) if needed. Exercise empty
and dense days, every source/filter, cursor boundaries, admission overload,
blocked SQL, cancellation and recovered ordinary requests. Keep user addresses
and credentials out of receipts. Dictionary/linkage tests must verify the stated
policy: actor fields/digests are absent while public tx linkage remains possible.
Do not assert unlinkability from an absent digest.

[The original #1205 playbook](AGENTS_INDEXER_EVIDENCE_DAILY.md) preserves the
accounting history; its actor-hash and global-limiter-only guidance is obsolete.

## Migration behavior

The [ordered-index migration](../indexer/migrations/20261006133000_evidence_daily_ordered_indexes.sql)
is applied by SQLx at startup. Its six ordinary index builds scan existing ledger
tables, consume storage and block ledger writes until the migration transaction
finishes. Export request timeouts do not apply to migration work. Document
startup/lock/storage behavior and record deployed-version acceptance separately;
see [the QA deployment scope](../docs/qa/issue-1397/README.md#database-migration-and-deployment-scope).
