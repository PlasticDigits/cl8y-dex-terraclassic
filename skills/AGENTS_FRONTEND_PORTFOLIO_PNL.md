# Agent playbook: Portfolio / Trader P&L human scale (GitLab #551)

Audience: third-party agents touching `/portfolio`, `/trader/:addr`, `GET /api/v1/traders/{addr}/positions`, or trader summary totals.

**Issue:** [GitLab **#551**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/551)  
**Invariants table:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **Trader positions human scale #551**)  
**Frontend:** [`docs/frontend.md`](../docs/frontend.md) § My Portfolio  
**Related skill:** [`AGENTS_FRONTEND_PORTFOLIO.md`](./AGENTS_FRONTEND_PORTFOLIO.md)

## Problem class

Same family as [#522](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522) / [#534](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534) / [#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548): indexer stores **raw integers**, the dApp passed them to `formatNum`. A 6-dec quote printed **38.29M** for 38.29 tokens. Header **Total realized P&L / volume / fees** summed those raw numbers across pairs with **different quote (and base) tokens**, so a public trader page ranked whoever traded the smallest-unit token.

## Stored units (`position_tracker`)

For factory `asset_0` = **base**, `asset_1` = **quote**:

| Field | Unit |
|-------|------|
| `net_position_quote` | raw **quote** |
| `total_cost_base` | raw **base** |
| `realized_pnl` | raw **base** (`(exit − avg) × quote_sold`) |
| `avg_entry_price` | raw base / raw quote (not human, not USD) |
| `traders.total_*` | **mixed** raw sums — not a unit |

JSON **keeps raw strings**. The dApp scales. Columns are **`NUMERIC(78, 18)`** so 18-dec raw amounts fit ([#676](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/676) **P676-1**).

## Invariants (P551-1–P551-6)

| ID | Rule |
|----|------|
| **P551-1** | `GET /api/v1/traders/{addr}/positions` keeps raw NUMERIC and adds `asset_0_decimals` / `asset_1_decimals` plus optional `asset_*_denom`. **Do not** match decimals by symbol alone (same class as [#557](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/557) **A1**). Missing decimals in the UI → **—**, never assume 6. |
| **P551-2** | **Net position** = human quote `raw ÷ 10^decimals_quote` + **quote symbol**. Never `formatNum(raw)`. |
| **P551-3** | **Cost basis** and **realized P&L** = human **base** `raw ÷ 10^decimals_base` + **base symbol**. P&L is **not** in quote units. |
| **P551-4** | **Avg entry** = `raw_avg × 10^(decimals_quote − decimals_base)` (human base per 1 human quote). Display with `formatPairPrice` (no compact `T`) and label `{base} / {quote}`. Inverse of tape human quote-per-base (**P522-1**). |
| **P551-5** | **Do not** render `traders.total_realized_pnl`, `total_volume`, `total_fees_paid`, `best_trade_pnl`, or `worst_trade_pnl` via `formatNum`. Fees / best / worst → **—** (mixed tokens). **Total Volume (USD)** is [#553](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553) (`total_volume_usd`). Header realized P&L = **USD sum** of per-pair P&L via **hub_prices** on UST1/USTR/cUSTC ([#560](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/560), **P560-1**). Unpriced pairs are omitted, not `$0`. Empty positions → `$0`. |
| **P551-6** | Tape Amount in / out / Price is **[#557](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/557)**. Profile/leaderboard **volume USD** is **[#553](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553)**. DEX hub USD catalog (replacing UST1=`$1` / USTR=`2.5×` USTC) is **[#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556)**. Header P&L USD wiring is **[#560](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/560)**. |

## Do / don’t

- **Do** scale through [`traderPositionDisplay.ts`](../frontend-dapp/src/utils/traderPositionDisplay.ts) (`formatScaledPosition`, `sumRealizedPnlUsd`, `traderUsdMarksFromHub`).
- **Do** label the token on every per-pair amount.
- **Don’t** add `traders.total_realized_pnl` across pairs and call it a total.
- **Don’t** treat avg entry as tape `price` or as USD.
- **Don’t** invent USD for unknown base tokens (`$1` UST1 / `2.5×` USTR pegs are banned — [#560](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/560)).

## Regression checklist

1. `cd indexer && cargo test --lib position_tracker -- --quiet`
2. `cd indexer && cargo test --test api_traders get_trader_positions_returns_rows -- --test-threads=1 --quiet`
3. Frontend: `traderPositionDisplay.test.ts`, `TraderPositionsTable.test.tsx`, `TraderSummaryStats.test.tsx`, `PortfolioPage.test.tsx`
4. `make verify-issue-551`
5. Header USD from hub: `make verify-issue-560` ([`AGENTS_FRONTEND_HUB_PNL.md`](./AGENTS_FRONTEND_HUB_PNL.md))
6. 18-dec persist / `trade_count`: `make verify-issue-676` (**P676**)

## Related

- [`AGENTS_FRONTEND_PORTFOLIO.md`](./AGENTS_FRONTEND_PORTFOLIO.md) — portfolio shell / APIs
- [`AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS.md`](./AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS.md) — `/portfolio` hides gem P&amp;L by default (**P674**, [#674](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/674)); `make verify-issue-674`
- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — P522-Q catalog used for tape USD
- [`AGENTS_FRONTEND_HUB_PNL.md`](./AGENTS_FRONTEND_HUB_PNL.md) — header realized P&amp;L USD from hub_prices (**P560-1–P560-6**, [#560](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/560))
- [`AGENTS_FRONTEND_PORTFOLIO_UNREALIZED.md`](./AGENTS_FRONTEND_PORTFOLIO_UNREALIZED.md) — mark-to-market + unrealized vs on-DEX cost (**P675**, [#675](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/675)); `make verify-issue-675`
- [`AGENTS_INDEXER_EXTERNAL_ORACLE.md`](./AGENTS_INDEXER_EXTERNAL_ORACLE.md) — USTC / LUNC feeds
- [`AGENTS_FRONTEND_CHARTS_OVERVIEW.md`](./AGENTS_FRONTEND_CHARTS_OVERVIEW.md) — do not `formatNum` mixed raw volume (#548); leaderboard volume USD is #553
- [`AGENTS_FRONTEND_TAPE_AMOUNTS.md`](./AGENTS_FRONTEND_TAPE_AMOUNTS.md) — tape amounts vs P&amp;L (#557 vs #551)
- [`AGENTS_INDEXER_TRADER_POSITIONS_DECIMALS.md`](./AGENTS_INDEXER_TRADER_POSITIONS_DECIMALS.md) — 18-dec storage + `/positions` vs `/trades` count (**P676-1–P676-8**, [#676](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/676))
