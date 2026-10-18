# DEX Prometheus monitoring (#1544)

Apply when changing DEX monitoring, Grafana datasource/dashboard provisioning,
target verification or alert delivery. Follow the
[runbook](../docs/runbooks/dex-prometheus-monitoring.md) and
[acceptance checklist](../docs/qa/issue-1544/README.md).

| ID | Invariant |
| --- | --- |
| DM1544-1 | Monitoring is a separate Coolify application from the DEX and existing Grafana/Loki. Configure the existing Grafana through its API. Never stop, restart or reset the serving DEX API, database or logs stack to install monitoring. Any application change follows [AR1–AR7](AGENTS_API_ROLLING_READINESS.md). |
| DM1544-2 | Exporter sampling owns at most four fixed-origin GETs per 30 seconds: `/health`, `/health/serving`, `/ready`, `/ready-api`. `/metrics` serves the cached snapshot and never triggers DEX, database, LCD or quote discovery work. Keep response/time bounds and disable redirects. |
| DM1544-3 | Serving liveness, indexing/leadership readiness and useful-quote readiness remain separate fixed purpose classes. A readiness HTTP503 is not automatically a serving outage. `/ready` reports an observed head, not an independently sampled live chain tip. |
| DM1544-4 | Verify both `cl8y-dex-api` and `cl8y-dex-indexer` jobs with `service="cl8y-dex-terraclassic-indexer"`; require exact target identity, successful scrapes, fresh scrape timestamps and fresh underlying observations. Global `sum(up)`, a datasource entry or an unrelated target cannot pass. |
| DM1544-5 | Missing Prometheus/datasource/required series, stale observations, wrong-service targets and unsuccessful datasource health are failures in monitoring verification. Keep native monitoring within its 19-read total cap; do not add unbounded retries. Executed failures remain visible. |
| DM1544-6 | Labels use fixed service/job/purpose/alert classes only. Never publish wallet/actor identifiers, transaction hashes, request IDs, provider URLs, query strings, arbitrary JSON fields, exception bodies or secrets. Bound evidence, query windows and receipt storage. Administrative interfaces remain private or authenticated. |
| DM1544-7 | Prometheus uses 7d/2GB block-retention policies on persistent storage, with 512MB memory/0.5 CPU. The retention target is not a hard disk quota; allow head/WAL and operational overhead. Exporter uses 256MB/0.25 CPU. Scrapes run every 30 seconds with a five-second timeout; monitoring load never borrows provider/quote capacity. Build from the repository root; copy committed Prometheus/rules/Alertmanager configuration into digest-based images rather than production host bind mounts. Restore/replacement rebuilds the exact selected Git SHA and preserves all named data/receipt volumes. |
| DM1544-8 | Alert rules cover missing scrapes/observations, serving availability and observed indexer lag/stall. Prove pending, firing, delivery and recovered states using a monitoring-only synthetic alert; do not cause production failures. Record agent ownership and the actual persistent local delivery destination. |
| DM1544-9 | Synthetic endpoint durations/statuses do not establish production traffic latency, 5xx rates or provider pressure. Document these instrumentation gaps and correlate bounded Loki queries. Monitoring success does not close unresolved [NR1505](AGENTS_NATIVE_QUOTE_RECOVERY.md) recovery/HTTP503 acceptance. |
| DM1544-10 | Consume only required inventoried credentials through openbao-tools `bao-exec`; never emit secret values, raw Coolify environment/config responses or raw logs. Record exact revisions, UTC, deployment status/log checks, replacement persistence, target-scoped PromQL and sanitized results before claiming live acceptance. |

Run `make verify-issue-1544`; keep each executed test process under 60 seconds.
Offline tests establish source behavior only. Check every live criterion in the
checklist before closing [#1544](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1544).

Credential recovery follows the [staging and collector follow-up](../docs/runbooks/dex-prometheus-monitoring.md#retrying-storage-staging-and-using-the-completed-collector-inventory).
Keep S3 candidates private and `staged_unverified` until actual HEAD200 and
CAS/readback promotion. `--resume` never prompts, renews no runtime credential,
and downgrades an older verified pending state before rechecking. Never adopt an
unverified candidate or interpret read verification as Loki write/adoption
success. Emit only fixed diagnostics; no raw exception/subprocess/HTTP bodies.
Collector environment names include digits (`CL8Y_*`); the earlier digit-blind
inventory is not absence evidence. Completed operator discovery does not grant
unavailable sudo or certify credential rotation.

For manual secret entry use the reviewed local `stage-logs-s3-operator.py`
launcher directly in the operator terminal. Its credential-bearing subprocess
retains `bao-exec --output=guarded` with all standard streams piped; new input
uses bounded private stdin JSON, never arguments, environment or files. The
launcher emits only validated closed reports. Do not disable the output guard
or run the terminal launcher in an agent transcript; agents use guarded,
noninteractive `--preflight`/`--resume`. A terminal guard rejection occurs before
the staging child launches and does not prove an S3 failure or credential change.

For S3 runtime adoption follow the [Loki-only transaction](../docs/runbooks/dex-prometheus-monitoring.md#adopting-staged-s3-credentials-without-restarting-the-logs-stack)
and [separate acceptance items](../docs/qa/issue-1544/README.md#credential-remediation-acceptance).
Never redeploy the full logs Compose application: Coolify stops every service
first. Only the exact Loki container may be replaced; preserve image, command,
WAL/data mounts, networks, security/resources and all noncredential environment,
with bounded verified rollback. Clone daemon-native Config/HostConfig through
the local Engine API; keep the original container until acceptance, and restore
that same ID on failure. Compose rendering is source attestation only. Journal
old/new values privately with CAS before
deployment; bind the unchanged candidate, source SHA and worker recipe hash.
Embed scripts from `git show` at that SHA. Use a separate Coolify Service with
raw Compose and `build.dockerfile_inline`, explicit mounts and `restart: "no"`;
Dockerfile Application custom run options cannot enforce this contract. Raw
Compose contains runtime environment references only, never private payload or
build arguments. Bind the exact Service/maintenance child and require closed
preflight/adoption receipts; an Application deployment status is not this
Service's proof. Only the maintenance Service may restart to enter adoption.

The one-shot socket-mounted worker has effective host-root authority. Use only
the reviewed owned job, read-only application directory and nonsecret state
volume; disable automatic retries, attach/detach only the existing application
network, and never deliver workstation identity, OpenBao or Coolify credentials
to it. No root SSH or ACL expansion. Unknown child/log fields remain private;
emit only closed reports. Ambiguous adoption requires status inspection.

Keep source checks, staged HEAD, independent candidate PUT/GET/DELETE, runtime
adoption, successful Loki storage writes, canonical Coolify credential readback,
maintenance secret/job removal, old-key revocation and ingestion rotation
distinct. Push/flush HTTP204 alone is insufficient; require successful chunk
storage/flush deltas without new flush failures, marker readback and preserved
historical reads. Do not claim uncached historical GET or exact-marker object
durability from those checks. Finalize only the two credential configuration
fields without stack deployment; remove the saved job payload and maintenance
Service while preserving shared volumes/networks. No source/staging/adoption
PASS closes the remaining revocation or ingestion acceptance.

Failed attempts retain their terminal receipt and latch. A verified read-only
recovery observation must establish current old-key matching, fresh readiness
and the fixed canonical source before `reset-preflight` can replace only the
maintenance recipe. Preserve any unavailable original-projection limitation.
Use a fresh nonce in both the state-volume key/name and image tag; stale or
wrong-nonce receipts cannot authorize adoption. A reset never deletes old
latches, changes candidate credentials or redeploys the logs application.
