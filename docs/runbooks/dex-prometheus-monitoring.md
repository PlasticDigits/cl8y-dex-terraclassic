# DEX Prometheus monitoring (#1544)

This repository owns a monitoring-only Coolify application under
[`monitoring/dex/`](../../monitoring/dex/). It complements the existing Grafana
and Loki at `https://logs.cl8y.com`. Deploy/configure this application separately
from the DEX API/indexer, database and logs stack. Source configuration is not
evidence that a deployment passed: record the
[live checklist](../qa/issue-1544/README.md) before closing
[#1544](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1544).

Agent invariants: [DM1544-1–DM1544-10](../../skills/AGENTS_DEX_MONITORING.md).
Existing operational signals: [launch monitoring](launch-monitoring.md).
Application release constraints: [API rolling readiness](../qa/api-rolling-readiness/README.md).

## Ownership, topology and resources

The repository's monitoring agent owns routine alert review and recovery.
Monitoring alerts are delivered to the persistent local webhook receipt on the
exporter and displayed in Grafana; this destination does not interrupt the sole
human developer. Do not describe a receipt as email/chat/pager delivery.
Human/device actions, when unavoidable, are separate concise cl8y-pm inbox cards.

The separate application contains Prometheus, the bounded Python exporter and
Alertmanager. It reuses the existing Grafana/Loki installation through Grafana
datasource/dashboard provisioning APIs. There is no second Grafana or Loki.
Prometheus and administrative/exporter control interfaces stay on the private
monitoring network; do not publish unauthenticated host ports. Grafana connects
to the monitoring application's private Prometheus service over its existing
Coolify network. Only Grafana's existing authenticated HTTPS entrypoint is public.

| Component | Persistence/bounds | Budget |
| --- | --- | --- |
| Prometheus | Named data volume; seven-day and 2GB retention; committed config/rules | 512MB RAM, 0.5 CPU; 30s scrape interval, 5s timeout |
| Exporter | Cached snapshot; persistent bounded alert receipts; read-only fixed DEX origin | 256MB RAM, 0.25 CPU; four reads per at least 30s, each with 3s timeout/16KiB body bound |
| Alertmanager | Named state volume; local webhook sends firing and resolved notifications | 128MB RAM, 0.25 CPU; 5s group wait, 10s group interval, 4h repeat |
| Drill worker | Named state/receipt volume; one terminal proof per fresh monitoring state | 64MB RAM, 0.1 CPU; one bounded step every 5s |
| Grafana/Loki | Existing deployment, datasource/dashboard changes through authenticated API | No restart/replacement to add DEX metrics |

Keep the named data/receipt volumes when replacing monitoring containers. An
application deletion, `down -v` or volume prune destroys historical evidence.
Retention bounds apply concurrently: the first reached expires old blocks.

## Observation contract and interpretation

The exporter samples only the fixed origin `https://indexer.dex.cl8y.com`:

| Purpose | Endpoint | Interpretation |
| --- | --- | --- |
| `liveness` | `/health` | Cheap process/API response |
| `serving` | `/health/serving` | Process-owned initial admission latch; serving health |
| `indexer_readiness` | `/ready` | Indexing/leadership status, checkpoint and last observed head |
| `quote_readiness` | `/ready-api` | Bounded configured useful-quote readiness sample |

The background producer owns these reads. `/metrics` returns its cached
snapshot and never causes an endpoint read, chain query, quote refresh or new
provider work. Redirects are disabled. Invalid/oversized/unavailable observations
omit their observation timestamp and heights; absence is not a zero-height
checkpoint. Valid HTTP503 readiness payloads preserve fresh status/height evidence.

| Metric | Labels/meaning |
| --- | --- |
| `dex_endpoint_up` | Fixed `purpose`; 1 only for a valid HTTP200 observation |
| `dex_endpoint_http_status` | Fixed `purpose`; numeric observed HTTP status |
| `dex_endpoint_duration_seconds` | Fixed `purpose`; duration of one synthetic health/readiness request |
| `dex_observation_timestamp_seconds` | Minimum valid observation time across all four endpoints in the current cycle; absent if any endpoint's observation is invalid |
| `dex_indexer_checkpoint_height` | Valid observed checkpoint |
| `dex_indexer_observed_head_height` | API's last observed head, when available |
| `dex_indexer_lag_blocks` | Nonnegative difference of that observed head and checkpoint |
| `dex_indexer_progress_timestamp_seconds` | Time this exporter first observed the current checkpoint change |
| `dex_monitoring_drill` | Fixed synthetic drill flag; resets automatically within 180s |
| `dex_alert_notifications_total` | Fixed `status=firing\|resolved`; count in the bounded persisted receipt window, a gauge despite the historical `_total` suffix |
| `dex_alert_last_notification_timestamp_seconds` | Most recent persisted alert delivery receipt time |

Prometheus applies the two exact job identities `cl8y-dex-api` and
`cl8y-dex-indexer` and service label
`service="cl8y-dex-terraclassic-indexer"`. Job metric selection prevents one
endpoint observation from implying all readiness classes passed.

Synthetic request duration/status is **not** production traffic p95 latency,
5xx rate or traffic volume. Provider admission/pressure counters are not exported
by the existing application. Those dashboard gaps stay explicit. Use bounded
Loki observations for provider context; never convert a log count into an
unqualified traffic SLO. A fresh exporter snapshot of `/ready` does not establish
an independently sampled live chain tip. A frozen observed head can hide lag;
checkpoint progress age provides a separate stall signal and can also rise
during an actual chain halt or exporter restart.

HTTP503 for `quote_readiness` or `indexer_readiness` does not establish serving
failure. Compare the `serving` purpose and fixed status classes before escalating.
These metrics cannot identify the cause of historical HTTP503s or satisfy
[#1505](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1505) recovery acceptance.

## Credentials and reproducible provisioning

Use the workstation `bao-exec` from
[openbao-tools](https://git.cl8y.com/PlasticDigits/openbao-tools), which reads
inventoried credentials into the child process environment. Scope `--only` to
the actual Coolify credentials required by the operation. The existing bounded
Grafana probes resolve its credentials from the selected Coolify logs service
inside that process; do not print raw Coolify environment responses.

```sh
bao-exec --check
bao-exec --only COOLIFY_WIRE_TOKEN,COOLIFY_URL -- python3 scripts/qa/probe-swap-monitoring-1479-1480.py --output /tmp/dex-monitoring.json
```

`bao-exec --check` reports access, never values. Do not use `env`, `set -x`,
credential-bearing command arguments, `curl -v`, unredacted exception bodies or
raw deployment logs as published evidence. The existing probes require
`COOLIFY_WIRE_TOKEN`; select other inventoried names only for tooling that
explicitly supports those names. Do not copy tokens between files.

Use the scoped
[`dex-monitoring-1544.py`](../../scripts/ops/dex-monitoring-1544.py) helper for
the independent Coolify application and existing Grafana API configuration:

```sh
# Exact reviewed main SHA; app UUID is returned by create (both are nonsecret).
REVISION='<exact-main-sha>'
bao-exec --only COOLIFY_WIRE_TOKEN,COOLIFY_URL -- python3 scripts/ops/dex-monitoring-1544.py create --revision "$REVISION"
MONITOR_APP='<returned-monitoring-app-uuid>'
bao-exec --only COOLIFY_WIRE_TOKEN,COOLIFY_URL -- python3 scripts/ops/dex-monitoring-1544.py deploy --app "$MONITOR_APP"
bao-exec --only COOLIFY_WIRE_TOKEN,COOLIFY_URL -- python3 scripts/ops/dex-monitoring-1544.py status --app "$MONITOR_APP"
bao-exec --only COOLIFY_WIRE_TOKEN,COOLIFY_URL -- python3 scripts/ops/dex-monitoring-1544.py provision
bao-exec --only COOLIFY_WIRE_TOKEN,COOLIFY_URL -- python3 scripts/ops/dex-monitoring-1544.py verify
```

Run commands separately; inspect each result before dependent mutations. Record
the monitoring app/deployment identity, exact revision, health/config validation
and sanitized deployment log outcome. Provisioning uses datasource stable UID
`cl8y-dex-prometheus` and dashboard UID `cl8y-dex-monitoring`.
Use Grafana's authenticated `/api/ds/query` for Prometheus queries: the existing
logs proxy restricts datasource proxy URLs to Loki and is preserved. Loki
continues through its existing allowed datasource proxy. Browser access to
private Prometheus is unnecessary.

Committed configuration:

- [`compose.yaml`](../../monitoring/dex/compose.yaml): independent application,
  resource bounds, named volumes and existing Grafana network attachment.
- [`prometheus.yml`](../../monitoring/dex/prometheus.yml): fixed scrape jobs,
  per-job sample/label/body bounds and metric allowlists.
- [`alerts.yml`](../../monitoring/dex/alerts.yml): five-second evaluation and
  fixed alert rules.
- [`alerts.test.yml`](../../monitoring/dex/alerts.test.yml): synthetic rule
  behavior checks run with `promtool test rules`, without changing the DEX.
- [`alertmanager.yml`](../../monitoring/dex/alertmanager.yml): local persistent
  receipt delivery, including resolved notifications.
- [`dashboard.json`](../../monitoring/dex/dashboard.json): fixed datasource UIDs,
  service/job/purpose queries, shared UTC time range and explicit coverage gaps.
- [`drill.py`](../../monitoring/dex/drill.py): durable monitoring-only
  pending/firing/delivery/recovery proof with bounded local HTTP requests.

The external logs network name in Compose is a nonsecret deployed resource
identity, not a portable universal default. Inventory the selected existing
Grafana service/network before deploying elsewhere and change only this
monitoring attachment. Prometheus's network alias is `dex-prometheus`, so the
Grafana datasource URL is `http://dex-prometheus:9090`.

## Scoped live verification

Run `make verify-issue-1544` first. Keep each executed test process under 60s.
The bounded existing monitoring probes enforce the provisioned datasource UID,
datasource health, exact DEX service/jobs, expected series, successful scrapes
and observation/scrape freshness. Missing Prometheus now fails verification.
Native monitoring has a hard 19-read cap including Loki and Prometheus; do not
repeat failing queries in an unbounded loop.

Representative PromQL (through Grafana's authenticated query API):

```promql
up{job=~"cl8y-dex-api|cl8y-dex-indexer",service="cl8y-dex-terraclassic-indexer"}
time() - timestamp(up{job=~"cl8y-dex-api|cl8y-dex-indexer",service="cl8y-dex-terraclassic-indexer"})
dex_endpoint_up{job="cl8y-dex-api",service="cl8y-dex-terraclassic-indexer",purpose="serving"}
dex_endpoint_http_status{job="cl8y-dex-api",service="cl8y-dex-terraclassic-indexer",purpose="quote_readiness"}
dex_indexer_lag_blocks{job="cl8y-dex-indexer",service="cl8y-dex-terraclassic-indexer"}
time() - dex_indexer_progress_timestamp_seconds{job="cl8y-dex-indexer",service="cl8y-dex-terraclassic-indexer"}
```

Check both scrape sample time and the embedded observation time (normally at
most 90s old). A fresh scrape of frozen cached observations cannot pass. A
global `sum(up)` or unrelated healthy job cannot pass. Dashboard time range and
datasource UID must match this deployment, and the existing bounded Loki queries
must still succeed. Keep metrics and logs separate in the sanitized receipt.

Compare serving checks before, during and after monitoring deployment. Do not
restart the DEX to clear a monitoring failure. If verification fails, fix the
monitoring configuration, replace only its isolated test/monitoring components
as needed and repeat the failing focused checks. Preserve failure evidence.

## Alert drill and recovery

Availability alerts distinguish absent scrapes/observations from failed serving
checks; indexer alerts use observed lag and checkpoint progress age. All alert
identities and purpose classes are bounded. The persistent local webhook receipt
is the configured delivery destination and Grafana provides its review surface.

The exporter exposes cached metrics on private port 9800 and administrative
controls on private port 9801. `POST /alerts` accepts Alertmanager webhook
notifications, retains only fixed alert/status/service/job/purpose classes and
persists at most 256 entries atomically in `/state/alerts.json`. `GET /alerts`
returns the latest 32 sanitized entries. Mount its persistent state volume with
ownership writable by uid 65534; never publish the admin port.

Exercise the dedicated monitoring-only synthetic drill control by posting
`{"enabled":true}` to `/drill` on the private exporter admin interface; post
`{"enabled":false}` to end it. Its automatic 180s expiry is a cleanup bound.
It changes a synthetic metric only; it never fails a
DEX endpoint, pauses indexing, mutates quote state or requests chain work.
Observe the rule's pending state, subsequent firing state, the bounded persisted
delivery receipt, then reset the drill and observe recovery/resolved delivery.
Record UTCs and the fixed drill name. A unit test that models the rule is not
proof of live alert evaluation/delivery. Waits are observation intervals, not
long-running tests; bound each individual command/test under 60 seconds.

The separate `dex-drill` service performs this proof automatically once per
fresh `drill-receipts` state volume. It persists intent before its single enable
request, then runs bounded five-second steps through pending, firing, delivery
and recovery. Ambiguous enable responses are never retried or used to renew
the original 180s flag. Completion is bounded by 240s/64 steps and requires
both a delivered firing receipt and a delivered resolved receipt, plus the
observed pending state. A missing pending observation remains FAIL even if
later firing and recovery work. Each step makes at most two private monitoring
requests; it has no DEX endpoint or credential access.

Its `/state/drill-state.json` and sanitized `/state/drill-receipt.json` survive
worker replacement. PASS/FAIL is terminal across replacement, so a restart
cannot erase failure evidence or re-enable the drill. Preserve the terminal
receipt and original failed attempts before creating any explicitly isolated
fresh rehearsal state. Do not delete a production volume to make a failed
check appear green. After automatic proof, manual drill controls are a separate
rehearsal and cannot replace its original receipt.

The committed delivery destination is exactly
`http://dex-exporter:9801/alerts`, receiver `dex-agent-receipts`. The synthetic
`DexMonitoringDrill` rule has a 20s pending period. Other availability/stall
rules wait 1m; observed lag above 20 blocks and quote unavailability wait 2m.

There is no configured external destination or outbound notification. Do not
claim email/chat/pager coverage from the local receipt. If a future change adds
external delivery, document its exact supported receiver and ownership, consume
its credentials through OpenBao, preserve local delivery and separately prove
external delivery and recovery.

For rollback, stop/revert only the new monitoring application or revert its
datasource/dashboard configuration through Grafana API. Preserve both named
volumes, reapply the committed datasource/rules/dashboard from the chosen
revision and repeat target freshness, Loki and drill checks after recovery.
If the DEX itself fails, use the existing API recovery/rolling runbooks; a
monitoring reinstall cannot repair or authorize application changes.
