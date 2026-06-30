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
- [ ] **Top pools by liquidity:** Run [Quick pool triage (SEC-G03)](../runbooks/emergency-commands.md#quick-pool-triage-sec-g03) — indexer SQL on `pair_reserves` (preferred) or `GET /api/v1/pairs?sort=volume_24h&order=desc` (activity proxy). Record top `pair_address` values for mitigation.

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
- **Rollback vs forward-fix (deploy surfaces):** [Rollback decision runbook](../runbooks/rollback-decision.md) — frontend, indexer, contract, and chain dependency criteria, commands, limitations, and recovery verification (SEC-H09, GitLab [#445](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/445)).
- **Indexer stale / wrong data:** [Indexer reorg / replay runbook](../runbooks/indexer-reorg-replay-dedup.md), [Wasm admin runbook](../runbooks/wasm-admin-migration.md) if contract-side.
- **Reorg halt (hash mismatch):** Stop indexer → dry-run `make indexer-reorg-recover HEIGHT=H` or `./scripts/indexer-reorg-recover.sh --height H` (add `CLEANUP=1` / `--cleanup-derived` for true fork) → `APPLY=1` / `--apply` → restart. Shallow vs deep steps in [runbook § Reorg handling](../runbooks/indexer-reorg-replay-dedup.md#reorg-handling).

## Incident timeline

Add rows **as events unfold** (governance txs, blacklist/pause actions, rollback steps). Paste tx hashes from [Emergency commands](../runbooks/emergency-commands.md) and record user impact per [user incident FAQ](../user-incident-faq.md).

| UTC Time | Tx Hash | Wallet | Token | Pair | Admin Action | User Impact |
|----------|---------|--------|-------|------|--------------|-------------|
|          |         |        |       |      |              |             |

## Communications

During the incident, record what was sent below. For **copy-paste announcement text**, use [Appendix: Communications templates](#appendix-communications-templates-sec-g05) ([SEC-G05](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/438)).

- **Internal:** (paste sent internal comms or link)
- **Public:** (paste sent public comms, or `N/A`)

## Post-incident

- **Root cause:** 
- **Follow-ups:** (issues, docs, runbooks)

---

## Appendix: Communications templates (SEC-G05)

**Paste-ready templates** for operator communications during incidents. Fill bracketed placeholders before sending. Trader/LP **background** (not per-incident copy): [user-incident-faq.md](../user-incident-faq.md).

**Placeholders (all templates):**

| Placeholder | Example |
|-------------|---------|
| `[TIMESTAMP_UTC]` | `2026-06-30T14:00:00Z` |
| `[PAIR_ADDRESS]` | `terra1abc…` pair contract |
| `[WALLET_ADDRESS]` | `terra1xyz…` blacklisted wallet |
| `[TOKEN_ADDRESS]` | `terra1token…` CW20 contract |
| `[BLACKLIST_TARGET]` | wallet / token / pair address (pick one) |
| `[IMPACT_DESCRIPTION]` | What users cannot do; funds safety |
| `[ESTIMATED_RESOLUTION]` | Next update time or ETA to unpause/unblacklist |
| `[COMPLETED_ACTIONS]` | Mitigations already applied (postmortem / retraction) |
| `[CONTACT_CHANNEL]` | Status page, Discord `#announcements`, email, etc. |
| `[REASON_IF_DISCLOSABLE]` | Compliance or incident reason, or `not disclosed` |

### 1. Pair paused — user-facing announcement

**Public** (Discord / status page / X):

```text
CL8Y DEX — Trading paused for pool [PAIR_ADDRESS]

As of [TIMESTAMP_UTC], governance paused the trading pair at [PAIR_ADDRESS].

Impact: [IMPACT_DESCRIPTION]. Swaps, liquidity changes, and limit orders for this pool are temporarily disabled. Wallet balances, LP tokens, and limit escrow remain on-chain and are not seized.

Estimated next update: [ESTIMATED_RESOLUTION]

Questions: [CONTACT_CHANNEL]

Background: https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/blob/main/docs/user-incident-faq.md#pair-pause
```

**Internal** (Slack / incident channel):

```text
[INCIDENT] Pair pause — [TIMESTAMP_UTC]
Pair: [PAIR_ADDRESS]
Governance tx: <hash>
Commander: <name>
Impact: [IMPACT_DESCRIPTION]
Public notice sent: yes/no @ [TIMESTAMP_UTC]
Next check: [ESTIMATED_RESOLUTION]
Contact channel: [CONTACT_CHANNEL]
```

### 2. Blacklist applied — compliance notice

Use `[WALLET_ADDRESS]`, `[TOKEN_ADDRESS]`, or `[PAIR_ADDRESS]` as `[BLACKLIST_TARGET]` depending on scope.

**Public** (if disclosure is appropriate):

```text
CL8Y DEX — Trading restriction applied [TIMESTAMP_UTC]

Governance added [BLACKLIST_TARGET] to the protocol trading blacklist.

Reason: [REASON_IF_DISCLOSABLE]

Impact: [IMPACT_DESCRIPTION]. Affected addresses cannot swap, add/remove liquidity, or place/cancel/claim limit orders until the restriction is lifted. Token balances in wallets are not confiscated.

Estimated next update: [ESTIMATED_RESOLUTION]

Questions: [CONTACT_CHANNEL]

Background: https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/blob/main/docs/user-incident-faq.md#wallet-blacklist
```

**Internal:**

```text
[INCIDENT] Blacklist applied — [TIMESTAMP_UTC]
Target: [BLACKLIST_TARGET] (wallet | token | pair)
Governance tx: <hash>
Reason (internal): <full case file summary>
Disclosable reason sent publicly: [REASON_IF_DISCLOSABLE]
Impact: [IMPACT_DESCRIPTION]
Public notice sent: yes/no
Next check: [ESTIMATED_RESOLUTION]
```

### 3. Exploit under investigation — interim user notice

Send when controls are applied but root cause or scope is still unknown. Avoid naming unconfirmed attackers or exploit mechanics.

**Public:**

```text
CL8Y DEX — Security review in progress [TIMESTAMP_UTC]

We detected unusual activity affecting [IMPACT_DESCRIPTION] and applied protective measures (pair pause and/or trading restrictions as needed). User funds in wallets and escrow remain on-chain; affected actions are temporarily blocked while we investigate.

We are working with governance to confirm scope and next steps. This is an interim update — not a final postmortem.

Estimated next update: [ESTIMATED_RESOLUTION]

Questions: [CONTACT_CHANNEL]

Do not share exploit details publicly. Report security issues via SECURITY.md.
```

**Internal:**

```text
[INCIDENT] Exploit investigation — interim comms [TIMESTAMP_UTC]
Scope (internal): <pairs, wallets, suspected vector>
Controls applied: <pause/blacklist tx hashes>
Impact (public wording): [IMPACT_DESCRIPTION]
Interim public notice sent: yes/no
Next update deadline: [ESTIMATED_RESOLUTION]
Do not publish PoC or attacker addresses until IC approves.
```

### 4. False alarm retraction

Use after wrongful pause or blacklist; complete [false-positive rollback checklist](../runbooks/blacklist-decision.md#false-positive-rollback-unblacklist) before sending.

**Public:**

```text
CL8Y DEX — Restriction lifted [TIMESTAMP_UTC]

Earlier today we paused or restricted trading for [PAIR_ADDRESS or BLACKLIST_TARGET]. After further review, governance determined this was a false alarm.

Completed actions: [COMPLETED_ACTIONS] (e.g. pair unpaused, blacklist removed via governance tx <hash>).

Impact: [IMPACT_DESCRIPTION] — trading and withdrawals for affected users are restored. No user funds were seized at any time.

We apologize for the disruption. Questions: [CONTACT_CHANNEL]
```

**Internal:**

```text
[INCIDENT] False alarm retraction — [TIMESTAMP_UTC]
Original control: <pause/blacklist tx>
Rollback tx: <unpause/unblacklist tx>
Rollback checklist: complete (see incident timeline)
Root cause of false alarm: <summary>
Public retraction sent: yes/no
Follow-up docs issue: <link if criteria misfired>
```

### 5. Postmortem summary

Publish after incident is closed (S1/S2) or as agreed with IC. Link from status channel and archive in the incident record.

**Public:**

```text
CL8Y DEX — Incident postmortem [TIMESTAMP_UTC]

Summary: [IMPACT_DESCRIPTION] — what happened in plain language (no exploit recipe).

Timeline:
- Detection: <UTC>
- Mitigation: <UTC> — [COMPLETED_ACTIONS]
- Recovery: <UTC>

Affected users: <who was blocked, which pairs/tokens, duration>

User funds: No wallet balances were confiscated. Escrow/LP remained on-chain during restrictions.

Follow-up actions: <patches, monitoring, docs, governance changes> — target dates in [ESTIMATED_RESOLUTION] or linked issues.

Contact: [CONTACT_CHANNEL]
```

**Internal** (attach full technical timeline, tx hashes, and root cause):

```text
[POSTMORTEM] [TIMESTAMP_UTC]
Severity: S?
IC: <name>
Technical root cause: <detail>
Affected contracts/indexer: <list>
Governance txs: <hashes>
Lessons / action items: <tickets>
Public postmortem published: yes/no @ [TIMESTAMP_UTC]
```
