# Agent playbook: one-sided zap execution floors (quote vs execute)

Use when changing **retail `/pool` one-sided zap-in/out sizing**, pre-sign min-swap copy, or multi-msg provide/unwrap amounts. Spec, AC, path tests T-Z1–T-Z14, and attack plan A-Z1–A-Z12 live on [GitLab **#559**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/559).

This is a follow-up to [#533](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/533) (frontend zap orchestration). Do **not** add a pair/router `Zap` execute unless LocalTerra rehearsal proves conservative floors still cannot work (**Z533-10**).

Reported production failure: columbus-5 UST1/cUSTC one-sided Add reverted `Overflow: Cannot Sub` on `provide_liquidity` (message index 3) because provide `TransferFrom` used the **quoted** ask while the swap only guaranteed `min_return`.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#559**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/559) | Spec: AC1–AC12, T-Z1–T-Z14, A-Z1–A-Z12 |
| [`oneSidedLiquidity.ts`](../frontend-dapp/src/utils/oneSidedLiquidity.ts) | `conservativeZapInProvide` / `conservativeZapOutExecution` |
| [`oneSidedLiquidityQuote.ts`](../frontend-dapp/src/utils/oneSidedLiquidityQuote.ts) | Quote snapshot stores **floor-trimmed** provide amounts |
| [`oneSidedLiquidityTx.ts`](../frontend-dapp/src/utils/oneSidedLiquidityTx.ts) | Builder rejects `provideAsk > swapMinReturn` |
| [`oneSidedLiquidityCopy.ts`](../frontend-dapp/src/utils/oneSidedLiquidityCopy.ts) | Human min-swap pre-sign line |
| [`OneSidedAddCard.tsx`](../frontend-dapp/src/components/pool/OneSidedAddCard.tsx) | Submit snapshot; human min-swap |
| [`OneSidedWithdrawCard.tsx`](../frontend-dapp/src/components/pool/OneSidedWithdrawCard.tsx) | Zap-out floors for swap + unwrap |
| [`docs/frontend.md`](../docs/frontend.md) § One-sided zap floors | **Z559-1–Z559-4** |
| [`AGENTS_FRONTEND_POOL_ONE_SIDED.md`](./AGENTS_FRONTEND_POOL_ONE_SIDED.md) | Parent **Z533** playbook |

## Invariants (Z559)

1. **Z559-1** — Execution amounts follow floors (`min_return` / `min_assets`); quotes may be optimistic. Zap-in `provideAsk ≤ swapMinReturn`. Do not `TransferFrom` the quoted ask. A fill in `(min_return, quote)` must never CW20-underflow.
2. **Z559-2** — Zap-in provide is ratio-trimmed to **conservative** post-swap reserves (worse fill → higher ask reserve still `+ swapIn` on the offer side). Leftover offer/ask stays in the wallet. Do not spend pre-existing ask balance to cover a shortfall (**Z533-4**, A-Z2).
3. **Z559-3** — Zap-out `swapAmount ≤ min_assets[sold]`. Unwrap send ≤ `min(wanted withdrawn, min_assets[wanted]) + swapMinReturn`. Never unwrap quoted `totalWantedCw20` when that exceeds the floor chain (**Z533-8**).
4. **Z559-4** — Pre-sign min-swap is **human** token units (not raw uints like `500571` next to `200 in`). Conservative LP dust / zero → one-sentence `Amount too small`; CTA blocked. Empty pool still `Empty pool. Use Advanced.`

## Rules of thumb

1. Keep solving `zapInSplit` on current pool + effective fee. Size **execution** `provideAsk` / `provideOffer` from `conservativeZapInProvide(split, swapMinReturn)`.
2. Route-in: `minimum_receive` floors the zap `amountIn`; wrap-in: solver input is post-`fee_wrap_bps` (W8, no burn tax). Then apply zap `min_return` to provide (AC5 / T-Z4 / T-Z5).
3. I14 / [#537](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/537): unwired pair → `discountBps = 0`. Do not let a wallet `get_discount` inflate `provideAsk` above on-chain output (T-Z9 / A-Z3).
4. Allowances must match conservative provide amounts. A18 order unchanged: wrap? → route? → swap → allowances ×2 → provide. Pool-only hybrid (`book_input = 0`).
5. Optional LCD `simulateHybridSwap` is belt-and-suspenders only — **not** a substitute for floors. Do not add LCD simulate-before-broadcast as the sole fix ([#475](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/475)).
6. **#489:** blocking errors stay one sentence. Do not add architecture essays.

## Verify

```bash
make verify-issue-559
make verify-issue-533
make test-frontend
# scoped units:
#   oneSidedLiquidity + oneSidedLiquidityTx + oneSidedLiquidityQuote + oneSidedLiquidityCopy
# Playwright smoke (5 workers):
#   frontend-dapp/e2e/pool-one-sided-533.spec.ts
# LocalTerra tx (1 worker; P4–P8 + P9):
#   frontend-dapp/e2e/pool-one-sided-533-tx.spec.ts
```

Issue: [GitLab **#559**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/559) (AC1–AC12, T-Z1–T-Z14, A-Z1–A-Z12).

## Related

- Parent one-sided flow: [`AGENTS_FRONTEND_POOL_ONE_SIDED.md`](./AGENTS_FRONTEND_POOL_ONE_SIDED.md) (`#533`)
- Copy: [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) (`#489`)
- Slippage default: [`AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md`](./AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md) (`#497`)
- Fee-discount chrome: [`AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md`](./AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md) (`#537` / I14)
- Wrap fees: [`AGENTS_WRAP_UNWRAP_BURN_TAX.md`](./AGENTS_WRAP_UNWRAP_BURN_TAX.md) (`#512`)
