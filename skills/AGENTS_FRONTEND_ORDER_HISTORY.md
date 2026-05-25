# Agent skill: Wallet order / swap history (indexer + UI)

Use when changing **trader history**, **CSV export**, **Limit orders** activity panels, or indexer routes under `/api/v1/traders/{addr}/…` related to [GitLab **#163**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/163).

## Product intent

- Traders need **durable** records of **swaps**, **limit fills** (as maker), and **limit cancellations** (when `owner` is indexed), with **timestamps**, **fees** where available, and **explorer links**.
- **CSV** must match the same filters as JSON (`format=csv`).

## Indexer (source of truth)

| Route | Purpose |
|-------|---------|
| `GET /api/v1/traders/{addr}/trades` | `swap_events` where `sender = addr`. Optional `pair=` (pair contract). Optional `format=csv`. JSON includes optional `commission_amount`, `spread_amount` when indexed. |
| `GET /api/v1/traders/{addr}/limit-fills` | `limit_order_fills` where `maker = addr`. Optional `pair=`. Optional `format=csv`. |
| `GET /api/v1/traders/{addr}/limit-cancellations` | `limit_order_cancellations` where `owner = addr`. Optional `pair=`. Optional `format=csv`. |

Invariants and caps: [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (trader history row). Parser / DB details: [`docs/limit-orders.md`](../docs/limit-orders.md).

## Frontend

- **Limits page** ([`LimitOrdersPage.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.tsx)): [`WalletIndexerHistoryPanel`](../frontend-dapp/src/components/trade/WalletIndexerHistoryPanel.tsx) — all three sections; invalidates query key `wallet-indexer-history` after place and after any cancel via [`useLimitOrderCancelMutation`](../frontend-dapp/src/hooks/useLimitOrderCancelMutation.ts) (book row **×**, **Cancel all mine**, or manual cancel — [GitLab **#162**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/162)).
- **Trade page** ([`TradePage.tsx`](../frontend-dapp/src/pages/TradePage.tsx)): same panel with `sections={['swaps']}` only.
- Client helpers: [`client.ts`](../frontend-dapp/src/services/indexer/client.ts) — `getTraderTrades`, `getTraderLimitFills`, `getTraderLimitCancellations`, `fetchTraderHistoryCsv`, `downloadTextAsFile`.

## UX / a11y

- Tables use **semantic** `<table>`; tx column uses [`getExplorerTxUrl`](../frontend-dapp/src/utils/terraExplorer.ts) when configured (address links: [`getExplorerAddressUrl`](../frontend-dapp/src/utils/terraExplorer.ts) — [`AGENTS_FRONTEND_TERRA_EXPLORER.md`](./AGENTS_FRONTEND_TERRA_EXPLORER.md)).
- Empty states explain **pair scope** (history is filtered to the **selected pair**).

## Related skills

- [`AGENTS_FRONTEND_TERRA_EXPLORER.md`](./AGENTS_FRONTEND_TERRA_EXPLORER.md) — tx / address explorer URL matrix.
- [`AGENTS_LOCALNET_TRADING_SWARM.md`](./AGENTS_LOCALNET_TRADING_SWARM.md) — LocalTerra smoke flows.
- [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md) — trade workspace layout; post-place **View order** / **Place another** on `/trade` ([GitLab **#161**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/161)).
