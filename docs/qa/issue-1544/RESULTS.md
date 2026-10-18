# #1544 results — 2026-10-18 UTC

The monitoring stack is deployed and the scoped Grafana/Loki/Prometheus checks
and live alert drill pass, including same-source replacement persistence.
**Issue #1544 remains open:** credential remediation has two operator custody
actions outstanding. A monitoring PASS does not establish completed
privacy remediation or resolve [#1505](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1505).

## Deployed change

| Identity | Value |
| --- | --- |
| Monitoring source | `fc5b128298d142481e55384b0fcd4b7f981aa228` |
| Separate Coolify application | `o4htirce3mwp89dzip3vdikt` |
| Successful deployment | `5pvlxx5gq3rpowsqx0f6p4zj` |
| Successful same-source replacement | `18s8f490jphntqc0lr7tunml` |
| Grafana | `https://logs.cl8y.com` |
| Datasource / dashboard | `cl8y-dex-prometheus` / `cl8y-dex-monitoring` |
| Serving DEX source observed throughout sampling | `5b623a04f35bb6b44d3e64df0014e458cad653fc` |

The separate application supplies digest-based, configuration-baked Prometheus
and Alertmanager images, a cached four-endpoint exporter, persistent data/alert
receipt volumes and a durable monitoring-only drill worker. Existing Grafana
and Loki are reused; the DEX API/indexer and logging containers were not
restarted for installation. Source and deployment fixes are merged through
[PR #1545](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1545),
[PR #1546](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1546) and
[PR #1547](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1547).

## Completed verification

- [x] Focused source checks: 49 unit controls plus two credential-staging
  controls; Compose, Prometheus/Alertmanager configuration parsers, synthetic
  alert-rule behavior, image builds and bounded full-stack runtime checks pass.
  Each executed test process stays below 60s. Native monitoring retains its
  hard 19-read budget.
- [x] [Strict live monitoring](live-monitoring.json), observed
  `2026-10-18T10:51:24.766051+00:00`: PASS in 11 reads. Datasource health is OK;
  both exact DEX jobs are up, with fixed service identity and fresh underlying
  observations. Recorded ages are 17.876s and 47.928s, within the 90s bound.
  The receipt retains the exact scoped PromQL. Missing/wrong/stale targets and
  datasource failures are rejected by focused negative tests.
- [x] [Monitoring after replacement](monitoring-after-replacement.json): PASS
  in 11 reads at `2026-10-18T10:58:45.597887+00:00`, independently verifying
  datasource health and exact fresh DEX targets after container replacement.
- [x] [Dashboard verification](dashboard-live.json), observed
  `2026-10-18T10:51:30.871878+00:00`: committed 19-panel model matches live;
  UTC/last-hour range and expected datasource binding pass. Six representative
  range queries return finite scoped samples. The independent observer revision
  is recorded separately as `57fbe254c82d4de582f391f9c1aa6a97024c40cf`.
- [x] Existing Loki queries pass in the same monitoring receipt. The 15-minute
  indexer aggregates include 22 error-pattern and 187 provider-pattern lines.
  These text-pattern counts do not prove outages, traffic error rates, provider
  pressure or the cause of historical HTTP503s.
- [x] [Live synthetic drill](live-drill.json): PASS, ten steps and zero
  transport errors. Pending at `10:51:36.039121Z`, firing at `10:51:58.038808Z`,
  delivered/disabled at `10:52:08.446476Z`, and recovered at `10:52:41.341942Z`.
  The agent-owned local webhook persisted firing and resolved receipts; no
  external or human notification is claimed. [Local drill](local-drill.json)
  also passes.
- [x] [Deployment logs](deployment-logs.json): deployment finished; 83 entries
  inspected in memory, zero other error-pattern entries and one expected
  missing cleanup-container entry. Raw messages/configuration are not exported.
- [x] [Serving continuity samples](api-continuity.json): 40/40 GETs returned
  HTTP200, 20/20 serving samples reported admitted, and the observed DEX SHA
  stayed unchanged across five windows from `10:33:50Z` through `10:56:56Z`.
  Sampling cannot prove uninterrupted availability between observations.
- [x] [Same-source replacement](replacement-after.json): all four monitored
  container identities changed and every named data-volume binding stayed
  unchanged. The replacement deployment finished, observed at
  `2026-10-18T10:58:23.277839Z`, with source
  `fc5b128298d142481e55384b0fcd4b7f981aa228`. Its
  [70 deployment-log entries](replacement-deployment-logs.json) contain one
  expected cleanup entry and zero other error-pattern entries.
- [x] Historical samples persisted: the exact
  [before](history-before.json)/[after](history-after.json) query at Unix time
  `1792320886` returns the same observation value `1792320859.844942`, predating
  the new startup. The [original drill proof](drill-after-replacement.json)
  retains all original markers, ten steps and zero errors after replacement;
  its terminal PASS persisted without a second enable request. The
  [replacement baseline](replacement-before.json) binds this comparison to the
  unchanged deployed source.
- [x] Metric labels and published receipts are bounded and sanitized;
  administrative controls remain private. This verifies monitoring controls,
  not completion of the separate session credential remediation below.

Coolify's aggregate application status is `running:unknown`, not
`running:healthy`. Finished deployment, actual target/datasource health and
persisted evidence were verified independently; the aggregate status is not
used as a substitute for those checks.

[Additional live metrics](live-metrics.json), observed
`2026-10-18T10:54:15.443890+00:00`, show all three monitoring infrastructure
scrapes up, successful Prometheus configuration load, one firing and one
resolved delivery receipt, and no active scoped alerts. At that observation,
indexer readiness returned HTTP503 with observed lag one block while liveness,
serving and useful-quote readiness returned HTTP200. These independent readiness
classes are preserved; the indexer status alone does not establish an API outage.

Production request latency/5xx counters and provider-pressure gauges remain
instrumentation gaps. Synthetic endpoint duration/status must not be presented
as traffic SLOs. `/ready` reports the last observed head, not an independent
live chain tip. #1505 keeps its own unresolved recovery/HTTP503 acceptance.

## Outstanding custody

- [ ] Complete [security remediation](security-remediation.json). Grafana's
  administrator credential was rotated, its old authentication rejected and
  OpenBao/Coolify replacement configuration aligned without a restart. S3
  replacement/adoption and ingestion-consumer inventory/adoption remain
  incomplete. cl8y-pm cards `c1c74912-795a-4108-bd86-4493c0b0b133` and
  `807d41d0-41a4-4e06-a690-344325ccb40e` cover the respective Console and sudo
  custody steps. Staging/inventory helpers consume secrets without publishing
  their values; staging does not switch Loki or revoke existing credentials.

Hosted CI remains unverified. Normal protected merge/push paths rejected the
release; the user explicitly requested overriding unavailable gates. No hosted
CI PASS is claimed, and the local/live evidence above remains separately scoped.

## Failures retained

The [README attempt history](README.md#scope-and-unresolved-acceptance) preserves
both failed deployments. The first failed on nested build/source paths. The
second stayed unhealthy because Prometheus rejected
`--web.enable-admin-api=false` (`unexpected false`), reproduced by a complete
local runtime after config parsers had passed. The flag was removed; admin APIs
remain disabled by default. Earlier host-bind initialization warnings did not
establish that second failure's cause. Baking configuration into images is a
retained deployment durability improvement. Subsequent success does not erase
either failed attempt.

Operations and verification commands:
[runbook](../../runbooks/dex-prometheus-monitoring.md),
[DM1544 agent invariants](../../../skills/AGENTS_DEX_MONITORING.md), and
[issue criteria](README.md#live-issue-criteria).

## Operator follow-up after the first cards

The collector-inventory card is done. The S3 card failed after both hidden
prompts; the original generic diagnostic does not identify the precise cause,
and no replacement record was saved. The existing key still passes signed
HEAD200, and current OpenBao staging authority/preflight passes. These checks
are not verification of the replacement key.

The helper now retains a private unverified candidate before the bucket check,
reports only fixed phase/status information, and supports no-prompt resumption.
One hidden re-entry is required for the discarded original input. A candidate
must reach verified-read state before agent-owned adoption; Loki write and
consumer adoption remain separate acceptance. The collector parser also now
recognizes digits in CL8Y names; its earlier empty key list was not absence
evidence. Completing discovery did not grant persistent privileged access.

The follow-up adds 16 synthetic S3 workflow/signing controls and two collector
parser/privacy controls; each focused process passes in under one second. No
runtime storage/ingestion credentials were changed or revoked during diagnosis.
See the [updated runbook](../../runbooks/dex-prometheus-monitoring.md#retrying-storage-staging-and-using-the-completed-collector-inventory)
and [security receipt](security-remediation.json). #1544 remains open.
