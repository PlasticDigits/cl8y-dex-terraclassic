# Agent playbook: Portfolio / Trader unrealized P&L (GitLab #675)

Audience: third-party agents touching `/portfolio`, `/trader/:addr`, `TraderPositionsTable`, `TraderSummaryStats`, or `formatScaledPosition` mark-to-market.

**Issue:** [GitLab **#675**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/675)  
**Invariants:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **Portfolio unrealized P&amp;L #675**)  
**Frontend:** [`docs/frontend.md`](../docs/frontend.md) § My Portfolio  
**Human scale:** [`AGENTS_FRONTEND_PORTFOLIO_PNL.md`](./AGENTS_FRONTEND_PORTFOLIO_PNL.md) (**P551**)  
**Hub realized USD:** [`AGENTS_FRONTEND_HUB_PNL.md`](./AGENTS_FRONTEND_HUB_PNL.md) (**P560**)  
**Hub catalog:** [`AGENTS_INDEXER_HUB_USD.md`](./AGENTS_INDEXER_HUB_USD.md) (**H1–H10**)  
**Verify:** `make verify-issue-675`

## Problem class

After [#551](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/551) / [#560](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/560), `/portfolio` and `/trader/:addr` showed **realized P&L only**. One-directional flow (pure accumulation, or a full conversion into the quote token) leaves `realized_pnl = 0` even when remaining quote is up or down vs entry. Users could not tell whether open positions were currently winning or losing.

[#217](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/217) deferred mark-to-market until product agreed on a convention. This skill is that convention.

## Convention (so the number does not mislead)

An **open position** is indexer **quote exposure** (`net_position_quote`) from on-DEX swaps — **not** the wallet CW20/native balance.

| Term | Meaning |
|------|---------|
| **Cost basis** | On-DEX `total_cost_base` (base tokens spent buying quote on this factory pair). |
| **Mark** | Current **hub USD** of remaining human quote. UST1 / USTR / cUSTC from `GET /api/v1/hub-prices` (DEX pool, including wrap discount vs CEX). LUNC / cLUNC use CEX `/oracle/price/lunc`. |
| **Unrealized P&L** | `mark_usd − cost_usd`, also labeled in **human base** (`unrealized_usd / base_usd`). One-directional buys: realized stays 0; unrealized **is** the mark vs cost. |
| **No cost basis** | Remaining quote (`net ≠ 0`) with `total_cost_base = 0` **and** `avg_entry_price = 0`. The DEX never recorded a buy (bridged/minted). Show the mark and **No cost basis** — never invent a P&L. |

Closed rows (`net = 0`) are not "no cost basis": mark `$0`, unrealized `$0`.

## Invariants (P675-1–P675-8)

| ID | Rule |
|----|------|
| **P675-1** | Unrealized = current hub value of remaining quote minus on-DEX cost. Documented as **holdings vs cost**, not a bare number. Realized stays the sell-side figure from `position_tracker`. |
| **P675-2** | Mark uses the **same hub catalog as #560** (`useTraderUsdMarks` / `traderUsdMarksFromHub`). Never `quoteTokenUsd` pegs (`$1` / `2.5×`). Never CEX USTC for cUSTC when hub `custc` exists. |
| **P675-3** | Remaining quote with zero cost and zero avg → **No cost basis** (mark still shown when the quote is priced). |
| **P675-4** | Unpriced quote (GEMX, missing hub tick) → mark and unrealized **—**, not `$0`. |
| **P675-5** | `GET /api/v1/traders/{addr}/positions` JSON stays **raw**. The dApp computes mark / unrealized. Not on-chain balances; LP stays in LP overview. |
| **P675-6** | Header **Total Unrealized P&L** sums priced rows that have on-DEX basis. Unpriced and no-basis rows are omitted, not `$0`. Empty positions → `$0`. Pending / all-omitted → **—**. |
| **P675-7** | `net_position_quote = 0` → mark `$0`, unrealized `$0` (not **No cost basis**). |
| **P675-8** | This skill + invariants + `make verify-issue-675`. Coolify frontend rebuild after merge so production shows the new columns. |

## Do / don’t

- **Do** compute through [`formatScaledPosition`](../frontend-dapp/src/utils/traderPositionDisplay.ts) + `sumUnrealizedPnlUsd`.
- **Do** share [`useTraderUsdMarks`](../frontend-dapp/src/hooks/useTraderUsdMarks.ts) (`queryKey: ['indexer-hub-prices']`).
- **Don’t** treat `net_position_quote` as a wallet balance or as realized P&L.
- **Don’t** fall back to `$1` UST1 or `2.5×` USTC when hub ticks are null.
- **Don’t** add mark / unrealized fields to the positions JSON unless a later issue asks for indexer-side pair last price.

## Regression checklist

1. Frontend: `traderPositionDisplay.test.ts`, `TraderPositionsTable.test.tsx`, `TraderSummaryStats.test.tsx`, `PortfolioPage.test.tsx`
2. `make verify-issue-675`
3. Optional: `make verify-issue-551` and `make verify-issue-560` (human scale + hub realized must still pass)

## Related

- [`AGENTS_FRONTEND_PORTFOLIO.md`](./AGENTS_FRONTEND_PORTFOLIO.md) — portfolio shell / APIs
- [`AGENTS_FRONTEND_PORTFOLIO_PNL.md`](./AGENTS_FRONTEND_PORTFOLIO_PNL.md) — per-pair human scale
- [`AGENTS_FRONTEND_HUB_PNL.md`](./AGENTS_FRONTEND_HUB_PNL.md) — header realized P&amp;L USD
- [`AGENTS_INDEXER_HUB_USD.md`](./AGENTS_INDEXER_HUB_USD.md) — hub ingest
- [`AGENTS_POST_MERGE_STACK.md`](./AGENTS_POST_MERGE_STACK.md) — Coolify frontend rebuild
