# Agent playbook: pair-scoped CL8Y fee-tier chrome (GitLab #537)

Use when changing Swap / Pool / Trade **fee display**, `FeeDisplay` strikethrough, unregistered Hold-CL8Y CTAs, or `useLimitOrderMakerFeeRates`. On-chain discounts apply only when the **pair** has `DISCOUNT_REGISTRY` set; querying `VITE_FEE_DISCOUNT_ADDRESS` alone is not enough.

Issue **#537 is implemented**. Do **not** advertise a wallet’s `get_discount` on an unwired pair (full `fee_bps` / `maker_fee_bps(fee_bps)` on execute). Parent wiring is [#535](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/535) (ops; out of scope here).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#537**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/537) | dApp mismatch: fee chrome vs pair registry |
| [`docs/reference/fee-discount-tiers.md`](../docs/reference/fee-discount-tiers.md) invariant **I14** | Advertise discount only when pair registry matches configured contract |
| [`docs/frontend.md` § Pair fee-tier chrome](../docs/frontend.md#pair-fee-tier-chrome) | Invariants **F537-1–F537-6** |
| [`pairDiscountRegistry.ts`](../frontend-dapp/src/utils/pairDiscountRegistry.ts) | Parse raw `Item<Option<Addr>>`, `pairFeeDiscountApplies`, `advertisedDiscountBps` |
| [`pairDiscountRegistry.ts` (LCD)](../frontend-dapp/src/services/terraclassic/pairDiscountRegistry.ts) | Raw key probe + `get_discount_registry` fallback |
| [`useFeeDiscountRegistryStatus.ts`](../frontend-dapp/src/hooks/useFeeDiscountRegistryStatus.ts) | Optional `pairAddr` gates `discountBps` + `pairDiscountApplies` |
| [`useLimitOrderMakerFeeRates.ts`](../frontend-dapp/src/hooks/useLimitOrderMakerFeeRates.ts) | Maker place bps uses full fee when unwired |
| [`AGENTS_FEE_DISCOUNT_TIERS.md`](./AGENTS_FEE_DISCOUNT_TIERS.md) | Tier ladder, I8 factory rollout, I13 placement shift |

## Rules of thumb

1. **Chrome ≠ quote.** HybridSimulation / `route/solve` with `trader` already fail-closed to full fee on unwired pairs. Do not “fix” quotes by applying `getTraderDiscount` client-side.
2. **Match addresses (F537-1).** A pair wired to a *different* registry than `VITE_FEE_DISCOUNT_ADDRESS` must not show the VITE discount.
3. **Fail-closed (F537-2, F537-3).** Missing pair, loading probe, raw 403 + no smart query, or JSON `null` → `advertisedDiscountBps = 0`. Never strikethrough a guessed discount.
4. **Raw key until query ships (F537-4).** Live pair wasm has no `GetDiscountRegistry`. LCD `/raw/ZGlzY291bnRfcmVnaXN0cnk=` (`discount_registry`). Do not add a pair QueryMsg variant in a frontend-only change.
5. **CTA (F537-5).** Hide pair-scoped “Hold CL8Y & register” / “not registered” when the pair cannot apply a discount. Keep the global registry-outage banner (#365 / #374).
6. **Maker place.** `useLimitOrderMakerFeeRates` must use `maker_fee_bps(fee_bps)` (half of full pair fee) on unwired pairs, not half of a discounted effective. **F537-6:** quotes stay execute-aligned; this issue is fee chrome only.

## Verify

```bash
make verify-issue-537
```

## Related

- Factory registry rollout: [`AGENTS_FEE_DISCOUNT_TIERS.md`](./AGENTS_FEE_DISCOUNT_TIERS.md) § Factory registry rollout (**I8**)
- Limit maker fee math: [`AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md) (`#157` / `#514`)
- Registry outage: [`AGENTS_FEE_DISCOUNT_TIERS.md`](./AGENTS_FEE_DISCOUNT_TIERS.md) § Registry outage observability (`#365`)
- Hybrid quotes: [`AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md)
