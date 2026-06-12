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
- [ ] **Reorg halt:** Log/webhook event `indexer_reorg_halt` / `INDEXER_REORG_HALT` with `stored_hash` ≠ `canonical_hash`? If yes → [reorg runbook](../runbooks/indexer-reorg-replay-dedup.md): dry-run `./scripts/indexer-reorg-recover.sh --height H`, apply with `--apply` (and `--cleanup-derived` if needed), restart indexer.

## Mitigation

- **Swap / contract issues:** Follow [Security model](../security-model.md) (pause, governance).
- **Indexer stale / wrong data:** [Indexer reorg / replay runbook](../runbooks/indexer-reorg-replay-dedup.md), [Wasm admin runbook](../runbooks/wasm-admin-migration.md) if contract-side.
- **Reorg recovery checklist:** Stop indexer → identify fork height `H` → `make indexer-reorg-recover HEIGHT=H` (dry-run) → `make indexer-reorg-recover HEIGHT=H APPLY=1` → verify `last_indexed_height` advances toward LCD tip → spot-check pair/candle API vs chain.

## Communications

- **Internal:** 
- **Public:** (if any)

## Post-incident

- **Root cause:** 
- **Follow-ups:** (issues, docs, runbooks)
