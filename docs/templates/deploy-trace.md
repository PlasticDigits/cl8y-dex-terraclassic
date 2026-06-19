# Deploy trace template (SEC-D12)

Copy this block into a comment on the **launch tracking issue** ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)) at the end of each production or staging deploy/migration. Replace placeholders; keep command output verbatim for audit.

**Why:** After a missed chain patch (e.g. Terra Classic IBC fix) or contract dependency update, operators must be able to identify exactly which git revision, wasm artifacts, code IDs, and chain version were live at deploy time ([GitLab #410](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/410)).

---

## Deploy trace — `<network>` — `<YYYY-MM-DD UTC>`

| Field | Value |
|-------|-------|
| **Operator** | `<name / role>` |
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

### Post-deploy verification

```bash
# Factory config
terrad query wasm contract-state smart <factory> '{"get_config":{}}' --node <lcd>

# Pool smoke (read-only)
PAIR_ADDR=<pair> TERRA_LCD_URL=<lcd> ./scripts/smoke-pool-swap.sh
```

**Verification output (paste or link):**

```
<paste query + smoke output, or link to CI/log artifact>
```

### Notes

- `<optional: migration code IDs, risk acceptance links, indexer/frontend env pins>`

---

**Runbook gates:** [launch checklist Phase 1](../runbooks/launch-checklist.md#deploy-trace-audit-record--required-before-leaving-phase-1) · [wasm admin migration Pre-flight](../runbooks/wasm-admin-migration.md#pre-flight) · [deployment guide](../deployment-guide.md#deploy-trace-audit-record)

**Agent playbook:** [`skills/AGENTS_DEPLOY_TRACE.md`](../../skills/AGENTS_DEPLOY_TRACE.md)
