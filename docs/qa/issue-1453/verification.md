# Issue #1453 implementation verification

Issue: [#1453](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1453).
Implementation: [PR #1454](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1454),
commit `b9809df5c31ad4cee875b27d5f609e83e3701adc`.
Agent procedure and skip policy: [README](README.md).
Invariants: [LF1453](../../../skills/AGENTS_LIQUIDITY_FEED.md).

The implementation separates pool observations from compatible hybrid generations,
pins reserve/fee/book reads, and bounds provider admission, failover and recovery.
Capacity pressure preserves discovery progress and scheduled snapshot phases rather
than silently dropping them. No production signing, configuration change, migration,
service restart or historical repair was performed.

## Scope and acceptance checklist

| Result | Criterion | Evidence and remaining scope |
|---|---|---|
| [x] PASS | AC1 — pool survives fee/book failures | Fresh independently committed reserves survive fee, bid/ask and timeout failures; failed hybrid generations keep their original time. Cold unknown fees remain ineligible for quotes. |
| [ ] PARTIAL | AC2 — unchanged coverage and failover | Fixed 6/18-decimal valuation, secondary/tertiary recovery and constrained provider saturation are mocked. Active deployed coverage/latency is unverified. |
| [x] PASS | AC3 — circuits and rate limits | 403/429/408/5xx/timeouts, capped Retry-After, one half-open probe, cancellation and typed capacity deferrals are exercised. Production defaults were not widened for tests. |
| [ ] PARTIAL | AC4 — live providers | Three configured providers were audited read-only; two passed exact-height pool probes, one lacked its response height header. The deployed revision was pre-fix; operational failover/recovery remains unverified. |
| [ ] PARTIAL | AC5 — local cached consumers | Pool/hybrid readers select their proper committed observations. No refresh is added to requests; complete removal of existing quote-path LCD calls remains [#1452](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1452). |
| [ ] PARTIAL | AC6 — stock totals and latency | Fixed complete totals, price-only movement, reserve decreases/zero, source/grain stock semantics and retained display pass locally. Deployment latency and funded LP controls remain unverified. |
| [x] PASS | AC7 — monotonic and atomic generations | Older/equal-time/lower/unknown-height writes cannot displace known newer data. Compatible hybrid/book transactions and atomic headline/pair/history rollback/recovery pass. |
| [ ] PARTIAL | AC8 — cited historical contribution | Original 03/04/05 UTC amounts/quality/identity are retained. A bounded log tail did not cover 04 UTC. Exact missing pool, reserves and original prices remain unknown; no repair was justified. |
| [x] PASS | AC9 — tests, documentation and retail | Focused verifier, numbered invariants, configuration examples, privacy/budget probe and agent instructions are crosslinked. Existing plain desktop/phone presentation is verified; only browser fixture path handling changed. |

Do not close production/history acceptance based on these local checks. Agents may
record unavailable checks as SKIP and overall PASS WITH SKIPS; executed failures
must be fixed and retested, never retrospectively converted to skips.

## Verification commands

Run in the isolated worktree with private `indexer/.env.qa` credentials. Each Rust
integration process creates and strictly migrates its own disposable child DB.

```bash
make verify-issue-1453
make verify-issue-1380
make verify-issue-1394
make verify-issue-1396
make verify-issue-1398
make verify-issue-1399
make verify-issue-1437
```

The production HTTP audit uses the [read-only probe](../../../scripts/qa/probe-issue-1453.py)
through the `openbao-tools/workstation/bao-exec` wrapper; see README for exact flags.
Credentials and URL values were never copied into public evidence.

## Completed checks and corrections

Initial executed failures were retained and corrected: provider configuration
fixtures, cancellation/cooldown assumptions, legacy grid rate exhaustion, discovery
admission starvation, a missing hub oracle anchor in the new valuation fixture,
and a Vite proxy prefix missing from the chart layout fixture. The legacy grid
fixture reserves a larger bounded test budget; production budgets stay unchanged.

The full-book materialization fixture has explicit scheduling margin on the shared
build host. Separate failure/recovery fixtures enforce production/scaled deadlines;
that functional margin is not a production latency receipt. The leader-loss fixture
terminates only a backend selected from its own database and login, in one SQL
statement; `pg_locks` must never be treated as database-local.

The disposable QA cluster initially lacked the separate runtime LOGIN required by
DB1396 permission tests. It was provisioned in that owned cluster and authenticated
permission tests retested. The published-loopback Docker bridge prerequisite timed
out; internal bridge and host IPv4 probes passed. Host IPv6 was disabled by the
kernel. No shared application DB or migration history was changed.

Final counts, UTC interval and source fingerprint are recorded in
[verification-results.json](verification-results.json). Library ignored benchmarks
are separate from skipped acceptance; the discovery benchmark was run by #1394.

## Read-only evidence

- [Original public overview and UTC series](public-before.json)
- [Configured provider/identity/header audit](operational-receipt.json)
- [Repeated provider audit](operational-retest.json)
- [Pinned pool query availability](operational-smart-query.json)
- [Bounded log availability](log-availability.json)
- [Historical reconciliation and no-repair decision](historical-reconciliation.json)

The log tail examined 1,000 rows from 07:24–07:47 UTC, with 75 snapshot failure
lines. It did not contain the cited 04 UTC window. USD differences between hours
are not a reconstructed missing contribution or proof of an LP withdrawal.
