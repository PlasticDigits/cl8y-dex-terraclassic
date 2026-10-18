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
| DM1544-7 | Prometheus retains at most seven days/2GB on persistent storage, with 512MB memory/0.5 CPU. Exporter uses 256MB/0.25 CPU. Scrapes run every 30 seconds with a five-second timeout; monitoring load never borrows provider/quote capacity. |
| DM1544-8 | Alert rules cover missing scrapes/observations, serving availability and observed indexer lag/stall. Prove pending, firing, delivery and recovered states using a monitoring-only synthetic alert; do not cause production failures. Record agent ownership and the actual persistent local delivery destination. |
| DM1544-9 | Synthetic endpoint durations/statuses do not establish production traffic latency, 5xx rates or provider pressure. Document these instrumentation gaps and correlate bounded Loki queries. Monitoring success does not close unresolved [NR1505](AGENTS_NATIVE_QUOTE_RECOVERY.md) recovery/HTTP503 acceptance. |
| DM1544-10 | Consume only required inventoried credentials through openbao-tools `bao-exec`; never emit secret values, raw Coolify environment/config responses or raw logs. Record exact revisions, UTC, deployment status/log checks, replacement persistence, target-scoped PromQL and sanitized results before claiming live acceptance. |

Run `make verify-issue-1544`; keep each executed test process under 60 seconds.
Offline tests establish source behavior only. Check every live criterion in the
checklist before closing [#1544](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1544).
