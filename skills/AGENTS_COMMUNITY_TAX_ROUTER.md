# Agent playbook: community tax router hops are Honest (GitLab #607)

Use when changing **community tax** classification, Swap/Trade copy for `code_id` **11611**, extra-debit Max, or anything that could tax (or pretend to tax) a **router** hop.

This is the **C-2 design close**. Chosen option: **1 — accept router hops as untaxed and disclose**. Do **not** implement option 2 (tax original trader) or option 3 (un-exempt the router) unless a new issue reopens H-01 or drops hybrid for this template.

Issue **#607 is a disclose-only close**. Contract classify / pair / router wasm stay as shipped. Follow-up implement ticket: **waived**.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#607**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/607) | Design close (option 1) |
| Audit C-2 | [`audits/INTERNAL_KIMIK3_1787468843.md`](../audits/INTERNAL_KIMIK3_1787468843.md) |
| [`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md) | **T592-13** + inbound 1:1 (**T592-1**) |
| [`AGENTS_FRONTEND_CREATE_TOKEN.md`](./AGENTS_FRONTEND_CREATE_TOKEN.md) | **C593-14** sell/buy copy |
| [`AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md`](./AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md) | Official Swap keeps hybrid (**#596**) |
| [`AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md) | Quote = execute; router when `ops.length >= 2` |
| [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) | Short hints; no architecture essays |
| [`tax.rs` `classify`](../smartcontracts/contracts/community-tax-token/src/tax.rs) | Sell/Buy skip when `from`/`to` is protocol-exempt |
| [`swapRouting.ts`](../frontend-dapp/src/services/terraclassic/swapRouting.ts) | `swapOpsRequireRouter` ⇔ `ops.length >= 2` |
| [`taxPreviewMaxSpend.ts`](../frontend-dapp/src/utils/taxPreviewMaxSpend.ts) | Pair-direct extra-debit Max + route hints |

## Invariants **R607-1–R607-8**

1. **R607-1 — option 1 only.** Buy/sell bps apply to **pair-direct** `Send`+`Cw20HookMsg::Swap` and pair→non-exempt EOA. Router hops are **Honest**. Write this as **T592-13**; do not “fix” classify to tax the router.
2. **R607-2 — H-01 held.** Do not add pair/router FoT balance-delta math. Do not teach the router to size extra-debit. Inbound to pair / router / escrow / AutoLP stays 1:1 (**T592-1**).
3. **R607-3 — still taxed.** Official **single-hop** pair `Send+Swap` (and Trade pair-direct) still extra-debits sell / splits buy. Extra-debit Max (**C593-9**) applies on that execute path only.
4. **R607-4 — still Honest.** Official multi-hop (`swapOpsRequireRouter`), hybrid ops that execute on the router, permissionless 1-op `execute_swap_operations`, and invoice wrap-routes that hop via the router collect **no** buy/sell tax. TransferTax SKU (default off) is the only wallet↔wallet tax after router→user.
5. **R607-5 — hybrid stays on.** Do **not** force pair-only execute for `code_id` 11611. That is option 3 and it does not stop off-dApp router use. **#596** is unchanged.
6. **R607-6 — disclose, don’t lecture.** Swap/Trade show a short hint: pair-direct sell → `Sell tax extra`; router execute → `Route skips buy/sell tax`. Create/Manage: `Buy/sell tax is pair-direct only.` No always-on architecture banner.
7. **R607-7 — extra-debit Max matches execute.** Cap Max for extra-debit only when the submit path is pair-direct. Router hops Send `amount` 1:1 — do not shrink Max as if extra-debit will fire.
8. **R607-8 — PoC is a property.** `poc_router_exemption_full_tax_bypass` stays. It proves router-named `PROTOCOL_EXEMPT` skips Sell/Buy and pair-direct EOA still pays. Not a defect to invert.

## Why not 2 or 3

- **Option 2** needs a trusted `trader` on every hop, a **11611 migrate**, buy-leg spec (pair→router→user), and hybrid quotes that include extra-debit. Wrong `trader` is an abuse vector. Rejected while H-01 + live 11611 stay as-is.
- **Option 3** un-exempts the router: hops fail `InsufficientForSellTax` (router holds `amount` only). Breaks official hybrid / multi-hop / #596 and invoice wrap-routes. dApp pair-only is not a complete substitute (off-dApp router still works).

## #603 importer

[#603](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/603) was blocked until this choice was written. Importers (if any) must keep **T592-13**: migrated 11611 tokens have Honest router hops. Do not ship an importer that promises “bps on every official Swap.”

## Verify

```bash
make verify-issue-607
```

Also: `make verify-issue-592` (T592 crates + docs) · `make verify-issue-593` (Create Token + extra-debit).

## Do not

- Classify `Send+Swap` as Sell when `from` is the router (option 2) without a new issue + migrate.
- Remove `PROTOCOL_EXEMPT` on the router (option 3) without accepting a pair-only tax token.
- Claim “every official Swap pays buy/sell tax.”
- Add pair/router FoT math to “make bps mean bps” on multi-hop.
