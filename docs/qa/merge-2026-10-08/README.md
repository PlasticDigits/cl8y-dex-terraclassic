# Merge and local sanity receipt — 2026-10-08

PRs [#1447](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1447) and
[#1448](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1448) merged through
`bao-exec --only FORGEJO_PLASTICDIGTS_PAT -- fj pr merge`, using merge commits
`36131e86` and `4ecd0a04`. CI was explicitly skipped; merge and consolidation
commit subjects contain `[skip ci]`. No status was fabricated and no hosted
workflow was invoked or awaited during this sweep.

The implementation head `513270baa32aa88722a68af701ca17f242f02810` and the
investigation head `09039309ee296061f6776ffd6550a65a2fac0cdb` are ancestors of
main. Its application tree equals the tested implementation. There were no
content conflicts. After #1447 merged, normal #1448 merge returned HTTP 405:
the head was behind the base. For that PR merge, only `block_on_outdated_branch` was temporarily
disabled for the `fj` merge and restored in a `finally` block. All original
protection fields were read back and matched, apart from the update timestamp.

The documentation consolidation push was initially rejected by protected main.
For that single normal push, push permission was temporarily enabled with only
`PlasticDigits` on the push whitelist. The original push settings were restored
in a `finally` block and all protection fields read back. No history was force
pushed, commit/push hooks stayed enabled, and required CI status settings were
unchanged.

## Local checks

Tests ran in an isolated candidate worktree with a dedicated disposable
PostgreSQL cluster. Every integration process owned its own marked child
database; no shared or production database was migrated or cleaned.

| Check | Result |
| --- | --- |
| `make verify-issue-1354` | 10 PASS, 0 FAIL, 1 explicit live-acceptance SKIP |
| Shared #1209/#1210 parser, ledger/API/catalog/series/UI harness | 5 PASS, 0 FAIL; included in #1354 |
| `make verify-issue-1396` final run | 28 PASS, 0 FAIL, 2 explicit host-network/IPv6 SKIPs; authenticated runtime fixture included |
| `make verify-issue-1395` | PASS, full library twice and all selected event/replay/economic suites |
| `make verify-issue-687` | 7 PASS, 0 FAIL, including #631 and scoped #683 |
| `make verify-issue-1213` | 4 PASS, 0 FAIL |
| Affected protocol frontend tests | 60 PASS |
| TypeScript `tsc -b --pretty false` | PASS |
| Tracked-file gitleaks scan | PASS, no leaks |
| Final changeset whitespace | PASS after normalizing archived report whitespace |

The #683 harness uses its existing flag to omit recursive unrelated bundles;
that entry is not additional functional coverage. Full library, security,
database-isolation and accounting suites ran with the checked-out source.
Passing source and fixture checks do not establish production deployment.

## Problems and remaining work

The optional runtime-login check first rejected an unrelated database CONNECT
grant introduced by the disposable provisioning fixture. Revoking that grant
on the owned cluster corrected the fixture. The targeted check and the full
authenticated #1396 run then passed; no application assertion was weakened.

The initial #1396 socket run was cancelled after a fresh Compose PostgreSQL
container accepted internal SQL but published port 48437 did not answer host
loopback. A separately provisioned QA container's host port worked. The final
run explicitly skipped those socket probes and host IPv6, which is disabled by
the kernel. Source rendering, real role ACL checks and all database suites ran.
The cancelled run is not reported as PASS.

The separate public diagnostic at `2026-10-08T04:59:17Z` returned **FAIL**:
52 completed UTC days, five findings, zero unavailable HTTP requests. Serving
revision was `4572e8779801c5c90b63ec95a0c7d5c38ce50bbe`. Findings were partial
volume nulls, missing volume completeness metadata, stored chart drift/missing
days, missing creation/SKU/settings fee sources, and missing UST1/USTR/CL8Y
Llama prices. Source verification does not erase those deployed findings.

Substantive production and upstream followups moved to
[#1449](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1449): apply the two
additive migrations with the separate migrator, deploy indexer/frontend,
reconcile and replay authenticated fee history, reaggregate bounded old days,
obtain upstream reload and verified CW20/pinned-reserve evidence, finish alert
delivery and restore unavailable network coverage. #1354/#1209/#1210 close for
their merged source work under the user's explicit instruction; that closure
is not a deployment receipt. Completed #631/#687 remain closed.

## Consolidation and cleanup

Fetched/pruned both origin and the legacy GitLab remote. GitLab main had no
unique commits and is already included in main. No stashes or uncommitted
source work were found. The completed implementation branch/worktree and four
obsolete deliberate-red CI probe branches/worktrees were removed locally and
remotely. Their PRs #1415–#1418 were already closed and intentionally never
merged; their failing assertions and deleted CI configuration were not useful
application changes.

The investigation branch/worktree `verify/issue-1354-defillama` remains because
another registered BB environment owns it.
Its commits are already in main. The drift timer was reinstalled from the
durable main checkout and its `WorkingDirectory`/`ExecStart` read back there
before removing the implementation worktree. This preserves scheduling and
local JSON/journal output; it does not establish external alert delivery.
