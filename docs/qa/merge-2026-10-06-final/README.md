# Final PR integration receipt — 2026-10-06

PRs #1424–#1427 are merged into `main` at `9f92681d`. Source implementation
issues #1396, #1397, #1398 and #1422 can close. Hosted CI was skipped by explicit
operator instruction: the status/outdated-branch merge requirements were relaxed
for this batch and restored after publication. No passing CI status was fabricated.
Production deployment is separate from this source and synthetic-chain receipt.

The integrated targets are factory **1.12.0**, pair **1.21.0**, router **1.4.0**,
community-tax token **1.3.0** and AutoLP **1.1.0**. Rollout defaults and checklists
now retain net execution floors, bounded hooks and the funded-query optimization.
Evidence indexes require the separate scoped migrator before server startup.

| Check | Final result |
| --- | --- |
| `make verify-issue-1396` | 28 PASS, 0 FAIL, 2 host-network SKIP; 564 library, 43 API-security, CLI, ACL and isolation checks repeated |
| `make verify-issue-1397` | 5 library, 15 API and 7 budget/recovery tests PASS twice |
| `make verify-issue-1398` | 7 PASS, 0 FAIL, 0 SKIP; 982 workspace tests and 168 focused frontend tests per pass |
| `make verify-issue-1422` source/client/API groups | 8 PASS, 0 FAIL; final chain and baseline acceptance completed independently below |
| Final funded 3M fixtures | All six scenarios and all 18 actual smart queries PASS twice; frozen old reverse failure REPRODUCED twice |
| `make verify-issue-1381` | 13 PASS, 0 FAIL, 2 SKIP: native wrap-stack and production execution |
| `make verify-issue-1390` | 15 PASS, 0 FAIL, 1 SKIP: production execution; real synthetic launch guards PASS |
| Rollout Python tests | 70 PASS twice |
| Frontend lint/types/build | PASS; five existing lint warnings; public synthetic build-only WalletConnect ID |
| Required changed-indexer formatting | PASS |
| Optimizer / checksums / normal commit hooks | PASS; all 14 artifact checksums match |

The fresh pinned optimizer built all 13 workspace artifacts. The separately
reviewed CW20 fixture dependency preserves its recorded checksum. The
[builder receipt](build-provenance.json) retains its actual source commit,
`79720f17`, and compiler/image/lock/source/output hashes. An exact temporary
snapshot of that commit passed the provenance verifier after copying its outputs;
every recorded source input still matches integrated `main`. That snapshot was
removed. This proves the recorded local build relationship, not independent
compiler trust or production deployment.

[First](funded-first.json) and [retest](funded-retest.json) receipts use a fresh
isolated SDK53 node at unchanged **3,000,000** smart-query gas. Both have all
18 successful smart queries and six successful accounting/rollback scenarios.
The multiple-tax reverse probe uses **2,139,981** gas in the first pass, including
transaction/wrapper overhead; probe gas is not the LCD meter reading. The
[frozen first](funded-first-baseline.json) and [retest](funded-retest-baseline.json)
artifacts match the previously recorded baseline hashes; their expected raw FAIL
is retained with `baseline_regression=REPRODUCED`.
[Funding](funding.json), [atomic invoice flows](funding-flows.json), and
[launch guards](guards.json) are additional synthetic execution evidence.

Merge problems were fixed and retested: retain all conflicted Make targets;
keep router 1.4.0 in the manifest/lockfile; reconcile fixture and rollout versions;
move evidence-index instructions to the separate migrator; isolate config fixtures
from checkout/ancestor dotenv credentials; wait for PostgreSQL's final TCP server
instead of its temporary initialization socket; rebuild and replace stale hashes.
Initial dotenv assertion failures, the startup readiness race, a missing frozen
artifact and a missing build-only WalletConnect ID are preserved in local logs;
corrected final checks pass. No assertion failure was relabeled as a skip.

The default bridge's published-loopback queries repeatedly timed out while
internal PostgreSQL TCP was healthy. Internal/external confinement and host-mode
IPv4 passed. Host IPv6 is disabled. Native wrap-stack configuration was unavailable
on the isolated fixture; direct funding/invoice/guard execution still passed.
Production role transition, schema/index migration, maximum-hook/AutoLP gas,
provider envelope and deployment acceptance remain in #1384, #1386 and #1421.
The existing scanner false positives for the public Venus address and broad legacy
indexer formatting drift are recorded in #1421; hosted CI and the whole-indexer
formatter do not receive invented PASS evidence. The required changed-source
formatter passes. Populated four-hop greedy support remains #1428; token-owned
legacy AutoLP manager recovery remains #1429.

Both remotes were fetched after the PR merges. Local `main` fast-forwarded to
`origin/main`; `gitlab/main` has no additional commits. No stashes were present.
All good source branches are contained in `main`. Original worker-owned branches
and 23 worktrees, including the historical build snapshot and four deliberate
CI-failure probes, are preserved as instructed. The probes deliberately remove
policy/assertions and are excluded from `main`. Only this batch's temporary
`integrate/1426` and `integrate/1427` branches are removed.

Public provenance uses explicit `path`/`sha256` arrays; restore those maps into
`metadata` to run the original verifier. Published transaction hashes use
lowercase hexadecimal; original raw receipts remain untouched in the local
evidence directory. These representations avoid secret-scanner false positives
for public hashes without weakening any scanner rule.

The [machine-readable summary](summary.json) records exact source scope,
results, failures corrected, remaining work and cleanup boundaries.
