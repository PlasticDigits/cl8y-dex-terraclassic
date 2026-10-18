# Live release evidence (2026-10-18)

Fix: [PR #1542](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1542),
backend `5b623a04f35bb6b44d3e64df0014e458cad653fc`.
The genuine complete source CI pipeline **526 passed**. The separately attested
frontend remains `2df4a471a9ca98a8672d911b67d4bd8134f411fd`.
The [machine-readable receipt](live-release-20261018.json) preserves the scope
of each observation; earlier failures remain in their original receipts.

- [x] Coolify deployment finished; its available deployment logs have zero
  configured failure-pattern matches and show new-container health before old
  container retirement. The source pin remains exact and automatic deployment
  stays disabled.
- [x] A 55-second handoff window observed 55 HTTP 200 health responses, zero
  failures, 28 old-revision and 27 new-revision samples. This samples the overlap;
  deployment logs separately establish completion. It is not a continuous-time
  uptime guarantee or native quote acceptance.
- [x] The new server served reviewed schema 67 with dynamic useful readiness true,
  both configured native scenarios, and explicit `pending_schema` cohort status.
  It omitted unavailable cohort rows rather than fabricating idle values.
- [x] After independent warmup, the 21-second native probe made 10 requests,
  including eight executable selected quotes. The seven initialized useful
  results had p95/max 300.023ms. The separate cold result was 889.546ms. This is
  bounded acceptance only; it does not replace the original sustained windows.
- [x] Bounded Grafana/Loki reads succeeded. The 15-minute service window spans
  old/new deployment activity and retains provider pressure, deadline and error
  pattern counts. Counts cannot establish causality or an error-free runtime.
- [ ] Prometheus: **SKIP**, no Prometheus datasource is provisioned in Grafana.

The online database and final API receipts are recorded below only after their
separate execution. No API stop/restart or automatic database restore is part of
this release sequence. Maintenance cleanup can stop only its own job and remove
that job's shown-once credential.

Issue-specific checklists and agent contracts remain linked from the
[coordinated release index](README.md). Full #1505 sustained/callback/browser
acceptance, #1229 governed chain migrations, #1330 third-party execution/fee
settlement and the remaining #1512 matrix are not certified by these observations.

## Online database operation

- [x] Fresh capacity observation on the exact snapshot mount passed the documented
  headroom/inode policy; its read-only probe credential was removed.
- [x] Read-only original 67 preflight passed before DDL. The separate online job
  applied only migration `20261018110000`, reaching 68 with complete original
  history, semantic ACLs and roles/credentials/memberships unchanged.
- [x] New-table runtime grants and four actual denial checks passed. The
  immutable 68,458,132-byte concurrent MVCC snapshot is private and retained.
  Automatic database restoration is forbidden.
- [x] Migration job terminal receipt passed and its shown-once credential was
  removed with readback. The API never received that credential.
- [x] The operation health window observed 55 new-revision HTTP 200 responses and
  no failures. Its exact window and snapshot timestamp are in the receipt; this
  remains sampled health rather than a continuous-time guarantee.

A fresh login using the runtime password was not tested inside the privileged
maintenance job (`runtime_login_tested=false`). The actual serving API already
uses that runtime login; no role/password/membership transition occurred.

## Final public API

[The 14-request smoke](live-api-20261018.json) passed in **13.347s**, under its
59-second outer limit. It bound the exact backend SHA before/after; checked the
anonymous UTC digest, incomplete today, rejected queries and method, canonical
ten-source fee mix, bounded token rows and OpenAPI. All three cohort windows
reported `available`, with exactly retail/MM/unknown aggregate fields, equal
source/cohort event counts, equal priced USD totals and 100% priced shares.

The observed historical windows are all `unknown` (208/1,257/3,837 events for
24h/7d/30d). This is honest missing historical membership, not proof that all
production actors are unregistered or that live MM activity was observed. The
actual retail→MM transition is established by the separately retained owned
LocalTerra ingestion/API receipt. No speculative mainnet backfill or funded
mainnet transaction was performed.

The matching apply receipt passed before cleanup. After stopping only that
maintenance job, its logs endpoint returned 400, so the terminal log timestamp
is unavailable. The snapshot UTC falls inside the 55-second health window; no
exact full DDL interval or continuous-time uptime claim is made. The worker
separately verified useful exact-revision serving immediately before DDL and
after migration.

- [x] A separate verify action passed 68/history/roles/ACLs/grants/denials and
  reused the same original snapshot. Its credential cleanup also passed.
- [x] Two final cached-health reads returned 200: indexer running/caught up to
  its observed head, registry configured/healthy with zero consecutive failures.
  These observations do not attribute earlier 503 responses.

## Retained runtime uncertainty

An exact 09:07:29–09:12:29 UTC error sample identifies five incoming API-service
HTTP 503 responses from `tower_http::trace::on_failure`. The lines have only
classification/latency, without a route or quote correlation; they cannot be
assigned to user traffic versus readiness probes. A sixth line reports the
existing background fee-registry config probe failing. The later healthy cached
probe does not erase those observations. #1505 remains open for request-purpose
triage, retained earlier failures and sustained acceptance. No error-free runtime
or permanent availability claim is made.
