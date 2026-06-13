# Supply-chain security (GitLab #380)

Hosted **GitLab CI** runs lightweight SCA, mandatory gitleaks, and DinD with TLS. Local equivalents map to Makefile targets below.

**Parent remediation:** [#376 — Full security report](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/376) (H-06, M-11, M-13, L-12).

## CI jobs (`.gitlab-ci.yml`)

| Job | Stage | When | Command |
|-----|-------|------|---------|
| `gitleaks` | `security` | Every **default-branch** pipeline + weekly schedule | `gitleaks detect --no-git --config .gitleaks.toml` (working tree; history predates #118) |
| `cargo-audit-smartcontracts` | `security` | Default branch (always) + MR when `smartcontracts/**` changes + schedule | `cd smartcontracts && cargo audit --deny warnings` |
| `cargo-audit-indexer` | `security` | Default branch (always) + MR when `indexer/**` changes + schedule | `cd indexer && cargo audit --deny warnings` |
| `npm-audit-frontend` | `security` | Default branch (always) + MR when frontend lockfiles change + schedule | `npm audit --audit-level=high --omit=dev` in `frontend-dapp` |
| `qa-wasm-artifacts` | `build` | Default-branch changes / manual web trigger | `make build-optimized` via **DinD TLS** (`DOCKER_HOST=tcp://docker:2376`) |

**Invariants (SC-1):**

- Security jobs are **not** `allow_failure: true`.
- `gitleaks` is **mandatory** on default-branch pipelines (M-13). Uses **`--no-git`** (current tree only) so pre-#118 historical commits do not fail the pipeline; new violations in tracked files still fail.
- Audit jobs run **unconditionally** on default branch; MRs are change-gated to keep overhead low (H-06).
- **Pipeline budget:** security stage targets **&lt; 5 minutes** added median duration (cargo-audit install cached; gitleaks image pull amortized).

### DinD TLS (M-11)

`qa-wasm-artifacts` uses `docker:24-dind` with default `DOCKER_TLS_CERTDIR=/certs` and `DOCKER_HOST=tcp://docker:2376`. Unencrypted `2375` is **not** used. TLS prevents sibling processes in the job pod from talking to the Docker API without client certificates.

## Local verification

```bash
make audit-smartcontracts   # cargo audit + smartcontracts/audit.toml
make audit-indexer          # cargo audit + indexer/audit.toml
make audit-frontend         # npm audit --audit-level=high --omit=dev
make gitleaks-detect        # full-tree scan (Docker image if gitleaks not installed)
make verify-gitleaks        # fixture must fail; clean tree must pass
```

Install `cargo-audit` once: `cargo install cargo-audit --locked`.

Pre-commit gitleaks remains **best-effort** when the binary is missing locally; **CI always runs** gitleaks (see [`.githooks/pre-commit`](../.githooks/pre-commit)).

## Allowlists

| Path | Tool | Purpose |
|------|------|---------|
| [`smartcontracts/.cargo/audit.toml`](../smartcontracts/.cargo/audit.toml) | `cargo audit` | Accepted RustSec findings in the CosmWasm workspace (transitive `curve25519-dalek`, unmaintained `derivative`, `rand` unsound warning) |
| [`indexer/.cargo/audit.toml`](../indexer/.cargo/audit.toml) | `cargo audit` | `rsa` Marvin attack (no upstream fix); `rand` unsound warning |
| [`docs/supply-chain-security.md`](./supply-chain-security.md) | npm | Runtime `elliptic` via `@cosmjs/crypto` / `@goblinhunt/cosmes` — see below |

Remove allowlist entries only after dependency upgrades clear the advisory.

## Accepted runtime risk: `elliptic` (L-12)

The frontend wallet stack pulls **`elliptic`** transitively through **`@cosmjs/crypto` ≤ 0.33.x**, used by **`@goblinhunt/cosmes`** (fork; see [frontend.md § Forked cosmes](./frontend.md#cosmes-fork-patches)).

| Item | Detail |
|------|--------|
| Advisory | [GHSA-848j-6mx2-7j84](https://github.com/advisories/GHSA-848j-6mx2-7j84) (moderate — risky crypto primitive) |
| CI gate | `npm audit --audit-level=high --omit=dev` — **does not fail** on moderate `elliptic` today |
| Upgrade path | Track **CosmJS ≥ 0.34** (replaces elliptic) when `@goblinhunt/cosmes` / `@dao-dao/cosmiframe` support it without breaking Terra Classic signing |
| Mitigation | Keys stay in wallet extensions (Keplr) or dev-only `VITE_DEV_MNEMONIC`; production build rejects inlined mnemonics ([#118](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/118)); gitleaks custom BIP39 rule |

**Follow-up:** bump cosmes/cosmjs when upstream releases a Terra-compatible stack without elliptic; re-run `npm audit` and drop this section when clean.

## Gitleaks

Config: [`.gitleaks.toml`](../.gitleaks.toml) — extends defaults plus `bip39-like-phrase-frontend-src` for `frontend-dapp/src` (GitLab #118). CI scans the **working tree** (`--no-git`); `target/`, `node_modules/`, and the CI fixture under `scripts/ci/gitleaks-fixture/` are allowlisted.

Agent playbook: [`skills/AGENTS_BUNDLE_DEV_WALLET.md`](../skills/AGENTS_BUNDLE_DEV_WALLET.md).

Attack/abuse check: `make verify-gitleaks` runs a **fixture** that must trip the custom rule (see `scripts/ci/gitleaks-fixture/`).

## Cross-links

- [docs/testing.md § CI](./testing.md#ci) — reference vs hosted jobs
- [docs/operator-secrets.md](./operator-secrets.md) — runtime secrets handling
- [`.github/workflows/README.md`](../.github/workflows/README.md) — portable job checklist (not executed on GitLab)
- [`skills/AGENTS_SUPPLY_CHAIN_SECURITY.md`](../skills/AGENTS_SUPPLY_CHAIN_SECURITY.md) — agent playbook
