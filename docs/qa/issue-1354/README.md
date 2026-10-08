# DeFiLlama reconciliation: agent checklist (#1354)

Issue: [#1354](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1354).
Contract: [DL1354-1–DL1354-10](../../../skills/AGENTS_DEFILLAMA.md#live-reconciliation-1354).
Results: [verification](verification.md), [public receipt](public-reconciliation.json),
[upstream copy receipt](upstream-volume.json). Keep completed #631/#687 closed.

## Agent procedure and skip policy

1. Read issue/comments with openbao-tools `workstation/bao-exec --only
   FORGEJO_PLASTICDIGTS_PAT -- fj issue view 1354` / `... view 1354 comments`.
   Record UTC time, checkout revision and serving `/health` revision separately.
2. Run `make verify-issue-1354` (public GETs only). The JSON receipt defaults to
   `/tmp/cl8y-defillama-1354.json`. Exit 1 means findings; exit 2 means unavailable
   evidence. Legacy regression/copy PASS is not accounting acceptance.
3. Run `make verify-issue-631` and `make verify-issue-687` with isolated QA
   credentials. Follow [DB1396](../../../skills/AGENTS_INDEXER_DATABASE_ISOLATION.md)
   and `make verify-issue-1396`. Never migrate/clean production or the QA control DB.
   `bash scripts/qa/reproduce-defillama-1354.sh` adds two temporary acceptance
   probes to the same harness and removes that file afterward. Both currently
   fail: partial priced volume and mixed-transaction pair fee preservation.
4. Run the [read-only fee report](../../../scripts/qa/reconcile-defillama-fees-1354.sql)
   through an existing authorized production connection. Review per-source/pair
   exclusions, unresolved scope and stored rollup deltas. Credentials and full
   reports stay private; publish reviewed aggregate findings only.
5. Check factory `Pairs`/`Pool` reserves and valued/unvalued token keys at a pinned
   block/time. Current price requests and last Llama snapshots do not establish
   that acceptance. Fix source accounting before historical writes/reaggregation;
   production rollout, price submission and upstream reload need separate receipts.

Agents finish available work first. The user's explicit permission allows
unavailable custody/device/upstream checks to be **SKIP**, and closure with skips
when available acceptance criteria pass. Record the reason, missing evidence and
followup; leave corresponding boxes unchecked. Executed failures remain **FAIL**.
Local SQL fixtures and third-party merges never establish production accounting.
#1354 stays open because available checks fail.

## Acceptance

- [ ] Live headline preserves a positive priced volume lower bound with explicit
  completeness counts; idle zero, all-unpriced null, no invented marks or double count.
- [x] Upstream [#9907](https://github.com/DefiLlama/dimension-adapters/pull/9907)
  recovery verified; anchored 30-day chart includes all ten formerly missing dates.
- [ ] Production day/source/pair reconciliation and old-day repair establish fees
  after exclusions; stored Llama fees match corrected dates.
- [ ] Mixed gem/ordinary transactions retain the ordinary pair's fee by its actual
  pair/event; unresolved legacy pair scope is explicit.
- [ ] #1209/#1210 ingest lands with live keys and real paid-event/replay evidence.
- [ ] Verified CW20 prices and same-block reserve inventory explain the TVL basket.
- [x] Repeatable public drift script, adapter boundary tests and cross-links exist.
- [ ] Scheduled monitoring/alert delivery has an operational receipt.

## Ready commands

An agent or operator can run public diagnostics from this checkout:

```sh
make verify-issue-1354
```

With an already provisioned **read-only** PostgreSQL service and private password
file, the production report command is:

```sh
PGSERVICE=dex_indexer_readonly psql -X -v ON_ERROR_STOP=1 \
  -v start=2026-08-17 -v end=2026-09-24 \
  -f scripts/qa/reconcile-defillama-fees-1354.sql > /private/path/defillama-fees.txt
```

The service/path above is an example, not provisioned access. Run it as an agent
when access is restored. The report uses a read-only repeatable snapshot, a 30s
statement limit and 500ms lock limit; no backfill/reload. Repeated day/source and
protocol totals are contextual: do not sum them across per-pair rows. Unknown
pair scope is not silently assigned to an ordinary pair.

## Remaining work

1. Fix live volume completeness and pair-scoped fee exclusion; resolve the
   economic-fee and Swagger regression failures. #1354 / #1269 / #683 / #1396.
2. Reconcile/reaggregate old days, add explicit old-day repair, then reload
   upstream fees and the missing August 17 volume. #1354 / #1269. Production SQL
   is currently an access skip.
3. Restore accurate tracking and implement missing sources. #1209 / #1210 / #1213.
4. Obtain verified CW20 prices and a pinned reserve comparison; schedule the
   drift check with an alert destination. #1354 / #629 / #1385.

These are agent-owned engineering tasks. The ready read-only commands gather
evidence; running them alone does not implement the fixes or close the issue.
