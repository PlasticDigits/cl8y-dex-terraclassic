# Agent playbook: Portfolio / Trader realized P&L USD from hub prices (GitLab #560)

Audience: third-party agents touching `/portfolio`, `/trader/:addr`, `TraderSummaryStats`, or `sumRealizedPnlUsd`.

**Issue:** [GitLab **#560**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/560)  
**Invariants:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **Portfolio hub P&L USD #560**)  
**Frontend:** [`docs/frontend.md`](../docs/frontend.md) § My Portfolio  
**Human scale (tokens):** [`AGENTS_FRONTEND_PORTFOLIO_PNL.md`](./AGENTS_FRONTEND_PORTFOLIO_PNL.md) (**P551**)  
**Hub catalog:** [`AGENTS_INDEXER_HUB_USD.md`](./AGENTS_INDEXER_HUB_USD.md) (**H1–H10**)  
**Verify:** `make verify-issue-560`

## Problem class

After [#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556), Charts/Trade `price_usd` uses **hub_prices** (UST1 from deepest UST1/cUSTC pool; USTR is **market-priced**, not `2.5 ×` USTC). Header **Total realized P&L** on `/portfolio` and `/trader/:addr` still used frontend `quoteTokenUsd` (**UST1 = `$1`**, **USTR = `2.5 ×` USTC**). That disagreed with H1–H3.

Per-pair human token amounts from [#551](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/551) stay. Only the **cross-pair USD header** changes.

Tape Amount in/out/Price is **[#557](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/557)** (out of scope).

## Invariants (P560-1–P560-6)

| ID | Rule |
|----|------|
| **P560-1** | Header realized P&L USD reads **`GET /api/v1/hub-prices`** (same query key as Protocol DEX card) for **UST1 / USTR / cUSTC**. Never `quoteTokenUsd` pegs (`$1` / `2.5×`). |
| **P560-2** | Unpriced hub ticks are **omitted**, not `$0`. Unknown bases (GEMX, …) omitted. Empty positions → `$0`. Pending load → **—**. |
| **P560-3** | `GET /api/v1/oracle/price/ustr` (and `ust1`, `custc`) stay **400**. Do **not** call `getOraclePrice('ustr')` from trader/portfolio. LUNC/cLUNC/`uluna` still use CEX `/oracle/price/lunc`. Native USTC/`uusd` may use hub `custc` or CEX `ustc`. |
| **P560-4** | Best / Worst header is **—** (`PnlValue` null), not `N/A` (**P551-5**). Fees stay **—**. |
| **P560-5** | Positions whose pair exists but an **asset row is missing** still serialize (symbol **—**, decimals omitted). UI shows **—**; do not drop the row. |
| **P560-6** | This skill + invariants + `make verify-issue-560`. Coolify frontend rebuild (`npm ci`) after merge so production matches hub marks. |

## Do / don’t

- **Do** convert through [`traderUsdMarksFromHub`](../frontend-dapp/src/utils/traderPositionDisplay.ts) + `sumRealizedPnlUsd`.
- **Do** share `queryKey: ['indexer-hub-prices']` with [`useProtocolHubPricesQuery`](../frontend-dapp/src/components/protocol/useProtocolHubPricesQuery.ts).
- **Don’t** fall back to `$1` or `USTR_PER_USTC` when hub UST1/USTR is null.
- **Don’t** treat hub USD as settlement, TWAP, or the `/ust1` window rate.
- **Don’t** `formatNum` `traders.best_trade_pnl` / `worst_trade_pnl` on this header.

## Regression checklist

1. Frontend: `traderPositionDisplay.test.ts`, `TraderSummaryStats.test.tsx`, `TraderPositionsTable.test.tsx`, `PortfolioPage.test.tsx`
2. Indexer: `cargo test --lib traders::position_map_tests -- --quiet` and `cargo test --test api_traders get_trader_positions_returns_rows -- --test-threads=1 --quiet`
3. `make verify-issue-560`
4. Optional: `make verify-issue-551` (human scale must still pass)

## Related

- [`AGENTS_FRONTEND_PORTFOLIO_PNL.md`](./AGENTS_FRONTEND_PORTFOLIO_PNL.md) — per-pair human scale
- [`AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS.md`](./AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS.md) — `/portfolio` hides gem rows from this USD sum by default ([#674](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/674))
- [`AGENTS_INDEXER_HUB_USD.md`](./AGENTS_INDEXER_HUB_USD.md) — hub ingest + Protocol card
- [`AGENTS_FRONTEND_TRADER_VOLUME_USD.md`](./AGENTS_FRONTEND_TRADER_VOLUME_USD.md) — header volume USD (#553)
- [`AGENTS_INDEXER_EXTERNAL_ORACLE.md`](./AGENTS_INDEXER_EXTERNAL_ORACLE.md) — CEX catalog stays 3 tickers
- [`AGENTS_FRONTEND_PORTFOLIO_UNREALIZED.md`](./AGENTS_FRONTEND_PORTFOLIO_UNREALIZED.md) — mark + unrealized uses the same hub marks (**P675**, [#675](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/675))
- [`AGENTS_POST_MERGE_STACK.md`](./AGENTS_POST_MERGE_STACK.md) — Coolify frontend rebuild so production matches hub marks ([#573](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/573))
