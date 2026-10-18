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
| Prometheus | Named data volume; 7d/2GB block-retention policies; committed config/rules copied into the built image | 512MB RAM, 0.5 CPU; 30s scrape interval, 5s timeout |
| Exporter | Cached snapshot; persistent bounded alert receipts; read-only fixed DEX origin | 256MB RAM, 0.25 CPU; four reads per at least 30s, each with 3s timeout/16KiB body bound |
| Alertmanager | Named state volume; committed routing config copied into the built image; local webhook sends firing and resolved notifications | 128MB RAM, 0.25 CPU; 5s group wait, 10s group interval, 4h repeat |
| Drill worker | Named state/receipt volume; one terminal proof per fresh monitoring state | 64MB RAM, 0.1 CPU; one bounded step every 5s |
| Grafana/Loki | Existing deployment, datasource/dashboard changes through authenticated API | No restart/replacement to add DEX metrics |

Keep the named data/receipt volumes when replacing monitoring containers. An
application deletion, `down -v` or volume prune destroys historical evidence.
Retention policies apply concurrently: the first reached expires old blocks.
The 2GB setting is a retention target, not a hard filesystem quota. Allow
additional disk for active head/WAL and operational overhead, and monitor actual
storage use independently of the configured retention policy.

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

The separate [security remediation receipt](../qa/issue-1544/security-remediation.json)
records credential custody follow-ups discovered during monitoring inventory.
Grafana administrator authentication was rotated and old authentication rejected;
S3 runtime adoption, canonical configuration persistence, old-key revocation
and ingestion credential rotation require separate evidence; staging is not
completion. Follow the scoped S3 transaction below before handing off any
remaining unavailable custody action. Do not claim all affected
credentials were rotated. The hidden-prompt
[S3 staging helper](../../scripts/ops/stage-logs-s3-credentials.py) verifies and
stages a replacement only; it does not switch Loki or revoke the current key.
The [collector inventory helper](../../scripts/ops/logs-collector-inventory.py)
requires operator sudo and exports metadata only. These security follow-ups
remain separate from monitoring deployment acceptance and preserve existing
ingestion/storage credentials until verified coordinated adoption.

### Retrying storage staging and using the completed collector inventory

The first S3 card failed after both hidden prompts with only a generic failure
message. That helper had not saved a replacement; the precise original failure
phase cannot be recovered. A read-only follow-up verified the existing key's
signed bucket HEAD returned HTTP200, and the pending OpenBao path is readable
and writable. This does not establish that the replacement key or its scope was
valid. The collector-inventory card completed, but its initial parser excluded
digits and missed `CL8Y_*` assignments. The corrected parser reports only
allowlisted names; an empty old key list never proves a collector has no secrets.
Systemd and Alloy references identify `/etc/default/alloy` as the host source.

The revised storage helper checks configuration and staging authority before
prompting. It saves a validated replacement privately as `staged_unverified`
with CAS/readback **before** probing S3, then promotes it to
`staged_verified_read` only after HEAD200. Failures emit fixed phase/reason and
HTTP status fields without bodies, credentials or raw subprocess errors. A
terminal-control failure cannot fall back to echoed input.

```sh
# Agent/noninteractive execution only: read-only configuration/authority check.
bao-exec --only COOLIFY_URL,COOLIFY_WIRE_TOKEN -- python3 scripts/ops/stage-logs-s3-credentials.py --preflight
# Operator terminal: launch directly, not inside another bao-exec.
python3 /home/answorld/repos/cl8y-dex-terraclassic/scripts/ops/stage-logs-s3-operator.py
# Agent/noninteractive execution: resume without further secret input.
bao-exec --only COOLIFY_URL,COOLIFY_WIRE_TOKEN -- python3 scripts/ops/stage-logs-s3-credentials.py --resume
```

The installed OpenBao output guard intentionally refuses any terminal-connected
standard stream before launching a child. Consequently the earlier direct
terminal `bao-exec ... stage-logs-s3-credentials.py` command is obsolete; its
guard rejection made no S3 request, read no replacement input and changed no
candidate. Certificate login and this terminal refusal are separate outcomes.

The reviewed local [operator launcher](../../scripts/ops/stage-logs-s3-operator.py)
shows hidden prompts outside that child. It first runs guarded preflight, then
automatically resumes a retained candidate or sends a new pair through a private
bounded stdin JSON pipe to `--input-json`. Both child output streams are pipes;
no raw-output option, secret arguments/environment or plaintext files are used.
The launcher validates exact report keys, fixed values and operation/exit-status
consistency before emitting a result; arbitrary child output/stderr is withheld.
Do not use the terminal launcher in an agent transcript or place it inside an
outer guarded `bao-exec`. Agents use noninteractive preflight/resume instead.
If login is expired, it reports a fixed certificate-login requirement before
prompting. Tests use a real PTY and the installed guard with synthetic fetch;
they prove echo is disabled, legacy execution is refused before launch, guarded
children have three nonterminal streams, and unknown downstream output cannot
reach the terminal. Staged bucket read verification and a separate owned
PUT/GET/DELETE probe do not establish that Loki has adopted the candidate.

`candidate_retention=confirmed` means the current candidate had private
readback; `unknown` means a storage request/readback was ambiguous, and `none`
means the latest input was not confirmed saved. `prior_candidate_present`
describes a separate older pending record. Check pending state privately before
retrying an ambiguous write. Default execution refuses to overwrite an existing
candidate; `--replace-pending` explicitly permits a new hidden-input candidate
and retains CAS even when the prior record has stale configuration. Rechecking
a previously verified candidate first downgrades it, so a negative read cannot
leave the older positive eligible for adoption. Only `staged_verified_read`
permits subsequent agent-owned adoption verification; even that proves read
access only, not Loki write permission or successful runtime rotation.

### Adopting staged S3 credentials without restarting the logs stack

Use the reviewed [`adopt-logs-s3.py`](../../scripts/ops/adopt-logs-s3.py) client,
[`loki-s3-adoption-worker.py`](../../scripts/ops/loki-s3-adoption-worker.py), and
[`replace-loki-s3-host.py`](../../scripts/ops/replace-loki-s3-host.py). Source
checks and the [focused acceptance checklist](../qa/issue-1544/README.md#credential-remediation-acceptance)
are separate from live adoption. Do not deploy the full existing logs application:
Coolify stops the complete Compose application before starting its replacement.
The credential transaction may replace only Loki; Grafana, the ingestion proxy,
collectors and the serving DEX remain running.

The client journals the expected old and new S3 credentials privately in OpenBao
using CAS and private readback before deployment. It binds the job to the exact
logs application/server, reviewed source revision and worker recipe hash; build
the embedded scripts from `git show` at that revision, never uncommitted bytes. An
ambiguous deployment outcome requires status inspection; never blindly retry an
adoption. `recover` reconciles an ambiguous Service creation against the journal
without starting or recreating a job. A staged candidate must remain unchanged
between preflight and adoption.

Create the maintenance job as a separate Coolify **Service** with raw Compose
and an explicit `build.dockerfile_inline`. Dockerfile Application custom run
options do not reliably preserve the required volume/restart settings. Define
the socket, read-only application directory, state volume and `restart: "no"`
in the Service's Compose document. Keep only environment references in raw
Compose: the private runtime payload must never appear in build arguments,
inline Dockerfile bytes or persisted raw Compose text. Coolify's private runtime
environment and generated configuration remain credential-bearing and must be
removed with the job; never publish them.

The one-shot maintenance job has effective host-root authority through its Docker
socket. This is an explicitly scoped owner deployment, not an unprivileged
container sandbox. Give it only the old/new S3 transaction payload and nonsecret
job identity; never copy workstation certificates, cached sessions, OpenBao or
Coolify tokens into it. Mount only the Docker socket, the existing application's
configuration directory read-only, and the job's named state volume. The volume
holds nonsecret receipts and a once-only latch, never the payload. Disable
automatic retries/restarts, attach only to the inspected existing application
network for verification, and detach afterwards. Do not use root SSH or broaden
OpenBao ACLs to run this transaction.

Agents execute each phase separately under guarded, noninteractive `bao-exec`,
checking its closed report before continuing. Use `create --revision <exact-main-sha>`
for preflight; `status` must show the exact owned Service/maintenance child and
a successful closed preflight receipt before `adopt`. Service work is verified
through worker receipts, not an Application deployment UUID/status. `adopt`
updates only the job's runtime mode and restarts only that maintenance Service.
After `status` proves successful adoption, use
`finalize` to persist and privately read back only the two canonical Coolify S3
credential fields, without redeploying the stack. Finally `cleanup` removes the
saved secret environment entry and the maintenance Service; confirm deletion
completed. Shared networks and data volumes must survive cleanup.

```sh
# Each invocation is an agent operation; never print raw captured responses.
bao-exec --only COOLIFY_URL,COOLIFY_WIRE_TOKEN -- python3 scripts/ops/adopt-logs-s3.py status
```

The Loki replacement uses the local Docker Engine API to clone the inspected
container configuration, preserving its exact image ID, command, mounts, networks,
security/resources and noncredential environment. Compose rendering only attests
the canonical source; never use Compose `up` for this transaction. Retain the
stopped original container under a temporary name until the replacement passes
all checks. Rollback removes only the failed replacement and restarts that same
original container ID, checking identity, old credentials and readiness separately.
Successful adoption removes the stopped original with volume deletion disabled.
Preserve WAL and historical data. Engine-generated IDs/hostnames change with a
new container; the image ID and effective runtime settings must remain equal.

If an attempt fails, retain its terminal receipt and latch. The installed
platform stores scheduled commands in a 255-character field: the disabled,
manual read-only recovery task fetches public diagnostic source at a full Git
SHA and verifies SHA256 before executing it. It exports only the closed
`loki_s3_diagnostic_v1` report. Confirm current old-key matching, a fresh Docker
readiness healthcheck and canonical `compose.yaml` attestation; this observation
does not reconstruct a lost original projection. Remove the diagnostic task.
Only then use `reset-preflight --revision <reviewed-sha>` to archive the failure
and recovery observation, update only the owned maintenance recipe, and start a
fresh preflight with a new image/state-volume nonce. Never delete a previous
latch to permit retry. Both the volume key and name contain the nonce, including
on platforms that rename volumes. Only a receipt matching the active nonce may
authorize adoption. The fixed canonical source override is journaled privately;
stdin Compose metadata such as `config_files="-"` cannot select a new file.

Live proof requires exact runtime credential matching, readiness, the same
historical sample before/after replacement, and successful backing-storage
writes. A push HTTP204 or asynchronous `/flush` HTTP204 alone is insufficient:
require increased successful stored/flushed chunk counters and no new flush
failures, plus marker readback. Historical store statistics do not prove a fresh
uncached S3 GET; marker readback may use an ingester, so it does not alone prove
that exact marker's object durability. The independent owned S3 object probe
establishes only that candidate's direct read/write/delete scope.

Record source/Service/maintenance-child identities, UTC, readiness, immutable-runtime checks,
storage proof, canonical readback and job/secret cleanup as separate sanitized
results. No phase revokes the previous S3 key. Keep old-key revocation and
ingestion-principal rotation pending until their own verification succeeds;
successful S3 adoption cannot close those items or #1544 by itself.

Completing the collector card granted no persistent sudo or Docker access.
Before ingestion rotation, prepare one reviewed privileged transaction with
preflight for every actual host/stack/future-worker consumer. Add a distinct
replacement ingestion username/password alongside the old proxy principal;
verify authenticated pushes and access restrictions. Adopt the new identity in
only the existing collector environment sources/processes, preserving WAL,
mounts, networks and rollback originals; update canonical OpenBao/worker
delivery and Coolify replacement configuration with CAS/readback. Prove fresh
per-source delivery before removing the old principal, then require old401 and
new204. Never restart the DEX or blindly execute expanded Coolify Compose, and
never treat completed discovery, KV changes or staging as completed adoption.

Use the scoped
[`dex-monitoring-1544.py`](../../scripts/ops/dex-monitoring-1544.py) helper for
the independent Coolify application and existing Grafana API configuration:

```sh
# Exact reviewed main SHA; app UUID is returned by create (both are nonsecret).
REVISION='<exact-main-sha>'
bao-exec --only COOLIFY_WIRE_TOKEN,COOLIFY_URL -- python3 scripts/ops/dex-monitoring-1544.py create --revision "$REVISION"
MONITOR_APP='<returned-monitoring-app-uuid>'
# For an existing monitoring app, select the new exact revision before deploy.
bao-exec --only COOLIFY_WIRE_TOKEN,COOLIFY_URL -- python3 scripts/ops/dex-monitoring-1544.py pin --app "$MONITOR_APP" --revision "$REVISION"
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

`pin` changes only the owned monitoring application's commit SHA, validates the
selected revision contains the Compose file and verifies exact readback; it
does not dispatch deployment. `deploy` uses a POST with the owned application
UUID and accepts exactly one matching deployment receipt. An ambiguous response
is a failure to reconcile through `status`, never a reason to repeat dispatch.

Coolify evaluates this nested Compose file from the **repository root**.
Its build context is `.`, despite the file residing in `monitoring/dex/`.
Prometheus and Alertmanager configuration is copied into their respective
images by the committed Dockerfiles; production has no configuration bind
mounts. This avoids depending on Coolify's nested host-path rewriting and
repository staging lifetime. Preserve the root build-context convention.
Local config checks must select the same project directory:

```sh
timeout 55s docker compose --project-directory "$PWD" -f monitoring/dex/compose.yaml config --quiet
```

Run that command from the checkout root. A differently rooted local check can
pass while the platform builds from another directory. Restore/replacement
must rebuild the images from the exact selected Git SHA, whose Dockerfiles pin
their base images by digest and copy that revision's configuration/rules.
Preserve the named TSDB, Alertmanager, alert-receipt and drill-receipt volumes.
Configuration changes require a new committed image build and monitoring-only
rollout; editing a checkout file cannot update a running image's configuration.

Committed configuration:

- [`compose.yaml`](../../monitoring/dex/compose.yaml): independent application,
  resource bounds, named volumes and existing Grafana network attachment.
- [`Dockerfile.prometheus`](../../monitoring/dex/Dockerfile.prometheus) and
  [`Dockerfile.alertmanager`](../../monitoring/dex/Dockerfile.alertmanager):
  digest-pinned base images with committed configuration copied into each image.
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
Config parser success does not validate startup arguments or container health.
Use a bounded isolated full-stack startup check as well; a local rehearsal
caught Prometheus rejecting `--web.enable-admin-api=false` even after parser
checks passed. Omit that unsupported flag: the admin API is disabled by default.
When confirming the disabled admin endpoint, Prometheus returns HTTP500 JSON
with `status=error`, `errorType=unavailable`, `error="admin APIs disabled"`.
Do not require HTTP403 as the disabled-admin contract.
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
datasource/dashboard configuration through Grafana API. Preserve every named
data/receipt volume, rebuild the images and reapply the datasource/dashboard
from the exact chosen revision, then repeat target freshness and Loki checks.
Verify the existing terminal drill proof survives replacement; any separate
fresh rehearsal must preserve its original result. Embedded rules/configuration
come from that revision's images rather than host files.
If the DEX itself fails, use the existing API recovery/rolling runbooks; a
monitoring reinstall cannot repair or authorize application changes.
