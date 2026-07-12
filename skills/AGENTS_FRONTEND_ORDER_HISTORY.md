# Agent skill: Wallet order / swap history (indexer + UI)

Use when changing **trader history**, **CSV export**, **Limit orders** activity panels, or indexer routes under `/api/v1/traders/{addr}/…` related to [GitLab **#163**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/163) / [**#479**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/479) (amount columns + CSV reliability).

## Product intent

- Traders need **durable** records of **swaps**, **limit fills** (as maker), and **limit cancellations** (when `owner` is indexed), with **timestamps**, **fees** where available, **size amounts**, and **explorer links**.
- **Wallet pair history UI** must show:
  - Swaps: **Amount in** / **Amount out** from `offer_amount` / `return_amount` (same `formatNum(raw)` as public [`TradesTable`](../frontend-dapp/src/components/ui/TradesTable.tsx) — raw chain integers; do not invent a third format).
  - Limit fills: **Token0** / **Token1** from `token0_amount` / `token1_amount` (base / quote).
  - Cancellations: **no** amount columns (API has none).
- **CSV** must match the same filters as JSON (`format=csv`, optional `pair=`). Client export `limit` must be **`TRADER_HISTORY_CSV_MAX_LIMIT` (200)** — the indexer clamp — never advertise a larger export. **Formula injection:** `csv_escape_cell` in [`text_csv.rs`](../indexer/src/api/text_csv.rs) prefixes cells starting with `=`, `+`, `-`, or `@` with `'` before RFC 4180 quoting (SEC-F12 / [#432](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/432)); unit tests `csv_escape_cell_neutralizes_*` and `trader_swaps_csv_neutralizes_formula_in_offer_asset`.
- **CSV is HTTP-only** — no Keplr / wallet signature. Failures must surface an **inline error** (`data-testid="wallet-history-csv-error"`); never silent no-op. `fetchTraderHistoryCsv` retries once on network/timeout like `fetchJson`.

## Indexer (source of truth)

| Route | Purpose |
|-------|---------|
| `GET /api/v1/traders/{addr}/trades` | `swap_events` where `sender = addr`. Optional `pair=` (pair contract). Optional `format=csv`. JSON/CSV include `offer_amount`, `return_amount`; optional `commission_amount`, `spread_amount` when indexed. |
| `GET /api/v1/traders/{addr}/limit-fills` | `limit_order_fills` where `maker = addr`. Optional `pair=`. Optional `format=csv`. Includes `token0_amount`, `token1_amount`. |
| `GET /api/v1/traders/{addr}/limit-cancellations` | `limit_order_cancellations` where `owner = addr`. Optional `pair=`. Optional `format=csv`. |

Invariants and caps (`limit` ≤ 200): [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (trader history row). Parser / DB details: [`docs/limit-orders.md`](../docs/limit-orders.md). Frontend product copy: [`docs/frontend.md` § Wallet swap and limit history](../docs/frontend.md#wallet-swap-limit-history).

## Frontend

- **Limits page** ([`LimitOrdersPage.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.tsx)): [`WalletIndexerHistoryPanel`](../frontend-dapp/src/components/trade/WalletIndexerHistoryPanel.tsx) — all three sections; invalidates query key `wallet-indexer-history` after place and after any cancel via [`useLimitOrderCancelMutation`](../frontend-dapp/src/hooks/useLimitOrderCancelMutation.ts) (book row **×**, **Cancel all mine**, or manual cancel — [GitLab **#162**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/162)).
- **Trade page** ([`TradePage.tsx`](../frontend-dapp/src/pages/TradePage.tsx)): same panel with `sections={['swaps']}` only.
- Client helpers: [`client.ts`](../frontend-dapp/src/services/indexer/client.ts) — `getTraderTrades`, `getTraderLimitFills`, `getTraderLimitCancellations`, `fetchTraderHistoryCsv`, `downloadTextAsFile`, `TRADER_HISTORY_CSV_MAX_LIMIT`.
- Mobile: keep `#352` horizontal scroll (`data-testid="wallet-history-table-scroll"`); amount columns must remain reachable via scroll — do not hide amounts behind a breakpoint without an accessible alternative.

## UX / a11y

- Tables use **semantic** `<table>`; tx column uses [`getExplorerTxUrl`](../frontend-dapp/src/utils/terraExplorer.ts) when configured (address links: [`getExplorerAddressUrl`](../frontend-dapp/src/utils/terraExplorer.ts) — [`AGENTS_FRONTEND_TERRA_EXPLORER.md`](./AGENTS_FRONTEND_TERRA_EXPLORER.md)).
- Empty states explain **pair scope** (history is filtered to the **selected pair**).
- CSV button: `data-testid="wallet-history-download-csv"`; failure: `role="alert"` + `wallet-history-csv-error`.

## Do not

- Change indexer schema for amounts — reuse existing fields.
- Weaken CSV formula-injection escaping (#432).
- Expand this panel to global (all-pair) wallet history in a drive-by.
- Require wallet signing for CSV.
- Request CSV `limit` > 200 or imply the client exports 500 rows.

## Related skills

- [`AGENTS_FRONTEND_TERRA_EXPLORER.md`](./AGENTS_FRONTEND_TERRA_EXPLORER.md) — tx / address explorer URL matrix.
- [`AGENTS_LOCALNET_TRADING_SWARM.md`](./AGENTS_LOCALNET_TRADING_SWARM.md) — LocalTerra smoke flows.
- [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md) — trade workspace layout; post-place **View order** / **Place another** on `/trade` ([GitLab **#161**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/161)).
