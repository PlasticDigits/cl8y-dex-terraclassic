# Runbook: CW20 code ID whitelist policy

Operational policy for factory `whitelisted_code_ids` and pair asset tokens. Complements [security-model.md § Code ID whitelist](../security-model.md) and invariant **P2** in [contracts-security-audit.md](../contracts-security-audit.md).

Parent remediation: GitLab [#377](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/377) (**H-01**).

## Prohibited templates

**Never whitelist fee-on-transfer CW20 code IDs** (or any template that debits less than the declared `Transfer` / `Send` amount).

The pair credits internal `RESERVES` from declared CW20 amounts, not balance deltas. Fee-on-transfer tokens desync reserves from on-chain balances and can break withdrawals (see `adversarial_token::fee_on_transfer_creates_reserve_imbalance`).

| Allowed | Forbidden |
|---------|-----------|
| Standard CW20 (Terraport / GDEX-style) with 1:1 transfer semantics | Templates that skim on `transfer` / `send` |
| Protocol-issued LP tokens (factory `lp_token_code_id`) | “Tax on transfer” forks unless pair logic is redesigned |
| Audited mintable CW20 used in local deploy | Adversarial / unaudited wasm |

## Pre-whitelist verification

Before governance adds a code ID:

1. **Obtain the canonical wasm** from the token issuer (not a third-party mirror).
2. **LCD `CodeInfo`** — confirm `code_id`, checksum, and uploader match expectations.
3. **Instantiate probe** on staging — `Transfer` / `Send` amount must equal recipient balance delta (no fee skimming).
4. **Document** the approved code ID in your deployment record.

### GDEX / TerraPort production code IDs

Use [`scripts/verify-cw20-code-ids.sh`](../../scripts/verify-cw20-code-ids.sh) against mainnet or staging LCD:

```bash
# Example: verify known production code IDs (set env vars for your deployment)
export LCD_URL=https://terra-classic-lcd.publicnode.com
export EXPECTED_GDEX_CW20_CODE_ID=...
export EXPECTED_TERRAPORT_CW20_CODE_ID=...
bash scripts/verify-cw20-code-ids.sh
```

The script queries `CodeInfo` for each configured ID and prints checksum + uploader. **Manual sign-off:** operator confirms the template is standard (non fee-on-transfer) before calling factory `AddWhitelistedCodeId`.

Reference Terraport contract table: [terraport.md § Contracts](../terraport.md).

## Launch checklist cross-link

Phase 0 of [launch-checklist.md](./launch-checklist.md) includes a whitelist item — complete this runbook before mainnet whitelist updates.

## Related tests

```bash
cd smartcontracts && cargo test fee_on_transfer -- --nocapture
```

Expected: `fee_on_transfer_creates_reserve_imbalance` **still passes** (documents risk when a bad code ID is whitelisted).
