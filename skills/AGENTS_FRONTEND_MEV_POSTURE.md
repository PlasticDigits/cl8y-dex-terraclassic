# Agent playbook: MEV / submission posture (docs-only)

Use when changing **MEV / public mempool documentation** or reviewing whether swap/trade UI should surface submission posture ([GitLab **#168**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/168), [**#299**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/299)).

## Product invariant

- Swaps and trades are signed in the wallet and broadcast to the **public** Terra Classic mempool. This build has **no** private RPC, bundle relay, or MEV-protection path.
- **Do not** add MEV posture UI (informational cards, toggles, or settings) on Swap (`/`) or Trade (`/trade`) — it would imply a user-controllable setting that does not exist ([#299](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/299)).
- **Do not** add a fake/disabled “MEV protection” toggle; add a real control only when submission code uses a protected path end-to-end.
- **Slippage tolerance** remains the on-chain sandwich guard — keep aligned with [`docs/swap-max-spread-ux.md`](../docs/swap-max-spread-ux.md) (GitLab #134). Swap Settings still exposes slippage presets and the high-slippage front-running warning.

## Docs map

| Concern | Location |
|--------|----------|
| Canonical MEV / submission posture | [`docs/frontend.md` § Swap page — MEV / submission posture](../docs/frontend.md#swap-mev-posture) |
| Slippage / price impact | [`docs/swap-max-spread-ux.md`](../docs/swap-max-spread-ux.md) |
| Hybrid routing disclosure | [`docs/limit-orders.md`](../docs/limit-orders.md) (GitLab #111) |
| Slippage UI + front-running warning | [`frontend-dapp/src/pages/SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) — Settings slippage card only |

## Related skills

- [`AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](./AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md) — route row / indexer vs client path.
- [`AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](./AGENTS_FRONTEND_RISK_DISCLAIMERS.md) — NFA / first-visit modal (separate from execution posture).

## Regression checklist (manual)

1. Open `/` — **no** `swap-mev-posture-notice` or “Transaction submission (MEV)” card anywhere (including Settings).
2. Open `/trade/:pairAddr` — same; no MEV posture UI.
3. Open Settings on `/` — slippage presets and limit-book controls still work; high slippage (>5%) still shows front-running warning.
4. Confirm [`docs/frontend.md#swap-mev-posture`](../docs/frontend.md#swap-mev-posture) documents public mempool risks and the no-toggle policy.
