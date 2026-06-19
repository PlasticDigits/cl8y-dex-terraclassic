# Agent playbook: deploy trace recording (SEC-D12)

Use when implementing or verifying **deploy-time audit records** so operators can identify which code and chain version were live after a missed security patch ([#410](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/410)).

## Canonical references

| Doc | Purpose |
|-----|---------|
| [docs/templates/deploy-trace.md](../docs/templates/deploy-trace.md) | **Copy-paste template** for launch tracking issue comments |
| [docs/runbooks/launch-checklist.md § Phase 1](../docs/runbooks/launch-checklist.md#deploy-trace-audit-record--required-before-leaving-phase-1) | Required checklist before leaving deploy Phase 1 |
| [docs/runbooks/wasm-admin-migration.md § Pre-flight](../docs/runbooks/wasm-admin-migration.md#pre-flight) | Same recording items for migrations/upgrades |
| [docs/deployment-guide.md § Deploy trace](../docs/deployment-guide.md#deploy-trace-audit-record) | Narrative deploy guide + post-deploy gate |
| [docs/supply-chain-security.md](../docs/supply-chain-security.md) | Wasm artifact integrity — trace complements CI checksums |
| `scripts/check_deploy_trace_docs.py` | Drift guard for SEC-D12 acceptance topics |

## Required fields (every deploy/migration)

1. **Git SHA** — `git rev-parse HEAD` from the tree that produced uploaded wasm
2. **Terra Classic chain version** — `terrad version` or `terrad status --node <rpc> | jq -r .node_info.version`
3. **Contract code IDs** — factory, pair, router, fee-discount (and hooks if applicable)
4. **`wasm-checksums.txt`** — full contents from `smartcontracts/artifacts/`
5. **Post-deploy verification output** — at minimum factory `get_config` and `scripts/smoke-pool-swap.sh`

Post the completed trace on the **launch tracking issue** ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)).

## Verification

```bash
make check-deploy-trace-docs
make verify-issue-410
```

No LocalTerra or Postgres required.

## Related

- Launch go/no-go gate: [`AGENTS_LAUNCH_GO_NO_GO.md`](./AGENTS_LAUNCH_GO_NO_GO.md)
- Supply-chain wasm policy: [`AGENTS_SUPPLY_CHAIN_SECURITY.md`](./AGENTS_SUPPLY_CHAIN_SECURITY.md)
- Redeploy decision after code changes: [`AGENTS_QA_REDEPLOY_DECISION.md`](./AGENTS_QA_REDEPLOY_DECISION.md)
