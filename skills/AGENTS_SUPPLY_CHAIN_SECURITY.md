# Agent playbook: CI supply-chain security (GitLab #380)

Use when changing `.gitlab-ci.yml`, dependency lockfiles, `.gitleaks.toml`, or DinD build jobs.

## Canonical references

| Doc / path | Purpose |
|------------|---------|
| [docs/supply-chain-security.md](../docs/supply-chain-security.md) | Hosted jobs, allowlists, DinD TLS, elliptic accepted risk |
| [docs/testing.md § CI](../docs/testing.md#ci) | Reference vs GitLab jobs |
| [`.gitlab-ci.yml`](../.gitlab-ci.yml) | `security` + `build` stages |
| [`.gitleaks.toml`](../.gitleaks.toml) | Secret patterns (mandatory in CI) |
| [`smartcontracts/.cargo/audit.toml`](../smartcontracts/.cargo/audit.toml) | Rust allowlist — contracts |
| [`indexer/.cargo/audit.toml`](../indexer/.cargo/audit.toml) | Rust allowlist — indexer |

## Local commands

```bash
make audit-smartcontracts
make audit-indexer
make audit-frontend
make gitleaks-detect
make verify-gitleaks
```

Install `cargo-audit` once: `cargo install cargo-audit --locked`.

## Rules of thumb

1. **Never** set `allow_failure: true` on `gitleaks` or audit jobs.
2. **Never** re-disable DinD TLS (`DOCKER_TLS_CERTDIR=""` / port `2375`) without a security review — use TLS on `2376` (M-11).
3. Add RustSec/npm ignores only with a comment in the allowlist file **and** a line in `docs/supply-chain-security.md`.
4. `npm audit` in CI uses `--audit-level=high --omit=dev` — moderate `elliptic` via cosmes is documented accepted risk (L-12); track CosmJS ≥ 0.34 upgrade.
5. Pre-commit gitleaks may skip when the binary is missing; **CI does not skip** (M-13).
6. Gitleaks fixture lives under `scripts/ci/gitleaks-fixture/` — do not move quoted BIP39-like strings into `frontend-dapp/src`.

## Cross-links

- Dev wallet / BIP39 gitleaks rule: [`AGENTS_BUNDLE_DEV_WALLET.md`](./AGENTS_BUNDLE_DEV_WALLET.md)
- QA wasm CI artifacts: [`AGENTS_QA_REDEPLOY_DECISION.md`](./AGENTS_QA_REDEPLOY_DECISION.md)
- Deploy trace at release time (git SHA, chain version, code IDs): [`AGENTS_DEPLOY_TRACE.md`](./AGENTS_DEPLOY_TRACE.md)
- Operator secrets: [docs/operator-secrets.md](../docs/operator-secrets.md)
