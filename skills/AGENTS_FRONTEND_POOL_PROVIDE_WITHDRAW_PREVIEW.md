# Agent playbook: pool provide auto-fill & withdraw preview (#480)

Use when changing **provide liquidity counterpart auto-fill**, **withdraw receive preview**, or related tests on `/pool`.

## Product invariants

### Provide auto-fill

| Rule | Behavior |
|------|----------|
| Non-empty pool | Editing Asset A or B auto-fills the counterpart from pool ratio when the other field is **empty** (or `.` draft). |
| Max / 50% | Always force-sync counterpart (`forceSync: true`). |
| Empty pool | Both reserves `0` → **no** auto-fill. |
| Manual override | After auto-fill, user edits filled side → leave other side unchanged; `pool-provide-ratio-warning` may show. |
| Native wrap | Ratio uses **post–mapper-fee** raw (`provideRawAdd*`; wrap_deposit untaxed — [#512](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/512)); invert via `amountForTargetNetAfterWrapMapperFee`. |

### Withdraw preview

| Element | Meaning |
|---------|---------|
| `pool-withdraw-estimated-receive` | Pro-rata underlying tokens at **0% slippage** (`estimateWithdrawAssetAmounts`). |
| `pool-withdraw-minimum-receive` | Floor after selected slippage % (`withdrawMinAssetAmounts`). |
| Receive labels | Wrapped symbols when “Receive as wrapped tokens” checked; native when unwrap checkbox unchecked and native equiv exists. |
| Pre-sign summary | Withdraw `amountLines`: LP line + expected underlying tokens. |

## Code map

| Concern | Location |
|---------|----------|
| Floor-ratio counterpart raw | [`provideLiquidityEstimate.ts`](../frontend-dapp/src/utils/provideLiquidityEstimate.ts) — `computeProportionalCounterpartRaw` |
| Human auto-fill + tax | [`poolProvideCounterpart.ts`](../frontend-dapp/src/utils/poolProvideCounterpart.ts) — `computeProvideCounterpartHuman` |
| Gross for target net | [`nativeTransferTax.ts`](../frontend-dapp/src/utils/nativeTransferTax.ts) — `grossUlunaForTargetNet` |
| Withdraw estimates | [`rawAmountMath.ts`](../frontend-dapp/src/utils/rawAmountMath.ts) — `estimateWithdrawAssetAmounts` |
| UI wiring | [`PoolPage.tsx`](../frontend-dapp/src/pages/PoolPage.tsx) — `setProvideAmountA/B`, withdraw preview block |

## Verification commands

```bash
cd frontend-dapp && npm run test:run -- \
  src/utils/__tests__/provideLiquidityEstimate.test.ts \
  src/utils/__tests__/poolProvideCounterpart.test.ts \
  src/utils/__tests__/rawAmountMath.test.ts \
  src/utils/__tests__/nativeTransferTax.test.ts \
  src/pages/PoolPage.test.tsx
```

## Related

- Native wrap tax: [`AGENTS_NATIVE_WRAP_TAX.md`](./AGENTS_NATIVE_WRAP_TAX.md)
- Pre-sign summary: [`AGENTS_FRONTEND_POOL_SIGNING_CONFIRMATION.md`](./AGENTS_FRONTEND_POOL_SIGNING_CONFIRMATION.md)
- Docs: [`docs/frontend.md` § Pool provide liquidity](../docs/frontend.md#pool-page--provide-liquidity-ui-invariants)
