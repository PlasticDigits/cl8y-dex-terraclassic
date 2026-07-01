# Agent playbook: post-deploy config verification (SEC-H03)

Use when verifying **on-chain factory / fee-discount / router configuration** after deploy or migration ([#441](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/441)).

## Problem (invariant Q2)

| ID | Invariant |
|----|-----------|
| **Q2** | After deploy, governance, treasury, default fee, whitelisted CW20 code IDs, fee-discount tiers, trusted router status, pair hooks, and blacklist clean-wallet probe must be queryable and match expected values. A single script runs all checks and exits non-zero on failure. |

Parent schema/stamp check: invariant **Q1** in [`AGENTS_QA_DEPLOY_VERIFY.md`](./AGENTS_QA_DEPLOY_VERIFY.md) ([#203](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203)).

## Automated check

| Step | Command |
|------|---------|
| Config assertions | **`make qa-verify-deploy-config`** → [`scripts/qa/verify-deploy-config.sh`](../scripts/qa/verify-deploy-config.sh) |
| Doc drift | **`make check-deploy-config-docs`** |
| Issue acceptance | **`make verify-issue-441`** |

### Queries asserted

1. Factory **`config`** — governance, treasury, `default_fee_bps` non-empty / positive
2. Factory **`get_whitelisted_code_ids`** — at least one code ID
3. Fee-discount **`get_tiers`** — at least one tier
4. Fee-discount **`is_trusted_router`** — `is_trusted=true` for configured router
5. First dual-CW20 pair **`get_hooks`** — reports hook list (pool-only default: none)
6. Factory **`blacklist_check`** — known-clean wallet not blacklisted / not blocked

### Optional expected values (mainnet / staging)

Set before running when values must match exactly:

```bash
export VERIFY_CONFIG_EXPECT_GOVERNANCE=terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7
export VERIFY_CONFIG_EXPECT_TREASURY=terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7
export VERIFY_CONFIG_EXPECT_DEFAULT_FEE_BPS=180
export VERIFY_CONFIG_EXPECT_HOOK_COUNT=0   # pool-only launch
export VERIFY_CONFIG_CLEAN_WALLET=terra1...  # address known not on blacklist (defaults to LocalTerra test1)
```

## Release sign-off

Paste full script output on the **launch / release tracking issue** ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)). Required in [launch checklist Phase 3](../docs/runbooks/launch-checklist.md#phase-3--post-deploy-verification-pool-only) and [deploy trace template](../docs/templates/deploy-trace.md).

## One-command repro

```bash
make deploy-local
make qa-verify-deploy        # Q1 schema + stamp
make qa-verify-deploy-config  # Q2 config assertions
```

## Files

| Path | Role |
|------|------|
| [`scripts/qa/verify-deploy-config.sh`](../scripts/qa/verify-deploy-config.sh) | Live LCD config verification |
| [`scripts/qa/test-verify-deploy-config.sh`](../scripts/qa/test-verify-deploy-config.sh) | Fixture unit tests |
| [`scripts/check_deploy_config_docs.py`](../scripts/check_deploy_config_docs.py) | Doc drift guard |
| [`docs/qa-invariants.md`](../docs/qa-invariants.md) | Invariant **Q2** |

## Cross-links

- Deploy config (SEC-H03): [`AGENTS_DEPLOY_CONFIG_VERIFY.md`](./AGENTS_DEPLOY_CONFIG_VERIFY.md) ([#441](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/441))
- Env/chain addresses (SEC-H04): [`AGENTS_DEPLOY_ENV_ADDRESSES_VERIFY.md`](./AGENTS_DEPLOY_ENV_ADDRESSES_VERIFY.md) ([#442](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/442))
- Launch checklist: [`docs/runbooks/launch-checklist.md`](../docs/runbooks/launch-checklist.md)
