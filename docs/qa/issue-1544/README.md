# #1544 monitoring acceptance

Issue: [#1544](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1544).
Implementation: [`monitoring/dex/`](../../../monitoring/dex/).
Operations: [Prometheus runbook](../../runbooks/dex-prometheus-monitoring.md).
Agent contract: [DM1544-1–DM1544-10](../../../skills/AGENTS_DEX_MONITORING.md).
Current live evidence and remaining work: [RESULTS.md](RESULTS.md).

Run `make verify-issue-1544` for focused source checks. Each test process must
finish under 60 seconds. Keep real failures in the record after fixes; a passing
fixture or configured datasource does not establish a successful deployment.
Missing Prometheus is FAIL, never `SKIP: no_prometheus_datasource_provisioned`.

## Live issue criteria

Check an item only against its sanitized live evidence. A source check alone
cannot complete these items.

- [x] Prometheus is healthy; configuration and sample storage survive monitoring
  replacement; both expected DEX scrape jobs produce fresh observations.
- [x] The existing Grafana at `https://logs.cl8y.com` reports datasource health;
  representative PromQL passes for the exact service and jobs; dashboard panels
  use the expected datasource and time range.
- [x] Existing Loki queries still work. Record logs and metrics separately;
  document missing traffic latency/5xx and provider pressure instrumentation.
- [x] A monitoring-only synthetic rule shows pending, firing, a persisted alert
  delivery receipt and recovered states without changing DEX behavior.
- [x] The bounded verifier rejects missing datasource, missing/stale series,
  wrong-service identity and failed scrapes. Record UTC, exact source/deployed
  revisions, scoped queries and bounded sanitized results.
- [x] The serving API stays available across monitoring rollout; deployment
  actions do not stop/restart it or reset production infrastructure.
- [ ] Labels and exported evidence omit actors, transaction/request IDs,
  provider URLs, query strings and credentials; administrative endpoints remain
  access-controlled and series/alert receipt counts stay bounded.
- [x] Provisioning, configuration, replacement/recovery, focused verification
  and agent guidance are committed and cross-linked; deployment logs are checked.

The privacy item remains open because the separate credential remediation is
incomplete; metric-label/privacy controls themselves pass. Serving continuity
is supported by 40/40 successful sampled requests, 20/20 admitted serving checks
and unchanged DEX source; sampling does not prove every interval between checks.
Same-source replacement changed all four container identities, preserved every
named volume binding and historical sample, and retained the original terminal
drill proof without a second enable request. Coolify's aggregate app status is
`running:unknown`; actual datasource and target health passed independently.
See [RESULTS.md](RESULTS.md) for artifacts, UTCs and exact limitations.

## Evidence contract

Record a UTC observation interval, Git revision, Coolify monitoring app/deployment
identity, existing Grafana identity and datasource UID. Report both jobs and the
fixed service label, successful/fresh scrapes, exporter observation age, endpoint
purpose outcomes and bounded representative queries. Preserve each attempted
verification result. Only publish sanitized counts, fixed enums, booleans,
timestamps and nonsecret deployment identities; retain no raw API responses,
environment values or log lines.

Capture the monitoring-only alert rule name and pending/firing/recovered UTCs,
the actual persistent local receipt and agent owner. Check replacement preserves
the named volumes and configuration, and compare serving liveness before/after.
The automatic drill's durable `/state/drill-receipt.json` must include pending,
firing, delivered, disabled and recovered markers; its terminal PASS/FAIL must
survive worker replacement without a second enable request. Keep original
failed receipts when a separate fresh rehearsal is needed.
Configuration persistence uses committed config/rules copied into built images,
with digest-pinned bases. Record the exact Git SHA used to rebuild replacement
images and confirm every named data/receipt volume is retained; production
configuration must not depend on host bind-path behavior.
If an external contact point is configured, additionally identify its nonsecret
type and successful delivery outcome; a local receipt must not be described as
delivery to a human, email or chat channel.

## Scope and unresolved acceptance

Preserved rollout failures before the configuration-image correction:

| Deployment | Source revision | Result |
| --- | --- | --- |
| `8apkcl1ijoox5tqxfja1tp49` | `47b2b9180fe9df131b7c0cdc1ca427038c92c63f` | FAIL before startup: nested Compose build/source paths differed from the platform's repository-root execution. |
| `oxpthkc1nxwkkd17tnidvkgn` | `93398a8100fb9c983e62cc34b9ff439021ac9a6b` | FAIL: exporter/drill built; Prometheus stayed unhealthy. A complete local runtime reproduced its startup rejection of `--web.enable-admin-api=false` with `unexpected false`. Status observed 2026-10-18T10:42:00.995892+00:00; 134 deployment-log entries inspected, five error-pattern mentions. |

The second attempt's earlier host-bind/config-initialization warnings did not
establish the cause of unhealthy Prometheus. The confirmed runtime defect was
the unsupported boolean-flag syntax; the correction removes that flag and
retains Prometheus's default-disabled admin API. A local disabled-admin request
returns HTTP500 with fixed JSON `status=error`, `errorType=unavailable` and
`error="admin APIs disabled"`; HTTP403 is not its disabled-state contract.
Configuration copied into committed Prometheus/Alertmanager images remains a
deployment durability improvement that removes production host-bind dependence.

Config parsers and unit tests passed before the startup failure; they do not
validate process argument acceptance or full-stack health. The corrected local
stack starts healthy and all five scoped scrape jobs report up. Its synthetic
drill subsequently completed recovery; [local proof](local-drill.json) and
[live proof](live-drill.json) both pass. The corrected live monitoring deployment
and scoped checks pass as recorded in [RESULTS.md](RESULTS.md). Keep the two
failed attempts alongside those subsequent results.

This stack observes four existing health/readiness endpoints. It does not add
traffic request histograms/counters or provider admission gauges, identify the
cause of historical HTTP503s, certify all markets, or establish an independent
live chain tip. Use existing bounded Loki probes to investigate provider and
native quote failures. [#1505](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1505)
retains its own acceptance; a healthy monitoring rollout cannot close it.

Credential custody follow-ups are recorded separately in
[security-remediation.json](security-remediation.json), including cl8y-pm card
identities and the [S3 staging](../../../scripts/ops/stage-logs-s3-credentials.py)
and [collector inventory](../../../scripts/ops/logs-collector-inventory.py)
helpers. Grafana rotation passed; remaining S3/ingestion custody work is
incomplete. Monitoring success cannot establish complete credential remediation.

If any live item fails or is unavailable, leave #1544 open and record exactly
what failed or remains unverified. Request any unavoidable human/device action
as one concise cl8y-pm inbox card per action; never substitute a source PASS for
missing live evidence.
