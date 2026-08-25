# REPORT: CW20 code ID 3 (columbus-5 `cw20-legacy`)

**Date:** 2026-08-25  
**Operator:** investigation (#627)  
**LCD:** `https://terra-classic-lcd.publicnode.com`  
**Procedure:** [`../../PROCEDURE.md`](../../PROCEDURE.md) ([#589](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/589))  
**Playbook:** [`../../../skills/AGENTS_CW20_CODE_ID_3.md`](../../../skills/AGENTS_CW20_CODE_ID_3.md)

The inspected artifact is a **decompilation / string fingerprint of LCD wasm**. It is **not** redistributable source. F6 pin does not replace this report.

This ticket is **not** a license to append `3` to `VITE_COMMUNITY_MIGRATE_CODE_IDS` or call `AddWhitelistedCodeId 3`.

## Verdict

Tracks may differ. Both are **NO-GO**.

| Track | Verdict | One-line reason |
|-------|---------|-----------------|
| **(A) migrate / adopt onto 11619 / 11626 / 11630** | **NO-GO** | cw2 `crates.io:cw20-base` passes `ALLOWED_SOURCE_CW2`; storage is `cw20-legacy` CanonicalAddr. Adopt would smash `TOKEN_INFO` (mint Some) or **silently drop balances** (mint None). No importer in this ticket. Crate now fail-closes (`AdoptLegacyLayout`). |
| **(B) factory pair-asset list** | **NO-GO** | #589 GO requires Layer A-lcd + B-lt on **pinned** bytes. Wasm exports `interface_version_7` (cosmwasm-std **0.16.0**); LocalTerra / current wasmd is `interface_version_8`. Fail closed. Everybody instantiate + **≥34 900** instances (B13 / H-01). Checksum / “looks like cw20-base” is not enough. |

- [ ] GO — Layer A + Layer B green on the **pinned LCD wasm**
- [x] **NO-GO** — do not whitelist; do not append migrate env; do not add FoT math

Re-run: `make verify-issue-627`. Do **not** treat `CODE_ID=3 LAYER_B_LT=1 make verify-issue-589` as a listing path.

## Identity

| Field | Value |
|-------|--------|
| `code_id` | **3** |
| `data_hash` (LCD) | `F9B4AB2202A5110B653E7DFE3E413B336D14234ED447D882E1D9BD5512B44891` |
| SHA-256 of downloaded wasm | same ([`wasm.sha256`](wasm.sha256)) |
| Match | **yes** (2026-08-25 LCD) |
| Creator / uploader | `terra15kc4dkquya20yx09avpdw8796unj709xuyxjyc` |
| Instantiate permission | **Everybody** |
| Approximate instantiate count | **≥ 34 900** (349 pages × 100; pagination still open — [`census.json`](census.json)) |
| Wasm size | 282151 bytes |
| `meta.json` | [`meta.json`](meta.json) |

Factory `GetWhitelistedCodeIds` (LCD 2026-08-25): **`[6036, 8266, 10184, 11630]`**. **3 is absent.**

## Storage layout (A — adopt smash)

[Columbus-5 dApp guide](https://github.com/terra-money/classic-mainnet/wiki/Preparing-Terra-DApps-for-Columbus-5,-Full-Guide): token contracts on this store use **cw20-legacy** because:

1. Balances are stored with **`CanonicalAddr`** of the addresses instead of **`Addr`**.
2. The `TokenInfo` key for raw queries is **`\u{0}\ntoken_info`** (`\x00\x0a` + `token_info`).

| Map | Code 3 (`cw20-legacy` 0.8 / cw-storage-plus 0.8) | Current `cw20_base::state` (adopt target) |
|-----|--------------------------------------------------|-------------------------------------------|
| `TOKEN_INFO` | Length-prefixed Item: **`\x00\x0atoken_info`**. `MinterData.minter` is **CanonicalAddr** | Unprefixed Item key **`token_info`**. `MinterData.minter` is **Addr** |
| `BALANCES` | `Map<&CanonicalAddr, Uint128>` — `\x00\x07balance` + 20/32 raw bytes | `Map<&Addr, Uint128>` — `\x00\x07balance` + `terra1…` UTF-8 |
| allowances | CanonicalAddr pair keys | Addr pair keys |
| `tax_map` | **absent** (query unknown variant) | wiped only if leftover namespaces exist |

Live MIR raw query of `\x00\x0atoken_info` **hits**. MIR cw2 raw: `crates.io:cw20-base` / `0.1.0`.

Adopt path today (`execute_adopt`):

1. cw2 name check **passes**.
2. Modern `TOKEN_INFO.may_load` reads unprefixed `token_info`. Code 3 wrote `\x00\x0atoken_info` → **`AdoptMissingTokenInfo`** (already fail-closed).
3. If `token_info` bytes exist but are CanonicalAddr `MinterData` → serde fail → **`AdoptLegacyLayout`**.
4. If TokenInfo somehow loads (mint None at the modern key) while `balance` suffixes are 20/32-byte CanonicalAddr → **`AdoptLegacyLayout`** (defense in depth; otherwise modern `BALANCES` would look empty — silent wipe vs `total_supply` / **M626-7**).
5. These guards are **not** an importer.

A layout adapter (walk CanonicalAddr → `addr_humanize` → rewrite `Addr` maps, then adopt) is a **new ticket**. It is not enabled here.

## Instance census (B13)

[`census.json`](census.json) (LCD 2026-08-25). Full walk stopped after **349** pages (**34 900** addresses, still `next_key`). Exact total is larger. That is already enough for an Everybody-list decision.

| Token | Address | code_id | Wasm admin | Minter | `tax_map` |
|-------|---------|---------|------------|--------|-----------|
| MIR | `terra15gwkyepfc6xgca5t5zefzwy42uts8l2m4g40k6` | 3 | `terra1mxuvsdls2766fskmck0ly04da876sl6n2d0znw` | none | unknown variant (`cw20_legacy::msg::QueryMsg`) |
| TWD | `terra19djkaepjjswucys4npd5ltaxgsntl7jf0xz7w6` | 3 | `terra1dfcptsy9x4wzhpv0m79v47ladgj02yyuf0asyu` | — | unknown variant |
| VKR | `terra1dy9kmlm4anr92e42mrkjwzyvfqwz66un00rwr5` | 3 | `terra1q7fw20xc6puu6tsyeelszjwwuy9dmt60ecqnx9` | none | unknown variant |
| WHALE | `terra1php5m8a6qd68z02t3zpw4jv2pj4vgw4wz0t8mz` | 3 | `terra1vjws6sa429u48dlw8s6mycr62nalyhakxc4v7v` | same + cap | unknown variant |
| KUJI | `terra1xfsdgcemqwxp4hhnyk4rle6wr22sseq7j07dnn` | 3 | `terra1u9yqqsfz28mk30anxnt4azp87e5r8q8haxvjeh` | none | unknown variant |
| sKUJI | `terra188w26t95tf4dz77raftme8p75rggatxjxfeknw` | 3 | `terra1vvj874nwtmxk0u0spj83d364xyhqk2e652jrck` | same, uncapped | unknown variant |

First-page Station-style instances mostly share wasm admin `terra1xxxkttk2vazvkm6clr8qdq74874ukqnn68tmeh` (not empty). **Do not** special-case MIR/KUJI addresses in the retail gate.

Listing **3** would admit **every** current and future instantiate of this wasm (H-01 / **B13**). Unlisting later would F6-freeze every pair still pinned to 3.

## Fetch

- Endpoints: columbus-5 LCD `https://terra-classic-lcd.publicnode.com` (2026-08-25).
- Hash mismatch / truncated download: **did not occur**. Pin `F9B4AB22…` matches LCD `data_hash`.

## Fingerprint

[`decomp/fingerprint.json`](decomp/fingerprint.json):

| Needle | Hit |
|--------|-----|
| `cw20_legacy` | **true** |
| `interface_version_7` | **true** |
| `addr_canonicalize` | **true** |
| `tax_map` / `cw20_taxed` / `UpdateTaxMap` / `fee_on_transfer` / `rebase` | **false** |
| `cw20_base` (underscore crate path) | false (cw2 string is `crates.io:cw20-base`) |
| `requires_terra` | true |
| `ibc_receive` | false |
| `balance_at` | false |

Exports: `interface_version_7`, `requires_terra`. **No** `interface_version_8`, **no** `migrate` export (does not block `MsgMigrateContract` onto new code).

Compiler / crates: **cosmwasm-std 0.16.0**; **cw-storage-plus 0.8.0**; rustc hash `a178d032…`; instantiate type `terraswap::token::InstantiateMsg`; state `cw20_legacy::state::TokenInfo`.

Query serde (live MIR): `balance`, `token_info`, `minter`, `allowance`, `all_allowances`, `all_accounts`. **No** `tax_map`, marketing, or `balance_at`.

Execute serde: `transfer`, `burn`, `send`, `mint`, `increase_allowance`, `decrease_allowance`, `transfer_from`, `send_from`, `burn_from`.

## Decompile

`decomp/` generated (`decompile-wasm.sh 3`, wabt 1.0.36). Fingerprint JSON committed; wat/c/objdump gitignored. Static strings match the table. Unreadable regions: stripped names.

## Catalogue

Legend: **static-pass** = LCD strings / live query; **fail-closed** = cannot execute pinned wasm on current LocalTerra (`interface_version_7`); **N/A** = no analogue or out of listing veto.

### A

| ID | Result | Notes |
|----|--------|-------|
| A1 | fail-closed for pinned wasm; live query + strings **no FoT vocab** | No `tax_map`. Not a listing GO. Honest Transfer on live MIR/Terraswap is appendix, not a gate |
| A2 | static-pass | No directional tax strings |
| A3 | static-pass | No rebase / `balance_at` |
| A4 | static-pass | Live `tax_map` unknown variant (`cw20_legacy::msg::QueryMsg`) |
| A5 | static-pass | Transfer path has no extra hook strings beyond Send→Receive |
| A6 | fail-closed | Send hook exists (`Cw20ReceiveMsg`); cannot B-lt |
| A7 | fail-closed | Allowance surface present; not executed on pinned bytes |
| A8 | static-pass | No backdoor strings; not proven on wasm |
| A9 | fail-closed | No blocklist strings; B7 not executed |
| A10 | static-pass | No pause execute |
| A11 | static-pass | No max-wallet strings |
| A12 | residual | Mint exists; minter is instance residual (B14) |
| A13 | static-pass | No flash-mint |
| A14 | residual; **not a listing GO** | Most sampled instances **have wasm admin**. F6 would freeze after migrate-off-template |
| A15 | static-pass | Single contract template |
| A16 | fail-closed | Live `token_info` readable; Transfer delta not executed here |
| A17 | static-pass | Sampled decimals **6** |
| A18 | fail-closed | Zero-transfer not executed |
| A19 | N/A / fail-closed | Pair does not self-TransferFrom |
| A20 | N/A | No `permit` |
| A21 | record | Name is string |
| A22 | static-pass | Live TokenInfo parse OK |
| A23 | static-pass | No idle rebase strings |
| A24 | fail-closed | Oversize path not executed |
| A25 | static-pass | CW20 only |
| A26 | static-pass | No `ibc_receive` |
| A27 | record | cosmwasm-std **0.16.0** is outside current CWA-2024-002 range; token does not use wrapping pow for listing GO |
| A28 | fail-closed | `requires_terra` present; 1:1 not executed on LocalTerra |
| A29 | N/A | No `balance_at` |
| A30 | N/A | No marketing/logo in serde |

### B

| ID | Result | Notes |
|----|--------|-------|
| B1 | fail-closed | P2 not executed on pinned wasm |
| B2 | fail-closed | control only |
| B3 | fail-closed | control only |
| B4 | fail-closed | control only |
| B5 | fail-closed | Receive hook exists |
| B6 | fail-closed | L1 not executed |
| B7 | fail-closed | Round-trip not executed |
| B8 | residual | Admins can migrate; F6 if listed — **not listed** |
| B9 | N/A | Not listed; F6 N/A |
| B10 | fail-closed | control only |
| B11 | fail-closed | control only |
| B12 | static-pass | Single address |
| B13 | **blocking for list** | Everybody + ≥34 900 instances. Report-only residual is **not** enough without A-lcd/B-lt GO |
| B14 | residual | sKUJI/WHALE minters; not a template veto by itself |
| B15 | N/A | No IBC mint |

### C

| ID | Result | Notes |
|----|--------|-------|
| C1 | pass | Fetch pin matches `data_hash` |
| C2 | pass | Decompile with wabt |
| C3 | **fail-closed** | Cannot run Layer A-lcd/B-lt write path on `interface_version_7` |
| C4 | N/A | 8654 remains known-bad; not this id |
| C5 | pass (harness) | Layer B-lt is never a silent pass; this REPORT is explicit NO-GO |
| C6 | pass | No keys in `codeids/` |
| C7 | pass | CertiK/Skynet file hashes **not** used |

### D

| ID | Result | Notes |
|----|--------|-------|
| D1 | static-pass | No height-tax strings |
| D2 | fail-closed | Magnitude matrix not run |
| D3 | fail-closed | Op-count loop not run |
| D4 | fail-closed | Caller matrix not run |
| D5 | static-pass | No entrypoint-tax strings; `tax_map` unknown |
| D6 | static-pass | Transfer path has no external query strings |
| D7 | static-pass | No swapAndLiquify |
| D8 | static-pass | No permissionless pair-register |
| D9 | residual | Wasm admin is issuer / Station-tool EOA |
| D10 | static-pass | Raw balances (canonical keys), not share×rate |
| D11 | static-pass | No cooldown execute |
| D12 | static-pass | No payable-transfer strings |
| D13 | static-pass | No SetDecimals |
| D14 | record | Everybody instantiate — hostile *instances* possible (minter/marketing). Residual, not the layout veto |
| D15 | record | AllAccounts exists; not fully walked |
| D16 | fail-closed | Event honesty not executed |
| D17 | static-pass | Serde list is the classic cw20-legacy set |
| D18 | fail-closed | Receive parser not fuzzed here |
| D19 | fail-closed | No reflection O(n) strings |
| D20 | record | std 0.16.0 fingerprint |
| D21 | residual | Unbounded AllAccounts possible |
| D22 | fail-closed | Sim-vs-exec not run |

### E

| ID | Result | Notes |
|----|--------|-------|
| E1 | N/A | Factory rejects identical assets |
| E2–E15 | fail-closed / N/A | DEX suite not executed on this wasm. **E10** Everybody spam is the list residual |

### catalog F

| ID | Result | Notes |
|----|--------|-------|
| F1–F8 | N/A | Off-chain; not a listing GO. Do not ingest 3 as a catalog template |

### G

| ID | Result | Notes |
|----|--------|-------|
| G1 | fail-closed | Cannot load v7 wasm in current multi-test VM as the candidate |
| G2 | pass (harness) | Mutants stay on 10184 analogue |
| G3 | record | Decomp shows `cw20_legacy` vs current `cw20_base` |
| G4 | pass | Primary LCD pin |
| G5 | pass | [`census.json`](census.json) notable + first-page admin sample |
| G6 | N/A | No time-tax to travel |
| G7 | N/A | Not listed — no F6 drill |
| G8 | pass | No IBC imports; `requires_terra` only extra export |
| G9 | pass | Fetch self-test unchanged |

### CH

| ID | Result | Notes |
|----|--------|-------|
| CH1–CH18 | N/A / fail-closed | No list GO. Silent-wipe adopt (mint None + CanonicalAddr balances) is the chain this REPORT blocks via `AdoptLegacyLayout` |

## Layer A (token-only)

Backend: **not executed** on pinned bytes. LocalTerra / current CosmWasm rejects `interface_version_7`.

Multitest (in-process, **not** the LCD wasm): honest 10184-analogue adopt still keeps balances and has no `tax_map` (`adopt_multitest::p3_*`). Legacy CanonicalAddr keys revert `AdoptLegacyLayout` (`adopt.rs` unit tests). Leftover `tax_map` is **absent** on live code-3 instances (query unknown variant).

## Layer B (DEX + limits)

**Explicit fail-closed** — not a silent skip. `LAYER_B_LT=1` against this wasm is expected to fail at store/instantiate. Do not whitelist from a skip.

Invariants P1/P2/B7/L1: **not proven** on pinned bytes.

## Factory-global impact

Approving this ID admits **every current and future instantiate** of this wasm (Station-minted tokens, MIR, TWD, VKR, WHALE, KUJI, …). ≥34 900 already exist. Everybody instantiate is a residual on quality templates when A-lcd/B-lt is green; **here A-lcd/B-lt cannot run**, so B13 **blocks** list.

## Instance admin / migrate residual (F6)

Sampled instances **have wasm admin**. `MsgMigrateContract` onto 11619 without a layout importer is **NO-GO**. If 3 were listed, Refresh after any later migrate; unlisting 3 would freeze every pair still pinned to 3. **Do not list.**

## Unverified third-party claim (C7)

- [x] I did **not** treat CertiK / Skynet **file** hashes or marketing copy as LCD `data_hash`
- [x] Optional appendix (source URL / rebuild) does **not** block this verdict

## Appendix (optional — never a gate)

- Columbus-5 migrate guide (legacy layout): https://github.com/terra-money/classic-mainnet/wiki/Preparing-Terra-DApps-for-Columbus-5,-Full-Guide
- Sibling same-era stores **not** this ticket: 147, 153, 610, 767, 1603, 1790, 4254, 5800. GDEX 9788 / Terraswap 7395 separate.
- Live factory whitelist 2026-08-25 includes **11630** (not 3). Historical issue text listed `[6036, 8266, 10184, 11611, 11619]`.
