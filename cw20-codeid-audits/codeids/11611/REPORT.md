# REPORT: CW20 code ID 11611 (`cl8y-community-tax-token`)

**Date:** 2026-08-23  
**Operator:** ops / #601 intake  
**LCD:** `https://terra-classic-lcd.publicnode.com`  
**Procedure:** [`../../PROCEDURE.md`](../../PROCEDURE.md) ([#589](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/589))  
**Issues:** [#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592) (crate) · [#601](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/601) (store + list)

The inspected artifact is a **decompilation / string fingerprint of LCD wasm**. It is **not** redistributable source. F6 pin does not replace this report. Pre-store stub: [`../community-tax-token/REPORT.md`](../community-tax-token/REPORT.md).

## Verdict

**GO** for factory `AddWhitelistedCodeId 11611` as the **named T592 exception** (inbound pair/router/escrow/AutoLP credit stays **1:1**; sell tax is extra-debit; buy tax is outbound split). Columbus-5 listed **11611** 2026-08-23 (height **30071160**, `GetWhitelistedCodeIds` **`[6036, 8266, 10184, 11611]`**). Do **not** whitelist a LocalTerra store id. Do **not** whitelist ALPHA **8654**. Do **not** whitelist launcher **11612** or AutoLP **11613** (not pair-asset CW20s).

- [x] GO — LCD pin matches; decomp + fingerprint; crate multitest covers T592 classification; harness known-bad stays red; catalogue filled
- [ ] NO-GO — inbound FoT / 8654 class / missing pin

**Reason:** SHA-256 of LCD bytes equals `CodeInfo.data_hash` (`9D33BF25…`). cw2 `crates.io:cl8y-community-tax-token` / `1.0.0`. No `tax_map` / `fee_on_transfer` / `rebase` / `ibc_receive`. Pair credit on sell is `amount` (extra-debit on the trader), not inbound FoT. **8654** / FoT mutants must stay red.

**Residual (C589-7):** this host had **no LocalTerra**. `CODE_ID=11611 make verify-issue-589` printed `SKIP Layer B-lt` (explicit, not a silent pass). Layer A-lcd / B-lt **did not execute** the pinned 11611 bytes. Close with `CODE_ID=11611 LAYER_B_LT=1 make verify-issue-589`. Generic mintable A-mt/B-mt is **not** A-lcd of 11611.

Re-run: `CODE_ID=11611 make verify-issue-589` · `CODE_ID=11611 LAYER_B_LT=1 make verify-issue-589` · `make verify-issue-592`.

## Identity

| Field | Value |
|-------|--------|
| `code_id` | **11611** |
| `data_hash` (LCD) | `9D33BF2539A9A5B2F13FD4B321CDBD0B0FD86D936D5D6BD6681955FA30210EC2` |
| SHA-256 of downloaded wasm | same ([`wasm.sha256`](wasm.sha256)) |
| Match | **yes** (LCD + local artifact `cl8y_community_tax_token.wasm`) |
| Creator / uploader | `terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv` (`cl8ydeploy`) |
| Instantiate permission | **Everybody** |
| Approximate instantiate count | **0** tokens at intake (2026-08-23). Launcher instance: `terra1af9xm63mev4hnf4z0nmmcsnd9f4lpac2vs205rmaeg3kdqlqudhq894lyz` |
| Wasm size | 535789 bytes |
| Store tx | [`C610FB95B4BF18F5C96B972545D8649993461FC631A219F80C927DF5172B2BF2`](https://finder.terraclassic.community/columbus-5/tx/C610FB95B4BF18F5C96B972545D8649993461FC631A219F80C927DF5172B2BF2) height **30071140** |
| Whitelist tx | [`241FE20E7649738DF8E34B778AE171E803C2962C1FC556242DFC1CE0A53CB30E`](https://finder.terraclassic.community/columbus-5/tx/241FE20E7649738DF8E34B778AE171E803C2962C1FC556242DFC1CE0A53CB30E) height **30071160** (DEX 2-of-3) |
| `meta.json` | [`meta.json`](meta.json) |

Sister stores (not listable as pair assets): launcher **11612** `A0F95FBA…` at `terra1af9xm63mev4hnf4z0nmmcsnd9f4lpac2vs205rmaeg3kdqlqudhq894lyz` (instantiate [`F7FCF13F…2317`](https://finder.terraclassic.community/columbus-5/tx/F7FCF13FB59148832AFA717FE3889830C19ACA33B18BFFB0122FB237BBDC2317) height **30071219**, admin CMM), AutoLP **11613** `B110CCD6…`.

## Fetch

- Endpoint: columbus-5 LCD `https://terra-classic-lcd.publicnode.com` (`fetch-lcd-wasm.sh 11611`). Secondary LCD not required this run (`LCD_URL_SECONDARY` unset — **G4** residual).
- Hash mismatch / truncated download: **did not occur**. Pin matches LCD `data_hash` and the CosmWasm-optimizer artifact. Fetch self-tests (C1/G9) green under `make verify-issue-589`.

## Fingerprint

Machine file: [`decomp/fingerprint.json`](decomp/fingerprint.json).

| Hit | Value |
|-----|--------|
| `cw20_base` | true |
| `tax_map` / `cw20_taxed` / `fee_on_transfer` / `rebase` / `reflection` | **false** |
| `ibc_receive` / `ibc_packet` | **false** |
| `requires_terra` / `requires_stargate` | **false** |
| `requires_iterator` | true |
| `permit` / `flash_mint` / `terraport_token` | **false** |
| `balance_at` / `total_supply_at` | **false** |
| `transfer` / `send` / `transfer_from` / `send_from` / `burn` / `mint` | true |

Exports include `instantiate`, `execute`, `migrate`, `allocate`, `deallocate`, `interface_version_8`, `requires_iterator` (no `requires_terra` on this binary).

Compiler / deps (from LCD string table): rustc commit `eeb90cda1969383f56a2637cbd3037bdf598841c`; `cosmwasm-std` **1.5.11**; `cw-storage-plus` 1.2.0; `serde-json-wasm` 0.5.2. cw2: `crates.io:cl8y-community-tax-token` / `1.0.0`.

Query serde (LCD strings): `balance`, `token_info`, `minter`, `allowance`, `all_allowances`, `all_spender_allowances`, `all_accounts`, `marketing_info`, `download_logo`, `get_config`, `get_features`, `get_exemptions`, `is_protocol_exempt`, `tax_preview`, `get_launcher_origin`. **No `tax_map`.**

Execute serde: standard CW20 (`transfer`, `burn`, `send`, allowances, `transfer_from`, `send_from`, `burn_from`, `mint`) plus `Receive` (UST1 invoice), `register_listed_pair`. Settings / SKU unlocks are **invoice hooks** on `Receive`, not free admin setters. `RegisterListedPair` queries factory `Pair` (T592-9) — not a permissionless “enable tax on any address” (D8).

**Declared / debit / credit / tax** strings present (`TaxPreview`). Sell extra-debit is **trader-side**; pair inbound credit stays `amount`.

## Decompile

`decomp/` generated (`decompile-wasm.sh 11611`, wabt). `fingerprint.json` is the committed pin; wat/c/objdump gitignored. Unreadable regions: stripped names; CFG not proven byte-identical to a rebuild (rebuild is appendix-only).

## Catalogue

Legend: **static-pass** = LCD strings/dump; **crate** = `cl8y-community-tax-token` multitest / `make verify-issue-592`; **control** = `cw20_codeid_harness` / mutants (not executed on 11611 bytes); **named-exception** = T592 extra-debit / outbound split (not inbound FoT); **pending-lt** = needs `LAYER_B_LT=1` on pinned wasm; **N/A** = no analogue or out of listing veto.

### A

| ID | Result | Notes |
|----|--------|-------|
| A1 | **named-exception** + crate; control red on mutants | Inbound `Transfer` / `TransferFrom` to pair/router/escrow/AutoLP credits `amount` (T592-1). Not 8654 inbound FoT. Sell `Send`+`Swap` extra-debits the trader; pair still gets `amount`. FoT mutants stay red |
| A2 | **named-exception** + crate | Directional **sell** extra-debit / **buy** outbound split only on **factory-registered** listed pairs. Wallet↔wallet honest unless TransferTax SKU. Must **not** match inbound-tax mutant |
| A3 | static-pass; crate idle | No rebase / reflection / `balance_at` live mutate. Balances change only on transfer/mint/burn |
| A4 | static-pass; control | No `tax_map` / `UpdateTaxMap`. 8654 remains known-bad |
| A5 | static-pass | Plain `Transfer` does not dispatch a recipient `WasmMsg`. `Send` is CW20 Receive only |
| A6 | control; pending-lt | Pair reentrancy tests exist; 11611 as offer token not executed on LocalTerra this host |
| A7 | control; crate | CW20 increase/decrease allowance; no overwrite-approve race |
| A8 | control | Unauthorized TransferFrom rejected on mintable analogue; no admin skip-allowance string |
| A9 | crate (T592-11); pending-lt B7 | `trading_enabled=false` blocks **both** buy and sell (launch guard SKU), not sell-only honeypot. Sell to listed pair bypasses `max_wallet` |
| A10 | residual (SKU) | `LaunchGuards.trading_enabled` is manager settings (invoiced), not a hidden pause. Residual: manager can halt both sides |
| A11 | residual (SKU) | `max_wallet` is launch-guard SKU; sell-to-listed-pair bypasses. Not a silent post-list shrink of pair balance |
| A12 | crate | Unauthorized mint rejected. MintControl instantiate-only; `RevokeMint` one-way + settings invoice (T592-6) |
| A13 | static-pass | No flash-mint execute |
| A14 | residual; **not blocking** | Wasm admin is CMM when launched via launcher (`cmm_governance`). F6 freeze on migrate-off-11611. Rogue `--admin` is catalog-filtered (`GetLauncherOrigin`) |
| A15 | static-pass | Single contract template |
| A16 | crate | `TaxPreview` debit/credit match execute classification; events use declared amounts |
| A17 | crate | `MAX_DECIMALS` 18; factory **P3** still rejects >18 |
| A18 | crate | Zero / oversize rejected or no-op per cw20-base paths |
| A19 | document | Pair does not self-TransferFrom |
| A20 | N/A | No `permit` |
| A21 | record | Name/symbol are strings; dApp escape is #593 |
| A22 | static-pass | `TokenInfo` readable |
| A23 | static-pass | No holder airdrop / interest drip |
| A24 | crate | Oversize transfer rejected; not max-uint credit-only |
| A25 | static-pass | CW20 only |
| A26 | static-pass | No IBC mint surface |
| A27 | static-pass | `cosmwasm-std` **1.5.11** (after CWA-2024-002 1.4.4). No wrapping-pow balance math |
| A28 | static-pass | `requires_terra` **false**. Native Tax2Gas is not this CW20 path |
| A29 | N/A | No `balance_at` |
| A30 | residual | Marketing/logo via cw20-base; DoS-only |

### B

| ID | Result | Notes |
|----|--------|-------|
| B1 | crate + control | Inbound 1:1 keeps **P2**. FoT mutant still desyncs (harness red). Extra-debit does **not** short the pair |
| B2 | control | Donation must not inflate LP shares (**P3**) |
| B3 | control | Flash provide/swap/withdraw no profit |
| B4 | control; pending-lt | Router dust (**R4**); hybrid 1:1 hops are #601 DEX residual |
| B5 | control | Hook + token reenter must not extract |
| B6 | crate (T592-7); pending-lt | Limit `PlaceLimitOrder*` `Send` is **1:1**. Pair→EOA refund/withdraw uses **buy tax** (same Transfer primitive) — documented, not a pair wasm change |
| B7 | crate; pending-lt | With `trading_enabled=true` and listed pair, buy and sell both work (extra-debit on sell). Launch-guard off is residual A10, not a one-way honeypot |
| B8 | residual | Manager cannot blacklist a pair address; factory `RegisterListedPair` is add-only for protocol entries (T592-9) |
| B9 | control | F6 freeze if instance migrates off 11611 |
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
| C3 | crate + A-mt/B-mt; **A-lcd/B-lt pending** | Do not treat mintable harness as 11611 lcd execute |
| C4 | pass | Known-bad FoT 1:1 / P2 stay red |
| C5 | pass (explicit skip) | `SKIP Layer B-lt: make has-localterra` printed 2026-08-23 |
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
| D11 | residual (SKU) | `cooldown_blocks` can reject a second transfer in-block if LaunchGuards on — batch/ladder residual (**CH3**) |
| D12 | static-pass | CW20 path does not require `info.funds` |
| D13 | static-pass | No SetDecimals |
| D14 | residual | Hostile init (non-CMM admin, extreme bps within cap) possible because Everybody instantiate. Catalog filter, not a template veto |
| D15 | pending-lt | 0 instances at intake; AllAccounts vs genesis when first token exists |
| D16 | crate | Tax events vs `TaxPreview`; indexer #594 must not treat sell `amount` as pair inbound shortfall |
| D17 | static-pass | Unknown `tax_map` should error (good) |
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
| E6 | control; pending-lt | Book reentrancy |
| E7 | residual D11 | Batch vs cooldown |
| E8 | residual | If launch-guard pause on, parked claim can brick until unpaused — same class as token pause |
| E9 | pending-lt | Router cycle; #601 hybrid residual |
| E10 | report-only | B13 + 100 LUNC fee |
| E11 | pending / #601 residual | Zap floors when used as pool asset |
| E12 | N/A | Not wrap / UST1 window wasm |
| E13 | residual | Treasury sink is configured; extra-debit does not steal pair fee token |
| E14 | crate | 6-decimal typical; cap 18 |
| E15 | control | Rounding loop on mintable |
| catalog F1–F8 | residual / #594 | Indexer must key on `code_id==11611`, CMM admin, launcher origin. Event vs extra-debit debit is a display concern, not inbound P2 |
| G1 | control | Mintable vs mutant |
| G2 | pass | `cw20_mutants.rs` |
| G3 | pending | Bindiff vs cw20-base not run |
| G4 | residual | Single LCD this run |
| G5 | residual | 0 token instances at intake; launcher `terra1af9xm…894lyz` is the only 11612 contract |
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

- **Crate:** `cd smartcontracts && cargo test -p cl8y-community-tax-token` / `make verify-issue-592` — inbound 1:1, extra-debit sell, outbound buy, 50 UST1 invoice fail-closed.
- **A-mt:** `cw20_codeid_harness` mintable + mutants via `make verify-issue-589` (2026-08-23: 34 tests ok).
- **A-lcd:** [`../../scripts/layer-a-lcd.sh`](../../scripts/layer-a-lcd.sh) **not run** (no LocalTerra).

## Layer B (DEX + limits)

- **B-mt:** harness P1/P2/P3/donation/flash/honeypot/limit/same-asset; FoT **P2** red.
- **B-lt:** [`../../scripts/layer-b-lt.sh`](../../scripts/layer-b-lt.sh) — **explicit skip** 2026-08-23: `SKIP Layer B-lt: make has-localterra`. When run, whitelist only the **local** store id (never treat that id as columbus-5 11611).

## Factory-global impact

Approving **11611** admits **every** instantiate of this wasm, including rogue `--admin` and non-launcher creates. That is accepted for the template: dApp (#593) and indexer (#594) **must** filter `ContractInfo.admin == CMM` and `GetLauncherOrigin`. Pair-create fee remains **100 LUNC**.

## Instance admin / migrate residual (F6)

Launcher stamps CosmWasm admin = `cmm_governance` (CMM `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2`). Manager cannot migrate. F6 freezes writes if an instance leaves 11611. Honest upgrade: source-review new id → whitelist → migrate → Refresh.

## Unverified third-party claim (C7)

- [x] I did **not** treat CertiK / Skynet **file** hashes or marketing copy as LCD `data_hash`
- [x] Optional appendix (in-repo crate / optimizer artifact) does **not** block this verdict — LCD pin is the gate

## Appendix (optional — never a gate)

- In-repo crate: `smartcontracts/contracts/community-tax-token` (workspace 1.0.0). Optimizer artifact hash matched LCD; **not** a required rebuild proof.
- Policy: [`docs/runbooks/cw20-whitelist-policy.md`](../../../docs/runbooks/cw20-whitelist-policy.md) named exception.
- Playbook: [`skills/AGENTS_COMMUNITY_TAX_CW20.md`](../../../skills/AGENTS_COMMUNITY_TAX_CW20.md) **T592-1–T592-12**.
