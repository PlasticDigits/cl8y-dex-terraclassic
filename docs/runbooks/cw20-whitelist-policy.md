# Runbook: CW20 code ID whitelist policy

Operational policy for factory `whitelisted_code_ids` and pair asset tokens. Complements [security-model.md § Code ID whitelist](../security-model.md), invariant **P2**, and listed-token pin **F6** ([#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582)) in [contracts-security-audit.md](../contracts-security-audit.md).

Parent remediation: GitLab [#377](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/377) (**H-01**).

## Prohibited templates

**Never whitelist any CW20 code ID whose recipient balance can differ from the declared `Transfer` / `Send` amount.** Two distinct mechanics break this (GitLab [#448](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/448), SEC-I01 H05):

- **Fee-on-transfer / transfer-tax** — debits less than the declared amount on `transfer` / `send`.
- **Rebase / elastic-supply / balance-mutating** — the holder's balance changes *after* receipt (supply rebase, interest accrual, reflection) independent of any transfer.

Both desync the pair's internal accounting. The pair (and limit-order escrow) credit declared CW20 amounts, **not** balance deltas, so a token that later reports a different balance leaves `RESERVES` / `PENDING_ESCROW` over- or under-backed versus the real balance, breaking withdrawals and escrow refunds (see `adversarial_token::fee_on_transfer_creates_reserve_imbalance`; the same imbalance arises from a post-receipt rebase).

| Allowed | Forbidden |
|---------|-----------|
| Standard CW20 (Terraport / GDEX-style) with 1:1 transfer semantics | Templates that skim on `transfer` / `send` |
| Protocol-issued LP tokens (factory `lp_token_code_id`) | “Tax on transfer” forks unless pair logic is redesigned |
| Audited mintable CW20 used in local deploy | Rebase / elastic-supply / reflection tokens |
| Fixed-supply, balance-stable CW20 | Adversarial / unaudited wasm |

## Pre-whitelist verification

Before governance adds a code ID:

1. **Obtain the canonical wasm** from the token issuer (not a third-party mirror).
2. **LCD `CodeInfo`** — confirm `code_id`, checksum, and uploader match expectations.
3. **Instantiate probe** on staging — `Transfer` / `Send` amount must equal recipient balance delta (no fee skimming).
4. **Source review** — confirm the template implements no fee-on-transfer, transfer tax, rebase / elastic supply, reflection, or any mechanic that mutates a holder's balance outside an explicit transfer. A balance held flat across a block (no transfer) must not change.
5. **Attach audit evidence** — record the source review or third-party audit reference alongside the approved code ID in the deployment record; do not whitelist on checksum match alone.

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

## Post-listing migrate (GitLab [#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582))

CreatePair-only whitelist is **not** enough: instance wasm admin can `MsgMigrateContract` onto fee-on-transfer / rebase wasm after listing. On-chain mitigations (chosen; not indexer watch, not “refuse 8266”):

| Control | Behavior |
|---------|----------|
| **(B) Pin** | Pair stores listing-time `code_id`s (`GetAssetCodeIds`). Live id must match. |
| **(A) Whitelist re-check** | Write paths query factory `IsCodeIdWhitelisted`. Removing a code id freezes pairs still pinned to it. |

**Honest token upgrade:** `AddWhitelistedCodeId` (new template) → migrate instances → governance `RefreshPairAssetCodeIds` (or Batch) → optional `RemoveWhitelistedCodeId` (old template). Refresh **refuses** to pin an unlisted live id.

**Severity:** **High** for permissionless 6036+migrate (any issuer with wasm admin). Residual risk on protocol-admin 10184/6036 is **our-key / our-upgrade** — still fail-closed until Refresh. **#581 / 8266:** listing is allowed only after factory **1.9.0** + pair **1.15.0** (this control) are live on the target factory, **or** SpaceUSD wasm admin is cleared, **or** wrap-to-10184. Do not treat (E) as the 8266 decision.

**Tests:** `asset_code_id_pin_tests::*`; `make verify-issue-582`. Playbook: [`skills/AGENTS_CW20_CODE_ID_PIN.md`](../../skills/AGENTS_CW20_CODE_ID_PIN.md). Invariant **F6** in [contracts-security-audit.md](../contracts-security-audit.md).

### Listed-asset wasm admin inventory (2026-08-20)

Snapshot from [#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582) (live factory: **14** pairs, **13** unique assets). **None** of the currently listed assets have a third-party wasm admin — existing TVL is protocol-key upgrade risk, not an outside issuer. That does **not** close permissionless 6036 `CreatePair` → migrate, and it does **not** make SpaceUSD safe to list before **F6** is live.

| Token | code_id | wasm admin |
|-------|---------|------------|
| UST1, USTR, cLUNC, cUSTC | 10184 | wrap-stack CMM gov `terra1xsecn4…` |
| CL8Y + soft gems (EMBER/CORAL/…) | 10184 | DEX 2-of-3 `terra1zlmv2…` |
| PEARL, QUARTZ | 6036 | DEX 2-of-3 `terra1zlmv2…` |
| SpaceUSD (not listed) | 8266 | issuer `terra133n0pv8…` |

Re-query LCD `ContractInfo` (code_id + admin) before any 8266 go. This table is not a live probe.
