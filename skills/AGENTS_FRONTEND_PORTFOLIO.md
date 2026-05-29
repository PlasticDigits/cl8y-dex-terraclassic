# Agent skill: My Portfolio (`/portfolio`)

Use when changing the **wallet-centric portfolio** surface, **trader positions** UI, or nav to **`/portfolio`** ([GitLab **#212**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/212)).

## Product intent

- Traders need a **first-class** place for **their** indexed exposure without hunting **More → Trader → My Profile**.
- Data is **indexer-only** and **public** (anyone can query any address); the route defaults to the **connected wallet** only.

## APIs (no auth)

| Route | Purpose |
|-------|---------|
| `GET /api/v1/traders/{addr}` | Profile / tier / aggregate stats — **404** if never indexed as trader |
| `GET /api/v1/traders/{addr}/positions` | Open quote exposure per pair — **`[]`** when flat |
| `GET /api/v1/traders/{addr}/trades` | Recent swaps (portfolio uses `limit=100`, no `pair` filter) |

Position accounting: [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (trader open positions row). Parser: [`indexer/src/indexer/position_tracker.rs`](../indexer/src/indexer/position_tracker.rs).

## Frontend map

| File | Role |
|------|------|
| [`PortfolioPage.tsx`](../frontend-dapp/src/pages/PortfolioPage.tsx) | Wallet-gated shell, summary, positions, recent activity |
| [`TraderSummaryStats.tsx`](../frontend-dapp/src/components/trader/TraderSummaryStats.tsx) | Shared profile + realized P&amp;L cards |
| [`TraderPositionsTable.tsx`](../frontend-dapp/src/components/trader/TraderPositionsTable.tsx) | Positions table; pair links → `/trade/{pairAddr}` |
| [`TraderPage.tsx`](../frontend-dapp/src/pages/TraderPage.tsx) | Public lookup; reuses shared components |
| [`navItems.ts`](../frontend-dapp/src/components/common/navItems.ts) | `Portfolio` in `PRIMARY_NAV_ITEMS` |
| [`App.tsx`](../frontend-dapp/src/App.tsx) | `/portfolio`, `/my-portfolio` redirect |

## Rules of thumb

1. **Do not** treat `net_position_quote` as wallet balance or unrealized P&amp;L — copy must say **indexer quote exposure · realized P&amp;L**.
2. **Do not** merge LP balances into the positions table — LP is on **`/pool`**.
3. **Do not** accept arbitrary `?addr=` on portfolio without `isValidTerraAddress` and explicit product approval; default is **connected wallet only**.
4. **404 profile ≠ empty portfolio** — still fetch positions/trades when wallet is connected.
5. Keep **outage** handling aligned with [`TraderPage`](../frontend-dapp/src/pages/TraderPage.tsx) (`isIndexerUnavailableError`, `MarketDataServiceOutageBanner`).
6. Extend **Vitest** + **Playwright** when changing empty states, nav, or API wiring.

## Tests

- [`PortfolioPage.test.tsx`](../frontend-dapp/src/pages/PortfolioPage.test.tsx)
- [`client.test.ts`](../frontend-dapp/src/services/indexer/__tests__/client.test.ts) — `getTraderPositions` URL
- [`e2e/portfolio.spec.ts`](../frontend-dapp/e2e/portfolio.spec.ts)

## Related

- [`AGENTS_FRONTEND_SHELL_NAV.md`](./AGENTS_FRONTEND_SHELL_NAV.md) — primary nav / `navItems.ts`
- [`AGENTS_FRONTEND_ORDER_HISTORY.md`](./AGENTS_FRONTEND_ORDER_HISTORY.md) — pair-scoped history on `/limits` and `/trade`
- [`AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md`](./AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md) — indexer outage banners
- [`docs/frontend.md` § My Portfolio](../docs/frontend.md#my-portfolio)
