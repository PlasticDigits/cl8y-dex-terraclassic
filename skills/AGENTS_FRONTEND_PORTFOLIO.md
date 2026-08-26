# Agent skill: My Portfolio (`/portfolio`)

Use when changing the **wallet-centric portfolio** surface, **trader positions** UI, or nav to **`/portfolio`** ([GitLab **#212**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/212), phase 2 [#217](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/217)).

## Product intent

- Traders need a **first-class** place for **their** indexed exposure without hunting **More → Trader → My Profile**.
- Data is **indexer-only** (limits, positions, swaps) plus **LCD** for LP CW20 balances; routes are **public** (anyone can query any address); the route defaults to the **connected wallet** only.

## APIs (no auth)

| Route | Purpose |
|-------|---------|
| `GET /api/v1/traders/{addr}` | Profile / tier / aggregate stats — **404** if never indexed as trader |
| `GET /api/v1/traders/{addr}/positions` | Open quote exposure per pair — **`[]`** when flat |
| `GET /api/v1/traders/{addr}/limit-placements` | Wallet-wide open limits (`owner`); same lifecycle/`status` as pair route — **`limit` ≤ 200** ([#217](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/217)) |
| `GET /api/v1/traders/{addr}/trades` | Recent swaps (portfolio uses `limit=100`, no `pair` filter) |
| `GET /api/v1/pairs` + LCD `balance` | LP overview: capped fan-out (`PORTFOLIO_LP_MAX_PAIRS` = 50, concurrency 5) — see [`portfolioFanOut.ts`](../frontend-dapp/src/utils/portfolioFanOut.ts) |

Position accounting: [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (trader open positions row, trader limit-placements row, **Trader positions human scale #551**). Parser: [`indexer/src/indexer/position_tracker.rs`](../indexer/src/indexer/position_tracker.rs).

**Out of scope on portfolio:** unrealized / mark-to-market P&amp;L — not in positions API; track separately if product adds spot/oracle ([#217](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/217)). **Raw vs human P&amp;L / cost / avg entry** is [#551](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/551) — [`AGENTS_FRONTEND_PORTFOLIO_PNL.md`](./AGENTS_FRONTEND_PORTFOLIO_PNL.md).

## Frontend map

| File | Role |
|------|------|
| [`PortfolioPage.tsx`](../frontend-dapp/src/pages/PortfolioPage.tsx) | Wallet-gated shell, summary, positions, open limits, LP overview, recent activity |
| [`PortfolioOpenLimitsSection.tsx`](../frontend-dapp/src/components/portfolio/PortfolioOpenLimitsSection.tsx) | Wallet-wide limits table; deep-links to `/trade/{pair}` and `/limits` |
| [`PortfolioLpOverviewSection.tsx`](../frontend-dapp/src/components/portfolio/PortfolioLpOverviewSection.tsx) | LP balances; distinct copy from trader positions |
| [`usePortfolioLpBalances.ts`](../frontend-dapp/src/hooks/usePortfolioLpBalances.ts) | Indexer pair list + capped LCD fan-out |
| [`TraderSummaryStats.tsx`](../frontend-dapp/src/components/trader/TraderSummaryStats.tsx) | Shared profile + realized P&amp;L cards. **Total Volume (USD)** uses `total_volume_usd` ([#553](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553)). Header identity is 4/6 + blockie ([#656](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/656)). |
| [`TraderPositionsTable.tsx`](../frontend-dapp/src/components/trader/TraderPositionsTable.tsx) | Positions table; pair links → `/trade/{pairAddr}` |
| [`TraderPage.tsx`](../frontend-dapp/src/pages/TraderPage.tsx) | Public lookup; reuses shared components |
| [`navItems.ts`](../frontend-dapp/src/components/common/navItems.ts) | `Portfolio` in `PRIMARY_NAV_ITEMS` |
| [`App.tsx`](../frontend-dapp/src/App.tsx) | `/portfolio`, `/my-portfolio` redirect |

## Rules of thumb

1. **Do not** treat `net_position_quote` as wallet balance or unrealized P&amp;L — copy must say **indexer quote exposure · realized P&amp;L**.
2. **Do not** `formatNum` raw position or trader-total fields — scale with decimals and label the token, or show **—** for mixed-unit totals ([#551](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/551), **P551-1–P551-5**).
3. **Do not** merge LP balances into the positions table — LP has its own **LP overview** section; pool actions stay on **`/pool`**.
4. **Do not** accept arbitrary `?addr=` on portfolio without `isValidTerraAddress` and explicit product approval; default is **connected wallet only**.
5. **404 profile ≠ empty portfolio** — still fetch positions/trades/limits when wallet is connected.
6. Keep **outage** handling aligned with [`TraderPage`](../frontend-dapp/src/pages/TraderPage.tsx) (`isIndexerUnavailableError`, `MarketDataServiceOutageBanner`). LP LCD failures are section-local (`RetryError`), not necessarily full-page outage.
7. **No unbounded N+1** — use `getTraderLimitPlacements` (preferred) or document caps for any client fan-out ([`portfolioFanOut.ts`](../frontend-dapp/src/utils/portfolioFanOut.ts)). LP fan-out skips invalid bech32 `lp_token` rows and per-pair LCD failures so one bad indexer pair does not fail the whole section ([`usePortfolioLpBalances.ts`](../frontend-dapp/src/hooks/usePortfolioLpBalances.ts)).
8. **No signing** on portfolio — cancel/claim stays on `/trade` / `/limits`.
9. **Share** (when present) emits canonical `/trader/{wallet}`, never `/portfolio` ([#665](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/665) — [`AGENTS_FRONTEND_SHARE_LINK.md`](./AGENTS_FRONTEND_SHARE_LINK.md)).
10. Extend **Vitest** + **Playwright** when changing empty states, nav, or API wiring.

## Tests

- [`PortfolioPage.test.tsx`](../frontend-dapp/src/pages/PortfolioPage.test.tsx)
- [`traderPositionDisplay.test.ts`](../frontend-dapp/src/utils/__tests__/traderPositionDisplay.test.ts) — human scale + USD totals (#551)
- [`TraderPositionsTable.test.tsx`](../frontend-dapp/src/components/trader/TraderPositionsTable.test.tsx)
- [`TraderSummaryStats.test.tsx`](../frontend-dapp/src/components/trader/TraderSummaryStats.test.tsx)
- Scale helper: [`traderPositionDisplay.ts`](../frontend-dapp/src/utils/traderPositionDisplay.ts) (`formatScaledPosition`)
- [`client.test.ts`](../frontend-dapp/src/services/indexer/__tests__/client.test.ts) — `getTraderPositions`, `getTraderLimitPlacements`
- [`e2e/portfolio.spec.ts`](../frontend-dapp/e2e/portfolio.spec.ts)
- Indexer: [`api_traders.rs`](../indexer/tests/api_traders.rs) (`get_trader_limit_placements_returns_owner_rows`)

## Related

- [`AGENTS_FRONTEND_SHELL_NAV.md`](./AGENTS_FRONTEND_SHELL_NAV.md) — primary nav / `navItems.ts`
- [`AGENTS_FRONTEND_PORTFOLIO_PNL.md`](./AGENTS_FRONTEND_PORTFOLIO_PNL.md) — human-scale P&amp;L / cost / avg entry (**P551-1–P551-6**, [#551](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/551)); `make verify-issue-551`
- [`AGENTS_FRONTEND_HUB_PNL.md`](./AGENTS_FRONTEND_HUB_PNL.md) — header realized P&amp;L USD from hub_prices (**P560**, [#560](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/560)); `make verify-issue-560`
- [`AGENTS_FRONTEND_ORDER_HISTORY.md`](./AGENTS_FRONTEND_ORDER_HISTORY.md) — pair-scoped history on `/limits` and `/trade`
- [`AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](./AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md) — lifecycle on limit-placements
- [`AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md`](./AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md) — indexer outage banners
- [`AGENTS_FRONTEND_TRADER_VOLUME_USD.md`](./AGENTS_FRONTEND_TRADER_VOLUME_USD.md) — Total Volume (USD) on this shared header ([#553](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553))
- [`AGENTS_FRONTEND_SHARE_LINK.md`](./AGENTS_FRONTEND_SHARE_LINK.md) — public `/trader/{wallet}` Share, never `/portfolio` ([#665](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/665))
- [`AGENTS_FRONTEND_TRADER_LEADERBOARD.md`](./AGENTS_FRONTEND_TRADER_LEADERBOARD.md) — global board is **`/trader` only**, not `/portfolio` (**TL-8**, [#657](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/657))
- [`docs/frontend.md` § My Portfolio](../docs/frontend.md#my-portfolio)
