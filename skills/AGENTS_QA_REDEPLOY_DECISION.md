# Agent skill: QA redeployment decision guide (GitLab #325)

## When to use

You changed code and need to know whether to **`make reset-qa`**, **`make deploy-local`**, **`make start-qa`**, or **no redeploy** before QA / live walks.

Parent context: [GitLab **#325**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/325). Deploy verification: [`AGENTS_QA_DEPLOY_VERIFY.md`](./AGENTS_QA_DEPLOY_VERIFY.md) (**Q1**). Volume reset: [`AGENTS_QA_FRESH_VOLUMES.md`](./AGENTS_QA_FRESH_VOLUMES.md).

## QA environment layers

| Layer | State lives in | Reset action |
|-------|----------------|--------------|
| LocalTerra chain | Docker volume `localterra-data` | `make reset-qa` (wipes volume) |
| Postgres / indexer DB | Docker volume `postgres-data` | `make reset-qa` |
| Deployed contracts | On-chain addresses | `make deploy-local` (new instantiation) |
| Indexer binary | `indexer/target/release/cl8y-dex-indexer` | `make start-qa` or prebuilt `INDEXER_QA_BIN` |
| Frontend env | `frontend-dapp/.env.local` | `make deploy-local` rewrites file |

**`make deploy-local`** redeploys contracts and rewrites env files. It does **not** wipe chain/Postgres volumes or restart the indexer.

**`make reset-qa`** wipes both Docker volumes, then runs full `start-qa` (deploy + indexer).

**`make start`** only starts Docker — no build or deploy.

## Decision table

### Requires `make reset-qa` (full wipe + redeploy)

| Change type | Reason |
|-------------|--------|
| Indexer Rust (`indexer/src/**`) | Stale binary; new addresses orphan prior Postgres rows |
| Smart contract source (`smartcontracts/contracts/**`) | New wasm + new addresses invalidate indexer history |
| Deploy script **instantiation params** (`scripts/deploy-dex-local.sh` fees, governance, code IDs) | Existing factory keeps old config until redeploy on fresh chain |
| Corrupted / known-bad chain state | Volume wipe is the reliable fix |
| Contracts + indexer + deploy script in one branch | Partial resets risk inconsistency |

### Requires `make deploy-local` only (no chain wipe)

| Change type | Reason |
|-------------|--------|
| Deploy script **non-param** logic (gas flags, waits, output) | New addresses; volumes may stay valid |
| Frontend needs updated env addresses | `deploy-local` rewrites `.env.local` |
| Incomplete prior deploy | Redeploy without wiping history |

When schema probes pass but **`.qa-deploy-stamp`** `git_sha` ≠ **`HEAD`**: **`make deploy-local && make qa-verify-deploy`** — no volume wipe.

### No redeployment needed

| Change type | Reason |
|-------------|--------|
| Frontend only (`frontend-dapp/src/**`) | Restart `make dev` |
| Docs / skills / `AGENTS.md` | No runtime impact |
| QA verify scripts (`scripts/qa/verify-issue-*.sh`) | Not deployment |
| Contract unit tests (`smartcontracts/**/tests/**`) | Cargo-local, not live chain |

## Quick reference

```
Indexer Rust changed?                    → make reset-qa
Contract Rust changed?                   → make reset-qa
Deploy script instantiation params?      → make reset-qa
Deploy script non-param logic only?      → make deploy-local
Frontend / docs / QA scripts only?       → no redeploy
```

## Faster QA bring-up (#325 optimizations)

| Optimization | How |
|--------------|-----|
| Skip redundant deploy | `start-qa` probes stamp + factory LCD (**Q1**); skips `deploy-local` when aligned |
| Skip optimizer when wasm fresh | `make deploy-local-no-build` or `QA_FETCH_CI_ARTIFACTS=1 make start-qa` |
| Reuse indexer binary | `cd indexer && cargo build --release`; `export INDEXER_QA_BIN=$PWD/target/release/cl8y-dex-indexer` |
| CI wasm / indexer packages | GitLab generic packages `qa-wasm/{sha}`, `qa-indexer/{sha}` — see `.gitlab-ci.yml` |
| Lighter deploy seed | `QA_DEPLOY_SEED=minimal\|charts\|wallet\|full` (default `full`) |
| Tx poll vs fixed sleep | `scripts/lib/terrad-wait-tx.sh` in `deploy-dex-local.sh` |
| Phase timing logs | `[timing]` lines in `start-qa` and `deploy-dex-local` |

**Default:** prefer **`make start-qa`** over **`make reset-qa`** unless contract/genesis/indexer schema changed or **`make qa-verify-deploy`** reports stale on-chain schema.

## Real example: commit `019ded6`

Factory `pair_creation_fee_uluna` fix + indexer Rust change → **`make reset-qa`**, not `deploy-local` alone:

1. Indexer needed recompile  
2. New addresses orphan Postgres for old factory  
3. On-chain factory retained `pair_creation_fee_uluna=0` until volume wipe  

## Verification

```bash
make test-qa-redeploy-decision    # wiring checks (no Docker)
make qa-verify-deploy             # stamp + schema probes
```

## Cross-links

- Operator README: [`scripts/qa/README.md`](../scripts/qa/README.md)
- Invariants: [`docs/qa-invariants.md`](../docs/qa-invariants.md)
- Cloud Agent idempotent setup: [`AGENTS.md`](../AGENTS.md) § LocalTerra
