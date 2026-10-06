# Issue #1400 — functional CI and build trust

[Issue/comments](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1400),
[CI1400 invariants and agent instructions](../../../skills/AGENTS_FORGEJO_FUNCTIONAL_CI.md),
[supply-chain policy](../../supply-chain-security.md),
[verification receipt](verification.md).

The required Forgejo workflow now defines gitleaks, locked contract workspace
tests, frontend lint/unit/cold production build, indexer library tests, and
isolated Postgres tests (`security`, `api_hooks`, `api_route_tax_funding1381`,
`indexer_ingestion_hardening`). Every PR includes these jobs. The trusted initializer rejects deletion or symlink substitution of the workflow/scanner source mirrors before executing source; configuration deletion cannot become a green missing-job result. A final
`functional-required` step checks completeness; the trusted engine's step
exit/status remains authoritative. Receipts in the shared checkout are
bookkeeping and are not an adversarial authentication boundary.

The final local optimizer produced 13 matching wasm hashes across two independent cold compiler runs; see [comparison receipt](cold-wasm-comparison.json). The first outer wrapper failed; the finalized immutable rerun completed and verified its receipt.

No persistent executable caches are used in Forgejo or GitLab. Identical
lockfiles in different trust levels still receive independent empty targets.
Release builds also use empty targets and manifest-pinned images. The optimizer
retains named target overlays and isolates its actual `/target` as well.

## Review and verification checklist

An implementing or reviewing agent owns every step. Do not give the human a
verification handoff. Record revision, UTC time, commands and outcomes.

- [ ] Run `make verify-issue-1400`; repeat it after changes. This checks image
  declarations, synthetic cache contamination/provenance/cleanup, and runner
  missing/failed/skipped receipts. It does not prove hosted enforcement.
- [ ] Run `cargo test --locked --workspace` in `smartcontracts`, frontend
  `npm ci`, `npm run lint`, `npm run test:run -- --maxWorkers=2`, and
  `VITE_WC_PROJECT_ID=ci-build-typecheck VITE_DEV_MODE=false npm run build`.
  Cold builds must ignore preexisting target/node_modules outputs. The minimal pinned Rust image installs its matching checksum-verified rustfmt component before contract formatting; a missing component remains a failure.
- [ ] Run indexer `cargo test --locked --lib -- --test-threads=16` repeatedly;
  repeat `indexer::pair_discovery::tests` at least 20 times. Confirm independent
  client/factory keys and no global cache clearing by unrelated tests.
- [ ] Provision a dedicated disposable Postgres service with synthetic
  role/database/password, and run the four named integration binaries serially.
  Host Cargo only; do not mount the checkout into root Docker for Cargo.
- [ ] Build/test the extension:
  `docker build -f scripts/ci/trusted-policy/Dockerfile -t dex-ci-policy .`.
  Its synthetic signed tests reject unsigned/wrong-repository/credential
  requests and ignore deleted, skipped or malicious contributor config.
- [ ] Verify reviewed image manifests:
  `python3 scripts/ci/verify-image-pins.py --registry`. Build the optimizer and
  indexer twice from clean targets; verify receipt inputs and compare artifact
  hashes. Treat nondeterminism as a failure to investigate, never cache PASS.
- [ ] Deploy the policy service from an exact reviewed commit with no secrets,
  no host sockets/volumes, nonroot runtime and bounded resources. Validate
  `/health`. Set Woodpecker repo 7 `config_extension_endpoint` to its HTTPS
  `/config`, `config_extension_exclusive=true`, `config_extension_netrc=false`.
  Read back settings, pinned policy hash, fork approval mode and timeout.
- [ ] Read Forgejo `main` branch protection through `/api/v1/repos/code/cl8y-dex-terraclassic/branch_protections`.
  Keep `ci/woodpecker/pr/woodpecker`, outdated-branch blocking and existing
  protection. Obtain genuine CI statuses and step logs on the exact PR head.
- [ ] Open synthetic green and deliberate-red **draft** PRs from source and
  docs-only branches. Break one real assertion in each of contracts/frontend/
  indexer, then remove YAML or required helpers in another PR. Inspect real
  Woodpecker status provenance and nonmergeability. Close test PRs afterward;
  do not attempt a production merge or fabricate statuses.
- [ ] On the isolated LocalTerra host provision the stack for the exact tested
  revision, then run `timeout --signal=TERM --kill-after=30s 45m bash scripts/ci/functional-localterra.sh`.
  It checks route-funded tax/min-return rollback, native/invoice paths, and
  four strict simulated-wallet signing/recovery/hybrid/limit cases. Check
  the reduced receipt reports zero failed/flaky/skipped cases. Repeat.
- [ ] Install/check the daily **03:00 UTC** schedule on that dedicated trusted
  host and run it before release. The maintaining agent monitors missing/red
  receipts, repairs failures and reruns. A missed scheduled run is unverified;
  an executed failure is FAIL. PR jobs must not gain a host Docker socket or
  production keys to make this schedule possible. Do not claim all existing
  browser specs run in hosted CI.
- [ ] Publish only reduced synthetic evidence. Run tracked-file gitleaks;
  exclude private browser reports, environment files and credentials.

## Skip policy

Run everything available first. The reviewing agent is explicitly permitted to
record unavailable checks as **SKIP** and mark the overall implementation
verification **PASS (available checks; skips recorded)**. Record the exact
reason, missing evidence and scope beside each skip; leave the corresponding
acceptance criterion unchecked. A failure that actually executed remains FAIL
and must be fixed or reported. Skips never establish remote merge enforcement,
release reproducibility, deployment success or issue acceptance/closure.

## Policy deployment and updates

The configuration extension uses Woodpecker's official signed extension
protocol, pinned public Ed25519 key, exact DEX repository identity and immutable
reviewed runner/scanner/TOML bytes baked into its image. It receives no netrc and
does not execute request-provided configuration, URLs, commands or issue text.
Returned config keeps the name `.woodpecker.yaml` to preserve the required
`woodpecker` context. Endpoint failure fails configuration retrieval; there is
no 204 fallback to contributor YAML.

The service commit is an independent policy release. New required test targets
or runner changes need a tested service update pinned to a new exact commit;
merging PR YAML alone cannot alter the enforced policy. No default-branch
checkout or mutable branch is an authority. Preserve the previous deployed
commit for rollback; restore it via a normal deployment, retaining the exclusive
extension settings and required status. Pin rotation verifies the new public
key out of band before deployment. No secrets are stored in this service.

Official protocols: [Woodpecker configuration extension](https://woodpecker-ci.org/docs/usage/extensions/configuration-extension)
and [signature verification](https://woodpecker-ci.org/docs/usage/extensions).

Local fixture bootstrap uses [build-locked-fixtures.py](../../../scripts/ci/build-locked-fixtures.py) and [optimize-project.sh](../../../scripts/ci/optimize-project.sh): exact CW20/USTR commits from the DEX lock, fresh named volumes and Python 3.11+. Fixture lock format 4 is converted to 3 for the pinned legacy optimizer without changing any parsed package/version/revision/checksum; receipts record both original and effective lock hashes. Actual external fixture cold builds remain unverified.
