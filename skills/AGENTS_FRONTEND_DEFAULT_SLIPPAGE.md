# Agent playbook: default Slippage protection (GitLab #497)

Use when changing the retail **Slippage protection** default, Swap/Trade slippage presets, insufficient-liquidity / route-impact warning copy that interpolates `slippageTolerance`, or tests that assert pre-sign **max spread** / min receive for a fresh session.

## Product invariants

1. **Default is 5%** for new Zustand sessions (`useDexStore.slippageTolerance` ← `DEFAULT_SLIPPAGE_TOLERANCE_PERCENT`).
2. **On-chain mapping:** `max_spread = (slippageTolerance / 100).toString()` (e.g. `5` → `"0.05"`).
3. **Route-impact / insufficient-liquidity warning** uses the same store value: `Route impact exceeds your {slippageTolerance}% protection…` on Swap.
4. **Presets** are shared: `SLIPPAGE_TOLERANCE_PRESETS_PERCENT` = `[0.5, 1.0, 5.0]` on Swap Settings and Trade market. Custom input still allows `0.01`–`50`.
5. **High-protection warn** (`HIGH_SLIPPAGE_PROTECTION_WARN_PERCENT = 5`) fires only when tolerance is **strictly greater than** 5% — the default itself does not show the front-running risk banner.
6. **Do not hard-code `0.5` as the product default** in UI, store, or “fresh session” tests. Component unit tests may still pass an arbitrary `maxSpreadPercent` prop.

Constants live in [`slippageProtectionCopy.ts`](../frontend-dapp/src/utils/slippageProtectionCopy.ts). Store wiring: [`dex.ts`](../frontend-dapp/src/stores/dex.ts).

## Code map

| Concern | Location |
|--------|----------|
| Default + presets + warn threshold | [`slippageProtectionCopy.ts`](../frontend-dapp/src/utils/slippageProtectionCopy.ts) |
| Zustand initial state | [`dex.ts`](../frontend-dapp/src/stores/dex.ts) |
| Swap Settings presets + warn + route-impact copy | [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) |
| Trade market presets | [`TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) |
| Pre-sign max spread row | [`SwapPreSubmitSummary.tsx`](../frontend-dapp/src/components/swap/SwapPreSubmitSummary.tsx) |
| Unit tests (constants + store) | [`slippageProtectionCopy.test.ts`](../frontend-dapp/src/utils/__tests__/slippageProtectionCopy.test.ts) |
| Integration (default on confirm) | [`SwapPage.test.tsx`](../frontend-dapp/src/pages/SwapPage.test.tsx), [`TradeMarketOrderPanel.submitSnapshot.test.tsx`](../frontend-dapp/src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx) |

## Verification

```bash
cd frontend-dapp && npm run test:run -- \
  src/utils/__tests__/slippageProtectionCopy.test.ts \
  src/pages/SwapPage.test.tsx \
  src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx \
  src/components/swap/__tests__/SwapPreSubmitSummary.test.tsx
```

Or: `make test-frontend` from repo root (Node 24 on `PATH`).

## Related

- Max-spread UX invariants: [`docs/swap-max-spread-ux.md`](../docs/swap-max-spread-ux.md) (§ frontend invariant **9**)
- MEV / public mempool posture: [`docs/frontend.md`](../docs/frontend.md) (Slippage protection is the on-chain guard)
- Pre-sign summary: [`AGENTS_FRONTEND_SWAP_SIGNING_CONFIRMATION.md`](./AGENTS_FRONTEND_SWAP_SIGNING_CONFIRMATION.md)
- On-chain `max_spread` math: [`AGENTS_MAX_SPREAD_HYBRID.md`](./AGENTS_MAX_SPREAD_HYBRID.md)
