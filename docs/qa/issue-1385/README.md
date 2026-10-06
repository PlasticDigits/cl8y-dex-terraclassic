# Liquidity rollout operations (#1385)

The actual funded LocalTerra withdrawal and LCD outage/recovery control passed on
2026-10-06. Production SQL migration and scoped history repair subsequently passed.
Public API/frontend serving acceptance then passed at integrated revision
`fab88dbbccb514657d786e3034e12c99423026eb`, with actual desktop/mobile browser checks.

The dedicated owned chain `cl8y-dex-ops1385-liquidity` used the repository-pinned
SDK53 image and an independently funded synthetic wallet/factory/pool. No shared
chain volumes were reset. The two synthetic CW20 contracts were explicitly pinned
as this process's cLUNC/cUSTC identities. USD quotes were fixed synthetic test
inputs ($0.0001 and $0.01), rather than claimed real market observations.

[Chain receipts](localterra-chain.json) retain optimized code checksums, successful
funding/provisioning transactions and the actual LP withdrawal. The production
snapshot writer read the real node through a local HTTP bridge. During an induced
503 outage it retained both old reserve amounts and their original timestamp.
After 21 seconds of actual expiry, the API returned stale/null, then recovered
the unchanged reserves. A real withdrawal subsequently reduced both reserves
from 1,000,000,000 to 100,000,900.

| Stage | API quality | TVL USD | Generation |
| --- | --- | --- | --- |
| Funded pool | fresh | 10.1000 | 1 |
| LCD outage after expiry | stale | null | 2 |
| LCD recovery, unchanged reserves | fresh | 10.1000 | 3 |
| Actual 90% LP withdrawal | fresh | 1.01000909 | 4 |

The [indexer receipt](localterra-indexer.json) also records the unchanged coverage
identity, degraded reason and suppressed percentage baselines. The source slot
retained the current withdrawal value rather than a previous good sample. The
opt-in Rust test passed (1 passed, 0 failed; 51.70 seconds).

Reproduce with a separately provisioned owned LocalTerra container, optimized
artifacts and a private QA env containing `TEST_DATABASE_ADMIN_URL` for the
dedicated marked QA control database:

```bash
python3 scripts/qa/localterra-liquidity-1385.py \
  --container YOUR_OWNED_LOCALTERRA_CONTAINER \
  --qa-env /private/path/qa.env \
  --evidence-dir /tmp/issue1385-control
```

The harness creates fresh owned assets/factory/pool and invokes
`cargo test --test ops_liquidity_1385 -- --ignored --nocapture`. Its integration
helper creates a new disposable QA database. The actual chain transactions use
only synthetic test keys; production terrad keys are unnecessary.

The [public before probe](public-before.json) still observed the legacy API
without quality metadata. The [extended hourly probe](history-public-before.json)
confirmed $218.5713203111132400/2 priced pools at 2026-10-02T05 and $0/0 priced pools
at 2026-10-04T02. Use the allowlisted `grain=hourly&limit=168` to include both
affected hours at this observation time; the ordinary 72-hour probe omits October
2. Repair must preserve the original rows, tag unsupported history unknown, and
never substitute current TVL.

The separate maintenance script was rehearsed against a fresh dedicated local
Postgres instance restored from the disposable test fixture: unique database and
public schema identification, immutable custom-format backup with checksum/TOC,
64 scoped table/sequence ownership transfers, standalone `migrate`, authenticated
runtime DML, and four actual SQLSTATE 42501 permission denials (migration-ledger
read, schema-marker write, table creation and role creation). Original migration
history remained unchanged. [Verification](maintenance-local-verification.json)
records the resulting schema marker. This is local PostgreSQL 16 evidence, not
production PostgreSQL 18 acceptance.

The maintenance inspector also matched every existing SQLx checksum to the exact
61 release artifacts. Adding an unrelated public table made inspection fail;
removing that disposable sentinel restored success. This check preserves the
scope of the application and does not rewrite stored migration checksums.

The combined [apply rehearsal](maintenance-local-apply.json) passed ownership,
migration and ACL checks on another fresh copied disposable fixture. Its missing
production history was explicitly unavailable. A separate synthetic two-window
[history rehearsal](maintenance-local-history.json) retained original evidence,
produced unknown/null source/hour/day values, and preserved a later valid monthly
close. The maintenance `apply` action runs transition, migration, verification and
history in one job, retaining stage progress and the pretransition rollback dump.
Caller-supplied hexadecimal `OPS_RUN_ID` values distinguish new executions from
older Coolify log entries. Rejection counts require bounded runtime diagnostics;
absence of a persistent rejection table is not a zero-count observation.

The actual [production backup](production-backup.json) retained 52,422,246 bytes,
SHA-256 `9a0ad0127ad847807d0879bb5828d0432d1642610f1b47307240b398ae8b6150`, with
validated custom-format TOC in the protected dedicated maintenance volume.
The [production maintenance receipt](production-maintenance.json) records 61
scoped ownership transfers, successful standalone migration, preserved existing
migration history, schema marker `20261006133000` and four actual runtime
permission denials. Only the new restricted runtime credential belongs in the
serving process; administrator and migrator credentials remain separate.

Production retained two original source samples and two original hourly rows.
Both documented hourly anomalies are now null/unknown. Later October 2/4 daily
closes and the current October monthly close kept their original numeric values
with legacy unknown quality; no current TVL was substituted. Both percentage
baselines were invalidated. The protected original backup and audit evidence
remain available for rollback/reconciliation.

The [public after probe](public-after.json) passed overview plus all three grains.
The observed headline/latest Hourly/Daily/Monthly rows shared generation 17,
coverage identity and as-of timestamp. Its partial $206,687.62174849048166731700
was explicitly qualified with a stale reserve, and both deltas remained null.
The [extended hourly after probe](history-public-after.json) confirmed both
documented anomalies as null/unknown on the serving API and recorded the actual
health revision. This is the production result, separate from the local fixture.

The [live browser receipt](browser-live.json) passed at 1280px and 390px with no
mocked responses or wallet connection. It exercised actual quality disclosure,
historical dashed gaps, keyboard tooltip/escape and viewport overflow. The desktop
view observed stale/unknown current liquidity; the mobile view observed a fresh
$250K recovery at generation 30. Neither stale data nor unknown history appeared
as a known $0/-100% collapse. Initial harness attempts were corrected to use the
existing first-visit risk acknowledgement interaction and the implemented
"Coverage unknown" tooltip label, then the complete check passed on retest.

The broad `make verify-issue-1380` attempt passed 20 protocol TVL and 5 bounded
snapshot unit tests, then was intentionally stopped before its integration group
to prioritize production maintenance. It has no overall PASS claim; existing
merged-suite receipts and the completed funded control remain distinct.
