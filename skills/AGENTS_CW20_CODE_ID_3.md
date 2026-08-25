# Agent playbook: columbus-5 CW20 code ID 3 (GitLab #627)

Use when anyone wants to **adopt**, **factory-list**, or **append env** for columbus-5 **code_id 3** (MIR / TWD / VKR / WHALE / KUJI / Station-minted tokens).

This playbook is the **investigation record**. It is **not** a license to list or migrate. Intake process for other ids remains [`AGENTS_CW20_CODE_ID_AUDIT.md`](./AGENTS_CW20_CODE_ID_AUDIT.md). Retail adopt allowlist remains [`AGENTS_FRONTEND_TOKEN_MIGRATE.md`](./AGENTS_FRONTEND_TOKEN_MIGRATE.md).

## Verdict (do not relitigate from cw2 name)

| Track | Status | Why |
|-------|--------|-----|
| **(A) adopt** onto 11619 / 11626 / 11630 | **NO-GO** | cw2 `crates.io:cw20-base` matches `ALLOWED_SOURCE_CW2`. Item key is `\x00\x0atoken_info` (modern is unprefixed `token_info`); balances are CanonicalAddr. Crate fail-closes `AdoptMissingTokenInfo` / `AdoptLegacyLayout`. |
| **(B) factory list** | **NO-GO** | Cannot run A-lcd/B-lt on `interface_version_7`. Everybody + ≥34 900 instances (B13). |

Canonical report: [`cw20-codeid-audits/codeids/3/REPORT.md`](../cw20-codeid-audits/codeids/3/REPORT.md).

## Invariants **C627-1–C627-8**

1. **C627-1 — no env append.** Do **not** add `3` to Coolify / default `VITE_COMMUNITY_MIGRATE_CODE_IDS`. Do not special-case MIR/KUJI addresses.
2. **C627-2 — no factory list.** Do **not** `AddWhitelistedCodeId 3`. Checksum / “looks like cw20-base” is not a GO.
3. **C627-3 — cw2 ≠ layout.** Live cw2 `crates.io:cw20-base` (MIR reports `0.1.0`) is **not** proof the maps match 10184. 10184 also reports `cw20-base`; 6036 reports `terraswap-token`.
4. **C627-4 — fail-closed adopt.** `execute_adopt` reverts `AdoptLegacyLayout` on CanonicalAddr `balance` suffixes or `TOKEN_INFO` serde fail. That is **not** an importer and **not** a retail GO.
5. **C627-5 — no LocalTerra GO.** `interface_version_7` / cosmwasm-std 0.16.0 cannot be stored on current LocalTerra (`interface_version_8`). A-lcd/B-lt fail-closed is **NO-GO**, not a skip-pass.
6. **C627-6 — B13 blast radius.** Approving 3 admits every instantiate. Census lower bound **≥ 34 900**. Unlisting later F6-freezes every pair still pinned to 3.
7. **C627-7 — no sibling bundling.** 147 / 153 / 610 / 767 / 1603 / 1790 / 4254 / 5800 / GDEX 9788 / Terraswap 7395 are **not** this ticket. 8654 / `cw20-tax` stay on their own reports.
8. **C627-8 — no ops from this ticket.** Do not Open exits, `RefreshPairAssetCodeIds`, or `RegisterListedPair` as part of #627.

## Add a future migrate source (not 3)

Follow [`AGENTS_FRONTEND_TOKEN_MIGRATE.md`](./AGENTS_FRONTEND_TOKEN_MIGRATE.md) “Add a future migrate source”. For **3**, stop: both tracks are **NO-GO**. A CanonicalAddr → Addr importer is a **new issue**, then a new REPORT GO, then env append. Do not implement the importer “while here”.

## Verify

```bash
make verify-issue-627
```

## Do not

- Append `3` because adopt already allowlists `crates.io:cw20-base`.
- Treat Layer B-lt skip / store failure as green.
- Factory-list 3 to “save” Terraswap MIR/KUJI LP.
- Open #627 leftovers as a whitelist MR.

## Related

- [`AGENTS_FRONTEND_TOKEN_MIGRATE.md`](./AGENTS_FRONTEND_TOKEN_MIGRATE.md) — retail gate; **S3-code3 NO-GO**
- [`AGENTS_CW20_CODE_ID_AUDIT.md`](./AGENTS_CW20_CODE_ID_AUDIT.md) — #589 intake
- [`AGENTS_CW20_CODE_ID_PIN.md`](./AGENTS_CW20_CODE_ID_PIN.md) — F6
- [`docs/runbooks/cw20-whitelist-policy.md`](../docs/runbooks/cw20-whitelist-policy.md)
- [#626](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/626) / [#589](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/589) / [#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582) / [#377](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/377)
