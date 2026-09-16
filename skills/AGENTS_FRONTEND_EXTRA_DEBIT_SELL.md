# Agent playbook: extra-debit Sell submit gate (Forgejo #1267)

Use when changing Swap / Trade Market **sell** CTA, Max, `TaxPreview`, or community-tax detection on the pay CW20.

This is a **dApp execute gate**. Pair/router wasm stay unchanged (**H-01** / **T592-1**). Do **not** reopen [#593](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593) (Max) or [#1228](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1228) (SendFrom allowance). Do **not** add FoT math.

Parent Max: [`AGENTS_FRONTEND_CREATE_TOKEN.md`](./AGENTS_FRONTEND_CREATE_TOKEN.md) **C593-9**. Router hops: [`AGENTS_COMMUNITY_TAX_ROUTER.md`](./AGENTS_COMMUNITY_TAX_ROUTER.md) **R607-7**. On-chain extra-debit: [`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md) **T592-2**.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [Forgejo **#1267**](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1267) | Typed 100% / reverse offer still submitted when `TaxPreview.debit > balance` |
| [`docs/frontend.md` § Create Token](../docs/frontend.md#create-token-community-tax) | **S1267-1–S1267-8** + **C593-9** |
| [`taxPreviewMaxSpend.ts`](../frontend-dapp/src/utils/taxPreviewMaxSpend.ts) | `extraDebitSubmitGate` + round-trip Max |
| [`useCommunityTaxSellBps.ts`](../frontend-dapp/src/hooks/useCommunityTaxSellBps.ts) | Live `GetConfig.sell_bps` (not catalog pin equality) |
| [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) / [`TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) | CTA + mutation guard |
| [`humanizeTerraTxError.ts`](../frontend-dapp/src/utils/humanizeTerraTxError.ts) | `InsufficientForSellTax` copy |

## Invariants **S1267-1–S1267-8**

1. **S1267-1 — LCD debit is execute truth.** When `TaxPreview.debit` parses, Swap/Trade **must not** build or sign if `debit > balance`. Classic `amount > balance` stays Insufficient Balance. Do not size extra-debit in pair/router wasm.
2. **S1267-2 — Max leaves debit room.** Max declared is `extraDebitMaxDeclaredRaw` so human round-trip `toRawAmount(fromRawAmount(declared))` still has `declared + floor(declared * sell_bps / 10000) ≤ balance`. User Sends `amount` 1:1; leftover tax is extra-debit (**R607-7**).
3. **S1267-3 — pin ≠ sell detector.** `VITE_COMMUNITY_TAX_CODE_ID` is Create Token / Manage template only (**C593-8**). Sell detection is LCD `GetConfig.sell_bps` on the pay contract. `code_id === pin` must not skip extra-debit on a listed tax wasm with a different live id.
4. **S1267-4 — unknown exempt fail-closed.** Manager-directory `true` → 0 extra-debit (Honest). `null` / loading / error keep `sell_bps`. Do not unlock 100% Max while exempt is unknown (**E609-7**).
5. **S1267-5 — pair-direct and router.** Extra-debit gate uses the same bps for pair `Send+Swap` and official-router hops (`swapOpsRequireRouter` / `ops.length >= 2`). Preview `to` is the pair or `VITE_ROUTER_ADDRESS`.
6. **S1267-6 — honest / 0-bps unchanged.** Native `uluna`, unknown-query honest CW20, and `sell_bps = 0` stay `amount ≤ balance`. GetConfig CosmWasm unknown-query is **not** tax. LCD transport timeout on GetConfig is treated as honest so UST1 Swap is not bricked; after an instance is already tax, missing preview falls back to local `sell_bps` (never assume 0-tax). Hostile `sell_bps` / debit strings → unresolved → block submit.
7. **S1267-7 — humanize, no ticket ids.** Map `InsufficientForSellTax` / `Insufficient balance for extra-debit sell tax` to `Not enough tokens after sell tax. Reduce the amount or tap Max.` No GitLab / Forgejo / issue numbers in UI.
8. **S1267-8 — no wasm / FoT / reopen.** Do not change pair or router contracts. Do not teach the dApp FoT reserve math. Do not reopen #593 / #1228 / #607 classify. `isCommunityTaxEnabled()` only means Create Token env is present (so LCD is queried); it is not `code_id === pin`.

## Verify

```bash
make verify-issue-1267
make verify-issue-593
make verify-issue-607
```

RTL + Vitest only. LocalTerra execute is not required for this gate.

## Do not

- Treat catalog pin equality as the Swap/Trade tax switch.
- Assume 0-tax when GetConfig `sell_bps` shape is hostile or detection is still loading.
- Skip the mutation guard because the CTA is disabled (race / keyboard).
- Surface raw CosmWasm logs or ticket numbers on the CTA.
