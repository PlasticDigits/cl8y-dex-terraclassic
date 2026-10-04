# Implementation verification receipt

Issue: [#1380](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1380).
Implementation PR: pending publication.
Review/checklist: [README](README.md).
Invariants: [Q1380](../../indexer-invariants.md#protocol-liquidity-quality-1380).
Agent protocol: [playbook](../../../skills/AGENTS_PROTOCOL_LIQUIDITY_QUALITY.md).

The implementation uses an isolated worktree, host Cargo, Node 24, deterministic
mock LCDs and a dedicated ephemeral Postgres container/database/lock on port 55438.
No production DB or chain mutation was performed. The test database can be replaced
with a fresh isolated Postgres configured in `indexer/.env`; use a unique
`TEST_DB_LOCK_FILE` when several agents run integrations.

Final aggregate result: **PASS — 13 passed groups, 0 failed, 2 explicit skips**
(`make verify-issue-1380`, exit 0). All five requested regression targets passed. Earlier browser readiness/report-directory
failures were resolved with explicit stat readiness waits, isolated artifact paths
and reduced worker concurrency; assertions were retained. An outdated FIFO test
now follows the mock's existing on-chain linked-list order.

Completed independent checks:

- Full indexer library: **461 passed** on the final source, including all 11 LCD
  tests. Oversized 200/500 responses do not cool down healthy pair requests.
- Aggregate backend passes: **20 valuation units**, **5 reserve units**, and
  **43 Postgres/API/reserve integration tests** per pass, repeated twice. The latter
  includes all **29 liquidity tests** and failure/recovery, genuine-zero controls,
  comparable 24h/30d baselines, populated cold inventory, empty proof expiry,
  concurrent writers, replay rejection, rollback after every persistence stage,
  restart/cache age, all grains, and scoped repair replay after healthy recovery.
- Frontend: **88 targeted tests passed twice**, plus **200 existing consumer tests**;
  TypeScript, ESLint, source formatting and gitleaks passed.
- Browser: **3 new quality** and **13 existing Protocol tests** passed together on
  final product/test code, with desktop/mobile widths, keyboard/tap, unknown legacy
  positive gaps, partial lower bounds, cached expiry, recovery and true zero.
- Requested `make verify-issue-569`, `689`, `655`, `664`, `1263` results are collected
  by the aggregate verifier. Nested duplicates are omitted; the selected regression
  union and assertions still execute.

## Completed issue implementation criteria

These checkboxes credit actual implementation/assertion evidence, not deployment.

- [x] **AC1:** unchanged positive reserve fixtures → stale/missing-price/failed
  input → recovery do not fabricate fresh zero, -100% or a complete zero chart point.
- [x] **AC2:** material supported pool loss and missing rows disclose degradation;
  permanently unsupported pairs do not block complete supported-catalog statistics.
- [x] **AC3 implementation controls:** validated drained pools/confirmed empty
  inventory can show zero; genuine reserve/price changes are recomputed without
  smoothing. Invalid one-sided zero and positive storage underflow stay unknown.
- [x] **AC4:** coherent serialized generations across both writers, rollback,
  retries/replay rejection, restart/cold bootstrap and independent cached age.
- [ ] **AC5 production acceptance:** source/grains/baselines and scoped idempotent
  repair pass local fixtures, including preservation of later healthy recovery.
  Applying/reconciling the cited production hours remains unverified.
- [x] **AC6:** plain quality/as-of, keyboard/mobile access, safe legacy/null payloads,
  and Volume/Fees and TVL-consumer compatibility are covered by RTL/browser/regressions.

## Public and synthetic evidence

[Public pre-deployment API evidence](public-before.json) was collected at
2026-10-04T08:16:32 UTC: approximately $247,930, 11 priced and 11 unpriced pools,
with no quality/as-of fields. The separate [public health read](health-before.json)
at 08:41:56 UTC reported revision `3f9310652ac83818749f5d32c4e32057b08817bc`.
These are before observations, not proof of the new revision being deployed.

[Query plans](query-plans.txt) show a one-row primary-key overview read and bounded
indexed grain reads. No GET LCD/event/source-history scan was added. Synthetic
screenshots: [390px mobile](mobile-quality.png), [1280px desktop](desktop-quality.png).

LocalTerra provisioning probe succeeded at block 477728. [Read-only recorded pool
inputs](localterra-reserve-fixture.json) include raw amounts and a separately
observed height/time; the pool query was not pinned to that height. Those unchanged
amounts were replayed through timeout/recovery. This is not live LP withdrawal.

## Explicit skips and relevant follow-up

- **SKIP: production revision, history application/reconciliation and after evidence.**
  This task publishes a PR; the new indexer/frontend and SQLx migrations are not
  deployed. Deploy the nullable API/frontend together, preserve repair evidence,
  then run the read-only probe with `--expect-quality` and compare all grains.
- **SKIP: isolated funded LocalTerra LP withdrawal and induced live-chain outage.**
  The existing shared chain is running and was read publicly, but no isolated funded
  control fixture was provisioned/executed. Synthetic zero/outage controls and the
  recorded-input replay are passed evidence for their stated scope only.

An agent may report **overall PASS with explicit skips** if all completed checks
pass. Skips are never checked off as completed criteria, production proof or wallet
execution. An executed assertion failure remains FAIL. No verification action is
assigned to the human; subsequent agents follow the [checklist](README.md).
