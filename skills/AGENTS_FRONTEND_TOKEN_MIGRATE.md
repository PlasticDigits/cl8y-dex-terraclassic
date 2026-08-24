# Agent playbook: Migrate Token adopt (GitLab #626)

Use when changing `/token/migrate`, the community-tax `migrate` foreign importer, catalog `GetMigrateOrigin` attest, or Terraport/GDEX LP copy after an in-place adopt.

Parent design: [#603](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/603) (keep open as the decision record). Template [#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592) ([`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md)). Create Token [#593](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593) ([`AGENTS_FRONTEND_CREATE_TOKEN.md`](./AGENTS_FRONTEND_CREATE_TOKEN.md)). Catalog [#594](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/594) ([`AGENTS_INDEXER_COMMUNITY_TOKENS.md`](./AGENTS_INDEXER_COMMUNITY_TOKENS.md)). F6 pin [#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582) ([`AGENTS_CW20_CODE_ID_PIN.md`](./AGENTS_CW20_CODE_ID_PIN.md)). ALPHA wrap vs drop [#558](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/558) stays open — this page does **not** replace POL wrap.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#626**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/626) | Impl + LP gate |
| [`docs/contracts-terraclassic.md` § migrate-adopt](../docs/contracts-terraclassic.md#migrate-adopt-gitlab-626) | Message shape + LP table |
| [`adopt.rs`](../smartcontracts/contracts/community-tax-token/src/adopt.rs) | Foreign importer |
| [`MigrateTokenPage.tsx`](../frontend-dapp/src/pages/MigrateTokenPage.tsx) | Retail page |
| [`communityTaxMigrate.ts`](../frontend-dapp/src/utils/communityTaxMigrate.ts) | Verdict + free-profile payload |
| [`community_tokens.rs`](../indexer/src/indexer/community_tokens.rs) | `GetMigrateOrigin` attest |

## Go / no-go (record before enabling a button)

| ID | Source | Status |
|----|--------|--------|
| **S3** | columbus-5 **6036** / **10184** | **go** — cw2 `crates.io:cw20-base` / `crates.io:cw20-mintable`, no `tax_map` |
| **S3-8266** | columbus-5 **8266** | **go** — cw2 `crates.io:terraport-token`; leftover `marketing_info` / `balance_at` unread; LP table 1:1 stay-1:1 |
| **S4** | columbus-5 **8654** (ALPHA) | **no-go** — page may load and show **not supported**. Do not whitelist. Do not wipe `tax_map` in this ticket. |

## Invariants **M626-1–M626-12**

1. **M626-1 — env gate.** `/token/migrate` + More-menu **Migrate Token** use the same `isCommunityTaxEnabled()` gate as Create Token (**C593-1**).
2. **M626-2 — free.** No 50 UST1, no `PayWithAnyToken`, no `?payee=` / `?manager=` / `?treasury=` prefill (**C593-10**).
3. **M626-3 — allowlist.** Contract allowlists cw2 ∈ `{cw20-base, cw20-mintable, terraport-token}`. `tax_map` prefix, unknown cw2, already-configured `cfg`/`feat` smash → revert, balances untouched.
4. **M626-4 — 8654.** Never `AddWhitelistedCodeId 8654`. Page shows **not supported** + S1/S2. No columbus-5 ALPHA `MsgMigrateContract` in this ticket.
5. **M626-5 — CMM admin.** Retail tx is `MsgMigrateContract` then `MsgUpdateAdmin` → CMM in one bundle. Source admin becomes **manager**. Manager cannot migrate.
6. **M626-6 — origin.** Write `CONFIG.launcher` to the official launcher. Write `GetMigrateOrigin`. Do **not** fake `launcher_tx`. Catalog attests CMM + origin + (`launcher_tx` **or** allowlisted migrate cw2).
7. **M626-7 — no mint/burn.** `total_supply` and balances stay. Source minter is revoked; MintControl stays off.
8. **M626-8 — caps.** Combined `max_*` ≤ 2500. Retail payload is tax-off zeros. Headroom without VariableRates is rejected.
9. **M626-9 — F6.** Page discloses CL8Y pairs freeze until governance `RefreshPairAssetCodeIds`. Token admin cannot Refresh. Do not Refresh a pair whose other asset is unlisted.
10. **M626-10 — Terraport/GDEX.** Do not `RegisterListedPair` those pair addrs. Honest templates stay 1:1 on external DEX. Extra-debit is CL8Y listed pairs only.
11. **M626-11 — already tax.** 11611 / 11619 (or LocalTerra tax store id) are **not** offered as adopt. Same-crate bump stays CMM ops (`MigrateMsg {}`).
12. **M626-12 — no Swap dump.** After success, link `/token/:addr/manage` only (**C593-11**).

## Terraport / GDEX LP (LCD 2026-08-24)

In-place adopt **keeps the CW20 address**. External DEX pairs are not F6-pinned.

| Source | Pair | Reserves (raw) | After adopt |
|--------|------|----------------|-------------|
| ALPHA **8654** `terra1x6e64…zysuxz` | Terraport ALPHA/LUNC `terra12u7kh…9e7p6` | ALPHA `25732882067035` / uluna `5603001027933` | **Not migrated by this ticket.** If S4 ever goes: `tax_map` gone ⇒ forward 1:1; historical 4.5% skim not unwound. |
| ALPHA **8654** | Terraport ALPHA/USTC `terra1jg2vu…wph` | uusd `54087298` / ALPHA `23466167250` | Same — do not RegisterListedPair. |
| **8266** Open `terra1qz56v…s74n3` | Terraport Open/LUNC `terra1uxr6m…nypyc` | Open `13056446286` / uluna `1733267547` | Address unchanged. 1:1 stay 1:1. `tax_map` query already unknown variant. |
| GDEX | — | — | No GDEX factory pin in-repo. Any GDEX pair keeps the CW20 address; still do not RegisterListedPair. |

LocalTerra analogue: a factory-whitelisted `cw20-mintable` gem (10184 analogue) adopts the same way; inbound Transfer to a CL8Y pair stays 1:1 (crate **P3**).

## Ops after adopt (not the retail button)

Whitelist already has **11619** → `SetPairPaused` on affected **CL8Y** pairs → `RefreshPairAssetCodeIds` (single; both live ids listed) → smoke extra-debit sell → unpause. Playbook: [`docs/runbooks/cw20-code-id-ops.md`](../docs/runbooks/cw20-code-id-ops.md).

## Verify

```bash
make verify-issue-626
make verify-issue-592
make verify-issue-593
make verify-issue-594
```

## Do not

- Whitelist **8654** to “save” Terraport LP.
- Skip `UpdateAdmin` (source admin could migrate off 11619).
- Offer adopt for 11611/11619 on the retail page.
- Turn hybrid off (**#596**).
- Change pair/router wasm.
