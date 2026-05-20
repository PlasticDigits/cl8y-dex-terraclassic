# Agent playbook: Swap MEV / submission posture disclosure

Use when changing **public mempool disclosure**, **`MevPostureNotice`**, **`mevPosture.ts` copy**, or Swap **Settings** layout next to routing controls ([GitLab **#168**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/168), W10-C3).

## Product invariant

- Traders must see **how** swaps are submitted (wallet → public mempool) and that **no** MEV-protection or private-RPC path exists in this build.
- **Do not** add a fake/disabled “MEV protection” toggle; add a real control only when submission code uses a protected path end-to-end.
- **Slippage tolerance** is the on-chain sandwich guard — keep copy aligned with [`docs/swap-max-spread-ux.md`](../docs/swap-max-spread-ux.md) (GitLab #134).

## Code map

| Concern | Location |
|--------|----------|
| Copy constants | [`frontend-dapp/src/utils/mevPosture.ts`](../frontend-dapp/src/utils/mevPosture.ts) |
| Unit tests | [`frontend-dapp/src/utils/mevPosture.test.ts`](../frontend-dapp/src/utils/mevPosture.test.ts) |
| Settings UI | [`frontend-dapp/src/components/swap/MevPostureNotice.tsx`](../frontend-dapp/src/components/swap/MevPostureNotice.tsx) |
| Mount (Settings open) | [`frontend-dapp/src/pages/SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) — after slippage card, before limit-book leg |
| Vitest | [`frontend-dapp/src/pages/SwapPage.test.tsx`](../frontend-dapp/src/pages/SwapPage.test.tsx) |
| Playwright | [`frontend-dapp/e2e/swap.spec.ts`](../frontend-dapp/e2e/swap.spec.ts) |

## Docs cross-links

- [`docs/frontend.md` § Swap page — MEV / submission posture](../docs/frontend.md#swap-mev-posture) — canonical invariants table.
- [`docs/swap-max-spread-ux.md`](../docs/swap-max-spread-ux.md) — slippage / price impact (complementary, not duplicate).
- [`docs/limit-orders.md`](../docs/limit-orders.md) — hybrid routing disclosure (GitLab #111).

## Related skills

- [`AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](./AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md) — route row / indexer vs client path.
- [`AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](./AGENTS_FRONTEND_RISK_DISCLAIMERS.md) — NFA / first-visit modal (separate from execution posture).

## Regression checklist (manual)

1. Open `/`, click **Settings** — **Transaction submission (MEV)** card visible (`swap-mev-posture-notice`).
2. Confirm copy mentions **public mempool** and **no** private RPC / MEV toggle.
3. Change slippage preset — notice shows updated **%** in the slippage sentence.
4. Collapse Settings — notice hidden.
5. With a direct CW20 pair, confirm **limit book leg** and **Indexer route check** still appear below the MEV card.
