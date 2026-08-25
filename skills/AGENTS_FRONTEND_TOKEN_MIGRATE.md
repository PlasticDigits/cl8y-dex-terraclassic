# Agent playbook: Migrate Token adopt (GitLab #626)

Use when changing `/token/migrate`, the community-tax `migrate` foreign importer, catalog `GetMigrateOrigin` attest, or Terraport/GDEX LP copy after an in-place adopt.

Parent design: [#603](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/603) (keep open as the decision record). Template [#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592) ([`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md)). Create Token [#593](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593) ([`AGENTS_FRONTEND_CREATE_TOKEN.md`](./AGENTS_FRONTEND_CREATE_TOKEN.md)). Catalog [#594](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/594) ([`AGENTS_INDEXER_COMMUNITY_TOKENS.md`](./AGENTS_INDEXER_COMMUNITY_TOKENS.md)). F6 pin [#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582) ([`AGENTS_CW20_CODE_ID_PIN.md`](./AGENTS_CW20_CODE_ID_PIN.md)). ALPHA wrap vs drop [#558](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/558) stays open — this page does **not** replace POL wrap. Post-merge leftover live after !418: [#628](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/628) ([`AGENTS_POST_MERGE_OPS_628.md`](./AGENTS_POST_MERGE_OPS_628.md); `make verify-issue-628`).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#626**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/626) | Impl + LP gate |
| [`docs/contracts-terraclassic.md` § migrate-adopt](../docs/contracts-terraclassic.md#migrate-adopt-gitlab-626) | Message shape + LP table |
| [`adopt.rs`](../smartcontracts/contracts/community-tax-token/src/adopt.rs) | Foreign importer |
| [`MigrateTokenPage.tsx`](../frontend-dapp/src/pages/MigrateTokenPage.tsx) | Retail page — title + one lead sentence; no env-var / 50 UST1 / cw2 essays (**#489**) |
| [`communityTaxMigrate.ts`](../frontend-dapp/src/utils/communityTaxMigrate.ts) | Verdict + free-profile payload |
| [`community_tokens.rs`](../indexer/src/indexer/community_tokens.rs) | `GetMigrateOrigin` attest |

## Go / no-go (record before enabling a button)

| ID | Source | Status |
|----|--------|--------|
| **S3** | columbus-5 **10184** | **go** — cw2 `crates.io:cw20-mintable` (LocalTerra mintable writes `cw20-base`) |
| **S3-6036** | columbus-5 **6036** | **page-go / chain-revert** — live instance cw2 is `crates.io:terraswap-token` (LCD 2026-08-25, [#628](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/628) leftover). Retail allowlist still includes 6036; `adopt.rs` will revert until a follow-up crate change. Do **not** append `terraswap-token` from #628. |
| **S3-8266** | columbus-5 **8266** | **go** — cw2 `crates.io:terraport-token`; leftover `marketing_info` / `balance_at` unread; LP table 1:1 stay-1:1 |
| **S4** | columbus-5 **8654** (ALPHA) | **go** — wipe `tax_info` / `tax_map` / `whale_info`; map 4.5% / 1% → buy 450 / sell 100. **Never** `AddWhitelistedCodeId 8654` (listing 11619 covers the address). |

## Invariants **M626-1–M626-12**

1. **M626-1 — env gate.** `/token/migrate` + More-menu **Migrate Token** use the same `isCommunityTaxEnabled()` gate as Create Token (**C593-1**).
2. **M626-2 — free.** No 50 UST1, no `PayWithAnyToken`, no `?payee=` / `?manager=` / `?treasury=` prefill (**C593-10**).
3. **M626-3 — migrate allowlist.** Retail gate is `VITE_COMMUNITY_MIGRATE_CODE_IDS` (default **6036, 10184, 8266, 8654**). This is **not** factory `AddWhitelistedCodeId`. 8654 is a normal list entry. Add future source ids by appending the env (Coolify) — do not special-case addresses. Contract still allowlists cw2 ∈ `{cw20-base, cw20-mintable, terraport-token, cw20-taxed}`. Unknown cw2 / `cfg`/`feat` smash → revert. Leftover `tax_info` / `tax_map` / `whale_info` are wiped on any allowlisted source.
4. **M626-4 — pair-asset whitelist stays separate.** Never `AddWhitelistedCodeId 8654` (H-01). After adopt, the current listed tax pin covers the address (**11626** was the #628 store; live pin is **11630** — [#628](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/628)). New migrate sources do **not** need to be factory-listed.
5. **M626-5 — CMM admin.** Retail tx is `MsgMigrateContract` then `MsgUpdateAdmin` → CMM in one bundle. Source admin becomes **manager**. Manager cannot migrate.
6. **M626-6 — origin.** Write `CONFIG.launcher` to the official launcher. Write `GetMigrateOrigin`. Do **not** fake `launcher_tx`. Catalog attests CMM + origin + (`launcher_tx` **or** allowlisted migrate cw2).
7. **M626-7 — no mint/burn.** `total_supply` and balances stay. Source minter is revoked; MintControl stays off.
8. **M626-8 — caps.** Combined `max_*` ≤ 2500. Honest adopt payload is tax-off zeros. Leftover FoT maps 4.5%/1% → buy 450 / sell 100. Headroom without VariableRates is rejected.
9. **M626-9 — F6.** Page discloses CL8Y pairs pause until governance refreshes them (`RefreshPairAssetCodeIds`). Token admin cannot Refresh. Do not Refresh a pair whose other asset is unlisted. Human words only — no `tax_info` / `RegisterListedPair` / `VITE_*` on the card.
10. **M626-10 — Terraport/GDEX.** Do not `RegisterListedPair` those pair addrs. Honest templates stay 1:1 on external DEX. Extra-debit is CL8Y listed pairs only. Adopt does **not** register existing CL8Y pairs either — Manage catch-up / permissionless register does ([#633](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/633) **R633-6** / [`AGENTS_COMMUNITY_TAX_AUTOREGISTER.md`](./AGENTS_COMMUNITY_TAX_AUTOREGISTER.md)). Adopt leftovers (venue inventory + post-refresh register tool) live on `/token/migrate` ([#634](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/634) **M634**).
11. **M626-11 — already tax.** 11611 / 11619 (or LocalTerra tax store id) are **not** offered as adopt. Same-crate bump stays CMM ops (`MigrateMsg {}`).
12. **M626-12 — no Swap dump.** After success, link `/token/:addr/manage` only (**C593-11**).

## Invariants **M634-1–M634-8** (migrate pair inventory)

Sibling of **M626**. Playbook for `/token/migrate` venue list + post-adopt register tool ([#634](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/634)). Helper: [`communityTaxMigratePairs.ts`](../frontend-dapp/src/utils/communityTaxMigratePairs.ts). Card: [`MigratePairInventory.tsx`](../frontend-dapp/src/components/community/MigratePairInventory.tsx).

1. **M634-1 — inventory on load.** Loading a token shows CL8Y factory pairs (0–n), Terraport rows when discoverable or statically known (ALPHA/Open table), and a GDEX/unknown-external instruction when no GDEX inventory exists.
2. **M634-2 — copy.** CL8Y pools pause until governance refresh; the manager cannot refresh; other DEX stays 1:1 and must not be registered. Retail card has no wasm / env / code-id essays. Optional “copy details for governance” may include addrs + operator `RefreshPair` language.
3. **M634-3 — no Refresh / pause / whitelist.** This page never sends `RefreshPairAssetCodeIds`, `SetPairPaused`, or `AddWhitelistedCodeId`. Do not expose those executes. Do not tell the user to Refresh a pair whose other asset is unlisted — create a new CL8Y pair versus a listed token instead.
4. **M634-4 — register gate.** Register CTA only for a **factory-verified CL8Y** pair, only after adopt, only when F6 pins match (or the pair is new/unfrozen), only for the connected manager + CMM admin + tax pin (**C593-8**). Terraport/GDEX never get a register button. Reuse `registerListedPair` / `verifyFactoryListedPair` — do not fork #633 highest-LP across venues. Do not batch register into the adopt tx. Half-migrated (wasm moved, admin not CMM) shows unverified-admin and does not register.
5. **M634-5 — known Terraport + degrade.** ALPHA / Open known Terraport pairs always appear when that token is loaded. Failed Terraport factory query keeps static rows + honest “could not list every other-DEX pool”; the page stays submitable.
6. **M634-6 — empty CL8Y.** No CL8Y pair is a valid empty state + **Create Pair** → `/create` (no query prefill). Not an error. New CL8Y pools register on create (#633).
7. **M634-7 — success + query params.** Success card repeats the inventory + ordered checklist. Primary next step is Manage (**M626-12**). `?token=` / `?addr=` / `?pair=` / `?payee=` do not prefill or retarget register.
8. **M634-8 — incomplete inventory is OK.** GDEX is instruction-only (no factory pin). Indexer miss falls back to CL8Y factory pagination / `Pair` probe. Catalog listing is not F6 / whitelist. Do not invent pairs.

## Terraport / GDEX LP (LCD 2026-08-24)

In-place adopt **keeps the CW20 address**. External DEX pairs are not F6-pinned.

| Source | Pair | Reserves (raw) | After adopt |
|--------|------|----------------|-------------|
| ALPHA **8654** `terra1x6e64…zysuxz` | Terraport ALPHA/LUNC `terra12u7khzrzn05a73xkpq6a5zrcazz2xmqn7lvupmqmca06pgcyt5qsa9e7p6` | ALPHA `25732882067035` / uluna `5603001027933` | After wipe: forward 1:1. Historical 4.5% skim not unwound. Do not RegisterListedPair. |
| ALPHA **8654** | Terraport ALPHA/USTC `terra1jg2vu97ssz2ldn6gztyl4fp9lfdtc23ffr65l4gpvuxw4znkmpxsja5wph` | uusd `54087298` / ALPHA `23466167250` | Same wipe → forward 1:1. Do not RegisterListedPair. |
| **8266** Open `terra1qz56v6p8ca3hh34wnj5yc3jykmw6jaaal0ukecscq8m9qqtgztnscs74n3` | Terraport Open/LUNC `terra1uxr6m55wxez5csnttz00893zur6pksn54nwlpye0c2pyuyyqp3qqknypyc` | Open `13056446286` / uluna `1733267547` | Address unchanged. 1:1 stay 1:1. `tax_map` query already unknown variant. |
| GDEX | — | — | No GDEX factory pin in-repo. Any GDEX pair keeps the CW20 address; still do not RegisterListedPair. |

LocalTerra analogue: a factory-whitelisted `cw20-mintable` gem (10184 analogue) adopts the same way; inbound Transfer to a CL8Y pair stays 1:1 (crate **P3**). That wasm writes cw2 `crates.io:cw20-base` — `GetMigrateOrigin.source_cw2` after adopt is `cw20-base`, not `cw20-mintable`. Live proof: [`localterra-634-migrate-inventory.sh`](../scripts/qa/localterra-634-migrate-inventory.sh).

## Add a future migrate source

1. Confirm cw2 is already in `ALLOWED_SOURCE_CW2` / `ALLOWED_TAXED_CW2` in `adopt.rs` (or add the crate name there).
2. Append the columbus-5 code id to Coolify `VITE_COMMUNITY_MIGRATE_CODE_IDS` (and the default list in `communityTaxMigrate.ts` if it should ship without env).
3. Do **not** factory-whitelist a FoT / `tax_map` wasm. Listing **11619** covers the address after adopt.

## Ops after adopt (not the retail button)

Whitelist already has **11619** → `SetPairPaused` on affected **CL8Y** pairs → `RefreshPairAssetCodeIds` (single; both live ids listed) → smoke extra-debit sell → unpause. Playbook: [`docs/runbooks/cw20-code-id-ops.md`](../docs/runbooks/cw20-code-id-ops.md).

## Verify

```bash
make verify-issue-626
make verify-issue-628
make verify-issue-634
# LocalTerra live (fail if chain missing):
VERIFY634_REQUIRE_CHAIN=1 make verify-issue-634
# or directly:
./scripts/qa/localterra-634-migrate-inventory.sh
make verify-issue-592
make verify-issue-593
make verify-issue-594
```

LocalTerra (`localterra-634-migrate-inventory.sh`, **M634** live): adopt a factory-listed `cw20-mintable` that already has a CL8Y pair — confirm that pair as the governance-refresh target; adopt does **not** register; after ops `RefreshPairAssetCodeIds` (never from `/token/migrate`), register lists **only** that factory pair; Terraport factory probe must not invent rows; a mintable with no CL8Y pair is a valid empty inventory. Columbus-5 Open/ALPHA LCD walkthrough stays on [#636](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/636). Autoregister leftovers: [`AGENTS_COMMUNITY_TAX_AUTOREGISTER.md`](./AGENTS_COMMUNITY_TAX_AUTOREGISTER.md) / [#635](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/635).

## Do not

- Whitelist **8654** to “save” Terraport LP.
- Skip `UpdateAdmin` (source admin could migrate off 11619).
- Offer adopt for 11611/11619 on the retail page.
- Turn hybrid off (**#596**).
- Change pair/router wasm.
