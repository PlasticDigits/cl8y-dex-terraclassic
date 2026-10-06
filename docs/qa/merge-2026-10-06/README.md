# Merge sanity receipt — 2026-10-06 UTC

All 12 open PRs were merged through `fj`: #1405–#1414, #1419 and #1420.
Local main pulled the integrated remote history. Connected implementation issues
are accepted in source scope; production/release work is tracked in
[#1421](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1421) and existing
#1384/#1385/#1386. The observed query-availability failure is actual code work in
[#1422](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1422).

Tested source snapshot: `02baafe326a9238a46c3c565256bffbcf830eba6`. Every tracked source file matched the
isolated test worktree byte-for-byte; its older artifact manifest was the sole
difference. [Machine-readable receipt](summary.json) records the suites and build
identity. Tests used a dedicated temporary PostgreSQL database and an independent
LocalTerra volume; no production database or wallet was used.

| Check | Result |
|---|---|
| Full contract workspace | 944 passed, zero failed/ignored |
| Strict workspace Clippy, all targets | PASS |
| Indexer library plus 28 PostgreSQL/API suites | 858 passed; broad-suite manual benchmark subsequently executed and passed |
| Frontend units | 353 files, 3,093 passed |
| Frontend lint and production build | Zero lint errors, five existing warnings; build PASS |
| `make verify-issue-1381` | 13 PASS, 0 FAIL, 2 explicit skips |
| `make verify-issue-1393` | 9 PASS, 0 FAIL, 0 SKIP; two optimized debt-market rehearsals |
| `make verify-issue-1394` | 23 PASS, 0 FAIL, 0 SKIP; default/high-thread repeats and benchmark |
| `make verify-issue-1395` | PASS; library and integrity DB retests plus related ingestion/accounting/recovery |
| Route rollout, chain attestation, oracle/financial guards, debt deployment scripts | PASS synthetic checks and documentation crosslinks |
| Functional runner/build-input tests and immutable declarations | 10 runner + 8 supply tests; 13 image declarations PASS; hosted CI skipped |
| Pinned optimizer | 13 owned contracts built, input/provenance/output verification PASS |
| Isolated funded routes and invoices | Six funding/rollback and two invoice checks PASS, including verifier retest |
| Isolated launch guards | 16 checks PASS, including repeated-capable cycles, owned refunds, purchased credit caps and atomic cooldown rollback |

Integration failures were fixed and retested: overlapping documentation/Makefile
entries; reverse/trust/guard fixture interfaces; valid reversed hop order;
missing settlement slots/refund fields; a fixture without the mandatory route
nonce; signer aliases and sink identity; API fixtures lacking chain attestation;
freeze-state restoration after assertions; replay mocks losing identity on reset;
and quote-cache assertions that must include bounded fresh identity probes.
Admission now bounds chain-identity HTTP work before uncached discovery and still
re-attests cached negative evidence. The relevant source/API/chain checks pass.

**Known failure retained:** the first funded reverse quote exceeded LocalTerra's
default 3M smart-query budget (`gasUsed=3,043,317`). A finite 15M query budget on
only the isolated test node enabled the funded fixtures. That result does not
establish compatibility with production providers; #1422 owns the repair. Runtime
fail-closed protections were retained. Native wrap/unwrap was skipped because no
existing public wrap-stack environment was supplied. Production signing, migration
and deployment were skipped and remain concrete rollout work.

The native build receipt records source `3d33ab43bd10e2e0b71e6c740cf9e6d48cd20ecc` with
`source_dirty=false`, Rust/Cargo 1.81.0 and
Binaryen 116. Later contract changes only corrected `cfg(test)` fixtures; deployed
contract code did not change. Release operations must retain/reproduce the exact
reviewed provenance. The separately reused synthetic CW20 fixture was checked
against its existing approved SHA256 and was not claimed as one of the 13 owned
contract builds. The tracked checksum manifest now reflects the actual integrated
outputs. `base64ct`'s 1.8.3 workspace declaration is unused directly; the effective
transitive lock remains 1.7.3 under the pinned optimizer.

The operator explicitly requested skipping CI and overriding merge gates. Only
the status/outdated-head requirements were temporarily disabled; their original
values were restored immediately after the PR batch. No fake status was posted.
OpenBao's private route was unavailable initially; existing authenticated `fj`
access was used until the route returned, after which remote operations ran under
`bao-exec`. Ordinary commit/push hooks and human Git identity were retained.

Every substantive local/remote branch tip was included in main. The four deliberate
negative CI probes were excluded. Worker-owned branches/worktrees were preserved.
Nine temporary integration branches and two merged Renovate branches were removed;
two records pointing at missing audit directories were pruned. There were no
stashes to recover. Temporary test worktree, database and chain resources are
removed after acceptance. No verification-only hold remains on implementation
issues; source closure does not imply deployment.
