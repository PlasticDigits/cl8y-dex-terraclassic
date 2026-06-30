# Agent playbook: rollback and forward-fix decision tree (SEC-H09)

Use when adding or changing **operator criteria** for rollback vs hotfix during incidents across frontend, indexer, contract, and chain-dependency surfaces — not user-facing FAQ copy ([GitLab **#445**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/445)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/runbooks/rollback-decision.md](../docs/runbooks/rollback-decision.md) | **Single** rollback/forward-fix decision tree for four incident types |
| [docs/runbooks/launch-checklist.md](../docs/runbooks/launch-checklist.md) | Launch gate — rollback section links here |
| [docs/runbooks/wasm-admin-migration.md](../docs/runbooks/wasm-admin-migration.md) | Contract migrate forward path; links rollback criteria |
| [docs/runbooks/emergency-commands.md](../docs/runbooks/emergency-commands.md) | On-chain pause/blacklist during contract/chain incidents |
| [docs/templates/incident-dex-indexer.md](../docs/templates/incident-dex-indexer.md) | Incident tracker — Mitigation + [#incident-timeline](../docs/templates/incident-dex-indexer.md#incident-timeline) |
| `scripts/check_rollback_decision_docs.py` | Drift guard for SEC-H09 acceptance topics + cross-links |

## Rules of thumb

1. **Classify blast radius first** — on-chain fund risk → pause before off-chain rollback.
2. **Frontend rollback** restores static artifacts; it does **not** undo user txs.
3. **Indexer rollback** may need paired `down.sql` under `indexer/migrations/revert/` — see [docs/testing.md § Manual rollback SQL](../docs/testing.md).
4. **Contract migrate-back** requires prior `code_id` on chain + compatible state — prefer pause + forward-fix when uncertain.
5. **Chain dependency** has no operator chain rollback — failover LCD/RPC and pause trading until network patch.
6. **Do not** duplicate the full tree in `security-model.md` or the user FAQ — **link** to the runbook.

## Verification

```bash
make check-rollback-decision-docs
make verify-issue-445
```

## Related

- Blacklist / pause (exploit mitigation): [`AGENTS_BLACKLIST_DECISION.md`](./AGENTS_BLACKLIST_DECISION.md), [`AGENTS_EMERGENCY_COMMANDS.md`](./AGENTS_EMERGENCY_COMMANDS.md)
- Indexer reorg recovery: [`AGENTS_INDEXER_INGESTION_HARDENING.md`](./AGENTS_INDEXER_INGESTION_HARDENING.md)
- IBC-hooks chain gate: [`AGENTS_IBC_HOOKS_DEPLOY.md`](./AGENTS_IBC_HOOKS_DEPLOY.md)
- Launch go/no-go: [`AGENTS_LAUNCH_GO_NO_GO.md`](./AGENTS_LAUNCH_GO_NO_GO.md)
