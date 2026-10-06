# Local implementation receipt — #1396

Recorded 2026-10-06 04:52 UTC. Base revision:
`6612df5704abcdc129aa7492113ed0b3b1641b4c`; tested dirty worktree branch
`fix/issue-1396-db-isolation`. The implementing PR carries the reviewed code and
this receipt. Only synthetic PostgreSQL identities/data were used; no shared or
production DB was reset, migrated, seeded or repaired.

Read issue #1396 and its 2026-10-05 comment with
`bao-exec --only FORGEJO_PLASTICDIGTS_PAT -- fj issue view 1396` and the same command
with `comments`. The comment confirms the earlier audit was not remediation.

## Commands and final outcomes

Privately inject the dedicated fixture's TEST_DATABASE_ADMIN_URL and
TEST_DATABASE_RUNTIME_PASSWORD (no values in this receipt), then run:

```sh
VERIFY_ISSUE_1396_SKIP_BRIDGE_HOST_REASON='Published loopback TCP on a custom bridge times out on this host while PostgreSQL is healthy; reproduced live; internal and host-opt-in sockets checked separately' make verify-issue-1396
make verify-issue-1400
bash scripts/lint-indexer-log-secrets.sh
git diff --check
```

`make verify-issue-1396`: **28 PASS, 0 FAIL, 2 SKIP — PASS WITH SKIPS**.
Build once, then run/retest the same executables:

| Suite | First | Retest |
|---|---:|---:|
| Library, including the normally ignored cache benchmark | 561 PASS | 561 PASS |
| Actual CLI no-contact seed/server checks | 4 PASS | 4 PASS |
| Fresh database isolation/ACL/marker/actual migrate/check/seed/clean | 6 PASS | 6 PASS |
| API security | 43 PASS | 43 PASS |
| API health | 10 PASS | 10 PASS |
| API aggregator | 6 PASS | 6 PASS |
| API newest-N candles | 12 PASS | 12 PASS |

Both passes also verified actual launcher credential scoping, 11 functional CI
source tests, fresh real PostgreSQL role/attack ACLs, networking config and available
socket probes. Shell syntax, whitespace and invariant/checklist cross-links passed.

`make verify-issue-1400`: **9 PASS, 0 FAIL, 0 SKIP**, including the signed policy
image build/tests and repeated functional/build-input/image-pin checks. All three
hosted workflow YAML files parsed successfully. Secret-pattern log lint passed.
Independent implementation review and final artifact review by another agent passed.

## Completed issue criteria

- [x] Runtime DML/sequence access works; role/database/schema/table/ALTER/TRUNCATE,
  role escalation, extension/program/file access, migration history, marker writes
  and unrelated schema/database access are denied by real ACL tests. Separate
  migrator login/DDL and QA ownership/CREATEDB boundaries work.
- [x] Default Compose retains loopback publication and repo commands explicitly
  select the base file. The named host opt-in is loopback-only. Internal TCP and
  bridge-peer access work; external namespace host access is denied. Opt-in IPv4
  loopback works. Missing/default bootstrap credentials are rejected.
- [x] Production/unknown/missing-mode seed and clean reject without contacting a
  real TCP listener. Help/errors are configuration-free. Authenticated isolated QA
  migrate/check/seed/clean succeeds; removing the DB marker rejects seed/clean
  without changing the seven synthetic rows. Seed itself never migrates.
- [x] Concurrent child processes own different fresh DBs. Parent data and control
  history remain unchanged; cleanup rejects the control pool. Strict migration
  histories, worktree credential separation and API regressions pass.
- [x] Invariants, implementation map, staged transition/rollback, local developer
  docs and agent checklist/skip policy are cross-linked.

## Explicit unavailable checks and scope

- [ ] Default bridge **published-host loopback positive query**: live queries timed
  out on this host while the PostgreSQL service was healthy and internal TCP worked.
  This is the known host publication/proxy infrastructure class; it is not a PASS.
  Internal/default-bridge confinement and the explicit host-mode workaround were
  still tested. Recheck published-loopback behavior on a functioning bridge host.
- [ ] Host **IPv6 positive socket query**: both host kernel IPv6 disable flags were
  `1`. The configuration names `::1` explicitly, but no IPv6 success is claimed.
  IPv4 loopback and external namespace rejection passed. Recheck on an IPv6-enabled
  isolated host; global host networking was not changed for this test.
- [ ] Production role/credential transition, rollout, actual live grants and exposure:
  not executed by this source implementation PR; use the staged runbook and collect
  separate receipts against the deployment. Local checks do not attest production.
- [ ] Hosted trusted Woodpecker policy deployment/PR acceptance: the updated policy
  image was built/tested locally, but no production policy resource was deployed.
  Deploy the reviewed immutable policy bundle from the integrated revision and
  verify its hash and genuine hosted logs. PR YAML alone does not replace it.

Initial failures were retained and resolved rather than counted as PASS: incomplete
startup timeouts, missing Compose command for the validation entrypoint, temporary
stale build state after a manifest change, SQLx's lossy helper URL adding rejected
query parameters, and exhaustion of the original 512 MB disposable tmpfs. The final
fixture used 4 GB with bounded WAL and passed both complete runs. An intermediate
verification run was interrupted while its script was being edited; the final
stable script was rebuilt and rerun. Unavailable platform probes remain unchecked
above. No assertion or ACL failures were hidden in the final result.

Follow the [agent checklist and explicit skip policy](README.md) and
[DB1396 invariants](../../../skills/AGENTS_INDEXER_DATABASE_ISOLATION.md).
