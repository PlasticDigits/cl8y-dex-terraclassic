# Deploy trace template (SEC-D12)

Copy this block into a comment on the **launch tracking issue** ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)) at the end of each production or staging deploy/migration. Replace placeholders; keep command output verbatim for audit.

**Why:** After a missed chain patch (e.g. Terra Classic IBC fix) or contract dependency update, operators must be able to identify exactly which git revision, wasm artifacts, code IDs, and chain version were live at deploy time ([GitLab #410](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/410)).

---

## Deploy trace — `<network>` — `<YYYY-MM-DD UTC>`

| Field | Value |
|-------|-------|
| **Operator** | `<name / role>` |
| **Governance multisig** | `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` |
| **Network / chain ID** | `<columbus-5 \| rebel-2 \| localterra \| …>` |
| **Deploy type** | `<initial launch \| migration \| hotfix>` |
| **Git SHA** | `<output of git rev-parse HEAD>` |
| **Git describe** (optional) | `<output of git describe --always --dirty>` |
| **Terra Classic chain version** | `<output of terrad version or terrad status --node <rpc> \| jq -r .node_info.version>` |
| **RPC / LCD** | `<endpoints used>` |

### Contract code IDs

| Contract | Code ID | Address (if instantiated) |
|----------|---------|---------------------------|
| Factory | | |
| Pair | | |
| Router | | |
| Fee-discount | | |
| Hook(s) | | |

### Wasm artifact hashes (`smartcontracts/artifacts/wasm-checksums.txt`)

```
<paste full wasm-checksums.txt contents>
```

### Test results (pre-deploy evidence — SEC-H08)

Record before production mainnet deploy (Phase 0). **CI-built artifacts:** link the GitLab pipeline for commit `<git-sha>` with green `test-contracts`, `test-indexer-integration`, and `test-frontend` jobs instead of re-pasting.

| Suite | Command | Evidence |
|-------|---------|----------|
| Contracts | `make test-contracts` | paste output or CI job link |
| Indexer integration | `make test-indexer-integration` | paste output or CI job link |
| Frontend unit | `make test-frontend` | paste output or CI job link |
| Pool swap smoke | `make smoke-pool-swap` | paste output (also in Phase 3 post-deploy) |

```bash
git rev-parse HEAD
make test-contracts
make test-indexer-integration
make test-frontend
```

**Test output (paste or link):**

```
<paste full test command output, or GitLab pipeline URL for commit <git-sha>>
```

### Post-deploy verification

```bash
# Scripted config assertions (SEC-H03 — governance, treasury, fee, whitelist, tiers, router, hooks, blacklist)
# scripts/qa/verify-deploy-config.sh
FACTORY_ADDRESS=<factory> ROUTER_ADDRESS=<router> FEE_DISCOUNT_ADDRESS=<fee_discount> \
  TERRA_LCD_URL=<lcd> make qa-verify-deploy-config

# Env/chain address cross-check (SEC-H04 — indexer vs frontend env + on-chain router factory wiring)
# scripts/qa/verify-env-addresses.sh
VERIFY_ENV_INDEXER_FILE=<indexer/.env> VERIFY_ENV_FRONTEND_FILE=<frontend/.env.production> \
  TERRA_LCD_URL=<lcd> make qa-verify-env-addresses

# Pool smoke (read-only)
PAIR_ADDR=<pair> TERRA_LCD_URL=<lcd> ./scripts/smoke-pool-swap.sh
```

**Verification output (paste or link):**

```
<paste full qa-verify-deploy-config + qa-verify-env-addresses output + smoke script output, or link to CI/log artifact>
```

### Notes

- `<optional: migration code IDs, risk acceptance links, indexer/frontend env pins>`

---

**Runbook gates:** [launch checklist Phase 1](../runbooks/launch-checklist.md#deploy-trace-audit-record--required-before-leaving-phase-1) · [wasm admin migration Pre-flight](../runbooks/wasm-admin-migration.md#pre-flight) · [deployment guide](../deployment-guide.md#deploy-trace-audit-record)

**Agent playbook:** [`skills/AGENTS_DEPLOY_TRACE.md`](../../skills/AGENTS_DEPLOY_TRACE.md) · [`skills/AGENTS_DEPLOY_CONFIG_VERIFY.md`](../../skills/AGENTS_DEPLOY_CONFIG_VERIFY.md) · [`skills/AGENTS_TEST_EVIDENCE_GATE.md`](../../skills/AGENTS_TEST_EVIDENCE_GATE.md) · [`skills/AGENTS_DEPLOY_ENV_ADDRESSES_VERIFY.md`](../../skills/AGENTS_DEPLOY_ENV_ADDRESSES_VERIFY.md)
