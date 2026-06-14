# Runbook: CW20 whitelist operations

Factory governance controls which CW20 **code IDs** may back pair assets. Whitelisting a malicious or non-standard template can desync reserves, break withdrawals, or enable trading DoS via hooks.

**Related:** [`docs/security-model.md`](../security-model.md) (Code ID whitelist), invariant **P2** in [`docs/contracts-security-audit.md`](../contracts-security-audit.md), [`docs/runbooks/launch-checklist.md`](launch-checklist.md).

---

## Prohibited templates

| Template | Risk | Policy |
|----------|------|--------|
| **Fee-on-transfer** | Pair credits reserves from declared `amount`, not balance deltas → `RESERVES` exceed on-chain balance (**P2**) | **Never whitelist.** Documented by `adversarial_token::fee_on_transfer_creates_reserve_imbalance`. |
| **Rebasing / balance-mutating** | Reserve accounting assumes static balances between messages | **Never whitelist** without explicit ADR and pair changes. |
| **Unaudited mint/burn hooks** | Supply manipulation, callback griefing | **Never whitelist** for production pairs. |

**On-chain mitigation is whitelist-only** — the pair does **not** reconcile balance deltas. Ops must verify code IDs **before** `AddWhitelistedCodeId`.

---

## Approved production templates (Terra Classic)

Verify on-chain `CodeInfo` before whitelisting. Known references:

| Template | Mainnet code ID (Columbus-5) | Notes |
|----------|------------------------------|-------|
| **Terraport / TerraSwap CW20** | Query `terra1ex0hjv3wurhj4wgup4jzlzaqj4av6xqd8le4etml7rg9rs207y4s8cdvrp` contract info or Terraport docs | Standard CW20; no fee-on-transfer. See [`docs/terraport.md`](../terraport.md). |
| **CL8Y DEX LP token** | Deploy-time `lp_token_code_id` from your factory `get_config` | Optimizer-built `cl8y-dex-cw20` artifact in this repo. |
| **CL8Y DEX pair asset CW20** | Deploy-time whitelisted IDs from factory | Must match `make build-optimized` checksums in CI. |

**GDEX** pair assets must use the same standard CW20 templates as above — not custom fee-on-transfer forks.

---

## Pre-whitelist verification

Use [`scripts/qa/verify-cw20-code-ids.sh`](../../scripts/qa/verify-cw20-code-ids.sh) against staging or mainnet LCD:

```bash
# Space-separated code IDs to verify (example)
bash scripts/qa/verify-cw20-code-ids.sh \
  --lcd https://terra-classic-lcd.publicnode.com \
  89 90
```

Manual checklist:

- [ ] `terrad query wasm code-info <CODE_ID>` — `creator` is expected governance/multisig.
- [ ] Wasm checksum matches optimizer output in [`smartcontracts/artifacts/checksums.txt`](../../smartcontracts/artifacts/checksums.txt) or approved release manifest.
- [ ] Sample token `token_info` — no transfer-tax fields in contract logic (review wasm or instantiate a test token).
- [ ] **Not** fee-on-transfer: deploy test pair on LocalTerra; `fee_on_transfer_creates_reserve_imbalance` pattern must **not** apply to production IDs.
- [ ] Record code ID, checksum, and approver in the launch ticket.

---

## Adding a code ID

```bash
terrad tx wasm execute <FACTORY> \
  '{"add_whitelisted_code_id":{"code_id":<ID>}}' \
  --from <gov> ...
```

Post-add: confirm `IsWhitelistedCodeId` query returns true; create a test pair on staging before mainnet pair creation.

---

## Incident: malicious ID whitelisted

1. **Pause** affected pairs via factory if trading must halt immediately.
2. **Do not** create new pairs with the bad asset.
3. Governance removes code ID: `RemoveWhitelistedCodeId` (existing pairs may remain — assess reserve desync per pair).
4. Communicate LP withdrawal risk if **P2** desync is present; see sweep runbook in security model.
