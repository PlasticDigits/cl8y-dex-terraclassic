# Issue #1400 verification receipt

Date: 2026-10-05 UTC. Base revision:
`9944ada10ee711ff8ffaa91f4c19b0f292de3924`. Scope: source implementation, synthetic regressions and
explicitly identified local/runtime evidence. Independent agent review completed;
this is engineering verification, not an external audit certification.

Canonical [checklist and skip policy](README.md),
[CI1400 invariants](../../../skills/AGENTS_FORGEJO_FUNCTIONAL_CI.md),
[issue/comments](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1400).

| Check | Command / evidence | Result and scope |
|---|---|---|
| Functional runner regressions | `python3 scripts/ci/functional-test.py`, repeated | PASS: failure, missing/stale/symlink receipts, fresh targets and synthetic cache contamination. Engine exit/status is authoritative; receipts are bookkeeping. |
| Build input regressions | `python3 scripts/ci/test-build-inputs.py`, repeated | PASS: poisoned binary replacement, fresh dual optimizer overlays/cleanup, source/lock/recipe/image/output tampering, frozen fixture Git selection and graph-preserving lock conversion. |
| Immutable image declarations/manifests | `python3 scripts/ci/verify-image-pins.py --registry` | PASS: 13 reviewed manifest references, including checkout plugin; original Alpine and gitleaks download checksum preserved. |
| Signed config policy | `go test -mod=readonly -v -count=2 ./...` in `scripts/ci/trusted-policy` | PASS on checksum-verified Go 1.26.8: signed requests, wrong repository, forwarded credentials, unsigned requests, deleted/empty/skipped/malicious contributor config, reviewed asset embedding and escaped shell variables. Local test execution used RAM-backed temporary compiler state to avoid host I/O saturation. |
| Discovery cache / indexer library | `cargo test --locked --lib -- --test-threads=16`; focused `indexer::pair_discovery::tests` | PASS: 470 library tests three times; 8 discovery tests 20 times. Fresh changed-source compilation with Rust 1.99; independent LCD/factory keys, clone reuse, no global test clears. |
| Contract workspace | `cargo test --locked --workspace --jobs 2`, immediate retest | PASS: 884 tests twice; no failed/ignored tests. Rust 1.99, local trusted dependency inputs with isolated recovered test target. This is source verification, not an exact Rust 1.96 container receipt. |
| Frontend first source run | `npm ci`; lint; full Vitest (2 workers); cold production build; no `.map` outputs | PASS: 352 files / 3,068 tests, zero lint errors (5 existing warnings), tsc/Vite build and no sourcemaps. |
| Frontend retest and correction | Full retest, then affected selector/Swap tests twice | Initial retest FAIL: one slow quote-start assertion and uncanceled blur work after jsdom teardown. Added timer lifecycle cleanup/regression and a bounded 5-second startup wait preserving funding assertions. Targeted 63 tests PASS twice; corrected full run PASS: 352 files / 3,069 tests, no unhandled errors, lint, cold tsc/Vite build and no sourcemaps. |
| Isolated Postgres route-tax reference subset | `api_route_tax_funding1381` binary on dedicated synthetic PostgreSQL 16.14 and exact pinned CI PostgreSQL 16.15 | PASS: 6 tests twice on 16.14 and once on 16.15. Fixture source matches; this was a precompiled reference binary, not fresh changed-source integration compilation. |
| Fresh four-binary Postgres subset | `cargo test --locked --test security --test api_hooks --test api_route_tax_funding1381 --test indexer_ingestion_hardening -- --test-threads=1` | PASS: freshly compiled changed-source binaries, 63 tests (42 security, 4 hooks, 6 route-tax, 11 ingestion) twice on the exact pinned PostgreSQL 16.15 service. No shared/production database used. |
| Optimized wasm | Fresh pinned optimizer, read-only source, isolated named volumes | First container compiled/optimized 13 contracts, but outer wrapper FAIL after its file changed during execution. Final immutable wrapper rerun PASS/exit 0: source/lock/image/output receipt independently verified; all 13 hashes match the first cold compiler outputs; container and volumes removed. No first-run outer PASS claimed. |
| Native release | Fresh empty target, Rust 1.99, 2 jobs | FAIL/resource bound: first run stopped after about 45 minutes while still compiling dependencies; no accepted release binary. Native artifact/provenance and native independent rebuild comparison are unverified. Cleanup passed. |
| Verification command | `make verify-issue-1400`, including finalized policy image | PASS: 9 PASS, 0 FAIL, 0 SKIP. Runner/supply tests repeated, immutable declarations, shell/diff checks, and signed policy Docker build/tests all passed. Earlier pending-container skip is superseded by this completed check. This command does not attest hosted or scheduled acceptance. |
| Live Forgejo protection | Read `/api/v1/repos/code/cl8y-dex-terraclassic/branch_protections` | PASS/read-only: main requires `ci/woodpecker/pr/woodpecker`, blocks outdated branches; no required context removed and no status fabricated. |
| Woodpecker settings | Repo 7 readback and bounded timeout update | PASS: timeout 120 minutes, fork approvals preserved, no volume/security trust escalation. Exclusive extension activation and actual policy deployment still pending. |

## Issue acceptance accounting

- [ ] **AC1 — deliberate security assertion failures block merge through the
  actual configured context:** policy and local negative tests implemented;
  genuine red/green draft PR and live exclusive-extension receipts remain
  unverified. Local PASS cannot establish this criterion.
- [x] **AC2 — executable cache trust isolation:** Forgejo/GitLab use fresh
  per-job executable state with no shared caches; identical lockfiles stay
  isolated. Synthetic poisoning/reuse tests pass. Production runner mount and
  image/cache configuration still requires its independent live readback.
- [x] **AC3 — immutable reviewed optimizer/build images and source/toolchain
  bytecode receipts:** implementation and registry/synthetic verification
  complete. Final cold-build receipt verified; all 13 wasm hashes match the first
  independent cold compiler run (whose outer wrapper failure remains recorded); no deployed artifact acceptance claimed.
- [x] **AC4 — explicit critical LocalTerra signing/rollback policy:** documented
  daily 03:00 UTC and pre-release, bounded 45 minutes, one browser worker, no
  retry/skip success, maintaining-agent failure ownership. Scheduler installation
  and actual critical chain/browser execution remain unverified. No claim that
  the existing full browser specification inventory runs in CI.

## Scope and unavailable evidence

Overall **implementation verification may PASS with explicitly recorded skips**
under [the agent skip policy](README.md#skip-policy). The attempted resource
timeouts and initial failed retest/wrapper remain recorded as failures; only
corrected checks that actually pass receive PASS. Release reproducibility,
hosted merge enforcement and scheduled/production execution are separate
acceptance scopes and are not inferred from source checks.

Pending evidence must be updated with actual results before claiming completion:
trusted service immutable deployment/settings/hash; real CI-created red/green
PR statuses; fully recorded native release promotion evidence; dedicated current-source LocalTerra
signing/rollback runs and scheduler receipts. The issue stays open for evidence
outside this implementation scope.

Base-image digests and locked Cargo/npm inputs do not freeze apt/apk package
repositories or all third-party download servers. Legacy production-only
`scripts/build-cw20-base-artifact.sh` is outside the newly isolated local fixture
build path; do not claim whole-repository release reproducibility. Frozen CW20
and USTR fixture revisions are selected from the DEX Cargo.lock; format-4 to
format-3 fixture lock conversion changes only the format marker and verifies
that every parsed package/version/revision/checksum remains identical.

GitLab QA publication source rules and synthetic branch/event evaluation PASS: both jobs refuse feature/unprotected refs, including manual web triggers; protected default-branch web remains eligible. Live GitLab protection/readback remains unverified.

Tracked-file gitleaks PASS. Three generic-key false positives were artifact SHA256 values labeled with token contract filenames; the public comparison now separates file names from sha256 fields. No scanner allowlist or secret-scan bypass was added. Four critical browser cases are confirmed by Playwright --list; collection is not execution.
