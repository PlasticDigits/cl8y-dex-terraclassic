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

Before governance adds a code ID, follow the **#589 harness** ([`cw20-codeid-audits/PROCEDURE.md`](../../cw20-codeid-audits/PROCEDURE.md), playbook [`skills/AGENTS_CW20_CODE_ID_AUDIT.md`](../../skills/AGENTS_CW20_CODE_ID_AUDIT.md)). A byte-identical optimizer rebuild is **not** required and is **not** a go/no-go input (optional appendix only).

1. **Fetch LCD wasm** — `cw20-codeid-audits/scripts/fetch-lcd-wasm.sh <id>`. SHA-256 **must** equal `CodeInfo.data_hash`. Fail closed on mismatch. No third-party mirror without that check.
2. **Decompile** that binary (`decompile-wasm.sh`; `wabt`). Do not skip decomp.
3. **Human audit** against [`cw20-codeid-audits/CATALOG.md`](../../cw20-codeid-audits/CATALOG.md) (every A–CH row).
4. **Automated suite** on **that wasm** — Layer A (token 1:1 / CW20 surface) + Layer B (DEX invariants **P1–P4**, **P10**, **R1–R4**, **L1–L3**, …). Known-bad **8654** / FoT mutants must **fail** 1:1 and **P2**. `LAYER_B_LT=1` must run [`layer-a-lcd.sh`](../../cw20-codeid-audits/scripts/layer-a-lcd.sh) + [`layer-b-lt.sh`](../../cw20-codeid-audits/scripts/layer-b-lt.sh) — not a stub ([#590](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/590)).
5. **Fill `codeids/<id>/REPORT.md`** from the report template. Explicit **go / no-go**. Approving the ID admits **every** instantiate of that wasm.

Do **not** whitelist on checksum match alone, on decomp “looks like cw20-base” alone, or on CertiK / Skynet **file** hashes. Staging 1:1 probes belong **inside** Layer A/B, not as a substitute. `make verify-issue-589`. Stacked post-merge: `make verify-issue-590`.

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

**Severity:** **High** for permissionless 6036+migrate (any issuer with wasm admin). Residual risk on protocol-admin 10184/6036 is **our-key / our-upgrade** — still fail-closed until Refresh. **#581 / 8266:** F6 is live on columbus-5. Dump crate is in the LCD wasm (strings/types/serde). **Go/no-go is [`cw20-codeid-audits/codeids/8266/REPORT.md`](../../cw20-codeid-audits/codeids/8266/REPORT.md)** under [#589](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/589) — LCD pin + decomp + catalogue + Layer A/B. An optimizer rebuild matching `953AD60C…` is an **optional appendix** only ([`audits/CW20-8266-581-hash-repro.md`](../../audits/CW20-8266-581-hash-repro.md)). CertiK file hashes are not `data_hash`. Whitelisting 8266 admits all instantiations of that template — accepted (LUNC CreatePair fee). Do not treat (E) as the 8266 decision. Do not `AddWhitelistedCodeId 8266` while that report is **NO-GO**.

**Tests:** `asset_code_id_pin_tests::*`; `make verify-issue-582`; `make verify-issue-584`. Playbook: [`skills/AGENTS_CW20_CODE_ID_PIN.md`](../../skills/AGENTS_CW20_CODE_ID_PIN.md). Invariant **F6** in [contracts-security-audit.md](../contracts-security-audit.md). Rollout script: [`scripts/upgrade-582-code-id-pin.sh`](../../scripts/upgrade-582-code-id-pin.sh) ([#584](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/584)).

**Exit-path policy (keep):** maximal freeze. Cancel / claim / withdraw stay gated with swap/provide/place. Opening exits is a separate contract change. Unfreeze is pause-through-refresh + private rebalance — [cw20-code-id-ops.md](./cw20-code-id-ops.md). Do not Refresh onto FoT. Do not de-whitelist **10184** by default.

### Listed-asset wasm admin inventory (2026-08-20)

Snapshot from [#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582) (live factory: **14** pairs, **13** unique assets). **None** of the currently listed assets have a third-party wasm admin — existing TVL is protocol-key upgrade risk, not an outside issuer. That does **not** close permissionless 6036 `CreatePair` → migrate, and it does **not** make SpaceUSD safe to list before **F6** is live.

| Token | code_id | wasm admin |
|-------|---------|------------|
| UST1, USTR, cLUNC, cUSTC | 10184 | wrap-stack CMM gov `terra1xsecn4…` |
| CL8Y + soft gems (EMBER/CORAL/…) | 10184 | DEX 2-of-3 `terra1zlmv2…` |
| PEARL, QUARTZ | 6036 | DEX 2-of-3 `terra1zlmv2…` |
| SpaceUSD (not listed) | 8266 | issuer `terra133n0pv8…` |

Re-query LCD `ContractInfo` (code_id + admin) before any 8266 go. This table is not a live probe. F6 is live; do not `AddWhitelistedCodeId 8266` until [`codeids/8266/REPORT.md`](../../cw20-codeid-audits/codeids/8266/REPORT.md) is **GO** (`make verify-issue-589`).
