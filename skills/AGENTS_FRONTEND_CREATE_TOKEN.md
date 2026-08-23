# Agent playbook: Create Token + manager console (GitLab #593)

Use when changing `/token/create`, `/token/:addr/manage`, `/tokens`, community-tax Swap max, or launcher invoice checkout.

Sibling: on-chain template [#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592); indexer catalog [#594](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/594); pay-with-any-token [#595](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/595); post-merge Coolify / LocalTerra [#602](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/602) ([`AGENTS_POST_MERGE_OPS_602.md`](./AGENTS_POST_MERGE_OPS_602.md)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#593**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593) | dApp create/manage |
| [`docs/frontend.md` § Create Token](../docs/frontend.md#create-token-community-tax) | Invariants **C593-1–C593-12** |
| [`CreateTokenPage.tsx`](../frontend-dapp/src/pages/CreateTokenPage.tsx) | Wizard |
| [`ManageTokenPage.tsx`](../frontend-dapp/src/pages/ManageTokenPage.tsx) | Manager console |
| [`communityTaxInvoice.ts`](../frontend-dapp/src/utils/communityTaxInvoice.ts) | Hooks + invoices |
| [`AGENTS_FRONTEND_PAY_INVOICE.md`](./AGENTS_FRONTEND_PAY_INVOICE.md) | Shared Pay card |
| [`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md) | On-chain SKUs |
| [`AGENTS_INDEXER_COMMUNITY_TOKENS.md`](./AGENTS_INDEXER_COMMUNITY_TOKENS.md) | Catalog API |

## Invariants **C593-1–C593-13**

1. **C593-1 — env gate.** Pages + More-menu **Create Token** live only when `VITE_COMMUNITY_TAX_CODE_ID` and `VITE_COMMUNITY_TOKEN_LAUNCHER` are set. Unset → unavailable (Mint/Wrap pattern).
2. **C593-2 — names.** Nav label is **Create Token**. Never Mint (faucet) or Create Pair.
3. **C593-3 — pay card.** Paid SKUs and settings Save **import** `PayWithAnyToken`. Do not assemble router ops on these pages.
4. **C593-4 — invoices.** Create = `50 UST1 × SKU count` to the **env launcher**. Enable Feature = 50 UST1 to the launcher. Save = **50 UST1 flat** to the **token**. Mint is not invoiced.
5. **C593-5 — MintControl.** Create-only. Manage unlock list must omit Minting.
6. **C593-6 — manager gate.** Submit compares connected wallet to LCD `GetConfig.manager`, not a URL param. Non-manager is read-only; config stays visible.
7. **C593-7 — unverified admin.** LCD `ContractInfo.admin ≠ CMM` → **Unverified admin** banner.
8. **C593-8 — template.** Manage requires LCD `code_id == VITE_COMMUNITY_TAX_CODE_ID`. 10184/8266/6036 must not show tax SKUs.
9. **C593-9 — extra-debit max.** Swap + Trade Market Max for this template uses sell extra-debit (`extraDebitSellBps` / TaxPreview). Do not offer 100% balance on a taxed sell.
10. **C593-10 — payee from env.** Invoice payee is launcher or token from config/query. Never `?payee=`.
11. **C593-11 — no Swap dump.** Created tokens are not auto-injected into Swap defaults (#562).
12. **C593-12 — free create.** 0 SKUs uses launcher `CreateToken` execute (UST1 `Send` of 0 is invalid). Paid SKUs stay on Receive + #595.
13. **C593-13 — instantiate caps.** Combined `max_buy+max_sell+max_transfer ≤ 2500`. Never default each max to 2500.

## Verify

```bash
make verify-issue-593
make verify-issue-602
```

Columbus-5 free-create is live on launcher **11614** (`terra126pr5…ahzwze`). Do not point Coolify at **11612** (`terra1af9xm…`). Post-merge Coolify + LocalTerra retail: [#602](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/602).

## Do not

- Fork Swap quote/execute onto Create/Manage.
- Mix Enable Feature into a settings Save.
- Treat indexer listing as F6 / whitelist.
- Hardcode a LocalTerra store id as `VITE_COMMUNITY_TAX_CODE_ID`.
