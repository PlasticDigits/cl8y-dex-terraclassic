# CW20 code-ID exploit / weird-token catalogue

Living audit checklist for [GitLab #589](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/589). Every row maps an ERC-20 analogue (or CosmWasm-native class) onto **CW20 + this DEX** and must be checked in `cw20-codeid-audits/codeids/<id>/REPORT.md`.

**Pass** on a row means the pinned LCD wasm does **not** exhibit the malice described. **Known-bad controls** (ALPHA **8654**, in-process FoT mutants) **must fail** 1:1 and **P2** — if they pass, the harness is wrong.

**Do not** add pair/router FoT balance-delta math (**H-01**). Invariant **F6** pin ([#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582)) stays live on-chain; **this catalogue decides whether a template may be added** to the factory whitelist.

---

## Sources

| Source | Use |
|--------|-----|
| [d-xo/weird-erc20](https://github.com/d-xo/weird-erc20) | ERC-20 non-standard token taxonomy |
| [Trail of Bits — Token integration checklist](https://github.com/crytic/building-secure-contracts) + [Non-standard tokens](https://github.com/crytic/building-secure-contracts/tree/master/development-guidelines/token_integration) | Integration checklist baseline |
| [Consensys Diligence — Token interaction checklist](https://consensys.github.io/smart-contract-best-practices/development-recommendations/token-interaction/) | Token interaction patterns |
| [cw-plus `cw20/src/msg.rs`](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw20/src/msg.rs) | Canonical CW20 surface |
| [fragwuerdig/cw20-taxed](https://github.com/fragwuerdig/cw20-taxed) + Columbus-5 ALPHA **8654** | Hidden / admin-settable tax; known-bad control |
| In-repo `cl8y-community-tax-token` ([#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592) / [#601](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/601)) | DEX-safe buy/sell/transfer (inbound 1:1). Columbus-5 **11611** listed 2026-08-23. REPORT: [`codeids/11611/REPORT.md`](codeids/11611/REPORT.md) (**GO**; A-lcd/B-lt executed). Rotate store **11619** (listed 2026-08-24): [`codeids/11619/REPORT.md`](codeids/11619/REPORT.md) (**GO**; A-lcd/B-lt executed). Adopt store **11626** (listed 2026-08-24): [`codeids/11626/REPORT.md`](codeids/11626/REPORT.md) (**GO**; A-lcd/B-lt executed). Pre-store stub: [`codeids/community-tax-token/REPORT.md`](codeids/community-tax-token/REPORT.md). Must **not** match 8654 inbound FoT. |
| [CWA-2024-002](https://github.com/CosmWasm/advisories), [ASA-2024-007](https://github.com/astroport-fi/astroport-core/security/advisories) | Wrapping pow; IBC unbacked mint |
| [`docs/exploit-replay-matrix.md`](../docs/exploit-replay-matrix.md) SEC-D02–D06; in-repo **P2** / **P3** / **R4** / **H1** | Historical replay + invariant IDs |
| EVM honeypot writeups — [TokenToolHub](https://tokentoolhub.com), [Hacken](https://hacken.io), [evm-token-guard](https://github.com/0xKitsune/evm-token-guard) | Honeypot / backdoor classes |
| Flash-loan / donation / first-depositor — Compound, ERC-4626 inflation; Osmosis SEC-D04 | LP share inflation |
| [GoPlus Token Risk Classification](https://whitepaper.gopluslabs.io/goplus-network/the-goplus-security-layer/security-data-layer/token-risk-classification) | Stateful / conditional risk fields |
| Trapdoor tokens — [BCRA 2025](https://doi.org/10.1016/j.bcra.2025.100370); [Trade or Trick (Xia 2021)](https://arxiv.org/abs/2109.00229); [Do not rug on me (Mazorra 2022)](https://arxiv.org/abs/2201.07220); [Art of the Scam (Torres USENIX Sec '19)](https://www.usenix.org/system/files/sec19-torres.pdf) | Honeypot / trapdoor taxonomy |
| [Honeypot.is](https://docs.honeypot.is/ishoneypot); [Cube Exchange honeypot](https://www.cube.exchange/what-is/honeypot-token); [PinkSale anti-bot](https://docs.pinksale.finance/pink-anti-bot/pink-anti-bot-guide); [SafeMoon.sol](https://github.com/safemoonprotocol/Safemoon.sol/blob/main/Safemoon.sol) | Stateful honeypots; auto-LP |
| [Lido stETH integration](https://github.com/lidofinance/docs/blob/main/docs/guides/lido-tokens-integration-guide); [ChainSecurity rebasing tokens guide](https://www.chainsecurity.com/blog/the-hitchhikers-guide-to-rebasing-tokens) | Share-based / rebase semantics |
| [ChainSecurity Curve LP oracle / Vyper 2023 reentrancy](https://www.chainsecurity.com/blog/curve-lp-oracle-manipulation-post-mortem) | Read-only reentrancy analogue |
| [MonoX $31M](https://medium.com/monoswap/exploit-post-mortem-33921a779b43); [Euler $197M](https://www.zellic.io/blog/euler-finance-exploit-analysis); [OpenZeppelin ERC-4626 inflation defense](https://www.openzeppelin.com/news/a-novel-defense-against-erc4626-inflation-attacks) | Same-token pair; donation inflation |
| [Etherscan spoof tokens](https://medium.com/etherscan-blog/spoof-tokens-on-ethereum-c2ad882d9cf6); [MyCrypto PoC](https://blog.mycrypto.com/bad-actors-abusing-etherscan-to-trick-you/); [Phantom Events arXiv 2502.13513](https://arxiv.org/html/2502.13513) | Event forgery |
| [Wormhole $326M](https://www.certik.com/blog/wormhole-bridge-exploit-incident-analysis); [EIP-7281 xERC20](https://eips.ethereum.org/EIPS/eip-7281) | Bridge minter rights |
| [CosmWasm CWA index 2023-001…2025-007](https://github.com/CosmWasm/advisories/blob/main/CWAs/README.md) incl. [CWA-2024-001](https://github.com/CosmWasm/advisories/blob/main/CWAs/CWA-2024-001.md) | Wasm / serde advisories |
| [Terra Classic Tax2Gas v3.3.0](https://github.com/classic-terra/documents/blob/main/chain-updates/v3_3_0.md); [Terraport $4M CertiK post-mortem](https://www.certik.com/blog/post-mortem-terraport-finance); [DeFiHackLabs](https://github.com/SunWeb3Sec/DeFiHackLabs) | Chain-specific ops; replay methodology |

---

## A. Malicious / weird CW20 (A1–A30)

| ID | Vector | ERC-20 analogue / citation | CW20 / DEX encoding | Listable? | Automated test |
|----|--------|----------------------------|---------------------|-----------|----------------|
| A1 | Fee-on-transfer / deflationary transfer | STA, PAXG, optional-fee USDT; Balancer 2020 STA drain; AuditBase M006 | `Transfer`/`Send`/`TransferFrom`/`SendFrom` recipient delta < declared; pair **P2** desync | **No** | `cw20_codeid_harness::mutant_a1_fot_breaks_one_to_one`; `adversarial_token::fee_on_transfer_creates_reserve_imbalance`; Layer A 1:1 on pinned wasm |
| A2 | Directional / sell-only / buy-only tax | BSC tax tokens; Token-Tax-Abuse-Science | Fee when `recipient` or `sender` is pair/router; wallet↔wallet 1:1 | **No** | `cw20_codeid_harness::mutant_a1_fot_breaks_one_to_one` (directional via `MutantConfig.directional_fee_recipient`); Layer A matrix over pair/router callers |
| A3 | Reflection / rebase / elastic supply | AMPL; weird-erc20 “balance modifications outside transfers” | Idle `balance` or `total_supply` changes; `balance_at` rewrites current | **No** | `cw20_codeid_harness::mutant_a3_rebase_idle_changes_balance`; `layer_a_mintable_idle_balance_stable` (control) |
| A4 | Hidden / admin-settable tax (`tax_map`, `UpdateTaxMap`) | cw20-taxed; ALPHA **8654** | Query/decomp shows tax config; setter can enable FoT after listing (F6 same-code-id setter invisible) | **No** | `cw20_codeid_harness::mutant_d1_tax_activates_after_height`; pinned **8654** must FAIL; decomp + `TaxMap` query in REPORT |
| A5 | Transfer hook / callback on plain `Transfer` | ERC-777 `tokensToSend`/`tokensReceived`; imBTC Uniswap; Lendf.me; Cream | `Transfer` (not only `Send`) dispatches `WasmMsg` to sender/recipient | **No** | `cw20_mutants` + `MutantConfig.transfer_callback`; reentrancy slice with pair as recipient |
| A6 | `Send` hook grief / reenter pair | ERC-677 `transferAndCall`; CosmWasm `Receive` | Receiver re-enters pair `Swap`/`Withdraw`/`PlaceLimit` mid-tx | Must not extract value | `reentrancy_tests::test_reentrant_swap_during_swap_rejected`; `layer_a_mintable_receiver_revert_rolls_back`; candidate wasm as offer token |
| A7 | Allowance race / `IncreaseAllowance` surprise | ERC-20 approve race; USDT “set 0 first” | CW20 increase/decrease — no `Approve {amount}` overwrite race | Document; **No** if spender exceeds cap | `layer_a_mintable_insufficient_allowance_rejected`; allowance double-spend window test on pinned wasm |
| A8 | Allowance backdoor | evm-token-guard `transferFrom` without allowance / `_isBot` | Admin or hardcoded addr moves tokens without allowance | **No** | `cw20_codeid_harness::mutant_a8_backdoor_skips_allowance` |
| A9 | Blocklist / allowlist honeypot | USDC/USDT blocklist; pair-address sell block | `Transfer` to pair reverts for non-admin after buy | **No** | `cw20_codeid_harness::mutant_a9_block_recipient_reverts`; `layer_b_b7_round_trip_swap_succeeds_honest` (control) |
| A10 | Pause / trading toggle | BNB/ZIL pause; “enable trading” honeypot | Admin pause stops pair `Transfer` | **No** unless we control admin (residual) | `cw20_codeid_harness::mutant_a10_pause_stops_transfer` |
| A11 | Max-tx / max-wallet shrink | Soft honeypot | Admin lowers cap below pair balance | **No** | `cw20_mutants` + `MutantConfig.max_wallet`; magnitude fuzz (D2) |
| A12 | Hidden mint / stealth balance write | Honeypot mint-to-drain | Mint without minter; direct balance map write | **No** | `cw20_codeid_harness::mutant_a12_hidden_mint`; `layer_a_mintable_unauthorized_mint_rejected` (control) |
| A13 | Flash mint | DAI flash mint | Intra-tx supply `u128::MAX` then burn; breaks first-deposit / donation | **No** unless proven unused and P3 holds | `cw20_mutants` + `FlashMint` execute; composite **E5** / **CH8** |
| A14 | Upgrade / `Migrate` surface | USDC proxy; weird-erc20 Upgradable | Instance admin + `MsgMigrateContract` — **F6** freeze | Residual; F6 required | `asset_code_id_pin_tests::*`; REPORT admin chain |
| A15 | Proxied / multi-address token | Rescue-drain via second entry point | Two contracts share balances; sweep/rescue confusion | **No** | Decomp multi-contract fingerprint; sweep confusion **B12** |
| A16 | Missing / lying query | Missing `bool` return; Tether Gold `false` success | Query `Balance` disagrees with transfer; `total_supply` ≠ Σ balances | **No** | `cw20_codeid_harness::mutant_a16_lie_balance_disagrees_with_transfer` |
| A17 | High / low decimals | USDC 6; YAM 24; Gemini 2 | `decimals > 18` → `CreatePair` reject (**P3**); `0` or extreme → overflow | Follow P3; **No** if math breaks **P1** | `layer_a_mintable_decimals_le_18`; `pair_coverage_tests::test_create_pair_rejects_cw20_above_bootstrap_decimal_cap`; **E14** boundary matrix |
| A18 | Revert on zero / to-zero / huge amount | LEND zero transfer; OZ to-zero; UNI uint96 | Pair/router/limits must not brick on honest dust | **No** if token reverts on amounts the pair uses | `layer_a_mintable_zero_amount_deterministic`; `layer_a_mintable_oversize_rejected`; D2 amount fuzz |
| A19 | `TransferFrom` self-semantics | DSToken vs OZ | Pair never relies on self-`TransferFrom`; still test | Document | Self-`TransferFrom` probe on pinned wasm; N/A if pair never uses path |
| A20 | Permit / phantom function | DAI permit; Multichain no-op permit | If `permit` exists and is no-op, DEX must not assume allowance changed | N/A if absent | Decomp + execute probe; N/A+reason if no `permit` msg in wasm |
| A21 | Name/symbol XSS | Etherdelta JS in token name | Record; dApp must escape (existing frontend) | Not whitelist veto alone | REPORT record; frontend escape — N/A+reason on-chain |
| A22 | Non-string metadata | MKR `bytes32` name | CW20 is string; binary metadata breaks queries | **No** if `TokenInfo` unreadable | `TokenInfo` query parse test on pinned wasm |
| A23 | Interest / airdrop to holders | cToken / rebase-like drip | Pair balance grows without `Provide` → **P2** desync | **No** | Idle-balance growth test (extends A3); `layer_a_mintable_idle_balance_stable` |
| A24 | Transfer less than `amount` when `amount == max` | cUSDCv3 max-uint special case | `Uint128::MAX` transfer credits only balance | **No** if pair can pass max | Max-uint transfer 1:1 probe on pinned wasm |
| A25 | Native-denom dual representation | CELO / POL / Uniswap V4 | CW20 wrap vs bank denom — factory rejects natives | **No** if dual-spend | Factory asset-info validation; decomp bank-denom hooks |
| A26 | IBC / bridged unbacked mint | ASA-2024-007; Terra 2024 Astroport | `ibc_receive` / IBC hooks → infinite-mint class | **No** unless proven unused | **G8** host-import + IBC enum; **B15** / **CH9** |
| A27 | CWA-2024-002 wrapping pow | cosmwasm-std < 1.4.4 `Uint256::pow` | Token uses wrapping pow for balances | **No** | **D20** CWA fingerprint; decomp pow usage |
| A28 | `requires_terra` / classic taxer integration | Terra Classic burn tax on **natives**, not CW20 | Strings alone ≠ FoT; prove 1:1 on CW20 path | Decide from tests | 1:1 transfer path on pinned wasm; REPORT Tax2Gas note |
| A29 | Snapshot used as rebase | 8266 `balance_at` | Idle current balance stable; snapshot ≠ live mutate | **No** if live mutate | `layer_a_mintable_idle_balance_stable`; snapshot query vs live `Balance` diff |
| A30 | Marketing / logo store blowup | Unbounded logo upload | DoS / gas grief only | Usually residual | Query gas benchmark (**D21**); REPORT record |

---

## B. DEX exploits using weird token (B1–B15)

| ID | Vector | ERC-20 analogue / citation | CW20 / DEX encoding | Listable? | Automated test |
|----|--------|----------------------------|---------------------|-----------|----------------|
| B1 | Reserve desync → insolvent withdraw | FoT / rebase after swap; Balancer STA class | After swap, LP withdraw fails or steals honest side (**P2**) | **No** (candidate); known-bad **must** desync | `cw20_codeid_harness::layer_b_b1_fot_desyncs_reserves`; `adversarial_token::fee_on_transfer_creates_reserve_imbalance` |
| B2 | First-depositor / donation inflation | Osmosis SEC-D04; ERC-4626 inflation | Donate then provide; LP shares must not inflate (**P3**) | Candidate must pass | `cw20_codeid_harness::layer_b_p3_donation_does_not_inflate_lp_shares`; `security_tests::test_direct_token_donation_does_not_inflate_lp_shares`; **E2** TOCTOU |
| B3 | Flash provide → swap → withdraw | Flash-loan LP inflation class | No risk-free profit | Candidate must pass | `cw20_codeid_harness::layer_b_b3_flash_provide_swap_withdraw_no_profit_honest`; `security_tests::test_flash_provide_swap_withdraw_no_profit` |
| B4 | Router dust attribution | weird-erc20 fee-on-transfer router accounting | Pre-seed router with candidate; hop uses delta only (**R4**) | Candidate must pass | `adversarial_token::router_ignores_pre_existing_dust_on_output_token`; router multihop with pinned wasm |
| B5 | Hook + token callback sandwich | ERC-777 + AMM callback | Token `Send` receiver + pair hook both re-enter; no double-pay | Must not extract value | `audit_invariant_tests::swap_fails_atomically_when_allowlisted_hook_reverts`; reentrancy + candidate offer token |
| B6 | Limit escrow skim | FoT on limit `Send` path | `PENDING_ESCROW` vs actual balance; cancel/claim honesty (**L1**) | Candidate 1:1 only | `cw20_codeid_harness::layer_b_l1_limit_place_escrow_one_to_one`; limit cancel/claim with pinned wasm |
| B7 | Honeypot: buy on pool, cannot sell | Trapdoor / Honeypot.is sell-block | Swap A→candidate succeeds; candidate→A reverts | **No** | `cw20_codeid_harness::layer_b_b7_round_trip_swap_succeeds_honest`; round-trip on pinned wasm |
| B8 | Pair blacklisted by token after TVL | USDC blocklist after listing | Token admin blocks pair address; withdraw/swap die | **No** unless we own admin | `mutant_a9_block_recipient_reverts` pattern with pair as blocked recipient |
| B9 | Migrate-after-list to FoT | Proxy upgrade to taxed wasm | Honest wasm → `MsgMigrateContract` FoT → writes fail closed (**F6**) | F6 freeze required | `asset_code_id_pin_tests::*`; candidate as honest side once |
| B10 | Sandwich / oracle tilt | SEC-D05 stale oracle class | Large swap vs `max_spread` / `min_return` still holds | Candidate must pass | `limit_order_tests::hybrid_*`; `oracle_tests::test_oracle_manipulation_resistance_same_block` |
| B11 | Commission on failed swap | Token reverts mid-settlement | Treasury unchanged on failed swap (**P10**) | Candidate must pass | `audit_invariant_tests::*` (failed swap treasury); revert mid-`Transfer` fixture |
| B12 | Sweep confusion | Lying `Balance` / dual address | Sweep steals pool despite honest reserves | **No** | `sweep_tests` + lying-balance fixture (**A16**); **E4** rebase×sweep |
| B13 | CreatePair spam / Everybody instantiate | Factory-global whitelist implication | Listing 8266 admits all instances — report-only | Report-only | N/A+reason: governance / ops note in REPORT.md; **E10** automates solver spam slice |
| B14 | Mint inflation vs “USD” ticker | SpaceUSD-style minter cap | Report residual (not **P2**) | Residual in REPORT | N/A+reason: minter cap + `TokenInfo` in REPORT; **D15** premine detection |
| B15 | IBC unbacked mint dumped on pool | ASA-2024-007; Wormhole class | IBC-mintable token drains pool | **No** unless out of scope + written | **G8** IBC enum; **CH9** conditional composite |

---

## C. Harness / process abuse (C1–C7)

| ID | Vector | ERC-20 analogue / citation | CW20 / DEX encoding | Listable? | Automated test |
|----|--------|----------------------------|---------------------|-----------|----------------|
| C1 | Rebuilt wasm ≠ LCD `data_hash` | Supply-chain trojan wasm | Fetch/pin step **FAIL**; suite refuses wrong binary | Procedure **FAIL** | `scripts/qa/verify-issue-589.sh` fetch step; `cw20-codeid-audits/scripts/fetch-lcd-wasm.sh` self-tests; **G9** |
| C2 | Skip decomp, run tests only | Audit theatre | Procedure / CI **FAIL** (both required) | Procedure **FAIL** | `scripts/qa/verify-issue-589.sh` decomp gate |
| C3 | Skip tests, decomp prose only | Audit theatre | Procedure / CI **FAIL** | Procedure **FAIL** | `scripts/qa/verify-issue-589.sh` Layer A+B gate |
| C4 | Mark 8654 / FoT as go | False negative | Reviewer veto; known-bad must be red | Harness **FAIL** | `scripts/qa/verify-issue-589.sh` known-bad job; `mutant_a1_*` must fail |
| C5 | Silent Layer B skip without LocalTerra | False green CI | **FAIL** or explicit skip reason | Procedure **FAIL** | `scripts/qa/verify-issue-589.sh` (`make has-localterra` hint; no silent pass) |
| C6 | Commit mnemonic / admin key in `codeids/` | Secret leak | Hook or CI secret scan; reject | Procedure **FAIL** | `scripts/qa/verify-issue-589.sh` secret scan step |
| C7 | CertiK Skynet file hash as `data_hash` | Third-party hash confusion | Procedure forbids; REPORT “unverified third-party claim” | Procedure **FAIL** | REPORT template checkbox; `fetch-lcd-wasm.sh` pins LCD `CodeInfo.data_hash` only |

---

## D. Stateful / conditional (D1–D22)

| ID | Vector | ERC-20 analogue / citation | CW20 / DEX encoding | Listable? | Automated test |
|----|--------|----------------------------|---------------------|-----------|----------------|
| D1 | Time/height-activated behavior | PinkSale anti-bot; Honeypot.is point-in-time caveat | Honest before height X, taxed/blocked after | **No** on delta | `cw20_codeid_harness::mutant_d1_tax_activates_after_height`; **G6** time-travel re-probe; **CH1** |
| D2 | Magnitude-dependent behavior | GoPlus `is_anti_whale`; SafeMoon `setMaxTxPercent`; Honeypot `maxBuy`/`maxSell` | Dust 1:1, whale taxed/reverted | **No** on threshold | `cw20_codeid_harness::mutant_d2_magnitude_tax`; amount fuzz 1 → supply |
| D3 | History/op-count-dependent activation | Cube Exchange stateful honeypot; token generators | First N transfers honest, then tax | **No** | **G6** op-count loop (1k ops then re-probe); decomp milestone setters |
| D4 | Caller-class discrimination | `extcodesize == 0` honeypot; Trapdoor modifiers | `ContractInfo` on `info.sender` branches | **No** | Transfer matrix: EOA, contract, pair, router, treasury, book |
| D5 | Entrypoint-selective taxation | cw20-taxed per entrypoint | `Transfer` 1:1 but `Send`/`TransferFrom` taxed | **No** | `cw20_codeid_harness::mutant_d5_send_taxed_transfer_honest`; per-entrypoint 1:1 |
| D6 | External dependency on transfer path | GoPlus `external_call`; Curve read-only reentrancy analogue | Token queries/calls second contract mid-transfer | Default **No** | Decomp submessage enum; mutate dependency + re-test |
| D7 | Token-as-trader (auto-LP / swapAndLiquify) | SafeMoon `swapAndLiquify` | Token → pair submessage mid pair execution | **No** | **E6**/**CH14** auto-LP fixture; pair fail-closed |
| D8 | Permissionless economic setters | SafeMoon `setAutomatedMarketMakerPair` (permissionless variant) | Anyone registers pair → enables tax | **No** | `cw20_codeid_harness::mutant_d8_permissionless_pair_register_enables_directional_tax`; **CH2** |
| D9 | Hidden owner / fake renounce | GoPlus `hidden_owner`; Terraport insider class | Admin contract with public re-seize path | Residual documented | Decomp admin chain; **CH15** migrate-to-other-whitelisted-id |
| D10 | Share-based / ghost dust | Lido stETH; ChainSecurity 1-wei ghost share | Full-balance transfer leaves dust; **P2** / sweep break | **No** | `cw20_codeid_harness::mutant_d10_ghost_dust_leaves_one`; sweep interaction **E4** |
| D11 | Rate-limited transfers | GoPlus `trading_cooldown`; PinkSale time limit | Per-block cooldown breaks batch flows | **No** | `cw20_codeid_harness::mutant_d11_cooldown_rejects_second_transfer_same_block`; **CH3** |
| D12 | Payable CW20 | Tax2Gas native tax; payable transfer tokens | `info.funds` required → pair/router DoS | **No** | `cw20_codeid_harness::mutant_d12_payable_requires_funds`; **CH18** |
| D13 | Dynamic decimals | Mutable `TokenInfo` | Post-bootstrap decimals break L20 price band | **No** | `cw20_codeid_harness::mutant_d13_set_decimals_mutates_tokeninfo`; **CH13** |
| D14 | Init-parameter fuzz matrix | Token generators; Everybody instantiate | Same wasm, hostile init enables FoT/honeypot | Hostile config **No** | Parameterized instantiate matrix in REPORT + harness; **G5** live-instance sample |
| D15 | Stealth genesis + pagination honesty | Hidden premine; truncated `AllAccounts` | State not reachable via queries | **No** | Init replay vs full `AllAccounts`/`AllAllowances` pagination |
| D16 | Event honesty | Etherscan spoof; Phantom Events | Events lie vs balance deltas | **No** for listing | `MutantConfig.lie_events`; indexer reconcile **F1** / **CH16** |
| D17 | Hidden execute/query variants | Art of the Scam trojan branches | Unknown variants mutate state | **No** | Fuzz unknown execute/query; decomp handler coverage **G3** |
| D18 | `Receive` msg parser robustness | CWA-2024-001 serde-json-wasm SO | Crafted `Cw20ReceiveMsg` panics pair | Must reject cleanly | Msg fuzz vs pair/router `Receive`: size, nesting, duplicate keys |
| D19 | Gas-scaling benchmark | SWC-128; SafeMoon reflection O(n) | Swaps exceed block gas at scale | **No** before brick | Gas curve vs holder count; **CH6** |
| D20 | Full CWA fingerprint sweep | CWA 2023-001…2025-007 | Affected cosmwasm-std / wasmvm range | Document decision | Fingerprint → advisory matrix row in REPORT.md |
| D21 | Query-side DoS | Unbounded `AllAccounts`; panicking queries | Indexer/route-solve poisoned | **No** or residual | Query gas + panic fuzz; indexer soak **catalog F7** |
| D22 | Simulation-vs-execution divergence | Honeypot.is simulation caveat | Quote-time ≠ exec-time under state change | Document residual | Quote→execute atomicity tests; REPORT residual note |

---

## E. Additional DEX vectors (E1–E15)

| ID | Vector | ERC-20 analogue / citation | CW20 / DEX encoding | Listable? | Automated test |
|----|--------|----------------------------|---------------------|-----------|----------------|
| E1 | Same-token pair | MonoX $31M (`tokenIn == tokenOut`) | Factory rejects identical assets | N/A (factory rejects) | `cw20_codeid_harness::layer_b_e1_create_pair_rejects_identical_assets`; **CH17** + hidden mint |
| E2 | Donation TOCTOU on Provide | Euler donateToReserves; OZ ERC-4626 inflation | Donation interleaved between quote and provide | Candidate must pass | Provide with min-shares under interleaved donation |
| E3 | Public burn of pair balance | Trapdoor burn backdoors | Unauthorized burn desyncs **P2** | **No** | Burn authorization matrix; post-burn sweep cannot extract LP value |
| E4 | Rebase × sweep both directions | Elastic supply + sweep | Rebase-down sweep extracts “excess”; rebase-up skim | **No** | Sweep under rebase fixture; **CH7** |
| E5 | Flash-mint composite | A13 + B2 chain | Mint MAX → donate → provide dust → burn → victim provide | **No** | Composite scenario test; **CH8** |
| E6 | Token-hook reentrancy on book | Limit `Send` escrow callbacks | Place/cancel/claim reentered via token | Must not extract value | Reentrancy fixtures on book ops (extends **B5**) |
| E7 | Hook side-effects mid-batch | Batch refund / ladder (#494) | Hook flips state between batch items | Must not strand | Batch atomicity under mid-batch state flip; **CH3** |
| E8 | Token pause during parked-expired claim | L22/#504 park; L6 pair pause | Token pause bricks parked claim | Residual in REPORT | Claim path with pausable fixture |
| E9 | Cyclic routes (A→B→A) | Multihop + rate limit / hook | Candidate twice in one tx | Candidate must pass | Router cycle with candidate at both ends; **R4** per hop |
| E10 | Solver/graph spam | Everybody instantiate spam | route/solve latency / 500 on bad queries | Graceful exclude | Solver benchmark 1k/10k junk pairs; fault-injection exclude hop |
| E11 | Zap floors | #533 / #559 one-sided zap | FoT/rebase breaks zap accounting | Candidate must pass | `make verify-issue-533`; `make verify-issue-559` with pinned wasm |
| E12 | Wrap-mapper / UST1 adjacency | #523 unwrap dual-read; #506 window | Conditional if template used for wrap-like asset | Conditional checklist | `make verify-issue-523`; `make verify-issue-506` when applicable |
| E13 | Fee-treasury in candidate token | F4 treasury rotation | Fees in FoT/rebase token drift | Document | Treasury accrual vs balance reconciliation fixture |
| E14 | Math-boundary fuzz | u128 max supply; 18 decimals | k-math, L20 band, TWAP overflow | **No** on panic/wrap | Boundary matrix: **P1** within rounding; extends A17 |
| E15 | Rounding arbitrage loop | Micro provide/swap/withdraw harvest | k decrease beyond documented rounding | Candidate must pass | N-iteration loop; extends **B3** / `test_flash_provide_swap_withdraw_no_profit` |

---

## F. Off-chain trust chain (catalog F1–F8)

> **Note:** Catalogue section **F** is off-chain trust. Invariant **F6** (listed CW20 `code_id` pin) is unchanged — see **A14** / **B9** / `asset_code_id_pin_tests::*`.

| ID | Vector | ERC-20 analogue / citation | CW20 / DEX encoding | Listable? | Automated test |
|----|--------|----------------------------|---------------------|-----------|----------------|
| catalog F1 | Event-vs-state indexer | Phantom Events; Etherscan spoof | Indexer tape/P&L from token events ≠ `Balance` | Indexer must reconcile | Indexer event-vs-balance reconciliation; **CH10** / **CH16** |
| catalog F2 | Metadata injection | NUL-byte name; XSS in symbol | Indexer ingest stall / broken display | dApp must survive | Metadata fuzz ingest test; **CH11** |
| catalog F3 | Hub-price tilt | Fake-depth pool tilts hub USD | Attacker pool moves hub beyond caps | Document caps | Hub derivation deviation test; **CH12** |
| catalog F4 | Oracle ticker collision | #515 / #580 ticker-scoped feeds | Ticker “USTC”/“FDUSD” mis-attribution | Address-keyed only | `make verify-issue-515`; `make verify-issue-580` colliding-ticker fixture |
| catalog F5 | Candle-close sniping | #568 mark-to-market | Dust swap at extreme skews USD mark | Mark must resist | `make verify-issue-568` dust-at-extreme fixture |
| catalog F6 | Wash-volume stats poisoning | Self-trade zero-cost volume | #553 / #576 / #577 inflated stats | Document heuristic | Volume self-trade attribution test |
| catalog F7 | Query-DoS → indexer lag | **D21** chained | Stale oracle chain sitewide | Must surface stale | `make verify-issue-577` stale overview; indexer lag under slow-query fixture |
| catalog F8 | P&L events vs deltas | #551 / #560 realized P&L | FoT events ≠ balance deltas | Flag unmetered pairs | P&L reconciliation with FoT fixture; **CH16** |

---

## G. Harness methodology (G1–G9)

| ID | Vector | ERC-20 analogue / citation | CW20 / DEX encoding | Listable? | Automated test |
|----|--------|----------------------------|---------------------|-----------|----------------|
| G1 | Differential vs cw20-base | Csmith / differential testing lineage | Randomized op sequences; diff state | Harness quality gate | `cw20_codeid_harness` proptest-style walks vs reference |
| G2 | Mutant library as oracles | Mutation testing (cargo-mutants) | One mutant per A/D row must be caught | Harness must catch all | `smartcontracts/tests/src/cw20_mutants.rs`; `mutant_*` + `layer_*` tests |
| G3 | Differential decompilation | Diaphora / BinDiff | Injected basic blocks vs cw20-base | Measurable decomp gap | Decomp coverage metric in REPORT; unmapped = audit focus |
| G4 | Multi-endpoint consensus fetch | LCD supply-chain | ≥2 LCD `CodeInfo.data_hash` agree before pin | Procedure gate | `fetch-lcd-wasm.sh` multi-endpoint mode |
| G5 | Live-instance sampling | Template vs instance honesty | Sample 8266+ instances: minter, admin, tax | REPORT blast radius | LCD instance enumeration script; REPORT factory-global section |
| G6 | Time-travel primitives | Required by D1/D3/D19 | Advance height/time; op-count histories | Harness infrastructure | `cw-multi-test` block advance in harness; **CH1** |
| G7 | Post-listing monitoring + F6 drill | Honeypot.is point-in-time caveat | Watch migrate/admin/tax; time-to-freeze | Ops residual | Runbook cross-link; not per-code-id automated pass |
| G8 | Host-import + IBC enum | Extends A26 | All IBC entrypoints + unexpected imports | **No** if IBC mint | Wasm fingerprint import table in REPORT |
| G9 | Seeded-wrong self-tests | Extends C1 | Truncated wasm, hash mismatch must fail | Harness must fail closed | `fetch-lcd-wasm.sh` + `verify-issue-589.sh` negative fixtures |

---

## CH. Exploit chains (CH1–CH18)

| ID | Vector | ERC-20 analogue / citation | CW20 / DEX encoding | Listable? | Automated test |
|----|--------|----------------------------|---------------------|-----------|----------------|
| CH1 | D1 time tax → post-list B7 honeypot | Stateful honeypot activation | Honest at audit; sells revert after height H | **No** | **G6** time-travel + **B7** re-probe; **G7** monitoring note |
| CH2 | D8 permissionless pair reg → D5 selective tax | SafeMoon pair registration | After `CreatePair`, DEX paths taxed | **No** | Post-create per-entrypoint 1:1; `mutant_d8_*` + `mutant_d5_*` |
| CH3 | D11 cooldown × L11 batch / #494 ladder | Batch + rate limit | Second transfer in batch reverts; strands funds | Must not strand | Batch ops with cooldown fixture; **E7** |
| CH4 | D2 min-transfer × L22 parked dust | Min amount vs dust refund | Park refund below minimum bricks claim | Must not brick | Dust-park claim with min-transfer fixture |
| CH5 | A8 backdoor × L1 escrow | Admin pulls pair escrow | Escrow drain while book looks intact | **No** | Escrow drain attempt must fail; balance divergence alarm |
| CH6 | D19 reflection O(n) × holder growth | SWC-128 gas DoS | Swaps exceed block gas at 10k holders | **No** before brick | Gas-curve projection row (**D19**) |
| CH7 | A3 rebase-down → sweep → rebase-up | Elastic + sweep | LPs short after sweep “excess” | **No** | **E4** rebase×sweep composite |
| CH8 | A13 flash mint → B2 donation → provide | Flash + inflation | Victim deposit at manipulated rate | **No** | **E5** composite |
| CH9 | A26/B15 unbacked mint → multihop dump | Wormhole / IBC class | Depeg cascade into #506 window | Conditional **No** | **G8** + router multihop dump scenario |
| CH10 | D16 event spoof → indexer → frontend | Phantom liquidity display | Users trade on fake tape; pair solvent | Indexer must flag | **catalog F1** reconciliation; spoofed tape test |
| CH11 | catalog F2 NUL metadata → indexer stall | Ingest DoS | Stale hub USD sitewide | Must surface stale | **catalog F2** fuzz + **catalog F7** |
| CH12 | catalog F3 hub tilt → #569 / #553 poison | Fake-depth pool | Protocol stats / leaderboard wrong | Caps documented | **catalog F3** deviation test |
| CH13 | D13 dynamic decimals → L20 price band | Post-bootstrap misprice | Resting limits fill at stale scale | **No** | Decimals-mutation through place/fill |
| CH14 | D7 swapAndLiquify → mid-sell reenter | SafeMoon mid-settlement | Token trades on pair during pair sell | **No** | **E6** / **D7** auto-LP fixture |
| CH15 | D9 fake renounce → migrate other whitelisted ID | Admin re-seize + migrate | F6 pin is listing id, not any whitelisted id | F6 must freeze | `asset_code_id_pin_tests::*` migrate-to-other-whitelisted-id |
| CH16 | D16 event-liar × catalog F8 | FoT events full, delivery less | Indexer P&L wrong; on-chain **P2** catches | Dual detection | **catalog F1** + **catalog F8**; **P2** on-chain |
| CH17 | E1 same-token pair × hidden mint | MonoX replay | Self-swap price overwrite then drain | **No** | **E1** factory reject + MonoX-style composite |
| CH18 | D12 payable × router | Payable transfer DoS | Every routed swap fails or leaks natives | **No** | Router with payable fixture; **D12** |

---

## Documented N/A mappings

| EVM / Solidity class | CosmWasm / this DEX handling | Catalogue row |
|----------------------|------------------------------|---------------|
| **`tx.origin` authentication tricks** | No `tx.origin`; `info.sender` only | N/A+reason — no CosmWasm analogue (Trapdoor paper class is EVM-only) |
| **`extcodesize == 0` caller checks** | `ContractInfo` query on sender | Covered as **D4** caller-class matrix |
| **`selfdestruct` / CREATE2 metamorphic redeploy** | No selfdestruct; code IDs immutable | **A14** / **B9** / invariant **F6** / **CH15** — surface is `MsgMigrateContract` |
| **Missing `bool` return / return-data bombs** | CosmWasm errors instead of false returns | N/A+reason — wasm memory model differs; failures are explicit errors |
| **Curve read-only reentrancy (`get_virtual_price`)** | Queries cannot re-enter mid-execution; indexer reads committed state only | Residual: token reads pair mid-transfer (**D6**); display staleness (**catalog F5** / **catalog F7**) |
| **Solidity compiler honeypots (Art of the Scam)** | No direct wasm analogue | **D20** CWA fingerprint + **G3** differential decompilation |

---

## Keeping this current

When intake or public research surfaces a **new** token or DEX exploit class:

1. Add a row to the appropriate section (A–CH) in **this file** in the **same MR** as the per-code-id `REPORT.md`.
2. Add or extend a harness test (`cw20_codeid_harness::*`, `cw20_mutants`, or existing `security_tests` / `audit_invariant_tests`) and reference it in the **Automated test** column.
3. Cross-link [`docs/exploit-replay-matrix.md`](../docs/exploit-replay-matrix.md) and [`docs/runbooks/cw20-whitelist-policy.md`](../docs/runbooks/cw20-whitelist-policy.md) if the class affects ops policy.
4. Re-run `make verify-issue-589` (when shipped) and ensure known-bad controls (**8654**, FoT mutants) still fail 1:1 and **P2**.
