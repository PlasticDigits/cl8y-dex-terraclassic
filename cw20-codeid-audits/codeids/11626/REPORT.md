# REPORT: CW20 code ID 11626 (`cl8y-community-tax-token` adopt)

**Date:** 2026-08-24  
**Operator:** ops / #628 intake  
**LCD:** `https://terra-classic-lcd.publicnode.com`  
**Procedure:** [`../../PROCEDURE.md`](../../PROCEDURE.md) ([#589](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/589))  
**Issues:** [#628](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/628) / [#626](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/626) · crate [#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592)

The inspected artifact is a **decompilation / string fingerprint of LCD wasm**. It is **not** redistributable source. F6 pin does not replace this report. Listed rotate pin stays [`../11619/REPORT.md`](../11619/REPORT.md). Honest pin stays [`../11611/REPORT.md`](../11611/REPORT.md). Pre-store stub: [`../community-tax-token/REPORT.md`](../community-tax-token/REPORT.md).

## Verdict

**GO** for factory `AddWhitelistedCodeId 11626` as the **named T592 exception** (inbound pair/router/escrow/AutoLP credit stays **1:1**; sell tax is extra-debit; buy tax is outbound split). This pin adds **#626 foreign adopt** (`AdoptMigrateMsg` / `GetMigrateOrigin` / leftover `tax_map` wipe). Columbus-5 **stored 2026-08-24** (height **30091582**) and **listed 2026-08-24** (height **30091644**, `GetWhitelistedCodeIds` **`[6036, 8266, 10184, 11619, 11626]`**). Keep **11619** listed until Refresh. Live `IsCodeIdWhitelisted 11611` is **false** (0 instances; not this add). Do **not** treat 11619 as the retail adopt target. Do **not** whitelist a LocalTerra store id. Do **not** whitelist ALPHA **8654**. Do **not** whitelist launcher **11612** / **11614** / **11620** / **11622** or AutoLP **11613** / **11621** (not pair-asset CW20s). Do **not** append columbus-5 code **3**.

- [x] GO — LCD pin matches; decomp + fingerprint; crate multitest covers T592 / option-2 / #608 / #609 / #626 adopt; harness known-bad stays red; A-lcd/B-lt executed; catalogue filled
- [ ] NO-GO — inbound FoT / 8654 class / missing pin

**Reason:** SHA-256 of LCD bytes equals `CodeInfo.data_hash` (`A7244C93…`). cw2 `crates.io:cl8y-community-tax-token` / `1.0.0`. Pair credit on sell is `amount` (extra-debit on the trader), not inbound FoT. **Fingerprint `tax_map` / `cw20_taxed` are true** because `adopt.rs` names those leftover namespaces / cw2 allowlist for the S4 wipe — **`UpdateTaxMap` is false**; there is no live `tax_map` execute. **8654** / FoT mutants must stay red.

**Layer A-lcd / B-lt (O601-1):** executed 2026-08-24 on pinned LCD bytes (`CODE_ID=11626 LAYER_B_LT=1 make verify-issue-589`). A-lcd retries community-tax `InstantiateMsg` (cw20-base init missing `manager`); 11626 then rejects SKU-gated `launch_guards` / `max_*` headroom unless those features are on — harness retries without those fields. `balance_at` is A29 N/A. B-lt does **not** `RegisterListedPair` — inbound 1:1 / provide / round-trip / limit escrow hold. **Tax-on suite** ([#623](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/623) / [`layer-b-tax-on.sh`](../../scripts/layer-b-tax-on.sh)): extra-debit / outbound / router `trader` / AutoLP floor after register. Do **not** merge that into B-lt.

Re-run: `CODE_ID=11626 LAYER_B_LT=1 make verify-issue-589` · `make verify-issue-592` · `make verify-issue-626` · `make verify-issue-607` · `make verify-issue-608` · `make verify-issue-609`.

## Identity

| Field | Value |
|-------|--------|
| `code_id` | **11626** |
| `data_hash` (LCD) | `A7244C93D8490CFD1063DC7B45C3F09E38E947008A0C6069A93E6191C2D9DA1C` |
| SHA-256 of downloaded wasm | same ([`wasm.sha256`](wasm.sha256)) |
| Match | **yes** (LCD + local artifact `cl8y_community_tax_token.wasm`) |
| Creator / uploader | `terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv` (`cl8ydeploy`) |
| Instantiate permission | **Everybody** |
| Approximate instantiate count | **0** tokens (2026-08-24). Canonical launcher: `terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze` (code **11622**; `token_code_id` still **11619** until `UpdateConfig`) |
| Wasm size | 579806 bytes |
| Store tx | [`95D5D44C07AB2284DD377684265C7E2BE573D05D52612FF2B8388B0896F97E01`](https://finder.terraclassic.community/columbus-5/tx/95D5D44C07AB2284DD377684265C7E2BE573D05D52612FF2B8388B0896F97E01) height **30091582** |
| Whitelist tx | [`03F74C9BEFF145732E1A358E06F785DBC499D8529096BF599191E5967896DC91`](https://finder.terraclassic.community/columbus-5/tx/03F74C9BEFF145732E1A358E06F785DBC499D8529096BF599191E5967896DC91) height **30091644** (DEX 2-of-3) |
| `meta.json` | [`meta.json`](meta.json) |

Sister stores (not listable as pair assets): canonical launcher instance `terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze` is code **11622**. Unused first launcher **11612** at `terra1af9xm63mev4hnf4z0nmmcsnd9f4lpac2vs205rmaeg3kdqlqudhq894lyz`. AutoLP launcher pin **11621**. Listed Honest pin remains [`../11611/REPORT.md`](../11611/REPORT.md). Listed rotate pin remains [`../11619/REPORT.md`](../11619/REPORT.md) until Refresh.

## Fetch

- Endpoint: columbus-5 LCD `https://terra-classic-lcd.publicnode.com` (`fetch-lcd-wasm.sh 11626`). Secondary LCD not required this run (`LCD_URL_SECONDARY` unset — **G4** residual).
- Hash mismatch / truncated download: **did not occur**. Pin matches LCD `data_hash` and the CosmWasm-optimizer artifact. Fetch self-tests (C1/G9) green under `make verify-issue-589`.

## Fingerprint

Machine file: [`decomp/fingerprint.json`](decomp/fingerprint.json).

| Hit | Value |
|-----|--------|
| `cw20_base` | true |
| `tax_map` / `cw20_taxed` | **true** (adopt wipe allowlist / leftover namespaces — **not** a live `tax_map` execute) |
| `UpdateTaxMap` / `fee_on_transfer` / `rebase` / `reflection` | **false** |
| `ibc_receive` / `ibc_packet` | **false** |
| `requires_terra` / `requires_stargate` | **false** |
| `requires_iterator` | true |
| `permit` / `flash_mint` / `terraport_token` | **false** |
| `balance_at` / `total_supply_at` | **false** |
| `transfer` / `send` / `transfer_from` / `send_from` / `burn` / `mint` | true |

Exports include `instantiate`, `execute`, `migrate`, `allocate`, `deallocate`, `interface_version_8`, `requires_iterator` (no `requires_terra` on this binary).

Compiler / deps (from LCD string table): rustc / `cosmwasm-std` **1.5.11**; `cw-storage-plus` 1.2.0; `serde-json-wasm` 0.5.2. cw2: `crates.io:cl8y-community-tax-token` / `1.0.0`.

Query serde (LCD strings): `balance`, `token_info`, `minter`, `allowance`, `all_allowances`, `all_spender_allowances`, `all_accounts`, `marketing_info`, `download_logo`, `get_config`, `get_features`, `get_exemptions`, `is_protocol_exempt`, `tax_preview`, `get_launcher_origin`, **`get_migrate_origin`**. **No `UpdateTaxMap`.**

Execute serde: standard CW20 plus `Receive` (UST1 invoice), `register_listed_pair`. Settings / SKU unlocks are **invoice hooks** on `Receive`. `RegisterListedPair` queries factory `Pair` (T592-9).

**Adopt (new vs 11619):** LCD strings include `contracts/community-tax-token/src/adopt.rs`, `crates.io:cw20-base` / `cw20-mintable` / `terraport-token` / `cw20-taxed` / `cw20_taxed`, wipe namespaces `tax_map` / `tax_info` / `whale_info`, `Foreign cw2  requires an allowlisted adopt payload`, `Source cw2  is not an allowlisted adopt template`, `Adopt is not used for same-crate community-tax upgrades`, `action` `migrate-adopt`, `GetMigrateOrigin` / `source_code_id`. Unknown cw2 / `cfg`/`feat` smash still reverts.

**Declared / debit / credit / tax** strings present (`TaxPreview`). Sell extra-debit is **trader-side**; pair inbound credit stays `amount`. Option-2 strings present: `hop_trader`, `hop_trader_debit`. SKU / #608 / #609 strings present.

## Decompile

`decomp/` generated (`decompile-wasm.sh 11626`, wabt). `fingerprint.json` is the committed pin; wat/c/objdump gitignored. Unreadable regions: stripped names; CFG not proven byte-identical to a rebuild (rebuild is appendix-only).

## Catalogue

Legend: **static-pass** = LCD strings/dump; **crate** = `cl8y-community-tax-token` multitest / `make verify-issue-592` / `626`; **control** = `cw20_codeid_harness` / mutants (not executed on 11626 bytes); **named-exception** = T592 extra-debit / outbound split (not inbound FoT); **lt-pass** = `LAYER_B_LT=1` on pinned wasm; **N/A** = no analogue or out of listing veto.

### A

| ID | Result | Notes |
|----|--------|-------|
| A1 | **named-exception** + crate; control red on mutants | Inbound `Transfer` / `TransferFrom` to pair/router/escrow/AutoLP credits `amount` (T592-1). Not 8654 inbound FoT. Sell `Send`+`Swap` extra-debits the trader; pair still gets `amount`. FoT mutants stay red |
| A2 | **named-exception** + crate; **in these bytes** | Directional **sell** extra-debit / **buy** outbound split on **factory-registered** listed pairs. Official-router hops tax authenticated `Swap.trader` (**T592-13** / #607 improved option 2). Wallet↔wallet honest unless TransferTax SKU. Must **not** match inbound-tax mutant |
| A3 | static-pass; crate idle; lt-pass | No rebase / reflection / `balance_at` live mutate. Balances change only on transfer/mint/burn. A-lcd idle 1:1 |
| A4 | static-pass (wipe strings) + crate; control | **No `UpdateTaxMap`.** Fingerprint `tax_map` / `cw20_taxed` are adopt **wipe** leftovers (S4) — wiped on foreign adopt, not a live tax map. 8654 remains known-bad |
| A5 | static-pass | Plain `Transfer` does not dispatch a recipient `WasmMsg`. `Send` is CW20 Receive only |
| A6 | control; lt-pass pair path | Pair reentrancy tests exist; 11626 as offer token executed on LocalTerra (B-lt swap / limit) |
| A7 | control; crate | CW20 increase/decrease allowance; no overwrite-approve race |
| A8 | control; lt-pass | Unauthorized TransferFrom rejected; no admin skip-allowance string |
| A9 | crate (T592-11 / H608); **in these bytes** | `trading_enabled=false` blocks **both** buy and sell (launch guard SKU), not sell-only honeypot. Sell to listed pair bypasses `max_wallet`. Cooldown is per user wallet |
| A10 | residual (SKU) | `LaunchGuards.trading_enabled` is manager settings (invoiced), not a hidden pause. Residual: manager can halt both sides |
| A11 | residual (SKU); crate #608; **in these bytes** | `max_wallet` is launch-guard SKU; sell-to-listed-pair **and** protocol/`to` (provide) skip the cap. User Buy / Transfer still capped |
| A12 | crate; lt-pass | Unauthorized mint rejected. MintControl instantiate-only; `RevokeMint` one-way + settings invoice (T592-6) |
| A13 | static-pass | No flash-mint execute |
| A14 | residual; **not blocking** | Wasm admin is CMM when launched via launcher (`cmm_governance`). F6 freeze on migrate-off-listed-id. Rogue `--admin` is catalog-filtered (`GetLauncherOrigin`) |
| A15 | static-pass | Single contract template |
| A16 | crate | `TaxPreview` debit/credit match execute classification; events use declared amounts |
| A17 | crate | `MAX_DECIMALS` 18; factory **P3** still rejects >18 |
| A18 | crate; lt-pass | Zero / oversize rejected or no-op per cw20-base paths |
| A19 | document | Pair does not self-TransferFrom |
| A20 | N/A | No `permit` |
| A21 | record | Name/symbol are strings; dApp escape is #593 |
| A22 | static-pass; lt-pass | `TokenInfo` readable |
| A23 | static-pass | No holder airdrop / interest drip |
| A24 | crate; lt-pass | Oversize transfer rejected; not max-uint credit-only |
| A25 | static-pass | CW20 only |
| A26 | static-pass | No IBC mint surface |
| A27 | static-pass | `cosmwasm-std` **1.5.11** (after CWA-2024-002 1.4.4). No wrapping-pow balance math |
| A28 | static-pass | `requires_terra` **false**. Native Tax2Gas is not this CW20 path |
| A29 | N/A | No `balance_at` |
| A30 | residual | Marketing/logo via cw20-base; DoS-only |

### B

| ID | Result | Notes |
|----|--------|-------|
| B1 | crate + control; lt-pass | Inbound 1:1 keeps **P2**. FoT mutant still desyncs (harness red). Extra-debit does **not** short the pair |
| B2 | control | Donation must not inflate LP shares (**P3**) |
| B3 | control | Flash provide/swap/withdraw no profit |
| B4 | control; residual | Router dust (**R4**); hybrid 1:1 hops are #601 DEX residual |
| B5 | control | Hook + token reenter must not extract |
| B6 | crate (T592-7); lt-pass | Limit `PlaceLimitOrder*` `Send` is **1:1**. Pair→EOA refund/withdraw uses **buy tax** (same Transfer primitive) — documented, not a pair wasm change |
| B7 | crate; lt-pass | Round-trip swap succeeded with `trading_enabled` default (guards SKU off on A-lcd init). Launch-guard off is residual A10, not a one-way honeypot |
| B8 | residual | Manager cannot blacklist a pair address; factory `RegisterListedPair` is add-only for protocol entries (T592-9) |
| B9 | control | F6 freeze if instance migrates off a listed id (11611 / 11619 / 11626 after ops) |
| B10 | control | max_spread / min_return are pair properties |
| B11 | control | Failed swap leaves treasury unchanged (**P10**) |
| B12 | static-pass | Single address |
| B13 | report-only | Everybody instantiate. Retail catalog must require CMM admin + `GetLauncherOrigin`. **100 LUNC** pair-create fee remains spam control |
| B14 | residual | MintControl is optional SKU; revoke is one-way. Not hidden mint |
| B15 | N/A | No IBC mint |

### C

| ID | Result | Notes |
|----|--------|-------|
| C1 | pass | Fetch pin = `data_hash` |
| C2 | pass | Decomp ran with wabt |
| C3 | crate + A-mt/B-mt; **A-lcd/B-lt executed** | Pinned 11626 bytes on LocalTerra 2026-08-24 (`layer-a-lcd.json` / `layer-b-lt.json`; local store id **20** is host-ephemeral — never list it) |
| C4 | pass | Known-bad FoT 1:1 / P2 stay red |
| C5 | pass (executed) | `LAYER_B_LT=1` ran A-lcd + B-lt on pinned 11626 (not a stub) |
| C6 | pass | No keys under `codeids/` |
| C7 | pass | LCD `data_hash` only |

### D

| ID | Result | Notes |
|----|--------|-------|
| D1 | crate | No height-activated hidden tax. Launch guards are instantiate / invoiced settings |
| D2 | crate | Caps `MAX_TAX_BPS` 2500; magnitude tax is configured bps, not a dust-honest / whale-FoT flip |
| D3 | static-pass | No first-N-honest then tax |
| D4 | crate | Classification uses listed-pair + protocol-exempt maps, not `extcodesize` |
| D5 | **named-exception** | `Transfer` inbound 1:1; `Send`+`Swap` extra-debit. Provide `TransferFrom` 1:1. This is T592, not 8654 Send-only inbound tax |
| D6 | static-pass | Transfer path does not call a second token; factory query only on `RegisterListedPair` |
| D7 | crate (T592-10) | `SkimToLp` is **not** called from `Transfer`/`Send`. AutoLP is a sister contract |
| D8 | crate (T592-9) | `RegisterListedPair` is permissionless but **factory Pair lookup** required; cannot register an arbitrary addr to turn on tax |
| D9 | residual | Manager ≠ wasm admin. CMM migrate residual = A14 |
| D10 | static-pass | Raw balances |
| D11 | residual (SKU); crate #608; **in these bytes** | **Same-sender** in-block batch / ladder can still cooldown (**CH3** / **H608-8**). Pair-wide halt is **fixed in this pin** |
| D12 | static-pass | CW20 path does not require `info.funds` |
| D13 | static-pass | No SetDecimals |
| D14 | residual | Hostile init (non-CMM admin, extreme bps within cap) possible because Everybody instantiate. Catalog filter, not a template veto |
| D15 | pending | 0 instances at intake; AllAccounts vs genesis when first token exists |
| D16 | crate | Tax events vs `TaxPreview`; indexer #594 must not treat sell `amount` as pair inbound shortfall |
| D17 | static-pass | Unknown `tax_map` query errors (good). Adopt **wipes** leftover `tax_map` / `tax_info` / `whale_info` on allowlisted sources |
| D18 | control | Pair Receive parser |
| D19 | static-pass | No O(n) reflection |
| D20 | record | std 1.5.11; CWA-2024-002 N/A. Other CWA rows: no IBC / wrapping pow |
| D21 | residual | `AllAccounts` pagination same as cw20-base |
| D22 | residual | dApp must use `TaxPreview` for sell max (#593). Hybrid quote=execute is DEX-side |

### E / catalog F / G / CH

| ID | Result | Notes |
|----|--------|-------|
| E1 | control | Factory rejects identical assets |
| E2 | control | Donation TOCTOU on provide |
| E3 | crate | Burn requires allowance / owner; no public burn of pair balance |
| E4 | N/A | No rebase |
| E5 | N/A | No flash mint |
| E6 | control; residual | Book reentrancy |
| E7 | residual D11 | Batch vs cooldown |
| E8 | residual | If launch-guard pause on, parked claim can brick until unpaused — same class as token pause |
| E9 | residual | Router cycle; #601 hybrid residual |
| E10 | report-only | B13 + 100 LUNC fee |
| E11 | residual | Zap floors when used as pool asset |
| E12 | N/A | Not wrap / UST1 window wasm |
| E13 | residual | Treasury sink is configured; extra-debit does not steal pair fee token |
| E14 | crate | 6-decimal typical; cap 18 |
| E15 | control | Rounding loop on mintable |
| catalog F1–F8 | residual / #594 | Indexer must key on listed `code_id` (11619 today; **11626** after whitelist + Coolify flip), CMM admin, launcher origin **or** `GetMigrateOrigin` attest. Event vs extra-debit debit is a display concern, not inbound P2 |
| G1 | control | Mintable vs mutant |
| G2 | pass | `cw20_mutants.rs` |
| G3 | pending | Bindiff vs cw20-base not run |
| G4 | residual | Single LCD this run |
| G5 | residual | 0 token instances; canonical launcher is **11622** `terra126pr5…ahzwze`; unused **11612** `terra1af9xm…` |
| G6 | control | multitest block advance |
| G7 | ops | F6 drill in `cw20-code-id-ops.md` |
| G8 | static-pass | Host imports: iterator; no IBC |
| G9 | pass | Fetch self-test |
| CH1 | N/A unless D1 | No hidden height tax |
| CH2 | crate | RegisterListedPair + factory check; not D8 mutant |
| CH3 | residual | Cooldown × batch |
| CH4 | crate T592-7 | Park refund is pair→EOA Transfer → **buy tax** (known) |
| CH5 | control | No allowance backdoor |
| CH6 | N/A | No reflection gas curve |
| CH7–CH9 | N/A | No rebase / flash / IBC mint |
| CH10 / CH16 | residual #594 | Extra-debit events vs pair credit |
| CH11–CH12 | catalog F | Metadata / hub tilt |
| CH13 | N/A | No dynamic decimals |
| CH14 | crate T592-10 | No mid-transfer AutoLP |
| CH15 | control F6 | Migrate to other listed id freezes |
| CH17 | control | Same-token pair rejected |
| CH18 | N/A | Not payable |

## Layer A (token-only)

- **Crate:** `cd smartcontracts && cargo test -p cl8y-community-tax-token` / `make verify-issue-592` · `make verify-issue-626` — inbound 1:1, extra-debit sell, outbound buy, foreign adopt, 50 UST1 invoice fail-closed.
- **#607 / #608 / #609 source:** option-2 hop trader, LaunchGuards per-wallet cooldown, manager-directory skip are **in these bytes**.
- **A-mt:** `cw20_codeid_harness` mintable + mutants via `make verify-issue-589`.
- **A-lcd:** [`../../scripts/layer-a-lcd.sh`](../../scripts/layer-a-lcd.sh) **executed** 2026-08-24 — Transfer / TransferFrom 1:1, unauthorized mint/burn_from rejected, idle stable. `balance_at` A29 N/A. Local token `terra1tc533…ee3gxq` (do not list this address). Local store id **20**.

## Layer B (DEX + limits)

- **B-mt:** harness P1/P2/P3/donation/flash/honeypot/limit/same-asset; FoT **P2** red.
- **B-lt:** [`../../scripts/layer-b-lt.sh`](../../scripts/layer-b-lt.sh) **executed** 2026-08-24 — provide 1:1, P2 reserves, round-trip swap, limit escrow 1:1, SendFrom. Whitelist only the **local** store id **20** (never treat that id as columbus-5 11626). B-lt stays **tax-off** (no `RegisterListedPair`). Pair `terra1jeqwj…kh2lfv`.
- **B-tax-on:** [`../../scripts/layer-b-tax-on.sh`](../../scripts/layer-b-tax-on.sh) (`make verify-issue-623`) — named tax-on suite vs B-lt split. Not a license to whitelist this LocalTerra store id or ALPHA **8654**.

## Factory-global impact

Approving **11626** admits **every** instantiate of this wasm, including rogue `--admin` and non-launcher creates. That is accepted for the template: dApp (#593) and indexer (#594) **must** filter `ContractInfo.admin == CMM` and (`GetLauncherOrigin` **or** allowlisted `GetMigrateOrigin`). Pair-create fee remains **100 LUNC**. Keep **11619** listed until CMM migrate + factory Refresh so any remaining 11619 pair pins stay writable.

Launcher `GetConfig.token_code_id` is still **11619** until DEX 2-of-3 `UpdateConfig`. Retail `/token/migrate` can target 11626 once Coolify `VITE_COMMUNITY_TAX_CODE_ID=11626`. Create Token still instantiates **11619** until that `UpdateConfig`.

## Instance admin / migrate residual (F6)

Launcher stamps CosmWasm admin = `cmm_governance` (CMM `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2`). Retail adopt is `MsgMigrateContract` then `MsgUpdateAdmin` → CMM. Manager cannot migrate. F6 freezes writes if an instance leaves a listed id. Honest upgrade: this report **GO** → whitelist 11626 → Coolify/indexer pin → optional CMM `MigrateMsg {}` of 11619 instances → Refresh. Zero instances at intake; no CMM migrate required to list.

## Unverified third-party claim (C7)

- [x] I did **not** treat CertiK / Skynet **file** hashes or marketing copy as LCD `data_hash`
- [x] Optional appendix (in-repo crate / optimizer artifact) does **not** block this verdict — LCD pin is the gate

## Appendix (optional — never a gate)

- In-repo crate: `smartcontracts/contracts/community-tax-token` (workspace 1.0.0, `adopt.rs`). Optimizer artifact hash matched LCD; **not** a required rebuild proof.
- Policy: [`docs/runbooks/cw20-whitelist-policy.md`](../../../docs/runbooks/cw20-whitelist-policy.md) named exception.
- Playbook: [`skills/AGENTS_COMMUNITY_TAX_CW20.md`](../../../skills/AGENTS_COMMUNITY_TAX_CW20.md) **T592-1–T592-13**; migrate [`skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md`](../../../skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md) **M626**.
- After GO, list is **done** (2026-08-24). Leftover is launcher `UpdateConfig` + Coolify:

```bash
# listed 11626; launcher GetConfig still token=11619 autolp=11621
# UPGRADE611_UPDATE_CONFIG=1 … (DEX 2-of-3)
# Coolify:
#   VITE_COMMUNITY_TAX_CODE_ID=11626
#   COMMUNITY_TAX_CODE_ID=11626
#   COMMUNITY_TAX_OPTION2_CODE_IDS=11626
#   VITE_COMMUNITY_MIGRATE_CODE_IDS=6036,10184,8266,8654
# keep 11619 listed until Refresh. Never AddWhitelistedCodeId 8654.
```
