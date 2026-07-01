# Agent playbook: swap signing confirmation (SEC-D11)

Use when changing **pre-sign swap summary** copy, labeled confirmation fields, or tests that guard against wallet phishing / signing confusion ([#409](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/409), SEC-D11).

## Product invariant

Before the wallet extension opens, taker swap surfaces must show a **labeled pre-sign summary** with:

| Field | Label | `data-testid` |
|-------|-------|---------------|
| Action | `Swap` (or `Market swap` on Trade market tab) | `swap-confirm-action` |
| Pair | `{offer} → {receive}` symbols | `swap-confirm-pair` |
| Offer | `{amount} {symbol}` | `swap-confirm-offer` |
| Receive (est.) | `{amount} {symbol}` | `swap-confirm-receive` |
| Max spread | `{slippageTolerance}%` (on-chain `max_spread`; retail label **Slippage protection**) | `swap-confirm-max-spread` |
| Min return | floor after slippage | `swap-confirm-min-return` |
| Chain | active network full label (`LocalTerra`, `Terra Classic`, …) | `swap-confirm-chain` |

Panel roots: **`swap-pre-submit-summary`** on `/swap`; **`trade-market-pre-submit-summary`** on Trade **Market** tab.

Values must come from the same submit snapshot as on-chain params ([`useSubmitAlignedSimQuote`](../frontend-dapp/src/hooks/useSubmitAlignedSimQuote.ts) — #356).

## Code map

| Concern | Location |
|--------|----------|
| Shared component | [`SwapPreSubmitSummary.tsx`](../frontend-dapp/src/components/swap/SwapPreSubmitSummary.tsx) |
| **Swap** wiring | [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) — above submit button when quote + positive pay |
| **Trade market** wiring | [`TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) |
| Chain label | [`networkDisplay.ts`](../frontend-dapp/src/utils/networkDisplay.ts) — `getNetworkBadgeCopy().fullLabel` |
| Unit tests | [`SwapPreSubmitSummary.test.tsx`](../frontend-dapp/src/components/swap/__tests__/SwapPreSubmitSummary.test.tsx) |
| Integration tests | [`SwapPage.test.tsx`](../frontend-dapp/src/pages/SwapPage.test.tsx) (Keplr wallet context), [`TradeMarketOrderPanel.submitSnapshot.test.tsx`](../frontend-dapp/src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx) |

## Verification commands

```bash
make test-frontend
cd frontend-dapp && npm run test:run -- \
  src/components/swap/__tests__/SwapPreSubmitSummary.test.tsx \
  src/pages/SwapPage.test.tsx \
  src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx
```

## Rules of thumb

1. **Do not** remove individual field `data-testid`s — SEC-D11 regression tests assert each labeled row.
2. **Do not** show factory/router on this panel — `/protocol` only ([#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378)).
3. Keep **Route** on the separate `swap-route-summary` / `trade-market-route-summary` row ([`AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](./AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md)); the pre-sign panel repeats pair symbols for phishing resistance, not hop detail.
4. Limit orders use [`LimitOrderPreSubmitSummary`](../frontend-dapp/src/components/trade/LimitOrderPreSubmitSummary.tsx) — labeled action/pair/side/amount/chain plus resting fee semantics ([#157](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/157), [#461](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/461) / SEC-I05).

## Related

- Trust boundaries: [`AGENTS_FRONTEND_TRUST_BOUNDARIES.md`](./AGENTS_FRONTEND_TRUST_BOUNDARIES.md)
- Route row: [`AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](./AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md)
- Docs: [`docs/frontend.md` § Swap pre-sign summary](../docs/frontend.md#swap-page-pre-sign-summary)
