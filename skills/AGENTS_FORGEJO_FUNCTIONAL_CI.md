# Forgejo functional CI and executable inputs (#1400)

Use when changing hosted workflows, optimizer/build images, release artifacts,
or the trusted Woodpecker configuration extension.

Canonical [review/verification checklist](../docs/qa/issue-1400/README.md),
[supply-chain policy](../docs/supply-chain-security.md),
[merge runbook](../docs/runbooks/forgejo-pr-merge.md), and
[issue #1400](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1400).

## Invariants CI1400-1–CI1400-10

1. **CI1400-1:** Forgejo requires `ci/woodpecker/pr/woodpecker` on the exact
   PR head. Only genuine Woodpecker results establish hosted enforcement.
   Never fabricate statuses, force merge, or drop a required context.
2. **CI1400-2:** All PRs, including docs-only, run gitleaks, contracts, frontend
   lint/unit/build, indexer library and the named isolated Postgres subset.
   Failure, timeout, skipped required steps and missing results cannot pass. Required workflow/scanner source mirrors must exist as regular files; deletion fails the trusted initializer. Invoke frontend test/lint/build tools directly; PR-owned npm script aliases cannot replace required checks. Deferred selector focus timers must cancel on reschedule/unmount; async quote tests preserve exact funding assertions under bounded startup waits.
3. **CI1400-3:** Required policy comes from a separately deployed reviewed
   immutable commit: exclusive signed config extension, `netrc=false`.
   PR YAML/scripts/TOML and issue text are never policy inputs. The public
   signing key is pinned; unsigned/wrong-repository requests fail closed.
4. **CI1400-4:** Required PR jobs have no secrets, Docker socket, host mounts,
   privileged mode, release commands, persistent volumes or shared caches.
   Fork approval controls stay enabled. Do not forward forge credentials.
5. **CI1400-5:** Executable Cargo targets, cargo-audit and npm state are fresh
   per job/build, independent of lockfile or branch. GitLab caches are disabled.
   Never promote PR targets to main/release. GitLab publication jobs require the protected default branch, including manual web triggers. Docker contexts exclude targets,
   node_modules, dist, artifacts and secrets.
6. **CI1400-6:** CI images, Docker FROM images and optimizer use reviewed
   manifest digests in [image-pins.json](../scripts/ci/image-pins.json).
   Preserve the existing Alpine and gitleaks download checksum. Locked
   dependency builds remain required; updating pins requires registry checks.
7. **CI1400-7:** Optimizer source is read-only, artifacts have caller ownership,
   and fresh named volumes cover **both** `/code/target` and `/target` plus
   the registry. Cleanup runs on failure/signals. Never run root Cargo on
   bind-mounted indexer targets.
8. **CI1400-8:** Release receipts identify source revision/dirty bytes,
   lockfile, compiler, builder image and artifact hashes. Verify them and
   compare an independent cold rebuild before promotion. An unsigned receipt
   alone authenticates neither the builder nor a deployment.
9. **CI1400-9:** Discovery rejection caches key on LCD client lifetime,
   factory and pair. Clones reuse entries; independent clients/factories
   remain isolated. Parallel repeated library tests cover the repaired flake.
10. **CI1400-10:** Critical LocalTerra signing/funding/rollback policy runs on
    an isolated trusted host daily at 03:00 UTC and before release, with one
    browser worker, no retries/skips and a 45-minute bound. The maintaining
    agent owns provisioning, failures, repair and reruns. Policy documentation
    is not proof that a scheduler or chain check ran.

The trusted frontend runner installs compiler, Python, pkg-config and native
Cairo/Pango/JPEG/GIF/SVG prerequisites plus DejaVu fonts before ordinary locked `npm ci`. A failed
prebuilt canvas download may use its actual native compilation fallback; do not
omit install scripts or native dependencies from the required job. Prerequisite
or npm installation failure removes any prior job receipt and remains FAIL.
Hermetic runner tests stub both apt-get and npm; never let their fixtures run
host package installation. Native fallback proof is a required worker step: remove only canvas build output,
force source rebuilding with two make jobs, require fresh nonempty objects and
binding, and assert exact RGBA plus DejaVu/Pango metrics/rendered alpha. Keep all
existing checks and the 25-minute cap. A prebuilt-only load or mocked runner PASS
does not establish compilation; local unavailable provisioning remains SKIP.

The [native dependency repair scope](../docs/qa/issue-1473/CI-NATIVE.md) retains
the failed installation trials and keeps policy deployment distinct from the
unchanged application deployment.

## Agent procedure

Run `make verify-issue-1400`, then the functional groups and negative tests
listed in the checklist. Inspect independent hosted logs and branch protection
for real enforcement. Deploy the extension only from an exact reviewed commit;
check `/health` policy hash and exclusive/netrc settings before testing PRs.
Record commands, UTC time, revision and scope. Complete runnable work first.

An agent may mark unavailable infrastructure, signing or deployment checks
**SKIP** with the reason and missing evidence, and report overall
**PASS (available checks; skips recorded)**. Executed failures stay **FAIL**;
skipped criteria stay unchecked and must never become hosted/deployment PASS.
No additional human approvals are required for implementation review.

Related: [supply-chain playbook](AGENTS_SUPPLY_CHAIN_SECURITY.md),
[deploy trace](AGENTS_DEPLOY_TRACE.md), [strict E2E](AGENTS_E2E_STRICT_CHAIN.md).

Frozen third-party LocalTerra fixtures: [build-locked-fixtures.py](../scripts/ci/build-locked-fixtures.py), [optimize-project.sh](../scripts/ci/optimize-project.sh); Python 3.11+, exact DEX-locked Git revisions, graph-preserving lock-format conversion and source/lock/image/toolchain/output receipts. Legacy production-only CW20 build scripts are a separate scope.
