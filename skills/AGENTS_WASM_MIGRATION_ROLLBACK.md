# Agent playbook: wasm migration rollback limitations (SEC-H05)

Use when implementing or verifying **operator guidance** for what can and cannot be reversed after a CosmWasm contract migration ([#443](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/443)).

## Canonical references

| Doc | Purpose |
|-----|---------|
| [docs/runbooks/wasm-admin-migration.md § Rollback and limitations](../docs/runbooks/wasm-admin-migration.md#rollback-and-limitations-sec-h05) | **Single** operator rollback/limitations section |
| [docs/runbooks/launch-checklist.md § Rollback / incident](../docs/runbooks/launch-checklist.md#rollback--incident) | Cross-link from launch runbook |
| [docs/testing.md § Manual rollback SQL](../docs/testing.md) | Indexer `.down.sql` under `indexer/migrations/revert/` |
| [docs/runbooks/wasm-admin-migration.md § SEC-C14](../docs/runbooks/wasm-admin-migration.md#automated-regression-sec-c14) | Forward migration state-preservation tests (not archived prior wasm) |
| `scripts/check_wasm_migration_rollback_docs.py` | Drift guard for SEC-H05 acceptance topics |

## Operator summary

1. **Reversible:** migrate back to prior `code_id` when prior wasm is still on chain **and** contract `admin` is intact (`terrad query wasm code <id>`).
2. **Irrecoverable:** admin cleared, prior `code_id` purged, or governance keys lost — pause/blacklist/comms per incident runbooks.
3. **Indexer DB:** separate from CosmWasm; manual `.down.sql` in `indexer/migrations/revert/` (not auto-run by `sqlx migrate`).
4. **Partial fleet:** stop further migrates, pause pairs if needed, revert migrated contracts when possible, align indexer/frontend versions.

## Verification

```bash
make check-wasm-migration-rollback-docs
make verify-issue-443
```

`make verify-issue-443` also runs `make test-contracts` (SEC-C14 migration rehearsal). No LocalTerra or Postgres required for the doc checks alone.

## Related

- Deploy trace after any migration reversal: [`AGENTS_DEPLOY_TRACE.md`](./AGENTS_DEPLOY_TRACE.md)
- Launch go/no-go gate: [`AGENTS_LAUNCH_GO_NO_GO.md`](./AGENTS_LAUNCH_GO_NO_GO.md)
- Emergency pause during partial migration: [`AGENTS_EMERGENCY_COMMANDS.md`](./AGENTS_EMERGENCY_COMMANDS.md)
