# Agent playbook: Trade order book row actions

Use when changing **`OrderBookPanel.tsx`** row layout, **cancel / edit / cancel-all** behavior on `/trade`, hook **`useLimitOrderCancelMutation`**, or Vitest **`OrderBookPanel.test.tsx`**.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Trade page — order book row actions](../docs/frontend.md#trade-book-row-actions) | Invariants: shared cancel mutation, `#order_id` column, wallet-only row actions, cancel-all semantics, test ids ([GitLab **#162**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/162)) |
| `frontend-dapp/src/hooks/useLimitOrderCancelMutation.ts` | Single `cancel_limit_order` mutation + invalidations (`limitPlacements`, `limitCancellations`, `limitBookPage`, `tradeBestBook`) |
| `frontend-dapp/src/pages/TradePage.tsx` | Wires wallet + paused query + shared mutation + `LimitBookTicketDraft` push into `TradeOrderTicket` |
| `frontend-dapp/src/components/trade/TradeOrderTicket.tsx` | Optional injected cancel mutation (avoids duplicate hook vs book); consumes book draft for ticket prefill |
| `frontend-dapp/src/types/limitBookTicketDraft.ts` | `LimitBookTicketDraft` payload for Edit-from-book |

## Rules of thumb

1. **Never** instantiate a second `useLimitOrderCancelMutation` for the same `/trade` surface — keep one instance in `TradePage` and pass it to both panels.
2. **Edit** is ticket prefill only; document that users must **cancel** the resting order before placing a true replacement (no on-chain amend).
3. **Cancel-all** uses indexer **active** placements only; if the indexer lags the chain, the button may omit very new orders — same limitation as “My limits (indexer)”.
4. When changing column grids or `data-testid`s, update **`OrderBookPanel.test.tsx`** and the invariants table in **`docs/frontend.md`**.

## Related

- Trade workspace layout: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- Limit price / crossing gates: [`AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md)
- Limit side control: [`AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md`](./AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md)
