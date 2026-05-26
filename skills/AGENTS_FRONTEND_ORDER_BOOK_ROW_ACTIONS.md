# Agent playbook: Trade order book row actions

Use when changing **`OrderBookPanel.tsx`** row layout, **cancel / edit / cancel-all** behavior on **`/trade` or `/limits`**, hook **`useLimitOrderCancelMutation`**, or Vitest **`OrderBookPanel.test.tsx`**.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Trade page — order book row actions](../docs/frontend.md#trade-book-row-actions) | Invariants: shared cancel mutation, `#order_id` column, wallet-only row actions, cancel-all semantics, **single** `/trade` ticket mount for book **Edit** prefill ([GitLab **#178**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/178)), test ids ([GitLab **#162**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/162)) |
| [docs/frontend.md § Limit orders page — order book row actions](../docs/frontend.md#limits-page-order-book-row-actions) | Same `OrderBookPanel` + `useLimitOrderCancelMutation` on **`/limits`** ([`LimitOrdersPage.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.tsx)); manual cancel form remains with copy pointing to row actions |
| `frontend-dapp/src/hooks/useLimitOrderCancelMutation.ts` | Single `cancel_limit_order` mutation + invalidations (`limitPlacements`, `limitCancellations`, `limitBookPage`, `limitBookPagePreview`, `tradeBestBook`, `wallet-indexer-history`) |
| `frontend-dapp/src/pages/TradePage.tsx` | Wires wallet + paused query + shared mutation + `LimitBookTicketDraft` push into **one** `TradeOrderTicket` (sub-lg vs desktop via `useMediaQuery` — do not mount two tickets) |
| `frontend-dapp/src/utils/tradePageLayout.ts` | `TRADE_DESKTOP_LAYOUT_MEDIA_QUERY` — Tailwind `lg` / `1024px` gate for `/trade` layout branches |
| `frontend-dapp/src/pages/LimitOrdersPage.tsx` | Wires the same book + cancel mutation + draft prefill into the standalone limits flow (no duplicate cancel hook) |
| `frontend-dapp/src/components/trade/TradeOrderTicket.tsx` | Optional injected cancel mutation (avoids duplicate hook vs book); consumes book draft for ticket prefill |
| `frontend-dapp/src/types/limitBookTicketDraft.ts` | `LimitBookTicketDraft` payload for Edit-from-book |

## Rules of thumb

1. **Never** instantiate a second `useLimitOrderCancelMutation` for the same `/trade` surface — keep one instance in `TradePage` and pass it to both panels.
2. **Never** mount two `TradeOrderTicket` instances on `/trade` (even if CSS-hidden). Book **Edit** uses `limitBookDraftKey` + `onLimitBookDraftConsumed`; a hidden ticket can clear the draft before the visible ticket applies it ([#178](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/178)).
3. **Edit** is ticket prefill only (no modal, no network request); users must **cancel** the resting order before placing a true replacement (no on-chain amend).
4. **Cancel-all** uses indexer **active** placements only; if the indexer lags the chain, the button may omit very new orders — same limitation as “My limits (indexer)”.
5. When changing column grids or `data-testid`s, update **`OrderBookPanel.test.tsx`**, **`TradePage.test.tsx`** (desktop **Edit** prefill), and the invariants table in **`docs/frontend.md`**.

## Related

- Deep book pagination: [`AGENTS_FRONTEND_DEEP_ORDER_BOOK.md`](./AGENTS_FRONTEND_DEEP_ORDER_BOOK.md) ([GitLab **#194**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/194))
- Trade workspace layout: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- Limit price / crossing gates: [`AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md)
- Limit side control: [`AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md`](./AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md)
