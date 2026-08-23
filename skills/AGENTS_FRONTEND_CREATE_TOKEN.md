# Agent playbook: Create Token + manager console (GitLab #593)

Use when changing `/token/create`, `/token/:addr/manage`, `/tokens`, community-tax Swap max, or launcher invoice checkout.

Sibling: on-chain template [#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592); identity + wallet helpers [#604](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/604); SKU init + percent taxes [#605](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/605); indexer catalog [#594](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/594); pay-with-any-token [#595](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/595); post-merge Coolify / LocalTerra [#602](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/602) ([`AGENTS_POST_MERGE_OPS_602.md`](./AGENTS_POST_MERGE_OPS_602.md)); router hops Honest [#607](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/607) ([`AGENTS_COMMUNITY_TAX_ROUTER.md`](./AGENTS_COMMUNITY_TAX_ROUTER.md)); ExemptionDirectory full tax skip [#609](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/609) ([`AGENTS_COMMUNITY_TAX_EXEMPT.md`](./AGENTS_COMMUNITY_TAX_EXEMPT.md)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#593**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593) | dApp create/manage |
| [`docs/frontend.md` § Create Token](../docs/frontend.md#create-token-community-tax) | Invariants **C593-1–C593-14**, **C604-1–C604-3**, **C605-1–C605-4** |
| [`communityTaxIdentity.ts`](../frontend-dapp/src/utils/communityTaxIdentity.ts) | Name/symbol/decimals + connected-wallet helpers (#604) |
| [`communityTaxCreateForm.ts`](../frontend-dapp/src/utils/communityTaxCreateForm.ts) | Shared form → hook args (#604 / #605) |
| [`CreateTokenPage.tsx`](../frontend-dapp/src/pages/CreateTokenPage.tsx) | Wizard |
| [`ManageTokenPage.tsx`](../frontend-dapp/src/pages/ManageTokenPage.tsx) | Manager console |
| [`communityTaxInvoice.ts`](../frontend-dapp/src/utils/communityTaxInvoice.ts) | Hooks + invoices |
| [`AGENTS_FRONTEND_PAY_INVOICE.md`](./AGENTS_FRONTEND_PAY_INVOICE.md) | Shared Pay card |
| [`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md) | On-chain SKUs |
| [`AGENTS_INDEXER_COMMUNITY_TOKENS.md`](./AGENTS_INDEXER_COMMUNITY_TOKENS.md) | Catalog API |

## Invariants **C593-1–C593-14**

1. **C593-1 — env gate.** Pages + More-menu **Create Token** live only when `VITE_COMMUNITY_TAX_CODE_ID` and `VITE_COMMUNITY_TOKEN_LAUNCHER` are set. Unset → unavailable (Mint/Wrap pattern).
2. **C593-2 — names.** Nav label is **Create Token**. Never Mint (faucet) or Create Pair.
3. **C593-3 — pay card.** Paid SKUs and settings Save **import** `PayWithAnyToken`. Do not assemble router ops on these pages.
4. **C593-4 — invoices.** Create = `50 UST1 × SKU count` to the **env launcher**. Enable Feature = 50 UST1 to the launcher. Save = **50 UST1 flat** to the **token**. Mint is not invoiced.
5. **C593-5 — MintControl.** Create-only. Manage unlock list must omit Minting.
6. **C593-6 — manager gate.** Submit compares connected wallet to LCD `GetConfig.manager`, not a URL param. Non-manager is read-only; config stays visible.
7. **C593-7 — unverified admin.** LCD `ContractInfo.admin ≠ CMM` → **Unverified admin** banner.
8. **C593-8 — template.** Manage requires LCD `code_id == VITE_COMMUNITY_TAX_CODE_ID`. 10184/8266/6036 must not show tax SKUs.
9. **C593-9 — extra-debit max.** Swap + Trade Market Max for this template uses sell extra-debit (`extraDebitSellBps` / TaxPreview) **on pair-direct execute only**. Router hops Send `amount` 1:1 — do not shrink Max as if extra-debit will fire (**T592-13** / **R607-7**). Manager-directory wallets skip extra-debit (**E609-7**); Extra exemptions hint must say buy, sell, and transfer.
10. **C593-10 — payee from env.** Invoice payee is launcher or token from config/query. Never `?payee=`.
11. **C593-11 — no Swap dump.** Created tokens are not auto-injected into Swap defaults (#562).
12. **C593-12 — free create.** 0 SKUs uses launcher `CreateToken` execute (UST1 `Send` of 0 is invalid). Paid SKUs stay on Receive + #595.
13. **C593-13 — instantiate caps.** Combined `max_buy+max_sell+max_transfer ≤ 2500`. Never default each max to 2500.
14. **C593-14 — pair-direct tax copy (#607).** Create/Manage: `Buy/sell tax is pair-direct only.` Swap/Trade: pair-direct sell → `Sell tax extra`; router execute → `Route skips buy/sell tax`. Do not claim advertised bps apply to every official Swap. Playbook: [`AGENTS_COMMUNITY_TAX_ROUTER.md`](./AGENTS_COMMUNITY_TAX_ROUTER.md).

## Invariants **C604-1–C604-3** ([#604](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/604))

1. **C604-1 — identity.** Name/symbol `^[A-Za-z0-9]+$` only (no spaces, hyphen, unicode). Name 3–50 (case preserved). Symbol 3–12, uppercased on submit. Decimals integer **6–18**. UI and `community-tax-token::instantiate` both reject; hook builders use the same parsers.
2. **C604-2 — connected wallet.** Empty treasury/manager autofill the connected address. Helper copy is exactly `connected wallet` / `not connected wallet` (bech32-normalized). Connecting does not overwrite a typed different address. Disconnect → helper `not connected wallet`; create CTA stays gated (**C593**). Ignore `?manager=` / `?treasury=` / `?payee=`.
3. **C604-3 — 11611 gap.** Columbus-5 listed wasm **11611** does not enforce identity until launcher `token_code_id` rotates after #589 REPORT **GO** + factory `AddWhitelistedCodeId`. Keep 11611 listed (**F6**). Do not whitelist launcher / AutoLP (**O601-2**). Frontend may ship first as a client gate.

## Invariants **C605-1–C605-4** ([#605](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/605))

1. **C605-1 — percent.** Retail tax fields are percent with exactly 2 decimal places (`2.50` → **250** bps). Empty → 0. Reject `2.501`, `1e2`, trailing junk. Combined cap 25.00%. Manage placeholders show `1.00` for `buy_bps: 100`, never raw bps. No hidden bps mode.
2. **C605-2 — SKU init.** Checkbox reveals init fields; uncheck hides and drops those keys from the hook. Wallet-to-wallet → `transfer_bps`. Split treasury → 1–4 sinks summing 100.00%. Extra exemptions → `initial_exempt` (≤20, no protocol addrs). Change rates later → explicit `max_*` ≥ current, combined ≤ 25.00%, immutable copy. Launch guards → explicit `trading_enabled` (UI default **off**). Mint cap is human-scale. Free create still cannot include paid payloads (**C593-12**).
3. **C605-3 — AutoLP.** Create with Auto liquidity instantiates+binds the sister when launcher `autolp_code_id` is set. Unset → block the SKU (do not take 50 UST1). Manage bind is settings batch + sister `UpdateConfig` after bind. `SkimToLp` is never called from token transfer (**T592-10**). No pair/router FoT math. Audit **H-1**: do not charge for a discarded sister.
4. **C605-4 — VariableRates gate (audit M-1).** Keep the paid SKU. Without it, instantiate `max_*` equals current rates (no CLI headroom). Settings `buy_bps` / `sell_bps` require the SKU. Do not leave `require_variable_or_free_profile` as `Ok(())`. Manage buy/sell stay locked until unlock.

## Verify

```bash
make verify-issue-593
make verify-issue-604
make verify-issue-605
make verify-issue-607
make verify-issue-602
```

Columbus-5 free-create is live on launcher **11614** (`terra126pr5…ahzwze`). Do not point Coolify at **11612** (`terra1af9xm…`). Post-merge Coolify + LocalTerra retail: [#602](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/602).

## Do not

- Fork Swap quote/execute onto Create/Manage.
- Mix Enable Feature into a settings Save.
- Treat indexer listing as F6 / whitelist.
- Hardcode a LocalTerra store id as `VITE_COMMUNITY_TAX_CODE_ID`.
