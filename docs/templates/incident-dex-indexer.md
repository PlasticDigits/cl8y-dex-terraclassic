# Incident response — DEX + indexer

**Copy this template** into your incident tracker and fill in as you go.

## Metadata

| Field | Value |
|-------|--------|
| Severity | S1 / S2 / S3 / S4 |
| Started (UTC) | |
| Commander / IC | |
| Status channels | |

## Summary

- **Symptom:** (user-visible, e.g. swaps fail, charts stale, API errors)
- **Scope:** On-chain DEX / indexer / both

## Triage

### On-chain (DEX)

- [ ] **Chain health:** RPC/LCD reachable; block height advancing?
- [ ] **Contracts:** Pause state, factory config, router trusted on fee-discount; recent migrations?
- [ ] **Tx samples:** Failed tx codes, out-of-gas, hook reverts?

### Indexer

- [ ] **Process:** Indexer running?
- [ ] **DB:** `DATABASE_URL` reachable; migrations applied?
- [ ] **LCD:** Matches chain; `RUN_MODE=prod` uses correct `LCD_URLS`?
- [ ] **Logs:** `tracing` errors from LCD or parser; block timestamp fallback warnings (`Invalid block timestamp` / `Missing block timestamp`) — see [`docs/operator-secrets.md`](../operator-secrets.md), [GitLab #200](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/200).
- [ ] **Reorg halt:** `INDEXER_REORG_HALT` stderr line or `target=indexer_reorg_halt` in logs — use `recovery_cmd` from JSON payload; dry-run [`indexer-reorg-recover.sh`](../../scripts/indexer-reorg-recover.sh) before `--apply` ([runbook](../runbooks/indexer-reorg-replay-dedup.md), GitLab #362).

## Mitigation

- **Swap / contract issues:** Follow [Security model](../security-model.md) (pause, governance).
- **Indexer stale / wrong data:** [Indexer reorg / replay runbook](../runbooks/indexer-reorg-replay-dedup.md) (shallow: `make indexer-reorg-recover HEIGHT=H` then `APPLY=1`; deep: snapshot restore), [Wasm admin runbook](../runbooks/wasm-admin-migration.md) if contract-side.

## Communications

- **Internal:** 
- **Public:** (if any)

## Post-incident

- **Root cause:** 
- **Follow-ups:** (issues, docs, runbooks)
