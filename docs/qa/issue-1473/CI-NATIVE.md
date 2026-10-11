# Trusted CI native canvas fallback

[PR #1476](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1476) includes the
phone viewport setup correction and this required CI installation repair.
The application remains deployed at
`ea627f7b0e8d5ebc9fde2bb5b243f451cf159f4d`; policy deployment and QA tool
revisions are recorded separately. No frontend, indexer or contract tree changes.

The [corrected 96-sample live reproduction](live-browser-setup-corrected-summary.json)
uses the fixed viewport helper against the same application. Its two latency
failures remain FAIL; zero visibility failures in that bounded trial do not
replace the capped full trial or establish complete acceptance.

[Hosted trials 413 and 414](ci-native-first-failures.json) both remain FAIL.
The first canvas 3.1.0 prebuilt installation found no binary; the one normal
same-head retry timed out. Both fell back to node-gyp and failed because
`pangocairo.pc` was absent. Frontend lint, unit tests and build were never reached;
indexer jobs were skipped and completeness failed. Existing application CI 410
is evidence for its tested revision, not PASS for these failed trials.

The trusted runner installs `build-essential`, `pkg-config`, `python3`,
`libcairo2-dev`, `libpango1.0-dev`, `libjpeg-dev`, `libgif-dev`, `librsvg2-dev` and `fonts-dejavu-core`
before ordinary `npm ci --no-audit --no-fund`, following the
[canvas native build requirements](https://github.com/Automattic/node-canvas/wiki/Installation%3A-Ubuntu-and-other-Debian-based-systems).
Node and other image declarations,
the 25-minute frontend bound, fresh per-job caches, direct six-job checks,
signature verification, exclusive extension and `netrc=false` are unchanged.
Installation failures remain nonzero and cannot write a successful receipt.

After ordinary npm ci, the trusted worker removes only `node_modules/canvas/build`
and forces `npm rebuild canvas --no-audit --no-fund --foreground-scripts` with
`npm_config_build_from_source=true` and the supported
`npm_package_config_node_gyp_jobs=2` limit. It requires a nonempty fresh native
binding and compiled objects, then checks exact RGBA pixels, positive DejaVu Sans
text metrics and rendered text alpha. Compiler logs and the render marker must
appear in the actual worker result; a prebuilt-only load cannot satisfy this
proof. Compile, missing output or render failures remain failed frontend jobs.
No lint/Vitest/TypeScript/Vite step is removed or replaced.

The current hermetic runner suite passes 15 tests twice, including dependency ordering,
failed prerequisite update/install blocking npm, clearing a previous successful
receipt, and failed dependency installation refusing success. Its apt-get/npm
stubs prevent host package mutations. Nine build contamination/provenance tests
also pass. The corrected issue-1400 verifier passes all nine checks without skips; signed
Go tests execute twice in the pinned builder and the policy image builds.
The [local pinned native fixture](canvas-native-local-unavailable.json) is an
explicit SKIP: its bounded exact-image pull made progress but never supplied a
usable image, so no container, native compilation or rendering ran. Actual
forced worker compile/load/render proof and policy rollout remain pending;
no successful rollout or hosted result is inferred.

Initial issue-1400 verification exposed five pre-existing stale pin-ledger
entries. Official registry manifest bytes verified all 13 currently declared
images; only the five ledger values were reconciled to those existing digests.
No image declaration or manifest changed. The original failed verification
remains recorded separately from the corrected retest.

The first local nonroot runtime trial failed because Docker inherited a mode
`0600` public signing-key source file. The Dockerfile now sets that public file
to `0444` explicitly, without changing its bytes or signature verification.
The deliberately mode-0600 source build passes actual nonroot health (HTTP 200)
with the verified bundle hash and rejects unsigned requests (HTTP 401). The [implementation receipt](ci-native-implementation-summary.json)
keeps the earlier verification and runtime failures separate.

A reviewed exact policy commit must be deployed independently before new
mandatory checks can use this runner. Keep the old policy revision for rollback;
check its health hash and exclusive signed settings, then inspect all six real
exact-head jobs/log markers. Repository-only YAML changes do not update the
enforced service. No application redeployment is needed for this scope.
