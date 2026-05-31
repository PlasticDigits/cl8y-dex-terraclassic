# Agent playbook: Trade order book row actions

Use when changing **`OrderBookPanel.tsx`** row layout, **cancel / edit / cancel-all** behavior on **`/trade` or `/limits`**, hooks **`useLimitOrderCancelMutation`** / **`useLimitOrderUpdatePriceMutation`**, or Vitest **`OrderBookPanel.test.tsx`**.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Trade page — order book row actions](../docs/frontend.md#trade-book-row-actions) | Invariants: shared cancel mutation, `#order_id` column, wallet-only row actions, cancel-all semantics, **single** `/trade` ticket mount for book **Edit** prefill ([GitLab **#178**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/178)), **#247** price-only update ([GitLab **#247**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/247)), test ids ([GitLab **#162**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/162)) |
| [docs/frontend.md § Limit orders page — order book row actions](../docs/frontend.md#limits-page-order-book-row-actions) | Same `OrderBookPanel` + `useLimitOrderCancelMutation` on **`/limits`** ([`LimitOrdersPage.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.tsx)); manual cancel form remains with copy pointing to row actions |
| [docs/limit-orders.md § UpdateLimitOrderPrice](../docs/limit-orders.md#messages-cosmwasm) | On-chain price relink — no maker fee, no escrow movement |
| `frontend-dapp/src/hooks/useLimitOrderCancelMutation.ts` | Single/batch cancel + invalidations |
| `frontend-dapp/src/hooks/useLimitOrderUpdatePriceMutation.ts` | **`update_limit_order_price`** + book/placement invalidations ([#247](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/247)) |
| `frontend-dapp/src/utils/limitOrderPriceEdit.ts` | **`isPriceOnlyLimitEdit`**, **`buildLimitBookEditContext`**, non-price change copy |
| `frontend-dapp/src/pages/TradePage.tsx` | Wires wallet + paused query + shared mutation + `LimitBookTicketDraft` push into **one** `TradeOrderTicket` (sub-lg vs desktop via `useMediaQuery` — do not mount two tickets) |
| `frontend-dapp/src/utils/tradePageLayout.ts` | `TRADE_DESKTOP_LAYOUT_MEDIA_QUERY` — Tailwind `lg` / `1024px` gate for `/trade` layout branches |
| `frontend-dapp/src/pages/LimitOrdersPage.tsx` | Wires the same book + cancel mutation + draft prefill into the standalone limits flow (no duplicate cancel hook) |
| `frontend-dapp/src/components/trade/TradeOrderTicket.tsx` | Optional injected cancel mutation; consumes book draft; **Update price** vs **Place limit** submit |
| `frontend-dapp/src/types/limitBookTicketDraft.ts` | `LimitBookTicketDraft` + `LimitBookEditContext` |
| `frontend-dapp/src/services/terraclassic/pair.ts` | **`updateLimitOrderPrice`** |

## Rules of thumb

1. **Never** instantiate a second `useLimitOrderCancelMutation` for the same `/trade` surface — keep one instance in `TradePage` and pass it to both panels.
2. **Never** mount two `TradeOrderTicket` instances on `/trade` (even if CSS-hidden). Book **Edit** uses `limitBookDraftKey` + `onLimitBookDraftConsumed`; a hidden ticket can clear the draft before the visible ticket applies it ([#178](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/178)).
3. **Edit** prefills the ticket (no modal). **Price-only** change → **`UpdateLimitOrderPrice`** via **`useLimitOrderUpdatePriceMutation`** ([#247](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/247)). **Size / side / expiry** change → block with **`LIMIT_EDIT_NON_PRICE_CHANGE_MESSAGE`**; user must cancel then place.
4. **Cancel-all** uses indexer **active** placements only; if the indexer lags the chain, the button may omit very new orders — same limitation as “My limits (indexer)”.
5. When changing column grids or `data-testid`s, update **`OrderBookPanel.test.tsx`**, **`TradePage.test.tsx`** (desktop **Edit** prefill), **`limitOrderPriceEdit.test.ts`**, and the invariants table in **`docs/frontend.md`**.

## Related

- Deep book pagination: [`AGENTS_FRONTEND_DEEP_ORDER_BOOK.md`](./AGENTS_FRONTEND_DEEP_ORDER_BOOK.md) ([GitLab **#194**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/194))
- Trade workspace layout: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- Limit price / crossing gates: [`AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md)
- Limit side control: [`AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md`](./AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md)
- Terra gas: [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md) — **`UPDATE_LIMIT_ORDER_PRICE_GAS_LIMIT`**
- Batch placement gas: [`AGENTS_LIMIT_ORDER_BATCH_LADDER.md`](./AGENTS_LIMIT_ORDER_BATCH_LADDER.md) — storage collapse **#247**
