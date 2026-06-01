# Agent playbook: Deep limit order book (pagination)

Use when changing **paginated book depth** on **`/trade` or `/limits`**, indexer **`GET .../limit-book`**, React Query **`useLimitBookInfinite`**, or Vitest / Playwright around **Load more depth**.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#194**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/194) | Deep book + UI pagination (remainder of **#102**) |
| [docs/frontend.md § Trade page — deep order book pagination](../docs/frontend.md#trade-page-deep-order-book) | UI invariants: indexer-only path, page size, keyset cursor, non-blocking fetch |
| [docs/integrators.md § On-chain limit book (LCD proxy)](../docs/integrators.md#on-chain-limit-book-lcd-proxy) | HTTP params, errors, LCD cost, rate limits |
| [ADR 0002](../docs/adr/0002-limit-book-surfacing.md) | Design: paginated `limit-book` vs legacy `limit-book-shallow` |
| [docs/limit-orders.md](../docs/limit-orders.md) | On-chain book + indexer proxy overview |
| [docs/indexer-invariants.md](../docs/indexer-invariants.md) | Indexer caps, 429, test mapping |
| `indexer/tests/api_limit_book_deep.rs` | Deep pagination + concurrent page stress (wiremock LCD) |
| `frontend-dapp/src/hooks/useLimitBookInfinite.ts` | Shared infinite query for one book side |
| `frontend-dapp/src/utils/limitBookPagination.ts` | `LIMIT_BOOK_UI_PAGE_SIZE`, `limitBookPageQueryKey` |
| `frontend-dapp/src/utils/limitBookInsertHint.ts` | **`resolveLimitInsertHintAfter`** — client fallback; prefer indexer **`GET .../limit-book/insert-hints`** ([#267](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/267)) |
| [integrators.md § Insert hints & price window](../docs/integrators.md#insert-hints-price-window-gitlab-267) | Batch hints + `price_from`/`price_to` window (**#267**) |
| `indexer/tests/api_limit_book_insert_hints.rs` | Resolver parity + HTTP regression (**#267**) |
| [`useLimitLadderPlacementPlan.ts`](../frontend-dapp/src/hooks/useLimitLadderPlacementPlan.ts) | Ladder preflight: price window + batch hints (**#268**) |
| [`limitLadderPlacementPlan.ts`](../frontend-dapp/src/utils/limitLadderPlacementPlan.ts) | Path selection: thin / single-anchor / hinted batch (**#268**) |
| [`LimitOrderLadderPanel.tsx`](../frontend-dapp/src/components/trade/LimitOrderLadderPanel.tsx) | Ladder UI submit + pre-submit skip/gas summary (**#268**) |
| `frontend-dapp/src/components/trade/OrderBookPanel.tsx` | Bids/Asks columns + **Load more depth** button |

## Rules of thumb

1. **Do not** reintroduce **`limit-book-shallow`** in retail **`OrderBookPanel`** — shallow is integrator/legacy only (max 20).
2. **Do not** walk the book synchronously in the UI — use **`useInfiniteQuery`** / **`fetchNextPage`** only.
3. Keep **`LIMIT_BOOK_UI_PAGE_SIZE`** in one module; prefetch ([`tradePairPrefetch.ts`](../frontend-dapp/src/utils/tradePairPrefetch.ts)) and the hook must stay aligned.
4. Best bid/ask preflight (**`limit=1`**) uses **`useTradeBestBookPrices`** — separate keys from **`limitBookPage`**.
5. When changing invalidations, update **`useLimitOrderCancelMutation`**, place/cancel/claim success handlers, and docs in **`docs/frontend.md`**.
6. After API or cursor semantics change, run **`cargo test -p indexer api_limit_book_deep`** and frontend **`OrderBookPanel.test.tsx`** + **`useLimitBookInfinite.test.tsx`** + **`limitBookInsertHint.test.ts`**.
7. **Insert hints ([#261](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/261), indexer **#267**):** prefer **`GET .../limit-book/insert-hints`** for ladder bands; until wired, **`resolveLimitInsertHintAfter`** over merged pages. Wire via **`placeLimitOrderWithAllowance(..., hintAfterOrderId)`**. Omit hint when `resolved: false` / `pagination_gap` — never guess. Invariant **L14**; [integrators.md § Insert hints & price window](../docs/integrators.md#insert-hints-price-window-gitlab-267).
8. **Ladder depth probe ([#268](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/268)):** **`LimitOrderLadderPanel`** uses **`useLimitLadderPlacementPlan`** — indexer **`price_from`/`price_to`** window + **`insert-hints`** only (no LCD). Playbook: [`AGENTS_LIMIT_ORDER_BATCH_LADDER.md`](./AGENTS_LIMIT_ORDER_BATCH_LADDER.md) §12.

## Related

- Row cancel / edit: [`AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md`](./AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md)
- Pair switch prefetch: [`AGENTS_FRONTEND_TRADE_PAIR_SWITCH.md`](./AGENTS_FRONTEND_TRADE_PAIR_SWITCH.md)
- Indexer outage copy (no false LCD fallback): [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
