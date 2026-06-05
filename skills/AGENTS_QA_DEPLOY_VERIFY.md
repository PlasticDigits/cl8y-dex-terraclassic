# Agent skill: QA deploy verification (GitLab #203)

## When to use

You are changing **`scripts/qa/start-qa.sh`**, **`scripts/deploy-dex-local.sh`**, **`scripts/qa/verify-deploy.sh`**, or debugging QA/live walks where LCD pair queries return **`unknown variant`** for entrypoints that exist in the current tree (`is_paused`, `expired_limit_refund`).

## Problem (invariant Q1)

| ID | Invariant |
|----|-----------|
| **Q1** | After **`make deploy-local`** on a QA path, the deployed pair at addresses in **`indexer/.env`** / **`.qa-deploy-stamp`** must accept current-schema smart queries. Fresh wasm on disk does **not** imply fresh on-chain behaviour when **`localterra-data`** / **`postgres-data`** volumes are reused without a full redeploy aligned to **`HEAD`**. |

Parent: [GitLab **#120**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120). Implementation: [GitLab **#203**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203). Volume reset: [`AGENTS_QA_FRESH_VOLUMES.md`](./AGENTS_QA_FRESH_VOLUMES.md) ([#202](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/202)). **Change-type → reset level:** [`AGENTS_QA_REDEPLOY_DECISION.md`](./AGENTS_QA_REDEPLOY_DECISION.md) ([#325](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/325)).

## Automated check

| Step | Command / hook |
|------|----------------|
| Deploy stamp | **`scripts/deploy-dex-local.sh`** writes **`.qa-deploy-stamp`** (`git_sha`, `factory_address`, `pair_address`) |
| Schema probe | **`make qa-verify-deploy`** (also runs inside **`make start-qa`** after deploy) |
| Probes | LCD `is_paused`, `expired_limit_refund` on stamp/factory pair |
| Stamp | Fail if **`.qa-deploy-stamp`** `git_sha` ≠ **`git rev-parse --short HEAD`** |
| Pair-creation fee on deploy | **`factory_create_pair`** in **`scripts/deploy-dex-local.sh`** reads on-chain `pair_creation_fee_uluna` and attaches `--amount` on every `create_pair` ([#318](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/318)); regression grep in **`scripts/qa/verify-issue-276.sh`**. **`scripts/e2e-seed-wrap-pairs.sh`** attaches the same fee when non-zero. |

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

## On-chain feature E2E after schema-changing deploys

`qa-verify-deploy` only probes that the deployed pair accepts current-schema
queries. For features whose **on-chain behaviour** must be re-checked after a
redeploy (not just schema acceptance), add a dedicated live E2E. Example —
**GitLab #238** hybrid-sim CL8Y fee-discount parity (invariant **L8**):

```bash
make deploy-local        # fresh wasm at new addresses (schema adds `trader`)
make qa-verify-deploy    # stamp == HEAD + schema probes
# start the indexer (route/solve check needs it), then:
make verify-issue-238    # scripts/qa/verify-issue-238.sh
```

`verify-issue-238` mints CL8Y to the dev wallet, registers a discount tier, and
asserts: pair/router `hybrid_simulation` accept `trader`; discounted sim >
undiscounted; **executed swap output == discounted sim `return_amount`** on the
same reserves; indexer `GET /route/solve?...&trader=` returns a trader-aware
quote. Prior #238 verification was **blocked on stale wasm rejecting `trader`** —
this script is the on-chain proof that a fresh deploy resolves it. See
[`skills/AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md) and
[`skills/AGENTS_FEE_DISCOUNT_TIERS.md`](./AGENTS_FEE_DISCOUNT_TIERS.md).

**GitLab #245** (off-chain wiring: dapp + indexer forward connected wallet as
`trader` on every quote LCD hop):

```bash
make verify-issue-245   # scripts/qa/verify-issue-245.sh
```

Runs frontend + indexer unit/integration tests, then **`verify-issue-238`** when
LocalTerra is up (proves indexer route/solve and pair preflight parity on live
wasm). Manual UI checklist (Trade/Swap receive preview) remains for human QA;
issue stays open until sign-off. Cross-links: [GitLab **#245**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/245),
[`skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md`](./AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md).

Standalone after **`git pull`** without redeploy (expect stamp mismatch):

```bash
make qa-verify-deploy   # should fail until make deploy-local
```

## Host RPC/LCD (`docker exec` fallback)

On some Linux hosts, **host** `curl` to published `127.0.0.1:26657` / `:1317` hangs while the chain is healthy in-container. **`verify-deploy`** and **`wait-localterra`** use [`scripts/lib/localterra-host-curl.sh`](../scripts/lib/localterra-host-curl.sh): try host curl (short timeout), then **`docker exec … curl http://127.0.0.1:…`** into the `localterra` service. The frontend still uses host URLs from `.env.local`.

## Postgres on external QA hosts (GitLab #245)

If Postgres only ships **`postgres:postgres`**, indexer integration tests fail until **`cl8y_legal`** exists. Run **`./scripts/setup-postgres-dev-databases.sh`** (auto-bootstrap via superuser) or create the role manually — [`skills/AGENTS_LOCAL_POSTGRES_DEV.md`](./AGENTS_LOCAL_POSTGRES_DEV.md) § Stack prerequisite. Verify: **`make test-setup-postgres`**.

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

- LocalTerra **SDK 0.53** image / fresh volumes after digest bump: [`docs/localterra-sdk53.md`](../docs/localterra-sdk53.md) ([GitLab **#292**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/292))
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
