# Supply-chain security (CI SCA, gitleaks, DinD TLS)

Remediation for [GitLab #376](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/376) via [#380](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/380).

## GitLab CI security stage

Jobs in [`.gitlab-ci.yml`](../.gitlab-ci.yml) `security` stage (unconditional on default branch, not `allow_failure`):

| Job | Tool | Notes |
|-----|------|--------|
| `gitleaks` | [gitleaks](https://github.com/gitleaks/gitleaks) v8.21.2 image | Incremental push scan via [`scripts/ci/gitleaks-detect.sh`](../scripts/ci/gitleaks-detect.sh) |
| `cargo-audit-smartcontracts` | `cargo audit` | Allowlist: [`smartcontracts/.cargo/audit.toml`](../smartcontracts/.cargo/audit.toml) |
| `cargo-audit-indexer` | `cargo audit` | Allowlist: [`indexer/.cargo/audit.toml`](../indexer/.cargo/audit.toml) |
| `npm-audit-frontend` | `npm audit --audit-level=high --omit=dev` | Runtime deps only; dev-only advisories excluded |

**Duration budget (H-06):** target **&lt;5 min** added median pipeline time vs pre-#380 (parallel `security` jobs).

### DinD TLS (M-11)

`qa-wasm-artifacts` uses Docker-in-Docker with **`DOCKER_TLS_CERTDIR=/certs`** (encrypted API on port **2376**). The prior `DOCKER_TLS_CERTDIR: ""` + `tcp://docker:2375` configuration allowed unencrypted Docker API access from co-processes in the same job pod.

## Local commands

Mirror CI before pushing:

```bash
# Secrets (incremental — same as CI on a single commit)
./scripts/ci/gitleaks-detect.sh

# Working tree only (excludes target/ and node_modules/ via .gitleaks.toml)
gitleaks detect --source . --no-git -c .gitleaks.toml

# Attack/abuse fixture — must exit non-zero
./scripts/ci/test-gitleaks-fixture.sh

# Rust SCA
cd smartcontracts && cargo audit --deny warnings
cd indexer && cargo audit --deny warnings

# Node SCA (runtime, high+)
cd frontend-dapp && npm ci --omit=dev && npm audit --audit-level=high --omit=dev
```

Pre-commit still runs `gitleaks protect --staged` when the binary is installed (best-effort locally); **CI gitleaks is mandatory** (M-13).

## Accepted dependency risks (L-12)

### `elliptic` (npm, transitive via `@goblinhunt/cosmes` / `@cosmjs/*`)

| Field | Value |
|-------|--------|
| Advisory | [GHSA-848j-6mx2-7j84](https://github.com/advisories/GHSA-848j-6mx2-7j84) |
| Path | `@goblinhunt/cosmes` → `@dao-dao/cosmiframe` → `@cosmjs/proto-signing` → `@cosmjs/crypto` → `elliptic` |
| Severity in npm audit | Below `--audit-level=high` threshold; tracked for cosmes/cosmjs upgrades |
| Mitigation | Forked cosmes with patch verification (`make test-frontend` / [#367](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/367)); no BIP39 literals in bundle ([#118](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/118)) |
| Upgrade plan | Re-evaluate when upstream `@cosmjs/*` ≥0.34 ships `elliptic` fix or cosmes drops the chain; bump `@goblinhunt/cosmes` and re-run `npm audit --audit-level=high --omit=dev` |

### Rust allowlists

Documented inline in each `.cargo/audit.toml` with advisory IDs and rationale. Prefer upgrading lockfile deps (e.g. `rustls-webpki`, `time`) over expanding ignore lists.

## Related

- [`.gitleaks.toml`](../.gitleaks.toml) — custom BIP39 rule for `frontend-dapp/src`
- [`skills/AGENTS_BUNDLE_DEV_WALLET.md`](../skills/AGENTS_BUNDLE_DEV_WALLET.md) — dev mnemonic handling
- [`docs/operator-secrets.md`](./operator-secrets.md) — production secrets
- [`gaps/GAP_1780200149.md`](../gaps/GAP_1780200149.md) — original H9/M-13/M-11 findings
