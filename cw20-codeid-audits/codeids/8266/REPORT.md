# REPORT: CW20 code ID 8266 (Terraport token V2)

**Date:** 2026-08-22  
**Operator:** harness intake (#589)  
**LCD:** `https://terra-classic-lcd.publicnode.com`  
**Procedure:** [`../../PROCEDURE.md`](../../PROCEDURE.md) ([#589](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/589))

The inspected artifact is a **decompilation / string fingerprint of LCD wasm** (plus the #581 dump review). It is **not** redistributable source. F6 pin does not replace this report.

Prior one-off notes (superseded as the **intake path**; hash-repro is appendix only): [`../../../audits/CW20-8266-581.md`](../../../audits/CW20-8266-581.md), [`../../../audits/CW20-8266-581-hash-repro.md`](../../../audits/CW20-8266-581-hash-repro.md), [`../../../audits/CW20-8266-581-classic-terraswap.md`](../../../audits/CW20-8266-581-classic-terraswap.md).

## Verdict

**NO-GO** for factory `AddWhitelistedCodeId 8266`.

- [ ] GO — Layer A + Layer B green on the **pinned LCD wasm**; catalogue rows pass or N/A+reason; residuals written
- [x] NO-GO — fail closed (do not whitelist; do not add FoT math)

**Reason:** LCD identity is pinned (`953AD60C…`). 2026-08-22 harness **executed** the pinned wasm on LocalTerra: **A-lcd** store + instantiate + 1:1 Transfer, **B-lt** whitelist of the *local* store id + `CreatePair` vs EMBER + 1:1 Transfer into the pair (logs `layer-a-lcd.json` / `layer-b-lt.json`, gitignored). That closes the “stub PASS” gap (**M590-6** / **C3** Transfer path). It does **not** clear factory-global impact (**B13**: Everybody instantiate, 1686+ columbus-5 instances), issuer wasm-admin / F6 residual (**A14**), minter cap residual (**B14**), or remaining catalogue rows (Send/TransferFrom matrix, round-trip swap **B7**, limit Send escrow **B6**, P2 as *offer* token). `classic_terraport` / `terraport_token` are not `cw20_base`. Optimizer rebuild hash is **not** a remaining gate (appendix). **#581 stays NO-GO** — do not `AddWhitelistedCodeId 8266` on columbus-5.

Re-run: `CODE_ID=8266 LAYER_B_LT=1 make verify-issue-589` after `make setup-cloud-localterra`. Flip to GO only when remaining catalogue + factory-global ops sign-off are written here.

## Identity

| Field | Value |
|-------|--------|
| `code_id` | **8266** |
| `data_hash` (LCD) | `953AD60CF6D8C9631B99ADC84C3ABF4083815743F86FF81B2A422FDFDF5F95C0` |
| SHA-256 of downloaded wasm | same (pin file); fetch with `scripts/fetch-lcd-wasm.sh 8266` |
| Match | **yes** (2026-08-21 LCD + store tx event) |
| Creator / uploader | `terra1yq3d4h9g4ncale3mcwuhcfdge8hzjx666umlhg` |
| Instantiate permission | **Everybody** |
| Approximate instantiate count | **1686+** on columbus-5 |
| Wasm size | 327509 bytes |
| Store tx | `5829A7EA57F177B17DA509FB1AE6221A016BACB15C4CC8ECC1C61E281B0D2BAC` |
| `meta.json` | [`meta.json`](meta.json) |

SpaceUSD instance (motivation, not the template): `terra1cvd5cgrs8rrl96hte34n57497u5f9cwuv3e6ztxgetkx4uzmcdyswv79zl` — wasm admin / minter `terra133n0pv8jwllxwmrhymgfllglw9l0j5e765x5d9`.

## Fetch

- Endpoints: columbus-5 LCD `https://terra-classic-lcd.publicnode.com` (2026-08-22 re-fetch in this harness). Secondary LCD optional (`LCD_URL_SECONDARY`).
- Hash mismatch / truncated download: **did not occur**. Pin `953AD60C…` matches LCD `data_hash`. Self-tests in `fetch-lcd-wasm.sh --self-test` cover C1/G9.

## Fingerprint

Exports: `execute`, `instantiate`, `query`, `allocate`, `deallocate`, `interface_version_8`, `requires_terra`, `requires_stargate`, `requires_iterator`. **No `migrate` export** (does not block `MsgMigrateContract` onto new code).

Compiler: rustc **1.66.0** / optimizer line **0.12.11**; `cosmwasm-std` **1.3.3**; cw2 `crates.io:terraport-token` / `0.0.0`.

Query serde (live SpaceUSD `tax_map` error): `balance`, `balance_at`, `total_supply_at`, `token_info`, `minter`, `allowance`, `all_allowances`, `all_spender_allowances`, `all_accounts`, `marketing_info`, `download_logo`. **No `tax_map`.**

Execute serde: `transfer`, `burn`, `send`, `increase_allowance`, `decrease_allowance`, `transfer_from`, `send_from`, `mint`, `update_minter`, `burn_from`, `update_marketing`, `upload_logo`.

**No** strings: `tax_map`, `cw20_taxed`, `fee_on_transfer`, `rebase`, `reflection`.

`requires_terra` is a Terra Classic capability export (A28) — **not** FoT by itself. Layer A-lcd must use LocalTerra, not stock cosmwasm-vm without `terra`. Machine fingerprint (2026-08-22 LCD re-fetch): [`decomp/fingerprint.json`](decomp/fingerprint.json) — `tax_map`/`cw20_taxed`/`fee_on_transfer`/`rebase`/`ibc_receive` **false**; `terraport_token`/`classic_terraport`/`balance_at` **true**.

## Decompile

`decomp/` is generated (`decompile-wasm.sh 8266`, wabt 1.0.36). 2026-08-22 LCD re-fetch: `fingerprint.json` committed; wat/c/objdump gitignored. Static strings match the table above. Unreadable regions: stripped names; CFG not proven byte-identical to the later public dump.

## Catalogue

Legend: **static-pass** = LCD strings/dump; **pending-lcd** = must re-run on stored 8266 wasm; **control** = detector/control proven in `cw20_codeid_harness` / existing tests, not yet on this wasm; **N/A** = no CosmWasm analogue or out of listing veto.

### A

| ID | Result | Notes |
|----|--------|-------|
| A1 | A-lcd Transfer 1:1 **pass** (2026-08-22 LocalTerra); Send/TransferFrom/SendFrom pending-lcd | No FoT vocab. Transfer debit=credit=`amount` on pinned wasm. Other CW20 write ops not yet on this binary |
| A2 | static-pass; pending-lcd | No pair-address tax strings |
| A3 | static-pass; pending-lcd | Idle rebase vocab absent; SnapshotMap is historical |
| A4 | static-pass | Live `tax_map` unknown variant. 8654 remains known-bad |
| A5 | static-pass; pending-lcd | Transfer does not list extra WasmMsg in dump |
| A6 | control | Pair reentrancy tests; candidate as offer token pending-lcd |
| A7 | static-pass | Increase/DecreaseAllowance only; no `Approve {amount}` |
| A8 | pending-lcd | No backdoor strings; still execute TransferFrom without allowance |
| A9 | pending-lcd | Round-trip swap B7 |
| A10 | pending-lcd | No pause execute variant in serde list |
| A11 | pending-lcd | No max-wallet strings |
| A12 | residual | Minter is instance admin; cap 1e9 human — report residual (B14), not hidden mint in wasm |
| A13 | static-pass | No flash-mint execute variant |
| A14 | residual | Issuer wasm admin; **F6** freeze required. `asset_code_id_pin_tests::*` |
| A15 | static-pass | Single contract template |
| A16 | pending-lcd | Balance vs transfer events |
| A17 | static-pass | SpaceUSD decimals **6** (≤ 18) |
| A18 | pending-lcd | Dump dropped InvalidZeroAmount; zero is not FoT |
| A19 | N/A | Pair does not self-TransferFrom |
| A20 | N/A | No `permit` in execute serde |
| A21 | record | Name is string; dApp escape is separate |
| A22 | static-pass | TokenInfo readable on live SpaceUSD |
| A23 | pending-lcd | Idle balance probe |
| A24 | pending-lcd | Uint128::MAX transfer |
| A25 | static-pass | CW20 only; factory rejects natives |
| A26 | static-pass | No `ibc_receive` in export/string baseline |
| A27 | record | cosmwasm-std 1.3.3 is inside CWA-2024-002 range — **D20** documented; token does not use wrapping pow for balances in dump |
| A28 | A-lcd 1:1 **pass** on LocalTerra | `requires_terra` present; Transfer 1:1 holds on CW20 path (not native tax). Not a FoT flag |
| A29 | static-pass; pending-lcd | `balance_at` snapshot-only in dump + live SpaceUSD check |
| A30 | residual | Logo 5KB limit in wasm strings |

### B

| ID | Result | Notes |
|----|--------|-------|
| B1 | pending-lcd; control red on FoT | Mutant + `adversarial_token` prove P2 desync. Candidate must **not** |
| B2 | control; pending-lcd | `layer_b_p3_donation_does_not_inflate_lp_shares` |
| B3 | control; pending-lcd | `layer_b_b3_flash_provide_swap_withdraw_no_profit_honest` |
| B4 | control | `adversarial_token::router_ignores_pre_existing_dust_on_output_token` |
| B5 | control | `reentrancy_tests` |
| B6 | pending-lcd | Limit Send escrow 1:1 |
| B7 | pending-lcd | `layer_b_b7_round_trip_swap_succeeds_honest` analogue |
| B8 | pending-lcd | Admin can still migrate (A14); not a pair blocklist in serde |
| B9 | control | `asset_code_id_pin_tests` FoT migrate freeze |
| B10 | control | Existing `security_tests` max_spread |
| B11 | control | **P10** treasury unchanged on failed swap |
| B12 | static-pass | Single address token |
| B13 | report-only | Everybody + 1686 instances; LUNC CreatePair fee accepted |
| B14 | residual | SpaceUSD minter cap 1e9 human vs ~530 circulating |
| B15 | N/A | No IBC mint surface in fingerprint |

### C

| ID | Result | Notes |
|----|--------|-------|
| C1 | pass (harness) | Fetch self-test refuses hash mismatch |
| C2 | pass (harness) | Decompile fails closed without wabt |
| C3 | A-lcd + B-lt **executed** (Transfer 1:1); suite incomplete | `layer-a-lcd.json` / `layer-b-lt.json` written this checkout (gitignored). Round-trip swap / Send matrix still pending — keep NO-GO |
| C4 | pass (harness) | FoT mutant tests must stay red |
| C5 | pass (harness) | `verify-issue-589` prints explicit Layer B-lt skip unless `LAYER_B_LT=1`, which **executes** wasm (not a stub, #590) |
| C6 | pass | No keys in `codeids/` |
| C7 | pass | CertiK/Skynet file hashes **not** used as `data_hash` |

### D

| ID | Result | Notes |
|----|--------|-------|
| D1 | pending-lcd; control | Height-activated tax mutant exists |
| D2 | pending-lcd; control | Magnitude tax mutant |
| D3 | pending-lcd | Op-count loop not run on LCD wasm |
| D4 | pending-lcd | Caller-class matrix (EOA vs pair vs router) |
| D5 | pending-lcd; control | `mutant_d5_send_taxed_transfer_honest` |
| D6 | static-pass | Dump transfer path has no external query |
| D7 | static-pass | No swapAndLiquify analogue in execute list |
| D8 | static-pass; control | No permissionless pair-register execute |
| D9 | residual | Admin is issuer EOA, not a fake-renounce contract |
| D10 | static-pass | Raw balances, not share×rate in dump |
| D11 | pending-lcd; control | Cooldown mutant |
| D12 | pending-lcd; control | Payable mutant; pair never attaches funds |
| D13 | static-pass | No SetDecimals execute |
| D14 | residual | Everybody instantiate; init is standard cw20 + marketing. Hostile *instances* still possible via minter (B14) |
| D15 | pending-lcd | AllAccounts vs init replay |
| D16 | pending-lcd | Event vs balance |
| D17 | static-pass | Unknown query `tax_map` errors (good) |
| D18 | control | Pair Receive parser; CWA-2024-001 on pair not token |
| D19 | pending-lcd | Gas vs holder count; 43 holders on SpaceUSD is not a projection |
| D20 | record | rustc 1.66 / std 1.3.3 — document CWA range; not automatic veto |
| D21 | pending-lcd | Query gas |
| D22 | residual | Hybrid quote=execute is a DEX property; token TOCTOU residual written |

### E / catalog F / G / CH

| ID | Result | Notes |
|----|--------|-------|
| E1 | control | `layer_b_e1_create_pair_rejects_identical_assets` |
| E2 | control | Donation then provide (B2); interleaved TOCTOU pending-lcd |
| E3 | pending-lcd | Unauthorized burn of pair balance |
| E4 | N/A unless A3 | No rebase surface |
| E5 | N/A unless A13 | |
| E6–E9 | control / pending-lcd | Book reentrancy, batch, cycle |
| E10 | report-only | Solver spam = B13 ops |
| E11 | N/A unless listed | Zap floors when used as pool asset |
| E12 | N/A | Not a wrap/UST1 window template |
| E13 | residual | If fees paid in 8266, FoT would drift — pending 1:1 |
| E14 | pending-lcd | 6 decimals, not u128 edge |
| E15 | control | Rounding loop on mintable |
| catalog F1–F8 | residual / existing indexer tests | Display-layer; not a whitelist veto alone. Hub tilt (F3) if SpaceUSD pool becomes largest-liquidity — ops |
| G1 | control | Mintable vs mutant differential not full proptest |
| G2 | pass | `cw20_mutants.rs` |
| G3 | pending | Bindiff vs cw20-base not run |
| G4 | harness | `LCD_URL_SECONDARY` |
| G5 | residual | 1686 instances; SpaceUSD + TERRA sampled in #581 |
| G6 | control | `update_block` in harness |
| G7 | ops | F6 freeze drill in cw20-code-id-ops.md |
| G8 | static-pass | Host exports listed above |
| G9 | pass | Fetch self-test |
| CH1–CH18 | see parents | Chains inherit pending-lcd from D1/A3/A8/… |

## Layer A (token-only)

Backend: **A-mt** (`cw20_mintable` + mutants) via `make verify-issue-589`. **A-lcd** script: [`../../scripts/layer-a-lcd.sh`](../../scripts/layer-a-lcd.sh) — store + instantiate + 1:1 Transfer of the pinned LCD wasm on LocalTerra. Logs: `layer-a-lcd.json` (gitignored).

**2026-08-22 LocalTerra execution** (pin `953AD60C…`): store + instantiate + 1:1 Transfer of the pinned bytes (local store ids vary per run, e.g. 34 then 35). Instantiate ticker must be `[a-zA-Z-]{3,12}` (digits fail Terraport validate). This is a **LocalTerra copy** — do not whitelist columbus-5 **8266** from this run.

## Layer B (DEX + limits)

Backend: **B-mt** (P1/P2/P3/donation/flash/honeypot round-trip/limit escrow/same-asset CreatePair; FoT **P2** red). **B-lt** script: [`../../scripts/layer-b-lt.sh`](../../scripts/layer-b-lt.sh) — whitelist the **local** store id only, `CreatePair` vs EMBER, 1:1 into pair. `LAYER_B_LT=1` must not PASS as a stub ([#590](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/590)).

**2026-08-22 LocalTerra execution:** local store id whitelisted on the *test* factory, `CreatePair` vs EMBER, `one_to_one_into_pair: true`. Does **not** run round-trip swap **B7**, limit escrow **B6**, or P2-as-offer. Factory-global **B13** still blocks GO.

## Factory-global impact

Approving **8266** admits **every** current and future instantiate (SpaceUSD, Terraport TERRA, Terraport LP CW20s, unknown minters). Ops accept LUNC pair-create fee as spam control. This is **not** an automated pass (B13).

## Instance admin / migrate residual (F6)

SpaceUSD wasm admin is the **issuer**. F6 freezes a listed pair if they migrate off 8266; it does not steal reserves. Hostage/freeze leverage remains. Honest upgrade path: whitelist new id → migrate → Refresh.

## Unverified third-party claim (C7)

- [x] I did **not** treat CertiK / Skynet **file** hashes or marketing copy as LCD `data_hash`
- [x] Optional appendix (source URL / rebuild) does **not** block this verdict

## Appendix (optional — never a gate)

- Public dump: https://github.com/Terraport-Finance/Terra-token-contract @ `d854a219` (dated after store; **not** proven byte-identical)
- Rebuild attempts: [`audits/CW20-8266-581-hash-repro.md`](../../../audits/CW20-8266-581-hash-repro.md) — 2.3–2.7 KB short of `953AD60C…`. **Not a go/no-go input.**
- CertiK Skynet file hashes ≠ wasm `data_hash`.
