# Agent playbook: Limit Bid / Ask side selector (trade + limits)

Use when changing **limit order side** UI on **`/trade`** ([`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx)) or **`/limits`** ([`LimitOrdersPage.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.tsx)), or the shared control in [`LimitOrderBidAskSideSelector.tsx`](../frontend-dapp/src/components/trade/LimitOrderBidAskSideSelector.tsx).

## Canonical references

| Doc / issue | Purpose |
|-------------|---------|
| [docs/frontend.md § Limit place — Bid / Ask side control](../docs/frontend.md#limit-place-bid-ask-side) | Invariants: radiogroup + roving tabindex, `data-testid` prefixes, escrow semantics ([GitLab **#153**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/153)) |
| [GitLab **#300**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/300) | **`/trade`** direction labels: **Buy {base}** / **Buy {quote}** via [`tradeDirectionSideLabels.ts`](../frontend-dapp/src/utils/tradeDirectionSideLabels.ts) — do not reuse base symbol on both buttons |
| [docs/frontend.md § Limit place — escrow amount](../docs/frontend.md#limit-place-escrow-amount) | Headline USD line, manual clear vs MAX re-apply on side change ([GitLab **#155**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/155)) |
| [`LimitOrderBidAskSideSelector.tsx`](../frontend-dapp/src/components/trade/LimitOrderBidAskSideSelector.tsx) | Shared `role="radiogroup"` / `role="radio"` button implementation |
| [`frontend-dapp/src/components/trade/__tests__/LimitOrderBidAskSideSelector.test.tsx`](../frontend-dapp/src/components/trade/__tests__/LimitOrderBidAskSideSelector.test.tsx) | Unit tests (click + keyboard) |

## Rules of thumb

1. **Do not revert to native `<input type="radio">`** for this control without re-validating **pointer responsiveness** and cross-browser controlled-radio behavior ([**#153**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/153)).
2. **Escrow mapping** stays in parents: `side === 'bid'` → escrow **token1**; `side === 'ask'` → escrow **token0** — the selector only toggles `side`. Parents must still **clear manual amounts** or **re-apply MAX** when `side` changes ([**#155**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/155) — see [`useLimitOrderForm.ts`](../frontend-dapp/src/hooks/useLimitOrderForm.ts)).
3. **Accessibility**: preserve **radiogroup** labelling, **`aria-checked`**, roving **tabIndex**, and **arrow / Home / End** behavior when extending.
4. **Styling**: keep **`tab-neo` / `tab-neo-active` / `tab-neo-inactive`** so **`:focus-visible`** stays consistent with [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md).
5. **`/trade` vs `/limits` labels**: on **`/trade`**, pass **`tradeDirectionSideLabels(base, quote)`** into **`bidLabel` / `askLabel`** — both buttons use **Buy** with distinct token symbols ([#300](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/300)). On **`/limits`**, keep escrow-oriented **Bid (escrow …)** / **Ask (escrow …)** copy.

## Related

- Limit order **price** field (tape reference, validation): [`AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md) ([GitLab **#154**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/154))
- **Escrow amount** headline USD + side-switch sizing: [docs/frontend.md § Limit place — escrow amount](../docs/frontend.md#limit-place-escrow-amount) ([GitLab **#155**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/155))
- Trade workspace layout: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- Parked / expired limits copy: [`AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](./AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md)
