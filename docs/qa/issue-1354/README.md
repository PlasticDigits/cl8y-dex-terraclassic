# DeFiLlama reconciliation: agent checklist (#1354)

Merged source closeout: PRs [#1447](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1447) and [#1448](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1448) are on main. Remaining production, paid-history, upstream and unavailable network work is tracked by [#1449](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1449). Source issue closure does not establish deployment; see the [merge receipt](../merge-2026-10-08/README.md).

Issue: [#1354](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1354).
Contract: [DL1354-1–DL1354-10](../../../skills/AGENTS_DEFILLAMA.md#live-reconciliation-1354).
Results: [implementation verification](verification.md), [earlier investigation](investigation-verification.md),
[public receipt](public-reconciliation.json), [upstream copy receipt](upstream-volume.json).
Keep completed #631/#687 closed. Reopened #1209/#1210 track separate paid-source rollout acceptance.

## Agent procedure and skip policy

1. Read issue/comments using `bao-exec --only FORGEJO_PLASTICDIGTS_PAT -- fj issue view 1354` / `... view 1354 comments`. Record UTC time, checkout revision and serving `/health` revision separately.
2. Provision DB1396 disposable QA roles and scoped `indexer/.env.qa`; each test process owns a fresh child. Run `make verify-issue-1354` and retest. It runs local accounting/history/parser/ledger/API/catalog/UI/adapter regressions and public diagnostics. Live drift is a real diagnostic finding.
3. Run `make verify-issue-631`, `make verify-issue-687`, `make verify-issue-1396`, and creation/invoice regression targets `make verify-issue-1209` / `make verify-issue-1210`. Inspect the actual failing assertion before changing a fixture; never weaken oracle/DB prerequisites or restore unconditional nullable uniqueness.
4. Read the [bounded SQL report](../../../scripts/qa/reconcile-defillama-fees-1354.sql). Compare ledger and both stored daily rollups by day/source/pair in one read-only repeatable snapshot; distinguish actual exclusions, legacy unresolved scope, unpriced values and stale materializations.
5. After actual ledger corrections and source rollout, run `cl8y-dex-indexer backfill-defillama --start 2026-08-17 --end YYYY-MM-DD` under a least-privilege runtime credential against a prepared schema. It accepts 1–400 completed days, rebuilds original stamps atomically per day and can be retried after contention/interruption. It never migrates, reprices or reloads Llama. Recheck readbacks after the 60s GET cache expiry, then obtain an upstream reload receipt for corrected dates.
6. Compare factory `Pairs`/`Pool` raw reserves against valued/unvalued token keys at the same pinned block/time. Obtain verified CW20 prices; never hardcode UST1=$1, USTR multipliers, or substitute indexer USD for reserve TVL.
7. Install read-only monitoring with `bash scripts/defillama/install-drift-monitor.sh`. Verify timer activation, JSON findings/correction history and a journal failure for missing/drifting dates. The rolling 90-day monitor does not replace explicit older-history audits. Keep its checkout alive; stop with `systemctl --user disable --now cl8y-defillama-drift.timer` before removal. External alert delivery requires its own receipt.

Finish available work first. An agent may mark an unavailable check **SKIP** with reason, missing evidence and bounded followup while marking overall **PASS WITH SKIPS / PASS (available checks; skips recorded)**. Corresponding live criteria remain unchecked. An executed assertion failure remains FAIL until fixed and retested; a skipped rollout does not erase known live diagnostic findings.

Verifier options: `VERIFY_ISSUE_1354_SKIP_RUST_REASON`, `_SKIP_DB_REASON`, `_SKIP_LIVE_REASON`; invoice harness `VERIFY_ISSUE_1210_SKIP_DB_REASON` / `_SKIP_UI_REASON`. Reasons must be explicit. A source-only review may set `_SKIP_LIVE_REASON` to undeployed PR/access/upstream evidence and run public diagnostics separately, retaining their actual FAIL receipt. Never call that a deployed accounting PASS.

## Acceptance

- [x] Positive priced volume lower bound with exact priced/unpriced counts and completeness; idle zero/all-unpriced null; no invented marks/double count (source + fixtures).
- [x] Upstream #9907 recovery: all ten formerly missing original-window dates present.
- [x] Actual pair/event-scoped gem fee exclusion; unique legacy fallback and explicit unresolved count preserve ordinary fees (source + fixtures).
- [x] Explicit bounded historical reaggregation, original-stamp preservation and retry/idempotence (source + fixtures).
- [x] Missing creation/SKU/settings source implementation, exact reserved emitters/payment provenance, schema/ledger/API/series/DeFiLlama/UI/replay; #1209/#1210 tracking restored.
- [x] Captured real paid LocalTerra creation/SKU receipts replayed through block ingestion into disposable DBs twice; exact raw amounts, outage retry and dedupe.
- [ ] Production day/source/pair reconciliation, complete historical ledger replay/repair and corrected stored Llama fee chart.
- [ ] Verified CW20 prices and same-block raw factory reserve valuation explain the TVL basket; time-appropriate CL8Y/ALPHA mark remains unverified.
- [x] Repeatable drift checks, bounded timer activation and local journal/JSON findings; correction readbacks recorded.
- [ ] External alert delivery and deployment/reload acceptance receipts.

## Read-only SQL

With an existing authorized read-only service and private password file:

```sh
PGSERVICE=dex_indexer_readonly psql -X -v ON_ERROR_STOP=1 \
  -v start=2026-08-17 -v end=2026-09-24 \
  -f scripts/qa/reconcile-defillama-fees-1354.sql > /private/path/defillama-fees.txt
```

The service/path is an example, not provisioned access. This uses a read-only snapshot, 30s statement limit and 500ms lock limit; no repair/repricing. The report shows the legacy transaction heuristic separately from the new event-scoped rule. Repeated contextual day/source/protocol totals must not be summed across pair rows. Keep real reports/credentials private; publish reviewed aggregates only.
