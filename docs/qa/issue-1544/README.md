# #1544 monitoring acceptance

Issue: [#1544](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1544).
Implementation: [`monitoring/dex/`](../../../monitoring/dex/).
Operations: [Prometheus runbook](../../runbooks/dex-prometheus-monitoring.md).
Agent contract: [DM1544-1–DM1544-10](../../../skills/AGENTS_DEX_MONITORING.md).

Run `make verify-issue-1544` for focused source checks. Each test process must
finish under 60 seconds. Keep real failures in the record after fixes; a passing
fixture or configured datasource does not establish a successful deployment.
Missing Prometheus is FAIL, never `SKIP: no_prometheus_datasource_provisioned`.

## Live issue criteria

Check an item only against its sanitized live evidence. A source check alone
cannot complete these items.

- [ ] Prometheus is healthy; configuration and sample storage survive monitoring
  replacement; both expected DEX scrape jobs produce fresh observations.
- [ ] The existing Grafana at `https://logs.cl8y.com` reports datasource health;
  representative PromQL passes for the exact service and jobs; dashboard panels
  use the expected datasource and time range.
- [ ] Existing Loki queries still work. Record logs and metrics separately;
  document missing traffic latency/5xx and provider pressure instrumentation.
- [ ] A monitoring-only synthetic rule shows pending, firing, a persisted alert
  delivery receipt and recovered states without changing DEX behavior.
- [ ] The bounded verifier rejects missing datasource, missing/stale series,
  wrong-service identity and failed scrapes. Record UTC, exact source/deployed
  revisions, scoped queries and bounded sanitized results.
- [ ] The serving API stays available across monitoring rollout; deployment
  actions do not stop/restart it or reset production infrastructure.
- [ ] Labels and exported evidence omit actors, transaction/request IDs,
  provider URLs, query strings and credentials; administrative endpoints remain
  access-controlled and series/alert receipt counts stay bounded.
- [ ] Provisioning, configuration, replacement/recovery, focused verification
  and agent guidance are committed and cross-linked; deployment logs are checked.

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
If an external contact point is configured, additionally identify its nonsecret
type and successful delivery outcome; a local receipt must not be described as
delivery to a human, email or chat channel.

## Scope and unresolved acceptance

This stack observes four existing health/readiness endpoints. It does not add
traffic request histograms/counters or provider admission gauges, identify the
cause of historical HTTP503s, certify all markets, or establish an independent
live chain tip. Use existing bounded Loki probes to investigate provider and
native quote failures. [#1505](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1505)
retains its own acceptance; a healthy monitoring rollout cannot close it.

If any live item fails or is unavailable, leave #1544 open and record exactly
what failed or remains unverified. Request any unavoidable human/device action
as one concise cl8y-pm inbox card per action; never substitute a source PASS for
missing live evidence.
