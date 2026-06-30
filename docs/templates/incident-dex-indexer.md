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

### Proactive anomaly signals (bootstrap TVL)

- [ ] **Anomaly checklist:** Review [Anomaly signals runbook](../runbooks/anomaly-signals.md) (SEC-G02, GitLab [#435](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/435)) — pool drain %, add/remove profit loop, route slippage deviation, failed tx burst, LCD-heavy 429 flood. Record which thresholds fired and first response taken.

### On-chain (DEX)

- [ ] **Chain health:** RPC/LCD reachable; block height advancing?
- [ ] **Contracts:** Pause state, factory config, router trusted on fee-discount; recent migrations?
- [ ] **Tx samples:** Failed tx codes, out-of-gas, hook reverts? → [Suspicious activity queries](../runbooks/suspicious-activity-queries.md) § 2 (LCD failed wasm) and § 2b (`hook_events`)

### Indexer

- [ ] **Process:** Indexer running?
- [ ] **DB:** `DATABASE_URL` reachable; migrations applied?
- [ ] **LCD:** Matches chain; `RUN_MODE=prod` uses correct `LCD_URLS`?
- [ ] **Logs:** `tracing` errors from LCD or parser; block timestamp fallback warnings (`Invalid block timestamp` / `Missing block timestamp`) — see [`docs/operator-secrets.md`](../operator-secrets.md), [GitLab #200](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/200).
- [ ] **Reorg halt:** `INDEXER_REORG_HALT` stderr line or `target=indexer_reorg_halt` in logs / webhook; note `height`, `stored_hash`, `canonical_hash`, `recovery_command` — dry-run [`indexer-reorg-recover.sh`](../../scripts/indexer-reorg-recover.sh) before `--apply` ([runbook](../runbooks/indexer-reorg-replay-dedup.md), [GitLab #362](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/362)).
- [ ] **Suspicious activity discovery:** Top traders, failed-tx wallets, pair/token spikes — [Suspicious activity queries runbook](../runbooks/suspicious-activity-queries.md) (SEC-G04, GitLab [#437](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/437)).

## Mitigation

- **Swap / contract issues:** Follow [Security model](../security-model.md) (pause, governance). **On-chain emergency controls:** copy-pastable factory commands in [Emergency commands runbook](../runbooks/emergency-commands.md) — pause/unpause pair, blacklist/unblacklist wallet, token, or pair ([SEC-B11](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/399)).
- **Trading blacklist (wallet / token / pair):** [Blacklist decision runbook](../runbooks/blacklist-decision.md) — confirmed exploit actor, malicious token, compromised pair, ToS escalation, and false-positive rollback checklist (SEC-B12, GitLab [#400](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/400)).
- **Indexer stale / wrong data:** [Indexer reorg / replay runbook](../runbooks/indexer-reorg-replay-dedup.md), [Wasm admin runbook](../runbooks/wasm-admin-migration.md) if contract-side.
- **Reorg halt (hash mismatch):** Stop indexer → dry-run `make indexer-reorg-recover HEIGHT=H` or `./scripts/indexer-reorg-recover.sh --height H` (add `CLEANUP=1` / `--cleanup-derived` for true fork) → `APPLY=1` / `--apply` → restart. Shallow vs deep steps in [runbook § Reorg handling](../runbooks/indexer-reorg-replay-dedup.md#reorg-handling).

## Incident timeline

Add rows **as events unfold** (governance txs, blacklist/pause actions, rollback steps). Paste tx hashes from [Emergency commands](../runbooks/emergency-commands.md) and record user impact per [user incident FAQ](../user-incident-faq.md).

| UTC Time | Tx Hash | Wallet | Token | Pair | Admin Action | User Impact |
|----------|---------|--------|-------|------|--------------|-------------|
|          |         |        |       |      |              |             |

## Communications

- **Internal:** 
- **Public:** (if any)

## Post-incident

- **Root cause:** 
- **Follow-ups:** (issues, docs, runbooks)
