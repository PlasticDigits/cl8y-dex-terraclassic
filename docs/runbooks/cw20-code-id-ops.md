# Runbook: CW20 code ID whitelist operations

Factory `CreatePair` accepts only CW20 tokens whose on-chain **code ID** is whitelisted. This blocks unknown wasm templates but **does not** prove token logic is safe ([#376](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/376) H-01).

**Related:** [security model § Code ID whitelist](../security-model.md), [launch checklist](./launch-checklist.md), adversarial test `fee_on_transfer_creates_reserve_imbalance`. **Adding** a code ID uses the #589 harness ([`cw20-codeid-audits/PROCEDURE.md`](../../cw20-codeid-audits/PROCEDURE.md), `make verify-issue-589`) — this runbook is post-listing freeze / refresh.

---

## Prohibited templates

**Never whitelist fee-on-transfer CW20 code IDs** (or any template that credits recipients less than the debited `amount` on `Transfer` / `Send`).

| Risk | Effect |
|------|--------|
| Pair credits reserves from declared `amount` | Internal reserves exceed actual CW20 balance → LP withdraw / pricing desync |
| Adversarial whitelist | Documented in `adversarial_token::fee_on_transfer_creates_reserve_imbalance` |

**Mitigation:** governance verifies production code IDs are **standard** (full-amount transfer) templates before `AddWhitelistedCodeId`. After listing, pair write paths **pin** the listing `code_id` and **re-check** factory whitelist (GitLab [#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582), **F6**) so a later `MsgMigrateContract` cannot trade as an unlisted / unpinned template.

**Do not** implement on-chain balance-delta reconciliation for this — prohibition + pin/re-check only (H-01 guardrail). Honest upgrades use `RefreshPairAssetCodeIds` after the new id is whitelisted. See [cw20-whitelist-policy.md](./cw20-whitelist-policy.md) and [`skills/AGENTS_CW20_CODE_ID_PIN.md`](../../skills/AGENTS_CW20_CODE_ID_PIN.md).

---

## Approved production templates (Terra Classic)

Before mainnet whitelist changes, confirm via LCD `wasm/code/<id>`:

| Template | Typical use | Verification |
|----------|-------------|--------------|
| **GDEX / project standard CW20** | CL8Y ecosystem tokens | Checksum matches audited release artifact |
| **TerraPort / TerraSwap CW20** | Ported TerraSwap-compatible tokens | `CodeInfo` checksum matches known TerraSwap CW20 reference |

Record the approved **code ID** and **checksum** in the deployment log when adding to the factory whitelist.

---

## Verification script

Use [`scripts/verify-whitelist-cw20-code-ids.sh`](../../scripts/verify-whitelist-cw20-code-ids.sh) before launch or after governance proposes new code IDs:

```bash
# Example: verify expected code IDs on columbus-5 LCD
LCD_URL=https://terra-classic-lcd.publicnode.com \
EXPECTED_CW20_CODE_IDS="1234,5678" \
EXPECTED_CW20_CHECKSUMS="abc...,def..." \
./scripts/verify-whitelist-cw20-code-ids.sh
```

The script queries `CodeInfo` for each ID and fails on checksum mismatch or query error.

---

## Launch checklist cross-check

- [ ] Every whitelisted code ID listed in factory config was verified with the script or manual `CodeInfo` query.
- [ ] No fee-on-transfer or experimental tax-token wasm in the whitelist.
- [ ] Pair creation on staging uses only tokens from verified code IDs.
- [ ] Factory **1.9.0** + pair **1.15.0** migrated **on chain** via [`scripts/upgrade-582-code-id-pin.sh`](../../scripts/upgrade-582-code-id-pin.sh) so listing-time pin + write-path re-check (**F6** / [#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582) / [#584](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/584)) is live, including `config.pair_code_id` on the new pair wasm. Columbus-5 **RAN 2026-08-21** (11602 / 11601). Merge of the contract MR is **not** enough.

```bash
terrad query wasm contract-state smart <factory> '{"get_config":{}}' --node <lcd>
# Confirm whitelisted_code_ids match deploy log
```

---

## Factory 1.9.0 → pair 1.15.0 migrate (GitLab #584)

**Order is factory first.** Pair 1.15.0 queries `IsCodeIdWhitelisted`. Against factory ≤1.8.0 that query fails (`unknown variant`) → `AssetCodeIdGuardUnavailable` on **every** gated write. The upgrade script **refuses** pair migrate unless factory cw2 ≥ 1.9.0 and the whitelist query parses.

```bash
# Read-only ContractInfo probe (columbus-5 LCD, every listed asset):
./scripts/qa/probe-columbus5-contract-info.sh

DRY_RUN=1 ./scripts/upgrade-582-code-id-pin.sh
UPGRADE582_LOCAL=1 ./scripts/upgrade-582-code-id-pin.sh
./scripts/upgrade-582-code-id-pin.sh
```

Pagination: `pairs` at **`limit: 30`** with `start_after` = last page `asset_infos`. After the loop, enumerated count **must equal** `GetPairCount`. If a pair is created between page 1 and page 2, the script **fails closed** — re-run (do not ignore the mismatch). Pre-flight `GET /cosmwasm/wasm/v1/contract/{addr}` must return a numeric `ContractInfo.code_id` for the factory and every listed asset.

After factory 1.9.0 migrate, **`UpdateConfig { pair_code_id }`** to the new pair wasm. Migrating existing pairs does **not** change what `CreatePair` instantiates. Columbus-5 2026-08-21 left `pair_code_id` on 11586 (1.14.0) until a follow-up tx — the script now does this before pair migrate. Retry after RPC RST is safe: pairs already on the target code id are skipped.

`IsCodeIdWhitelisted` / `GetAssetCodeIds` LCD reads are retried. A dropped second whitelist read is an **LCD flake**, not `pin1=null` (publicnode aborted smoke that way on 2026-08-21). Optional `UPGRADE582_REFRESH=1` parses wasm `has_more` / `next_start_after` and loops until `has_more=false` (do not assume one page).

Post-migrate smoke: every pair `GetAssetCodeIds` (hard-error on pre-1.15.0 — not “empty pins ok”) + `IsCodeIdWhitelisted` for both pins + one `HybridSimulation`. **Simulation succeeding is not “pair is tradable”** — execute paths stay gated; queries are ungated by design.

dApp + indexer visibility ([#585](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/585)): quotes can still appear; `route/solve` excludes frozen hops; pair APIs flag `code_id_frozen`. This does **not** replace on-chain write-path fail-closed. Playbook: [`AGENTS_FRONTEND_CODE_ID_FREEZE.md`](../../skills/AGENTS_FRONTEND_CODE_ID_FREEZE.md); `make verify-issue-585`.

Paste onto [#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391) / [#584](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/584): LCD ContractInfo probe output, factory cw2 before/after, pair count, migrate tx hashes, smoke table. Template: [deploy-trace.md](../templates/deploy-trace.md).

---

## Exit-path policy (keep — maximal freeze)

**Decision:** keep the current on-chain gate. Cancel, claim, and withdraw stay behind `gate_asset_code_ids` together with swap / provide / place / fill.

A third-party token wasm admin (the actor F6 defends against) can freeze **all** user value in a pair (LP underlying, maker escrow, parked claims) until governance whitelists a replacement template **and** refreshes pins. Users cannot self-rescue. Accounting would allow open exits under FoT (recipient bears outbound tax; pair-side `PENDING_ESCROW` / `RESERVES` stay consistent) — that is a **follow-up contract issue**, not silently done in the ops script.

**SLA target:** pause the incident pair (`SetPairPaused`) as soon as drift is confirmed; whitelist + refresh + private rebalance + unpause within the same incident window. There is no one-click protocol halt in the upgrade script.

Also: `GetAssetCodeIds` **hard-errors** on pre-1.15.0 pairs (not `null`). `CleanLimitBook` still parks during freeze while claims stay gated (stranded parked escrow until unfreeze + claim). `UpdateLimitOrderPrice` is ungated (no funds).

---

## Incident: code-id drift / freeze

1. Confirm drift: `GetAssetCodeIds` vs LCD `ContractInfo.code_id` per asset; `IsCodeIdWhitelisted` for **live** ids.
2. **`SetPairPaused { paused: true }` first** (keep paused through refresh). Do **not** refresh while the pair is unpaused if external price moved. Pause command: [emergency-commands.md § Pause](./emergency-commands.md#1-pause-a-pair).
3. Source-review replacement wasm. `AddWhitelistedCodeId` only if the new template is accepted (keep the old id listed until done). If the new wasm is FoT/rebase → **do not whitelist**; use `BlacklistToken` / `BlacklistPair`; **do not Refresh**. Pre-whitelist source review remains the only FoT-at-listing gate.
4. `RefreshPairAssetCodeIds` (single) for the incident pair. For many pairs: batch with the skip procedure below.
5. **Corrective arb / re-seed while still paused** (treasury/keeper; [`rebalance-mint-ust1-lp.sh`](../../scripts/rebalance-mint-ust1-lp.sh) pattern for hub pairs). First-swap-after-unpause vs stale TWAP/`OBSERVATIONS` / reserves is otherwise extractable (depth-limited). Refresh timing is MEV-sensitive.
6. `SetPairPaused { paused: false }` only after rebalance/smoke. Do **not** announce the unpause block in public chat before the txs land.
7. Never lead with `RemoveWhitelistedCodeId(10184)`. Thirteen of fourteen live assets share mintable **10184** — de-whitelist is a protocol halt. Milder controls: `SetPairPaused` / `BlacklistPair` / `BlacklistToken`.

### Batch refresh skip (`start_after`)

`RefreshPairAssetCodeIdsBatch` sends one execute per pair in a **single tx**. The incident pair whose token migrated to an **unlisted** id **must** fail refresh → whole batch reverts, blocking later-indexed pairs. `has_more` / `next_start_after` are wasm events. `start_after` is exclusive (`start_idx = start_after+1`); `limit` clamps to `[1,30]`.

Copy-paste example (good pair, bad pair at `PAIR_INDEX=4`, then batch past the bad index until `has_more=false`):

```bash
# Good pair (index 3): single refresh
terrad tx wasm execute "$FACTORY" \
  "$(jq -nc --arg p "$GOOD_PAIR" '{refresh_pair_asset_code_ids:{pair:$p}}')" \
  --from "$GOVERNANCE_KEY" --chain-id "$CHAIN_ID" --node "$NODE" --gas auto -y

# Bad pair (index 4): do not Refresh onto unlisted/FoT. Blacklist or new whitelist first.

# Batch remaining pairs: start_after=4 starts at index 5. Rerun until has_more=false.
terrad tx wasm execute "$FACTORY" \
  '{"refresh_pair_asset_code_ids_batch":{"start_after":4,"limit":30}}' \
  --from "$GOVERNANCE_KEY" --chain-id "$CHAIN_ID" --node "$NODE" --gas auto -y
```

The incident pair stays frozen until policy (blacklist vs new whitelist + single `RefreshPairAssetCodeIds`).

### Unfreeze (Chain A)

Pause stays **on** through `RefreshPairAssetCodeIds`. Private rebalance while paused. Then unpause. Do not treat “just Refresh” as the full unfreeze. Do not publish “we unpause at height H”.


```bash
terrad query wasm contract-state smart <factory> '{"get_config":{}}' --node <lcd>
# Confirm whitelisted_code_ids match deploy log
```
