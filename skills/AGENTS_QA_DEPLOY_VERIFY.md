# Agent skill: QA deploy verification (GitLab #203)

## When to use

You are changing **`scripts/qa/start-qa.sh`**, **`scripts/deploy-dex-local.sh`**, **`scripts/qa/verify-deploy.sh`**, or debugging QA/live walks where LCD pair queries return **`unknown variant`** for entrypoints that exist in the current tree (`is_paused`, `expired_limit_refund`).

## Problem (invariant Q1)

| ID | Invariant |
|----|-----------|
| **Q1** | After **`make deploy-local`** on a QA path, the deployed pair at addresses in **`indexer/.env`** / **`.qa-deploy-stamp`** must accept current-schema smart queries. Fresh wasm on disk does **not** imply fresh on-chain behaviour when **`localterra-data`** / **`postgres-data`** volumes are reused without a full redeploy aligned to **`HEAD`**. |

Parent: [GitLab **#120**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120). Implementation: [GitLab **#203**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203). Volume reset: [`AGENTS_QA_FRESH_VOLUMES.md`](./AGENTS_QA_FRESH_VOLUMES.md) ([#202](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/202)).

## Automated check

| Step | Command / hook |
|------|----------------|
| Deploy stamp | **`scripts/deploy-dex-local.sh`** writes **`.qa-deploy-stamp`** (`git_sha`, `factory_address`, `pair_address`) |
| Schema probe | **`make qa-verify-deploy`** (also runs inside **`make start-qa`** after deploy) |
| Probes | LCD `is_paused`, `expired_limit_refund` on stamp/factory pair |
| Stamp | Fail if **`.qa-deploy-stamp`** `git_sha` ≠ **`git rev-parse --short HEAD`** |

On failure: non-zero exit + copy pointing to the right fix:

| Failure | Fix |
|---------|-----|
| Schema probe (`unknown variant` on `is_paused` / `expired_limit_refund`) | **`make reset-qa`** / **`QA_FRESH_VOLUMES=1 make start-qa`** — see [`AGENTS_QA_FRESH_VOLUMES.md`](./AGENTS_QA_FRESH_VOLUMES.md) ([#202](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/202)) |
| Stamp `git_sha` ≠ **`HEAD`** (schema probes passed) | **`make deploy-local && make qa-verify-deploy`** — no volume wipe required |

See [`scripts/qa/README.md`](../scripts/qa/README.md) § Stale deployed contracts.

## One-command repro

```bash
make start-qa          # includes qa-verify-deploy
# or after deploy only:
make qa-verify-deploy
```

Standalone after **`git pull`** without redeploy (expect stamp mismatch):

```bash
make qa-verify-deploy   # should fail until make deploy-local
```

## Host RPC/LCD (`docker exec` fallback)

On some Linux hosts, **host** `curl` to published `127.0.0.1:26657` / `:1317` hangs while the chain is healthy in-container. **`verify-deploy`** and **`wait-localterra`** use [`scripts/lib/localterra-host-curl.sh`](../scripts/lib/localterra-host-curl.sh): try host curl (short timeout), then **`docker exec … curl http://127.0.0.1:…`** into the `localterra` service. The frontend still uses host URLs from `.env.local`.

```bash
make test-localterra-host-curl   # wiring + live probe when compose is up
make qa-verify-deploy            # uses exec fallback automatically
```

## Files

| Path | Role |
|------|------|
| [`scripts/qa/verify-deploy.sh`](../scripts/qa/verify-deploy.sh) | Post-deploy verification |
| [`scripts/qa/test-verify-deploy.sh`](../scripts/qa/test-verify-deploy.sh) | Unit checks for LCD helpers (`make test-qa-verify-deploy`) |
| [`scripts/qa/test-localterra-host-curl.sh`](../scripts/qa/test-localterra-host-curl.sh) | Exec fallback wiring (`make test-localterra-host-curl`) |
| [`scripts/lib/localterra-host-curl.sh`](../scripts/lib/localterra-host-curl.sh) | Host curl + `docker exec` fallback |
| [`scripts/lib/lcd-smart-query.sh`](../scripts/lib/lcd-smart-query.sh) | Shared LCD smart-query helpers |
| [`scripts/wait-localterra.sh`](../scripts/wait-localterra.sh) | `make wait-localterra` |
| [`docs/qa-invariants.md`](../docs/qa-invariants.md) | Invariant **Q1** + failure-mode table |
| [`.qa-deploy-stamp`](../.qa-deploy-stamp) | Machine-local stamp (gitignored) |
| [`scripts/qa/start-qa.sh`](../scripts/qa/start-qa.sh) | Calls verify after `deploy-local` |
| [`scripts/deploy-dex-local.sh`](../scripts/deploy-dex-local.sh) | Writes stamp in Phase 6.3 |

## Cross-links

- Operator runbook: [`scripts/qa/README.md`](../scripts/qa/README.md)
- Strict E2E (uses `is_paused` on laptop): [`AGENTS_E2E_STRICT_CHAIN.md`](./AGENTS_E2E_STRICT_CHAIN.md) ([#201](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201))
- Limit tx E2E: [`AGENTS_E2E_LIMIT_ORDERS_TX.md`](./AGENTS_E2E_LIMIT_ORDERS_TX.md) ([#195](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/195))
- Pair pause invariant L6: [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md)
- Local dev: [`docs/local-development.md`](../docs/local-development.md)

## Agent checklist after contract-side changes

1. Run **`make start-qa`** (or **`make deploy-local`** + **`make qa-verify-deploy`**) on the QA host.
2. Confirm **`.qa-deploy-stamp`** `git_sha` matches **`HEAD`**.
3. Re-**`scp`** **`frontend-dapp/.env.local`** to laptops before live walks.
4. If verification fails, use the **failure-mode table** above (stamp mismatch → redeploy; schema probe → reset volumes) — do not silence the check.
