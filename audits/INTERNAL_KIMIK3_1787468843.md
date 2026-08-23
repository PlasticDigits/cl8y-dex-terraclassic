# Internal Security Audit — Community Tax CW20 + DEX integration

**ID:** `INTERNAL_KIMIK3_1787468843`
**Date:** 2026-08-23
**Scope:** `cl8y-community-tax-token` (columbus-5 **11611**), `cl8y-community-token-launcher` (**11614** `terra126pr5…ahzwze`), `cl8y-community-tax-autolp` (**11613**), factory / pair / router integration, Create Token + Manage Token dApp, indexer catalog (`/community-tokens`), invoice pay path (#595).
**Method:** Full-crate review, DEX write-path review, indexer/API/SQL review, frontend invoice + swap path review, web research (FoT/AMM, CosmWasm reentrancy, honeypot launch-guard patterns), `glab` for open issues, multitest PoCs that **pass on current code** (they demonstrate the defect).
**Prior audits treated as still-active:** `INTERNAL_KIMIK3_1785897304`, `INTERNAL_KIMIK3_1786830980`, `INTERNAL_KIMIK3_1787230030` (F6 pin). Open related issues: #603 (migrate-adopt design), #604 (identity validation), #605 (SKU init + percent taxes), #526 (EOA / 2-of-3 queue).
**Mainnet state at audit:** launcher config queried live (`token_code_id=11611`, `autolp_code_id=11613`, `router=terra1e7s0h9…rsrw`). LCD `code/11611/contracts` was **empty** — no community tax token instantiated yet. Window is pre-launch.

---

## 0. Executive summary

The template is **DEX-safe in the intended sense** (T592-1): inbound credits to pair / router / escrow stay 1:1. Pair and router wasm are unchanged. Sell tax is extra-debit; buy tax is an outbound split. Invoice amounts are exact-50-UST1 and revert on wrong token / wrong amount / no-op. Wasm admin is CMM, not the manager. Indexer attestation (`code_id` + CMM admin + launcher origin + launcher-emitted `create_token_ready`) is sound. SQL is parameterized. No SQLi, no reflection/rebase/blacklist APIs, no inbound FoT.

**The product as shipped on columbus-5 is not ready for retail Create Token / Manage Token.** Recast after lead review (2026-08-23): C-1 is a **product High** (dead official path, no fund loss). C-2 is a **design decision** (T592-1 + router exempt vs advertised buy/sell; **O601-7** already deferred router multi-hop). Live wasm **11611 / 11614 / 11613** — token/launcher/AutoLP code fixes need a CMM/2-of-3 migrate; C-1 has a dApp-only workaround.

| Sev | ID | Disposition | One-line |
|-----|----|-------------|----------|
| **High** (product) | C-1 | **New** (bundle launcher invoices) | Launcher `enable_feature` always `Unauthorized` — dApp Enable Feature is dead. |
| **Design** | C-2 | **New design issue** (three options) | Protocol-exempt router skips buy/sell on multi-hop and permissionless 1-op router Send. Official single-hop pair Send still taxes. |
| Residual / #605 | H-1 | **Expand #605** | AutoV2Lp SKU is payable; same-tx bind is documented out of v1, but **no later bind execute** exists. |
| **Medium** | H-2 | **New** (bundle launcher invoices) | Duplicate SKUs in `features[]` are double-charged. UI checkboxes do not send dups. |
| **High** | H-3 | **New** (bundle launch guards) | `cooldown_blocks > 0` rate-limits the **pair** (from+to recorded). Stronger than 11611 D11 same-sender residual. |
| **High** | H-4 | **New** (bundle launch guards) | `max_wallet` applies to the pair on `TransferFrom` provide — LP adds brick after sells grow the pair above the cap. |
| Residual / spec | H-5 | **Do not file** | `trading_enabled=false` locks pair→EOA (T592-11 / 11611 A9/A10/E8). Spec-change later, not a new bug. |
| Split | H-6 | On-chain **T592-7 / 11611 B6** (do not file); quote → **#593** | Buy tax on pair→EOA is documented; dApp does not quote the outbound split. |
| #605 note | M-1 | **Expand #605** | `variable_rates` is theater. **Implement only if needed; otherwise remove** the SKU. |
| **Medium** | M-2 | **New** (bundle AutoLP) | AutoLP skim has no slippage / `min_return`; permissionless sandwich. |
| **Medium** | M-3 | **New** (bundle AutoLP) | AutoLP `pair` must be factory-listed **and** have the tax token as one side. |
| Design | M-4 | **Do not file** as a vuln | Manager tax/halt/treasury is the product. Disclosure can ride a frontend ticket. |
| **Medium** | M-5 | **New** | ExemptionDirectory must skip **transfer, buy, and sell** (not transfer-only). |
| **Low** | L-1 | **Bundle with C-1** | QA smoke sends UST1 **directly to the token**, so it never sees C-1. |
| — | L-2 | Already **#604** | Decimals 0–5 / charset. |
| — | L-3 | Already **#605** | Manage Token editors. |
| **Info** | I-1 | Analysis only | No 11611 instances at audit — C-2 latent until first `RegisterListedPair`. |

**Verdict:** **Do not market Enable Feature, AutoLP, or launch guards as working.** File the bundled issues in §14. Do **not** mint tickets from Chains J–R, prior-audit indexer residuals, #526, or #603. PoCs live in `smartcontracts/contracts/community-token-launcher/tests/audit_poc.rs` (9/9 pass against current wasm — they **demonstrate** defects; invert when fixed).

---

## 1. Scope map — extra surfaces found in the codebase sweep

Beyond the three crates, the sweep added these as in-scope because they decide whether tax is paid, whether funds can exit, or whether the catalog lies:

| Area | Why |
|------|-----|
| `community-tax-token` `tax.rs` / `invoice.rs` / `pair_registry.rs` | Classification, extra-debit, launch guards, invoice payer check |
| `community-token-launcher` `create_token` / `enable_feature` / `reply` | Paid create, dead AutoLP instantiate, broken forward of EnableFeature |
| `community-tax-autolp` `SkimToLp` + `reply` | Permissionless swap-then-provide; no slippage; manager pair |
| Pair `execute_receive` / `Swap` / `ProvideLiquidity` / `WithdrawLiquidity` / limit place/fill/cancel/claim | Tax kind per primitive; escrow accounting |
| Router `execute_swap_operations` + reply hops | Official Swap default; protocol-exempt bypass |
| Factory whitelist + F6 pin | 11611 listed; launcher/AutoLP correctly **not** listed |
| dApp `SwapPage` / `quoteCw20ViaRouteSolve` / `swapOpsRequireRouter` | Single-hop often still quotes via indexer; ≥2 hops **always** router |
| dApp `buildEnableFeatureInvoice` / `PayWithAnyToken` | Payee is launcher; atomic Send+hook |
| dApp `useCommunityTaxSellBps` | Sell extra-debit only; **no buy-tax quote adjust** |
| Indexer ingest + LCD probe + `/community-tokens` | Attestation, `include_unattested`, parameterized SQL |
| QA `localterra-community-tax-smoke.sh` | Direct-to-token SKU path hides C-1 |
| Open #603 / #604 / #605 | Migrate-adopt, identity, SKU init — product already knows some gaps |

Web-research additions that were then checked against this code:

- PancakeSwap FoT + `sync()` reserve desync — **does not apply** (inbound 1:1, pair wasm unchanged, no token→pair `sync`).
- CosmWasm IBC-hooks reentrancy / Astroport 2024 — **does not apply** to these three crates (no IBC entry; AutoLP reentrancy lock is local).
- Honeypot catalog (dynamic sell tax, trading toggle, max-wallet-as-sell-lock, cooldown-as-trap) — **does apply**. Launch guards + manager rate edits are the same levers, documented as SKUs.

---

## 2. Findings

### C-1 — High (product): launcher `EnableFeature` is broken; dApp Enable Feature cannot succeed

**Where:** `community-tax-token/src/invoice.rs` `execute_receive` requires `payer == config.manager`. `community-token-launcher/src/contract.rs` `enable_feature` forwards UST1 with `Cw20ExecuteMsg::Send` from the **launcher**, so the token sees `cw20.sender = launcher`.

```25:28:smartcontracts/contracts/community-tax-token/src/invoice.rs
    let payer = deps.api.addr_validate(&cw20.sender)?;
    if payer != config.manager {
        return Err(ContractError::Unauthorized {});
    }
```

```204:210:smartcontracts/contracts/community-token-launcher/src/contract.rs
    let send = WasmMsg::Execute {
        contract_addr: cfg.ust1.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Send {
            contract: token.to_string(),
            amount: paid,
            msg: to_json_binary(&InvoiceHookMsg::EnableFeature { sku: sku.clone() })?,
```

**dApp:** `buildEnableFeatureInvoice` sets `payee: launcher`. Settings batches correctly target the **token** and work.

**Impact:** Every post-create SKU unlock from the official UI reverts. UST1 is not taken (atomic revert) — **not a Critical fund-loss**. Users cannot buy TransferTax / SplitRouter / AutoV2Lp / ExemptionDirectory / VariableRates / LaunchGuards after a free create. Direct-to-token `Send` still works (CLI / QA smoke) — so the contract is usable, the **product path is not**.

**PoC:** `poc_launcher_enable_feature_always_unauthorized` — **inverted** in [#606](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/606) (official launcher path unlocks; CMM receives 50 UST1). Live **11611 / 11614** still need a CMM / DEX 2-of-3 migrate.

**Live wasm:** 11611 + 11614. A **dApp-only** workaround (point `buildEnableFeatureInvoice` at the **token**, keep launcher for create) needs no migrate. A launcher-forward fix needs a token and/or launcher migrate.

**Fix (pick one):**
1. Token: treat `payer == launcher && origin.launcher == payer` as manager-authorized, and keep the inner hook manager-only for direct pays; or
2. dApp: pay Enable Feature to the **token** (manager `Send`) — no migrate; or
3. Launcher: do not re-Send. Have the token expose a launcher-only `EnableFeature { sku, manager }` after the launcher already collected the invoice (splits the 50 UST1 path).

Do **not** loosen `payer == manager` for arbitrary addresses. Bundle **L-1** (QA smoke must use the official path) and **H-2** (SKU dedupe) in the same launcher-invoice issue.

---

### C-2 — Design: protocol-exempt router skips buy/sell on router hops

**Where:** instantiate stamps `PROTOCOL_EXEMPT` on `config.router` when set. Mainnet launcher **does** set the official router (`terra1e7s0h9…rsrw`). Classification:

- Sell requires `Send` + `Cw20HookMsg::Swap` to a listed pair **and** `!is_protocol_exempt(from)`.
- Buy requires `from` is a listed pair **and** `!is_protocol_exempt(to)`.

A router hop is: user → router (honest inbound) → router `Send+Swap` to pair (sell, but `from=router` is exempt → **Honest**) → pair `Transfer` to router (buy, but `to=router` is exempt → **Honest**) → router `Transfer` to user (wallet↔wallet; transfer tax SKU only, default off).

**What is actually untaxed (do not overclaim):**
- Official **multi-hop** (`swapOpsRequireRouter` is `ops.length >= 2`) and hybrid ops that execute on the router.
- Permissionless **1-op** `execute_swap_operations` (any integrator / bot).
- Invoice wrap-routes that hop via the router.

**What still taxes:** official **single-hop pair** `Send+Swap` (and some Trade market pair-direct paths). Not “every official Swap.”

**PoC scope:** `poc_router_exemption_full_tax_bypass` uses an address named `"router"` that is `PROTOCOL_EXEMPT`. It does **not** run router wasm. Classification is demonstrated; official-route wording is inferred.

**If the router were not exempt:** the router itself would pay sell extra-debit and would typically fail `InsufficientForSellTax` (it only holds `amount`). That is why the exemption exists — and why it also deletes the tax.

**Impact:** Once a token is listed and registered, advertised buy/sell bps are not collected on router hops. Direct pair swaps still tax — two economic regimes. **O601-7** already said hybrid / router multi-hop are **not** #601 close gates. File a **design** issue with the three options; do not file “fix Critical” with no chosen invariant.

**Fix (design, not a one-liner) — pick one and write it into T592:**
1. **Accept router hops as untaxed.** Disclose on Swap / Trade and in T592. Pair-direct still taxes. Closest to H-01 (unchanged router wasm, inbound 1:1).
2. **Tax the original trader on `Send+Swap` even if `from` is protocol-exempt.** Needs a trusted `trader` (pair already has this for fee discount). Router wasm still unchanged; extra-debit stays on the user, not the router.
3. **Stop marking the router exempt and refuse router-routed sells of this template.** Users `Send` the tax token to the **pair** only. Breaks hybrid / #596 for this `code_id` unless the dApp forces pair-only execute.

Teaching the router to size extra-debit violates H-01 / “do not add pair/router FoT math” and is **rejected** unless H-01 is explicitly reopened.

There is no clean fix that keeps both “inbound 1:1 + unchanged router wasm” and “router hops pay tax.” This is the product’s central contradiction.

**Status (2026-08-23):** [#607](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/607) chose **option 1** (disclose). Written as **T592-13**. Contract follow-up **waived**. Playbook: [`skills/AGENTS_COMMUNITY_TAX_ROUTER.md`](../skills/AGENTS_COMMUNITY_TAX_ROUTER.md). `poc_router_exemption_full_tax_bypass` stays as a documented property.

---

### H-1 — Residual / #605: AutoV2Lp SKU is paid and never bound

Launcher `create_token` builds `AutolpInit` then `let _ = (code_id, autolp_init);`. Token instantiate always gets `autolp: None`. `reply` only emits `create_token_ready`. `apply_autolp_settings` errors `AutoLP contract not bound; enable AutoLp via launcher` if `cfg.autolp` is None — and there is **no** execute that sets it later.

This is **documented v1** (`AGENTS_COMMUNITY_TAX_CW20.md`: same-tx instantiate+bind is **out of v1**, “pay SKU, then bind”; `docs/contracts-terraclassic.md`; REGISTRY “bind later”). The **new** fact vs that plan: **bind later is impossible** — no bind API.

Mainnet launcher has `autolp_code_id=11613`, so the SKU is offered and charged.

**PoC:** `poc_autov2lp_paid_but_never_bound` — PASS.

**Do not open a second AutoLP ticket.** Expand **#605** with: (1) no later bind execute; (2) refuse the SKU at create **or** add a bind path before charging.

---

### H-2 — Medium: duplicate SKUs double-charge

`paid_skus = args.features.len()`. `Features::from_skus` is idempotent (second `TransferTax` is a no-op). Invoice is `50 UST1 * len`. A crafted hook `[transfer_tax, transfer_tax]` pays 100 UST1 for one flag.

The official UI uses unique checkboxes and does **not** send duplicates. Impact is overpay to CMM on a crafted hook, not theft of third-party funds — **Medium**, not High.

**PoC:** `poc_launcher_duplicate_sku_double_charge` — **inverted** in [#606](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/606) (duplicate SKUs reject; fee not kept).

**Fix:** unique-set the SKU list (or reject duplicates) before multiplying. Bundle with C-1 / L-1.

---

### H-3 — High: `cooldown_blocks` bricks the pair

`apply_launch_guards` on Buy/Sell calls `check_cooldown` for **both** `from` and `to`, then `record_trade_blocks` writes **both**. The listed pair is `to` on every sell and `from` on every buy. After the first trade, the pair’s `LAST_TRADE_BLOCK` blocks every other wallet until `cooldown_blocks` elapse.

Not a per-wallet anti-snipe. A global pair halt. `LAST_TRADE_BLOCK` is documented as “last taxed swap block **per wallet**” (`state.rs`); implementation writes **both** `from` and `to`. Stronger than 11611 REPORT **D11** (same-sender in-block batch residual).

**PoC:** `poc_cooldown_bricks_pair` — originally PASS (bug). **Fixed in crate** [#608](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/608); PoC inverted (H608-1). LCD 11611 pin is pre-fix until token migrate.

Existing multitest `launch_guards_*` now covers per-wallet cooldown and provide after cap. `trading_enabled` exit lock remains **H-5**.

---

### H-4 — High: `max_wallet` bricks provide

Sell to a listed pair bypasses `max_wallet` (T592-11). Provide is `TransferFrom` → Honest, so `max_wallet` **does** apply to the pair’s new balance. After enough sells, `pair_balance > max_wallet` and every `ProvideLiquidity` reverts.

**PoC:** `poc_max_wallet_bricks_provide` — originally PASS (bug). **Fixed in crate** [#608](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/608); PoC inverted (H608-4).

---

### H-5 — Residual / do not file: `trading_enabled=false` locks exits (documented T592-11)

Buy **and** sell are blocked (T592-11, documented). Pair→EOA `Transfer` is classified **Buy**. So the same flag also reverts:

- LP `WithdrawLiquidity` (pair pays underlying)
- Limit cancel / claim / parked refund when the ask/offer is the tax token
- Any other pair payout of the tax token

Manager can collect buys, then flip the flag (50 UST1 settings batch; no timelock). Classic honeypot lever ([BlockMind / OpenLiquid / Token-Tax-Abuse-Science](https://docs.blockmind.app/blog/honeypot-crypto-token) “trading toggle after purchases”).

**PoC:** `poc_trading_disabled_locks_withdrawals` — PASS. Scope: pair `Transfer` to an EOA (Buy). Mock pair has no `WithdrawLiquidity` / book; withdraw/cancel/claim lock is inferred from T592-7 classify.

**Do not file as a new bug.** T592-11, 11611 **A9/A10/E8** already record pause-both-sides including parked claim. A later spec-change (“carve out withdraw/cancel/claim”) can reopen this; it is not a surprise defect.

---

### H-6 — Split: on-chain buy tax is T592-7; missing buy quote is #593

Documented in T592-7 / 11611 **B6**: pair→EOA Transfer is Buy — “same primitive.” Consequence:

- Limit **fill** paying the tax token to the maker is taxed (worse fill than `belief_price`).
- Limit **refund** of the tax token is taxed.

**Do not file the on-chain half as a new bug.**

dApp `useCommunityTaxSellBps` only adjusts **sell** extra-debit. Receiving the tax token (Swap You Receive, Trade buy) uses wallet `simulate_swap_operations` / pair `Simulation`, which do **not** model the outbound split. Users see a quote they will not receive. **No PoC** for this half — expand **#593** (C593-9 is sell-max only).

No oracle is involved; this is quote/execute divergence on the tax token itself.

---

### M-1 — Medium: `variable_rates` SKU does not gate rate changes

`require_variable_or_free_profile` always `Ok(())`. Caps are `max_*_bps` from instantiate. A free-profile token can raise sell to its cap via a 50 UST1 settings batch. Frontend `instantiateTaxCaps` only widens caps when the SKU is checked — so the SKU’s only effect is **optional extra room at create**, which the user could have typed as `max_*` anyway (and the contract will accept any `max_*` ≤ 2500 combined).

Retail copy: “Adjust buy/sell after launch (still capped).” That is true **without** paying 50 UST1 for the SKU.

**PoC:** `poc_variable_rates_sku_is_theater` — PASS.

**Approved direction:** **implement or remove** — implement the gate **only if** product still needs a paid “change rates later” SKU; otherwise drop the SKU from the dApp catalog and stop charging 50 UST1 for a no-op. Do **not** leave the theater. Expand **#605** (do not open a second VariableRates ticket).

---

### M-2 — Medium: AutoLP skim has no slippage (once bound)

`execute_skim` `Send`s half the balance to `cfg.pair` with `Cw20HookMsg::Swap { max_spread: None, min_return: None, hybrid: pool_only }`. Permissionless. Anyone can sandwich the skim. Tax that was supposed to deepen the book is extracted as arb.

T592-10 is respected (skim is not in Transfer/Send). The economic hole is the missing floor.

---

### M-3 — Medium: AutoLP pair is an unvalidated manager pointer

`UpdateConfig { pair }` only `addr_validate`s. No factory `Pair` lookup. Manager (or a compromised manager key) points `pair` at a contract that accepts the hook and keeps the tokens.

**PoC:** `poc_autolp_manager_can_skim_to_fake_pair` — PASS.

**Approved direction:** `pair` must (1) be a **factory-listed** CL8Y pair (`factory.Pair` lookup) and (2) have **the tax token as one of the two `asset_infos`**. Reject anything else. Same check on instantiate if `pair` is set. Bundle with **M-2** (`min_return` / skim floor). Latent until H-1 / #605 bind exists.

(Earlier hypothesis that `UpdateConfig` wipes omitted `Option` fields was **wrong** — fields merge. Retracted.)

---

### M-4 — Design / do not file as a vuln: manager is a standing honeypot operator

Even after C-1–H-5 are fixed, the manager can:

- Raise buy/sell to instantiate caps (up to 25% combined) in one 50 UST1 batch
- Flip `trading_enabled`
- Change `treasury` / sinks / Wallet sink
- Mint if MintControl was bought
- Add manager exemptions (transfer tax only)

Wasm admin is CMM (T592-5) — manager cannot migrate. That is the right split. It does **not** stop economic rugs. dApp does not show a persistent “manager can change tax / halt exits” banner on Swap/Trade for these tokens (only “Sell tax extra”).

---

### M-5 — Medium: ExemptionDirectory must skip transfer, buy, **and** sell

`is_manager_exempt` is consulted only in the Transfer branch (`is_transfer_exempt`). Buy and Sell ignore it. Retail hint: “Manager-chosen wallets skip tax.”

**Approved direction:** a manager-directory exempt address skips **transfer, buy, and sell** tax (not transfer-only). Protocol-exempt / listed-pair rules stay as they are. Do not let exemption disable launch-guard `trading_enabled` / cooldown / max_wallet unless a later spec says so. New issue (not a copy-only fix).

**Fix:** [#609](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/609) — `classify` → Honest for those three kinds; launch guards use `classify_trade`. Playbook [`skills/AGENTS_COMMUNITY_TAX_EXEMPT.md`](../skills/AGENTS_COMMUNITY_TAX_EXEMPT.md). Columbus-5 **11611** needs CMM migrate before live instances skip buy/sell.

---

### L-1 — Low: QA smoke hides C-1 (bundle with C-1)

`scripts/qa/localterra-community-tax-smoke.sh` SKU unlock now Sends UST1 to the **launcher** with `{enable_feature:{token,sku}}` ([#606](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/606) **T606-7**). `verify-issue-601` requires `sku_unlock_via_launcher`. Do not revert to a direct-to-token shortcut.

### L-2 — Low: decimals 0–5 and charset

On-chain `decimals <= 18` only. `cw20-base` symbol `[a-zA-Z\-]{3,12}` is the only name/symbol gate. Tracked as #604.

### L-3 — Low: Manage Token missing editors

No launch-guard editor, no sink editor, no AutoLP bind UI. Tracked as #605.

### I-1 — Info: no 11611 instances on columbus-5 at audit time

C-2 / H-3–H-6 have no live victims yet. C-1 / H-1 / H-2 / M-1 already affect anyone who uses Create Token today.

---

## 3. Confirmed-safe properties

| Property | Evidence |
|----------|----------|
| Inbound 1:1 to pair / factory / self / AutoLP | `classify`: `to_pair` without Swap hook → Honest; TransferFrom to pair Honest; multitest |
| Pair/router wasm not patched for FoT | Policy H-01 held; no `tax_map` / balance-delta |
| Exact invoice; wrong token/amount/no-op revert | `assert_exact_invoice`; fee not kept |
| Wasm admin = `cmm_governance` | Launcher `Instantiate { admin: Some(cmm) }` |
| MintControl instantiate-only; `RevokeMint` one-way | invoice + multitest |
| Protocol exemptions not removable | `CannotRemoveProtocolExempt` |
| `RegisterListedPair` checks pair + factory | `pair_registry.rs` |
| No reflection / rebase / pause / blacklist APIs | T592-8 held (launch guards are the pause stand-in — H-5) |
| AutoLP not called from Transfer/Send | T592-10; `SKIMMING` lock |
| Indexer does not trust event `code_id` | LCD `ContractInfo` + CMM admin + origin + launcher emitter |
| Catalog SQL parameterized | `community_tokens.rs` `$1…$4`; limit/offset clamped |
| Default list hides unattested | `include_unattested` opt-in |
| Pay invoice: last msg exact amount + hook; swap `to` ≠ payee | `assertPayInvoiceMsgs` |
| F6: 11611 listed; 11612/11613/11614/8654 not listed | deploy-trace + factory policy |

---

## 4. Test coverage

### 4.1 Executed this audit

| Suite | Result |
|-------|--------|
| `cl8y-community-tax-token` lib/multitest | 23 pass |
| `cl8y-community-token-launcher` unit | 3 pass |
| `cl8y-community-tax-autolp` unit | 2 pass |
| **New** `cl8y-community-token-launcher --test audit_poc` | **9 pass** (defect demonstrations) |
| Frontend `communityTaxInvoice` / `communityTaxSku` / `taxPreviewMaxSpend` / `ManageTokenPage` | 16 pass |

Indexer integration / Playwright / LocalTerra smoke **not** re-run in this pass (`verify-issue-592` / `601` / `593` exist; smoke misses C-1 per L-1).

### 4.2 Existing coverage (what is actually tested)

Token multitest: instantiate, inbound 1:1, sell extra-debit, insufficient sell, buy split, transfer tax on/off, invoice auth, settings fee/noop, unactivated SKU, enable_feature **direct-to-token**, mint, protocol exempt, spoof pair, preview, split sinks, combined cap. One thin launch-guard test (sell bypasses max_wallet).

Launcher: free create stamps CMM admin; paid create forwards fee; wrong invoice rejected. **No** `enable_feature`. **No** duplicate SKU. **No** AutoLP bind.

AutoLP: skim below threshold; `UpdateConfig` manager-only. **No** successful skim, **no** slippage, **no** fake pair.

### 4.3 Missing tests (should flip when fixed)

| Gap | Maps to |
|-----|---------|
| Launcher EnableFeature happy path | C-1 |
| Router-exempt vs EOA sell/buy | C-2 |
| Duplicate SKU rejected | H-2 |
| AutoLP instantiate + `cfg.autolp` set | H-1 |
| Cooldown does not use pair as subject | H-3 |
| Provide while pair > max_wallet | H-4 |
| Withdraw / cancel while `trading_enabled=false` | H-5 |
| Limit fill / refund buy-tax amounts | H-6 |
| VariableRates required to raise above instantiate rates **or** SKU removed | M-1 |
| Skim `min_return` / sandwich | M-2 |
| AutoLP pair must be factory-listed | M-3 |
| QA smoke via launcher EnableFeature | L-1 |

---

## 5. Indexer / database / Rust server

**Attestation** (`refresh_one`): `code_id == COMMUNITY_TAX_CODE_ID && admin == CMM && launcher_tx present && GetLauncherOrigin.launcher == configured launcher`. Event ingest of `create_token_ready` requires `emitter == launcher`. Enable/settings/mint events require the token already in catalog.

**API:** `/community-tokens` Postgres-only (no LCD amplification). Default `attested_cmm`. `include_unattested` is an explicit query flag. Limit 1–100, offset ≤ 10_000.

**SQL:** bound parameters; `lower(contract_address)` lookup. No string-concat queries. No PII beyond public chain addresses.

**Residuals (not new, still active from prior audits):** ingestion not transactional (PARSER-04), reorg completeness (REORG-01/02), no statement timeout (I389-25). Community-token tables inherit those.

**Not a leak:** catalog does not expose private keys, invoices beyond on-chain `invoice` attribute, or manager off-chain identity.

**Gap:** catalog does not surface `launch_guards.trading_enabled`, `max_wallet`, or “tax bypassable via router” — retail discovery of honeypot state is LCD-only.

---

## 6. Frontend

| Path | Status |
|------|--------|
| Free create → launcher `CreateToken` | Works |
| Paid create → UST1 Send + hook to launcher | Works (modulo H-2 if UI ever sends dup SKUs — it does not) |
| Enable Feature → launcher | **Broken (C-1)** |
| Settings batch → token | Works |
| Sell extra-debit max-spend | Works (`useCommunityTaxSellBps`) |
| Buy-side quote | **Missing (H-6)** |
| Manager / launch-guard / honeypot disclosure on Swap | Missing (M-4) |
| AutoLP skim button | Only if `cfg.autolp` bound — never for launcher tokens (H-1) |

`PayWithAnyToken` is otherwise careful (min-receive ≥ invoice, `to` ≠ payee, hook match). Enable Feature failure is an on-chain Unauthorized after the user already signed — poor UX plus a dead SKU store.

---

## 7. Tokenomic / economic / oracle

**Oracle:** community tax tokens are **not** in the hub-price / Venus / CoinGecko path. No new oracle-manipulation surface **from this product**. Prior ORACLE-01 / I389-05/06 still apply to USTC/LUNC/FDUSD display around any pair that later includes a tax token as a quote.

**Economic attacks that do apply:**

| Vector | Status |
|--------|--------|
| Tax evasion via router (C-2) | Live design once listed |
| Manager tax hike / trading halt (H-5, M-4) | Live |
| AutoLP sandwich (M-2) | Latent until bind exists |
| AutoLP drain to fake pair (M-3) | Latent until bind exists |
| Limit-maker unexpected buy tax (H-6) | Live if anyone places limits |
| Launch-guard pair DoS (H-3, H-4) | Live if SKU on |
| FoT reserve desync (Pancake `sync()` class) | **Mitigated** by T592-1 |
| Reflection / rebase | Not implemented |
| Flash-loan fee-tier (SUP-05) | Unrelated; still active on CL8Y discount |

**Honeypot mapping (from web research):**

| Classic pattern | This template |
|-----------------|---------------|
| Trading toggle | `trading_enabled` (H-5) |
| Cooldown trap | pair-wide cooldown (H-3) |
| Max wallet as LP/sell lock | provide brick (H-4); sell still exits |
| Dynamic / admin tax | settings batch to cap (M-1, M-4) |
| Hidden 99% sell tax | Cap 25% combined — not a 99% trap, still a hike |
| Blacklist | **Absent** (good) |
| Proxy upgrade by manager | **Absent** (CMM admin) |

---

## 8. Access control / privileges

| Actor | Can | Cannot |
|-------|-----|--------|
| Manager | Settings, SKU unlock (direct-to-token), mint if SKU, AutoLP UpdateConfig, exemptions | Migrate, change wasm admin, remove protocol exempt, take invoice UST1 (goes to CMM) |
| CMM / wasm admin | `MsgMigrateContract` on token / launcher / AutoLP | Manager executes |
| DEX 2-of-3 (launcher admin) | Migrate launcher; change `token_code_id` if a future migrate adds UpdateConfig | Instantiate tokens as manager |
| Anyone | `RegisterListedPair` (factory-checked), `SkimToLp`, swap | Invoice hooks |
| Router (if stamped) | Untaxed hops (C-2) | — |

#526 (EOA vs 2-of-3 accept queue) remains an **active** governance residual; a compromised 2-of-3 can migrate 11611/11614 to new wasm (Chain D in §10).

---

## 9. Attack checklists

### 9.1 Smart-contract

| Vector | Result |
|--------|--------|
| Reentrancy (token Transfer/Send) | Safe — no external call before balance write; AutoLP `SKIMMING` |
| Reentrancy (AutoLP reply) | Lock cleared in reply **before** provide messages — if provide re-entered skim, lock is false. Provide is to the pair, not AutoLP. Residual: a malicious pair hook could call `SkimToLp` again in the same tx after reply unlock. Manager-chosen pair (M-3) makes this real. |
| Integer overflow | `checked_*`; `overflow-checks=true` release |
| Unauthorized invoice | Manager check (too strict → C-1) |
| Spoof pair | Factory lookup — OK |
| cw2 migrate | same-crate only; #603 importer would be a new branch |
| Event spoof → catalog | Launcher emitter + LCD pin — OK |

### 9.2 DeFi

| Vector | Result |
|--------|--------|
| FoT reserve desync | Mitigated (inbound 1:1) |
| Router tax bypass | **C-2** |
| Sandwich AutoLP | **M-2** |
| Sandwich user swap | Unchanged DEX (slippage / min_return on user path) |
| Manager honeypot | **H-5 / M-4** |
| Limit book insolvency from buy tax | Pair debits full `amount`, user/maker credited `amount - tax` — pair accounting OK; **maker** is short |

### 9.3 Database / API

SQLi / injection: **clean**. Unattested opt-in: **OK**. Deep pagination cap: **OK**. Missing honeypot fields: §5.

### 9.4 Frontend / E2E

C-1 not covered by unit tests (they only assert invoice **shape**). No E2E for Enable Feature. Sell hint is the string `Sell tax extra` — not a rate, not a buy warning.

---

## 10. Exploit-chaining investigation

Method: High+ findings above + **still-active** nodes from `INTERNAL_KIMIK3_1787230030` §8.5 / §9 + open #526 / #603 + out-of-repo Terra Classic / router / manager-key / 2-of-3. Ordered by severity.

### Chain J — Router tax evasion at scale (in-repo; **new**)

**Nodes:** C-2 (router exempt) + official hybrid/router execute (#596) + bots / `execute_swap_operations` + empty 11611 catalog (no victims **yet**).

1. First community token launches with 5% buy/sell. Marketing assumes sinks fill.
2. All multi-hop and any 1-op router Send pay 0 tax. Direct pair button still taxes, so unsophisticated sellers fund the treasury; flow / arb / invoice wraps do not.
3. Manager sees empty sinks, raises tax (M-1) or halts trading (H-5) — punishing the only path that was paying.

**Severity: Critical once a token has liquidity.** **Mitigation:** C-2 fix before first create; or dApp-force pair-only execute for this `code_id` (does not stop off-dApp router use).

### Chain K — Enable Feature dead + AutoLP paid vapor (in-repo; **new**)

**Nodes:** C-1 + H-1 + dApp `buildEnableFeatureInvoice` + #605.

Retail pays 50 UST1 at create for AutoV2Lp (if they check it) and cannot unlock other SKUs from the UI. Direct-to-token CLI still works — operators who read the smoke script can unlock; retail cannot. Looks like a rug even when it is a bug.

**Severity: High (integrity / support), not theft** (reverts keep UST1).

### Chain L — Launch-guard honeypot (in-repo + well-known out-of-repo pattern)

**Nodes:** H-3 + H-4 + H-5 + M-4 + web honeypot catalog + missing Swap banner.

1. Manager creates with LaunchGuards, seeds LP, `trading_enabled=true`, low tax.
2. Retail buys (possibly via router: 0 buy tax — Chain J — even worse for the buyer later).
3. Manager sets `trading_enabled=false` (or cooldown / max_wallet). LP and limits cannot exit (H-5). Sells revert.
4. Optional: manager is exempt from transfer tax only (M-5) and moves inventory wallet-to-wallet; swaps still blocked for others.

CMM cannot stop this without a token migrate (and #603 is design-only / no-go until written). **Severity: High** (intended SKU, abused). **Mitigation:** H-5 exit carve-out; dApp warning; optional CMM-enforced `trading_enabled` delay.

### Chain M — Manager key OR 2-of-3 → tax / migrate (in-repo + out-of-repo)

**Nodes:** M-4 + #526 + prior Chain D (SUP-01 no timelock) + #603 temptation.

- **Manager key theft:** tax hike, halt, treasury repoint, mint (if SKU), AutoLP fake pair (M-3). Cannot migrate.
- **2-of-3 theft:** migrate 11611 to arbitrary wasm (F6 then freezes pairs until refresh — prior L-01). If #603 ships a foreign importer without a tight allowlist, this becomes “adopt + steal balances” rather than freeze.

**Severity: High, conditioned on key compromise.** Same as prior Chain D, with a new retail blast radius (every community token).

### Chain N — F6 freeze × tax-token exits (prior L-01 + H-5)

**Nodes:** 1787230030 L-01 (code-id drift freezes withdraw/cancel/claim) + H-5 (trading flag freezes the same exits).

Two independent freeze layers on the same funds. A 11611 token whose manager halts trading **and** whose wasm is later migrated off-pin is frozen until **both** the manager re-enables trading **and** governance refreshes. Users cannot tell which layer reverted (raw errors).

**Severity: Medium** (compounded liveness).

### Chain O — Unfreeze arb × tax-token TWAP (prior Chain A)

If a 11611 pair freezes (F6 or H-5) while CEX-unrelated reserves sit, then unfreezes, first swap extracts stale curve value. Buy tax on the exit (if any) goes to the manager treasury — attacker still profits on the other leg.

**Severity: Medium**, same as prior Chain A, new fee sink.

### Chain P — #603 in-place migrate from 8654 (out-of-repo ALPHA + in-repo)

8654 is a known-bad `tax_map` / inbound FoT control. An allowlisted importer that copies `BALANCES` but leaves a code path reading old keys, or that skips CMM admin, re-opens AMM-01 (reserve desync) **and** Chain H (whitelisted-FoT). Issue text already forbids this; the chain is “someone ships #603 carelessly.”

**Severity: High if shipped wrong; currently design-only.**

### Chain Q — Catalog attestation bypass (dead-end, verified)

Forging `create_token_ready` from a random contract does not catalog: emitter must be the configured launcher. LCD probe will not attest without CMM admin + origin. `include_unattested` is explicit. **Does not chain** to a fake “official” token in the default list.

### Chain R — Prior supply-chain / oracle / agent (still active, orthogonal)

Chains B, C, I, G from 1787230030 are **unchanged**. They do not need the tax token. They can still deliver a malicious Create Token build (wrong launcher / payee) if `VITE_COMMUNITY_TOKEN_LAUNCHER` is swapped in a poisoned frontend — users would instantiate against an attacker launcher that stamps a different admin. **Check:** Coolify env must pin `terra126pr5…ahzwze` (#602). Residual: FE-01 / INF-28.

### What does **not** chain

- **C-1 → theft:** revert; UST1 stays in the manager wallet.
- **C-2 → pair insolvency:** inbound still 1:1; pair reserves match balances.
- **H-2 → extra SKU unlock:** duplicates do not enable a second feature, they only overpay CMM.
- **Indexer → tax evasion:** catalog cannot change classification.
- **Pancake `sync()` class:** no token callback into pair reserves.

---

## 11. Recommended fix order

1. **C-1 + H-2 + L-1** — Launcher invoices: EnableFeature path (or dApp→token), unique SKUs, QA smoke via the official path.
2. **C-2** — Design issue: write the three options and pick one before the first taxed token has liquidity. Do not implement until chosen.
3. **H-3 + H-4** — Cooldown per **wallet** only (do not record the pair); `max_wallet` skip listed pairs on Honest provide, not only Sell.
4. **M-5** — ExemptionDirectory skips transfer **and** buy/sell.
5. **M-2 + M-3** — AutoLP: factory-listed pair with the tax token as one side + skim `min_return` (gated on #605 bind).
6. **#605 notes** — H-1 (no later bind API) and M-1 (implement VariableRates only if needed, else remove).
7. **#593 note** — H-6 buy-side quote (on-chain maker tax stays T592-7).
8. **Do not file** H-5, M-4, L-2, L-3, I-1, Chains J–R, prior-audit residuals.
9. Do not ship #603 importer until C-2 policy is settled.

---

## 12. PoC inventory

File: `smartcontracts/contracts/community-token-launcher/tests/audit_poc.rs`

| Test | Finding |
|------|---------|
| `poc_launcher_enable_feature_always_unauthorized` | C-1 |
| `poc_launcher_duplicate_sku_double_charge` | H-2 |
| `poc_autov2lp_paid_but_never_bound` | H-1 |
| `poc_router_exemption_full_tax_bypass` | C-2 |
| `poc_cooldown_bricks_pair` | H-3 |
| `poc_max_wallet_bricks_provide` | H-4 |
| `poc_trading_disabled_locks_withdrawals` | H-5 |
| `poc_variable_rates_sku_is_theater` | M-1 |
| `poc_autolp_manager_can_skim_to_fake_pair` | M-3 |

```text
cargo test -p cl8y-community-token-launcher --test audit_poc
# 9 passed; 0 failed  (2026-08-23)
```

A fix should **fail** the corresponding PoC (or invert the assertion). Do not delete the file until the finding is closed. These tests **encode the defect** — green means “bug still present,” not “feature works.” Invert them in the fix MR; do not treat them as a long-lived CI happy-path suite.

PoC comment IDs were corrected (PoC 3 = H-1, PoC 5 = H-3, PoC 6 = H-4, PoC 7 = H-5). C-2 / H-5 comments now state harness limits (no router wasm; no pair book).

---

## 13. References

- Skills: `AGENTS_COMMUNITY_TAX_CW20.md` (T592, O601), `AGENTS_FRONTEND_CREATE_TOKEN.md` (C593), `AGENTS_INDEXER_COMMUNITY_TOKENS.md` (I594)
- Prior: `audits/INTERNAL_KIMIK3_1787230030.md` §9 Chains A–I
- Issues: #592 #593 #594 #601 #602 #603 #604 #605 #526
- External: PancakeSwap FoT+sync post-mortem; Trail of Bits CosmWasm patterns; BlockMind / OpenLiquid honeypot mechanics; Token-Tax-Abuse-Science
- Live LCD: launcher `GetConfig` 2026-08-23 — router set, `autolp_code_id=11613`; zero 11611 instances

---

## 14. Issue disposition (approved 2026-08-23)

Lead-review punch list accepted. Additional product decisions: C-2 is a design issue (three options + tradeoffs); M-5 exemptions apply to transfer **and** buy/sell; M-3 pair must be factory-listed **and** have the tax token as one side; M-1 implement VariableRates only if needed, else remove (note on #605).

### 14.1 Live wasm / migrate

| Code | Columbus-5 | Who migrates | Notes |
|------|------------|--------------|--------|
| Token **11611** | Factory-listed | CMM wasm admin | Any `tax.rs` / `invoice.rs` change (H-3, H-4, M-5, C-1 option 1) needs a new store + migrate. |
| Launcher **11614** | Live Create Token | DEX 2-of-3 | H-2 dedupe; C-1 option 3. |
| AutoLP **11613** | Stored, not listed | CMM | M-2 / M-3. No instances required until #605 bind. |
| dApp only | Coolify | — | C-1 option 2 (Enable Feature payee = token) needs **no** migrate. L-1 smoke is scripts. |

Do not describe contract edits as if they go live from `main` without a migrate playbook.

### 14.2 File / do-not-file

| Bundle | Findings | Action |
|--------|----------|--------|
| Launcher invoices | C-1, H-2, L-1 | **New issue** |
| Router tax invariant | C-2 | **New design issue** (options, no implementation until chosen) |
| Launch-guard liveness | H-3, H-4 | **New issue** (11611 migrate) |
| ExemptionDirectory | M-5 | **New issue** (11611 migrate) |
| AutoLP hardening | M-2, M-3 | **New issue** (11613; gated on #605 bind) |
| SKU init / AutoLP bind / VariableRates | H-1, M-1, L-3 | **Notes on #605** |
| Buy-side quote | H-6 frontend | **Note on #593** |
| Pause exits / manager privilege / identity | H-5, H-6 on-chain, M-4, L-2, I-1 | **Do not file** |
| Chains J–R, PARSER-04, REORG, #526, #603 | analysis | **Do not file** |

### 14.3 PoCs on `main`

`audit_poc.rs` lands as defect-demonstration tests. Invert assertions in the fix MR for the matching finding. Do not delete until that finding is closed.
