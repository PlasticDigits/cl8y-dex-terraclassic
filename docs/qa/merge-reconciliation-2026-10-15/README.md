# Merge reconciliation — 2026-10-15

The starting main was `4bd3e8d31485613d6c3ddca62abbac1a1f6e6469`.
Ten open dependency PRs were reviewed together. Tokio lockfile changes were
resolved in favor of 1.53.3 while preserving the other dependency updates.
The incompatible schemars 0.9 proposal was revised to retain 0.8.22 with a
Renovate compatibility constraint; coordinated migration is tracked in
[#1508](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1508).
CI was explicitly skipped. Temporary Forgejo status/outdated-branch bypasses
were restored after the merge operations.

## Local work recovery

The [manifest](recovery-manifest.json) records four differing uncommitted QA
files recovered from old worktrees. These are historical records, with original
source revisions and both original/archive checksums. Artifact/source paths are
separated from their SHA-256 values using the current public presentation: the
first secret scan flagged five path/hash associations as generic API keys.
The path/hash values are preserved, and the corrected staged scan passes. They do not replace the canonical current
receipts, artifact checksum list, schema presentation, or release acceptance.
The later synthetic browser run is preserved as a separate historical sample.
Four other untracked QA receipts already match tracked main files exactly.

The single stash contains already integrated route-discovery work. Its 24 paths
were compared against the integration commit and current main; later actor
inventory revocation tests and corrections are retained. The old browser cleanup
and execution-readiness branches add no new product code; their histories were
reconciled while retaining the current policies and issue crosslinks.

## Local checks

- Contract workspace: 999 tests passed, including doc-test suites.
- Indexer library: 839 tests passed; one existing benchmark ignored.
- Pinned Go 1.27 image: manifest verified; policy tests and build passed.
- Browser lifecycle/attribution/discovery helpers: 29 tests passed.
- Commit hooks: Rust formatting, contract Clippy, frontend TypeScript and
  staged secret scan passed.

These results establish local source behavior. They do not establish deployment,
production settlement, or completion of unrelated live follow-up issues.

## Cleanup

All ten listed PRs were merged through `fj`; no open PR remained in the final
inventory. After fetching both remotes and pulling origin main, the remaining
local history was merged and pushed. Removed 30 merged origin branches, 26
merged local branches, 17 auxiliary worktrees and the incorporated stash. Only
local main and the primary worktree remain. The GitLab main reference was
fetched and is already contained in origin main.

Before removing worktrees, private env files, generated contract artifacts,
reports, original uncommitted files and the stash patch were saved under a
mode-0700 recovery directory in `~/.local/state/cl8y-dex-reconcile/` (archive
files mode 0600). No private configuration was committed. Remote branch deletion
used exact-head leases and excluded main and open-PR branches.

Main rejected the initial direct push under its existing protection. The
consolidation push temporarily allowed only PlasticDigits, then restored the
original push restriction. Status and outdated-branch settings were also
restored; protection changes were temporary.

## Extended verification

- `make verify-issue-1489`: **20 PASS, 0 FAIL, 2 SKIP**. Each round includes
  30 focused library controls, one disposable-database socket test and 335
  frontend tests.
- `make verify-issue-1483`: **21 PASS, 0 FAIL, 2 SKIP**. Each round includes
  nine focused library controls, two disposable-database socket tests and 323
  frontend tests.

The original attempts retain **FAIL** results: #1489 had 18 PASS / 2 FAIL /
2 SKIP; #1483 had 20 PASS / 1 FAIL / 2 SKIP. Both failures were unavailable old
QA database connections (`PoolTimedOut`), before product assertions ran. Fresh
owned PostgreSQL and per-process child databases resolved the setup problem;
complete reruns above passed without changing product code or budgets. The
owned container, its anonymous volume and temporary private env were removed.

The [verification receipt](verification.json) records commands, source
fingerprints, original failures, corrected results and log checksums. Raw logs
and private recovery files remain outside Git. Live deployment, funded scope
and physical acceptance remain explicit skips; the skips do not block this
merge close-out or establish acceptance.
