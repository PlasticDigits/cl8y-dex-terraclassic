# Agent playbook: env/chain address cross-check (SEC-H04)

Use when verifying **frontend and indexer env contract addresses** match each other and on-chain wiring after deploy or manual env edits ([#442](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/442)).

## Problem (invariant Q4)

| ID | Invariant |
|----|-----------|
| **Q4** | `FACTORY_ADDRESS`, `ROUTER_ADDRESS`, and `FEE_DISCOUNT_ADDRESS` in `indexer/.env` must match `VITE_*` counterparts in `frontend-dapp/.env.local` (or production env). Router on-chain `config.factory` must equal env `FACTORY_ADDRESS`; factory and fee-discount contracts must respond to `config` at the configured addresses. |

Parent schema/stamp check: invariant **Q1** in [`AGENTS_QA_DEPLOY_VERIFY.md`](./AGENTS_QA_DEPLOY_VERIFY.md) ([#203](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203)). On-chain config assertions: invariant **Q2** in [`AGENTS_DEPLOY_CONFIG_VERIFY.md`](./AGENTS_DEPLOY_CONFIG_VERIFY.md) ([#441](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/441)).

## Automated check

| Step | Command |
|------|---------|
| Env/chain address cross-check | **`make qa-verify-env-addresses`** → [`scripts/qa/verify-env-addresses.sh`](../scripts/qa/verify-env-addresses.sh) |
| Wired into deploy verify | **`make qa-verify-deploy`** (calls env address script after schema/stamp — Q1 + Q4) |
| Doc drift | **`make check-deploy-env-addresses-docs`** |
| Issue acceptance | **`make verify-issue-442`** |

### Queries asserted

1. **Env parity** — indexer `FACTORY_ADDRESS` / `ROUTER_ADDRESS` / `FEE_DISCOUNT_ADDRESS` == frontend `VITE_*` counterparts
2. **Factory** — `config` at `FACTORY_ADDRESS` returns governance + treasury (proves address is a factory)
3. **Router** — `config` at `ROUTER_ADDRESS`; `factory` field == env `FACTORY_ADDRESS`
4. **Fee-discount** — `config` at `FEE_DISCOUNT_ADDRESS`; `governance` non-empty

### Staging / mainnet env files

Override paths when env files differ from LocalTerra defaults:

```bash
export VERIFY_ENV_INDEXER_FILE=/path/to/indexer/.env
export VERIFY_ENV_FRONTEND_FILE=/path/to/frontend-dapp/.env.production
export TERRA_LCD_URL=https://<lcd>
make qa-verify-env-addresses
```

## Release sign-off

Paste full script output on the **launch / release tracking issue** ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)). Required in [launch checklist Phase 4](../docs/runbooks/launch-checklist.md#phase-4--off-chain-stack-if-applicable) (SEC-H04).

## One-command repro

```bash
make deploy-local
make qa-verify-deploy        # Q1 schema + stamp + Q4 env addresses
make qa-verify-deploy-config  # Q2 config assertions
```

## Files

| Path | Role |
|------|------|
| [`scripts/qa/verify-env-addresses.sh`](../scripts/qa/verify-env-addresses.sh) | Live LCD env/chain address verification |
| [`scripts/qa/test-verify-env-addresses.sh`](../scripts/qa/test-verify-env-addresses.sh) | Fixture unit tests |
| [`scripts/check_deploy_env_addresses_docs.py`](../scripts/check_deploy_env_addresses_docs.py) | Doc drift guard |
| [`docs/qa-invariants.md`](../docs/qa-invariants.md) | Invariant **Q4** |

## Cross-links

- Deploy config (SEC-H03): [`AGENTS_DEPLOY_CONFIG_VERIFY.md`](./AGENTS_DEPLOY_CONFIG_VERIFY.md) ([#441](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/441))
- Deploy trace (SEC-D12): [`AGENTS_DEPLOY_TRACE.md`](./AGENTS_DEPLOY_TRACE.md) ([#410](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/410))
- Frontend optional startup check: [`frontend-dapp/src/utils/deployAddressVerification.ts`](../frontend-dapp/src/utils/deployAddressVerification.ts) (`VITE_VERIFY_DEPLOY_ADDRESSES=true`)
- Launch checklist: [`docs/runbooks/launch-checklist.md`](../docs/runbooks/launch-checklist.md)
