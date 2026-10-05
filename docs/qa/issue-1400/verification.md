# Issue #1400 verification receipt

Date: 2026-10-05 UTC. Implemented and hosted-tested revision:
`9ef0548a502c61ecf44f7e4a6c3441825b5f74c9`; base
`9944ada10ee711ff8ffaa91f4c19b0f292de3924`.
Independent agent review completed. This is engineering verification, not an
external audit certification. [Checklist / skip policy](README.md),
[CI1400 invariants](../../../skills/AGENTS_FORGEJO_FUNCTIONAL_CI.md),
[issue/comments](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1400).

## Completed checks

| Check | Outcome / evidence scope |
|---|---|
| `make verify-issue-1400` | PASS: 9 checks, zero failures/skips. Includes repeated runner/supply tests, immutable declarations, shell/diff checks, signed policy Docker build/tests. |
| Runner regression tests | 10 tests twice: deleted/symlink policy mirrors, command failures, missing/stale/symlink receipts, fresh targets, synthetic poisoning and npm-script alias bypass. Receipts are bookkeeping; trusted engine exit/status is authoritative. |
| Build input tests / image registry | 8 tests repeated; 13 manifest references verified against registry bytes. Original Alpine and gitleaks download checksum preserved. |
| Signed policy | Go tests twice and Docker build/tests PASS. Wrong repository, forwarded credentials, unsigned calls, malicious/empty contributor config, reviewed asset embedding and escaped shell variables. Runtime health/hash and unsigned 401 verified. |
| Indexer library / cache flake | Fresh changed-source compilation: 470 tests three times with 16 threads, 8 discovery tests 20 parallel repeats. Independent client/factory keys, clone reuse, no global test clears. |
| Contracts | 884 tests twice, zero failures/ignored tests; local Rust 1.99. Hosted pinned Rust 1.96.1 contract job also PASS. |
| Frontend | 352 files / 3,069 tests, lint zero errors (5 existing warnings), cold tsc/Vite build and no sourcemaps. Affected 63 tests twice; post-hook 42 tests PASS. Hosted frontend job also PASS. |
| PostgreSQL | Fresh isolated compilation: 63 tests twice on exact pinned PostgreSQL 16.15 (42 security, 4 hooks, 6 route tax, 11 ingestion). Hosted isolated subset also PASS. No shared/production DB used. |
| Optimized wasm | Final immutable wrapper PASS/exit 0, source/lock/image/output receipt verified, all 13 hashes match independent first cold compiler outputs. Container and volumes removed. [Comparison](cold-wasm-comparison.json). |
| Publication / secrets | Synthetic branch/event evaluation: both GitLab publication jobs refuse feature/unprotected refs, including web triggers. Tracked/staged gitleaks and commit hooks PASS. No scanner allowlist/bypass added. |

## Genuine hosted enforcement

The policy service is separately deployed from exact commit `9ef0548a...`,
nonroot with 128 MiB / one CPU limits and automatic/preview deployments disabled.
Live policy SHA256 is
`c8334c5e7f1c3499782b3f328df6b2c9b6bc445c252e4fc5534f23c6eee9f85a`.
Woodpecker repo 7 uses exclusive signed configuration and `netrc=false`, retains
fork approvals, timeout 120 minutes, volumes/security trust disabled. Forgejo
main protection still requires `ci/woodpecker/pr/woodpecker` and blocks outdated
branches. No context was removed, no forced merge attempted, no fake status posted.

| Genuine pipeline | Result |
|---|---|
| [307](https://ci.cl8y.com/repos/7/pipeline/307/1), implementation #1414 | All six required steps actually ran and passed; exact tested commit has genuine successful required context. [Receipt](live-green.json). |
| [306](https://ci.cl8y.com/repos/7/pipeline/306/1), config deletion #1415 | Trusted initializer rejects missing workflow/helper mirrors despite deleted PR YAML. Required context failure. [Receipt](live-config-deletion.json). |
| [308](https://ci.cl8y.com/repos/7/pipeline/308/1), contract assertion #1416 | Deliberate tax expectation 10001 vs actual 10000 fails `maximal_offer_and_extreme_conservation` (67 pass / 1 fail in dex-common), required context failure. [Receipt](live-contract-red.json). |
| [309](https://ci.cl8y.com/repos/7/pipeline/309/1), frontend assertion #1417 | Deliberate gross-input expectation 1010001 vs actual 1010000 fails router funding regression (3068 pass / 1 fail), required context failure. [Receipt](live-frontend-red.json). |
| [310](https://ci.cl8y.com/repos/7/pipeline/310/1), indexer assertion #1418 | Deliberate clone-cache count expectation 2 vs actual 1 fails discovery regression (469 pass / 1 fail), required context failure. [Receipt](live-indexer-red.json). |

Each negative pipeline also fails completeness; skipped downstream work cannot
make it green. Synthetic drafts #1415–#1418 were closed without merging.
Pipeline result and branch requirement readback establish the enforcement scope;
we did not attempt to merge deliberately broken source into main.

## Issue criteria

- [x] **AC1:** Actual contract/frontend/indexer assertion failures produce the
  failed configured required context; genuine green, missing-config and
  deliberate-red receipts recorded.
- [x] **AC2:** Executable cache trust isolation, including identical lockfiles,
  implemented and contamination/reuse regressions verified. No shared caches
  or executable targets restored; protected-default publication enforced.
- [x] **AC3:** Reviewed immutable image inputs and source/lock/toolchain/output
  receipts implemented; optimized bytecode receipt and independent cold output
  comparison verified. Native release reproduction is a separate incomplete scope.
- [x] **AC4:** Explicit critical LocalTerra signing/funding/rollback policy:
  daily 03:00 UTC and before release, 45-minute bound, one worker, no retry/skip
  success, maintaining-agent failure ownership. Four selected browser cases
  confirmed by Playwright collection; scheduler/signing execution not claimed.

## Failures, skips and limits

Initial hosted pipelines 303/305 failed because the minimal pinned Rust image
lacked cargo-fmt. Corrected policy installs the matching checksum-verified
component; pipeline 307 passed. An initial frontend retest exposed a slow quote
startup assertion and an uncanceled blur callback after unmount; exact funding
assertions are preserved with bounded startup wait and timer cleanup regression.
Corrected full-suite and hosted verification pass. An initial optimizer outer
wrapper failed after its script changed during execution; its compiler output
comparison is separate from the finalized immutable wrapper PASS.

Native cold release build exceeded its resource bound (exit 143 after 2749s),
cleaned its temporary target, and accepted no artifact: **FAIL/resource timeout**.
Native independent reproduction, dedicated current-source LocalTerra signing/
rollback execution, scheduler installation, actual external fixture cold builds,
live GitLab protection readback and production rollout remain **unverified**.
These may be recorded **SKIP** under the explicit agent policy while overall
available implementation verification remains PASS. An executed failure remains
FAIL; unit/source/readonly receipts never establish deployment acceptance.

Base-image digests/locked Cargo/npm inputs do not freeze apt/apk repositories or
all third-party downloads. Legacy production-only `build-cw20-base-artifact.sh`
is outside this fixture scope. Frozen CW20/USTR fixture commits come from DEX
Cargo.lock; deterministic format-4 to format-3 conversion changes only the marker
and verifies every package/version/revision/checksum, recording both lock hashes.
Python 3.11+ required. Public comparison records separate filename/sha256 fields
so artifact digests cannot be mistaken for credentials; no gitleaks weakening.
