# Exact event amounts (#1395)

Issue: [#1395](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1395).
Implementation: [PR #1406](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1406); results and tested revision are in
[verification.md](verification.md).
Invariants: [A1395](../../indexer-invariants.md#exact-event-amounts-1395).
Agent playbook: [AGENTS_INDEXER_EVENT_AMOUNTS.md](../../../skills/AGENTS_INDEXER_EVENT_AMOUNTS.md).

## Ingestion contract

Raw token quantities accept only 1–38 ASCII digits, including genuine zero.
Reject missing mandatory values, signs, whitespace, decimal notation, exponents,
Unicode digits and oversized strings before constructing `BigDecimal`. Prices
retain exact decimal notation within `NUMERIC(38,18)` (at most 20 integer and
18 fractional digits). Token decimals are metadata for humanization, not permission
to accept fractional raw amounts. Optional absent fields retain their documented
`None` semantics; malformed present optional amounts reject their complete action.
Zero treasury fees produce no row, as required by the fee ledger's positive check.

Swaps, liquidity, fills, placements, parked escrow and hook amount actions use the
same strict parser as pinned wrap/unwrap and UST1 mint/redeem fees. Lifecycle batch
columns preserve record indices so rejecting a row cannot attach its neighbor's
fee or price to another order. Missing column entries have no row index; an
ambiguous batch is rejected as a whole rather than guessing an association. Reserved `_contract_address` remains the provenance
boundary; ordinary `contract_address` never identifies the emitter. Existing
factory pair verification and exact wrap/window pins remain in force.

Derived prices and reconstructed reserves are checked against their storage
domains before ledger mutation; an overflow rejects the complete affected action.
Accepted raw fields remain exact and are never clipped to fit derived storage.
Rejected actions are discarded before ledger/position/volume mutation. They are
not persisted as fake zero and are not retried as infrastructure failures. Valid
siblings still ingest, and the block checkpoint advances normally. There is no
quarantine table containing arbitrary input or user data. Structured diagnostics
include only static event kind, field and typed `missing`/`invalid`/`overflow`
reason, limited to eight messages per minute across the process. Suppressed counts
appear on the first subsequent rejection after the minute resets; absence of that
subsequent event does not imply an exact persisted count. A process restart resets
the log budget. Rejection never uses tx hashes, addresses, tokens or raw payloads
in these diagnostics.

Replay keeps the existing unique keys. Swap ordinals include rejected swap emissions;
pinned fee ordinals include zero/rejected fee actions. Replaying unchanged input
cannot shift later valid identities or double-count positions/fees. LCD/DB failures
follow [PR1464](../../../skills/AGENTS_PROVIDER_RECOVERY.md): typed provider availability
retains the pending height; unsafe nonavailability processing/database/state failure
halts immediately with a durable marker, preventing automatic replay from hiding
partial projections. Required positive-fee asset lookup failures propagate; optional
USD valuation remains separate. This change does not introduce
atomic block transactions or retroactively certify historical coerced rows.
Existing liquidity and hook uniqueness keys still coalesce repeated valid actions
on the same pair/contract and action kind within one transaction. This PR fixes
numeric rejection and sibling preservation within those keys; adding occurrence
identities to those two ledgers is a separate schema change.

The migration widens parked `remaining_escrow` to `NUMERIC(56,18)` so all supported
38-digit raw amounts fit. It preserves the old scale and any historical fractional
evidence without rounding. New events only store integers. Under
[DB1396](../../../skills/AGENTS_INDEXER_DATABASE_ISOLATION.md), a separate one-shot
migrator applies schema changes; runtime never migrates. Raw trader fees widen to `NUMERIC(78,18)` and realized trader/position P&L
to `NUMERIC(114,18)`, covering raw products and cumulative accounting, so valid
large quantities cannot fail after source rows insert. Optional USD fits checks
include PostgreSQL rounding to scale before integer precision validation. Raw
volume aggregates and candles widen to `NUMERIC(58,0)` (38-digit amounts plus
bigint count headroom); raw 38-digit saturation caps are removed from ingestion
and refresh/healing queries. Existing USD aggregation policy remains separate.
No smart-contract migration is needed. Rebuild/restart the indexer to deploy.
For changed canonical input, the existing [reorg recovery runbook](../../runbooks/indexer-reorg-replay-dedup.md)
uses explicit derived cleanup, now including `protocol_fee_events`. Non-height-keyed
positions/rollups still require their existing rebuild or snapshot recovery.

## Agent review and verification checklist

1. Record UTC time and `git rev-parse HEAD`. Read the issue/comments, A1395 and the
   playbook. Use synthetic identities and an isolated Postgres database in
   `indexer/.env.qa` with per-process disposable child databases; never point tests at production. Follow
   [local Postgres setup](../../../skills/AGENTS_LOCAL_POSTGRES_DEV.md) if unavailable.
2. Run `make verify-issue-1395`. The command runs the library suite, new parser and
   Postgres regressions twice, related block/fee/evidence/volume/position suites,
   shell syntax and documentation cross-links. Preserve exact commands and outcomes.
3. Inspect the assertions below against the diff, confirming complete action
   rejection, column alignment, bounded diagnostic content and original provenance.
   Execute and fix any reproducible failure before retesting.
4. Inspect the migration with maximum raw escrow and legacy fractional data. Review
   cursor-only replay versus actual canonical replacement and fee cleanup. Use the
   existing rebuild procedure for non-height-keyed derived state.
5. If deployment access is available, record the deployed revision and privately
   inspect malformed-action diagnostics and normal checkpoint progress. Do not
   inject malicious production events or publish user activity. Deployment is
   separate from local synthetic acceptance.

An agent may mark **overall PASS with explicit skips** when an item is unavailable
and every executed check passed. First attempt available prerequisites. Record the
item, attempted command/prerequisite and reason as **SKIP**, leave its acceptance
box unchecked, and state the scope of the overall PASS. Unavailable holds/gates do
not block implementation or PR publication. An executed failure remains **FAIL**
until fixed and retested; skip permission cannot relabel it. No skipped item proves
deployment or historical repair. The verifier supports unavailable library/database
scope via `VERIFY_ISSUE_1395_SKIP_LIB_REASON` and
`VERIFY_ISSUE_1395_SKIP_DB_REASON`; nonempty reasons are required.

- [ ] **AC1:** Malformed mandatory/present optional values never create zero ledger
  entries or position/volume changes. Real zero and missing optional fields retain
  schema policy. Cover each economic type and all malformed syntax classes.
- [ ] **AC2:** Typed rejection and bounded diagnostics omit values/identities;
  repeated poison input, valid siblings and partial retries preserve block progress.
- [ ] **AC3:** Maximum supported raw values, decimal metadata, lifecycle columns,
  multihop pair fee identities and replay remain exact and deduplicated.
- [ ] **Database/API:** Reconcile accepted ledger rows, position counts, volume and
  daily evidence; run block retry/hash guard and isolated recovery/replay tests.
- [ ] **Deployment:** Record deployed revision and actual indexer behavior.
- [ ] **Historical repair:** Independently investigate previously coerced rows;
  no historical zero can be classified as corruption from its stored value alone.
