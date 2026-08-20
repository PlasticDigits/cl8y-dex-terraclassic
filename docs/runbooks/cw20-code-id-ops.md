# Runbook: CW20 code ID whitelist operations

Factory `CreatePair` accepts only CW20 tokens whose on-chain **code ID** is whitelisted. This blocks unknown wasm templates but **does not** prove token logic is safe ([#376](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/376) H-01).

**Related:** [security model § Code ID whitelist](../security-model.md), [launch checklist](./launch-checklist.md), adversarial test `fee_on_transfer_creates_reserve_imbalance`.

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
- [ ] Factory **1.9.0** + pair **1.15.0** migrated so listing-time pin + write-path re-check (**F6** / [#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582)) is live.

```bash
terrad query wasm contract-state smart <factory> '{"get_config":{}}' --node <lcd>
# Confirm whitelisted_code_ids match deploy log
```
