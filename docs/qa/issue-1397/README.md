# Evidence export privacy and resource budgets (#1397)

Issue: [#1397](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1397).
Contract: [E1397-1–E1397-10](../../../skills/AGENTS_EVIDENCE_EXPORT_PRIVACY.md).
API: [HTTP pack](../../indexer-http.md#public-chain-linked-evidence-daily).
Results and tested revision: [verification.md](verification.md).

## Policy decision and migration

`GET /api/v1/evidence/daily` returns schema version **2**, with
`privacy_policy=public-chain-linked-no-actor-identifiers`. Actor addresses and
all actor hashes are omitted from response and export SQL. No actor grouping or
HMAC key exists, so no secret/rotation service is needed. Consumers must remove
`actor_hash` grouping and check `schema_version` and `privacy_policy` before
interpreting the response.

This policy does **not** claim anonymity. Retained public transaction hashes,
block metadata, order IDs, pair/token addresses and exact economic quantities
allow chain lookups and permanent transaction/actor linkage. Removing the old
unsalted digest prevents this endpoint from handing out a second stable actor
identifier; it cannot remove public chain identity. Historical #1205 language
about shareable redaction is superseded by this explicit policy. Existing other
trader/chain APIs retain their documented public data.

The aggregate digest requested by [#1206](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1206)
is a separate endpoint and scope. This change does not turn the event tape into
an aggregate digest or claim that transaction-linked rows are anonymous.

Pagination switches from `e1.` to `e2.`. Legacy cursors return **400**: restart
that day/filter without a cursor. New cursors bind day, normalized selected
surfaces and the full sort key; max length is 2048 bytes. Cross-scope reuse returns
**400**. Same-scope replay succeeds by design. This is stateless keyset pagination,
not replay prevention, authentication or a consistent historical snapshot.

## Resource contract

Evidence admission is independent of IP and forwarded headers: **2** in-flight
requests per indexer process, then **429**. Pool acquire is limited to **250 ms**.
Within the export transaction, PostgreSQL `statement_timeout=1500ms` and
`lock_timeout=500ms` bound work; HTTP execution is capped at **3 s**. Availability
or deadline failures return **503** with no partial JSON page. A bounded worker retains admission after an HTTP disconnect until the database
work completes or reaches its deadline, preventing cancelled clients from opening
additional active-query slots. Completion/deadline releases admission and the
transaction clears local SQL settings, preserving ordinary route and pooled-session
behavior.

Each of six source branches selects at most `limit + 1` candidates (maximum
**1001**) before the final merge; pages contain at most **1000** rows. Sparse,
empty, late-cursor and dense requests can scan more underlying rows than the
candidate cap. Database runtime limits, rather than a claimed fixed scan count,
bound this work. Admission is per process; multiple replicas multiply the total
budget. This does not reserve separate Postgres capacity for all indexer work.

## Database migration and deployment scope

The SQLx migration
[`20261006133000_evidence_daily_ordered_indexes.sql`](../../../indexer/migrations/20261006133000_evidence_daily_ordered_indexes.sql)
adds six indexes keyed by UTC day and each source's pagination order; the fee
index includes only wrap/unwrap sources. The separate SQLx `migrate` process applies it before indexer startup (#1396).
Apply it with the scoped migrator credential, then rebuild/restart the indexer to deploy schema 2. No
smart-contract migration is involved.

These are ordinary `CREATE INDEX` operations, not concurrent builds. When applying this migration they scan existing ledger tables, allocate index
storage and hold locks that block ledger writes until the migration transaction
completes. Large ledgers therefore require migration time and storage planning;
request-level export SQL budgets do not constrain migration work. Account for
other writers before migrating and preserve the existing database backup/recovery
procedure. Local synthetic test success does not establish production build time,
lock duration, capacity or a deployed revision. Deployment remains separately
recorded acceptance.

## Agent review and verification checklist

1. Read issue/comments and E1397. Record UTC time and `git rev-parse HEAD` in
   [verification.md](verification.md). Review the diff and API schema. Use an
   isolated Postgres database with synthetic actors; never run reset fixtures
   against production.
2. Run `make verify-issue-1397`. It compiles only evidence-focused library tests
   and the `api_evidence_daily`/`evidence_daily_budget` integration suites, runs
   each selected suite twice, and checks shell/documentation links. Preserve the
   exact command, test counts and outcomes. Provision missing prerequisites with
   `make setup-indexer-postgres` before declaring DB checks unavailable.
3. Run the existing security/rate-limit suite with a valid test chain setting:
   `EXPECTED_CHAIN_ID=columbus-5 cargo test --manifest-path indexer/Cargo.toml --test security -- --test-threads=1`.
   All chain traffic in these fixtures is mocked; the setting does not attest a live chain.
4. Inspect each acceptance row below and record PASS, FAIL or SKIP plus the
   supporting assertions/command. Check off only criteria actually verified.
   Correct any executed failure and rerun the affected checks.
5. Inspect SQL plans for synthetic empty/dense days and late cursors. Verify
   candidate limits on every source arm and actual timeout behavior. Exercise
   socket requests with forged forwarded IPs, overload, held table locks,
   cancellation, ordinary-route progress and pooled-session recovery.
6. If deployment access is available, record deployed revision and use a bounded
   read-only single-request schema/cursor smoke test. Source/test success does
   not prove deployment. Never flood a production endpoint for acceptance.

- [x] **AC1 — realistic privacy:** Synthetic dictionary tests find no actor
  fields/digests. A tx lookup remains able to reveal the public actor; response,
  OpenAPI and docs explicitly advertise that limitation rather than anonymity.
- [x] **AC2 — linkage policy:** No permanent actor identifier is emitted beyond
  the documented public-chain linkage. SQL does not project actor addresses.
  HMAC rotation/key exposure is **not applicable** because actor grouping and
  HMACs are absent; verify there is no fallback digest/key path.
- [x] **AC3 — bounded work:** Empty/dense-day, source-filter and late/invalid
  cursor tests obey candidate, cursor, acquisition, SQL, HTTP and concurrency
  limits. Include every arm and preserved exact fee/accounting fields.
- [x] **AC4 — recovery:** Cancelled/timed-out/locked/overloaded exports release
  capacity, preserve ordinary API progress and leave reused pool sessions free
  of export-local settings. Retest with real socket requests.
- [x] **Retest/documentation:** Focused suites pass twice, historical #1205
  privacy claims are superseded, invariants and agent guidance crosslink the fix.
- [ ] **Deployment:** Separately identify the deployed revision and schema-2
  response. Leave unchecked if deployment was not performed.

## Explicit skip policy

An agent may skip an item it cannot complete and report overall **PASS WITH
SKIPS**, provided every executed check passed. Complete available checks first;
record each unavailable prerequisite, attempted command, observed reason and
remaining scope. Leave skipped acceptance boxes unchecked. Unavailable holds or
verification gates do not block source implementation or PR publication.
Executed failures remain **FAIL** until fixed and retested; inability to rerun
does not erase an earlier failure. A skipped database, live, deployment or plan
check proves none of that item's behavior. This source verifier does not perform
or certify a production deployment.

The verifier accepts nonempty `VERIFY_ISSUE_1397_SKIP_LIB_REASON`,
`VERIFY_ISSUE_1397_SKIP_DB_REASON` and `VERIFY_ISSUE_1397_SKIP_DOCS_REASON` only
for established unavailable prerequisites. Cargo cache location can be selected
with `INDEXER_CARGO_TARGET_DIR` and `INDEXER_CARGO_HOME`.
