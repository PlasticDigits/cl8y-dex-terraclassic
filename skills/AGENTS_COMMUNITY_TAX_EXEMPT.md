# Agent playbook: ExemptionDirectory full tax skip (GitLab #609)

Use when changing manager-directory exemptions on the community tax CW20, Swap/Trade extra-debit Max for that template, or Create/Manage Token exemption copy.

Parent template [#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592) ([`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md)). Audit **M-5** in [`INTERNAL_KIMIK3_1787468843`](../audits/INTERNAL_KIMIK3_1787468843.md). Create Token SKU hint [#593](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593) ([`AGENTS_FRONTEND_CREATE_TOKEN.md`](./AGENTS_FRONTEND_CREATE_TOKEN.md)).

Product decision (2026-08-23): exempted users skip **transfer, buy, and sell** tax — not transfer-only.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#609**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/609) | Tax skip + guards stay on |
| [`tax.rs`](../smartcontracts/contracts/community-tax-token/src/tax.rs) `classify_trade` / `classify` | Economic kind vs Honest skip |
| [`docs/contracts-terraclassic.md` § Classification](../docs/contracts-terraclassic.md#classification-t592-7) | T592-7 table |
| [`communityTaxSku.ts`](../frontend-dapp/src/utils/communityTaxSku.ts) | Retail hint |
| [`taxPreviewMaxSpend.ts`](../frontend-dapp/src/utils/taxPreviewMaxSpend.ts) `effectiveExtraDebitSellBps` | Swap/Trade Max |

## Invariants **E609-1–E609-7**

1. **E609-1 — full tax skip.** If `from`, `to`, or an authenticated official-router hop `trader` is `MANAGER_EXEMPT` and the economic kind is Sell / Buy / Transfer → **Honest** (0 bps). `TaxPreview` matches execute. Pair still credited / debited `amount` (no inbound FoT).
2. **E609-2 — launch guards stay on.** Exemption does **not** skip `trading_enabled`, cooldown, or `max_wallet`. Guards use `classify_trade` (economic kind), not the Honest tax kind. Sell to a listed pair still bypasses `max_wallet` (**T592-11**).
3. **E609-3 — protocol list unchanged.** Listed pair / router / factory / AutoLP / self stay protocol-exempt. Manager cannot `remove_exempt` those (`CannotRemoveProtocolExempt`). `add_exempt` does not make an address a listed pair or allow spoof `RegisterListedPair` (**T592-9**).
4. **E609-4 — inbound 1:1.** Exempt sell still credits the pair exactly `amount`. Protocol inbound to pair/router/escrow/AutoLP stays 1:1 (**T592-1**).
5. **E609-5 — paid directory.** SKU `exemption_directory` required to mutate the list. Invoice still **50 UST1** for the settings batch (**T592-4**). No permissionless self-add.
6. **E609-6 — retail copy.** Hint / Manage helper: skip **buy, sell, and transfer** tax. Do not say transfer-only.
7. **E609-7 — extra-debit Max.** Swap/Trade Max uses `effectiveExtraDebitSellBps`: manager-exempt connected wallet → 0 extra-debit. Unknown exempt keeps config `sell_bps` (fail closed). Prefer `TaxPreview` / `IsProtocolExempt.manager` over assuming 100% balance.

## Verify

```bash
make verify-issue-609
cd smartcontracts && cargo test -p cl8y-community-tax-token
```

Columbus-5 **11611** instances still run pre-#609 wasm until CMM migrate. Do not treat listed 11611 as already skipping buy/sell until migrate.

## Do not

- Skip launch guards for directory wallets.
- Let `remove_exempt` strip protocol entries.
- Credit the pair `amount * (1 - sell_bps)` (inbound FoT / **H-01**).
- Treat indexer catalog events as on-chain exempt without the token already in catalog ([#594](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/594)).
