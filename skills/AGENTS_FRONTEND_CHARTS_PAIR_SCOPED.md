# Agent playbook: Charts pair-scoped 24h stats + leaderboard (GitLab #666)

Audience: third-party agents touching `/charts`, `/charts/:pairAddr`, pair 24h Stats placement, Charts Leaderboard, or `GET /api/v1/traders/leaderboard?pair=`.

**Issue:** [GitLab **#666**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/666)  
**Invariants:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **Charts pair-scoped stats + leaderboard #666**), [`docs/frontend.md`](../docs/frontend.md) § [Charts pair-scoped page](../docs/frontend.md#charts-pair-scoped)  
**Related:** [#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548) Protocol / `GET /overview` USD (not a Charts census), [#565](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/565) / [#564](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/564) pair 24h content, [#553](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553) Volume USD column, [#657](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/657) Trader **global** board, [#215](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/215) outage, [#280](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/280) cache, [#653](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/653) flat tiles.

## Problem class

`/charts` used to show a **DEX-wide** overview strip (vol / USTC / trades / **Pairs** / **Tokens**) above Find pair, then pair 24h Stats **below** the candles, then a **DEX-wide** leaderboard. Retail could not tell which numbers belonged to the selected chart. Global census lives on `/protocol`. Charts answers “what is *this* pair doing?”

## Layout

| Order | Block | Scope |
|-------|--------|--------|
| 1 | Title **Charts & Analytics** | — |
| 2 | Outage banner (#215) | Indexer HTTP |
| 3 | **Find pair** | Pair picker |
| 4 | **24h Stats** (`charts-pair-24h-stats`) | Selected pair |
| 5 | `PriceChart` | Selected pair |
| 6 | TWAP Oracle | Selected pair |
| 7 | Recent Trades | Selected pair |
| 8 | Leaderboard | Selected pair |

Do **not** restore `charts-overview-*` census tiles. Do **not** keep a second vol/trades strip.

## Invariants (CS-1–CS-15)

| ID | Rule |
|----|------|
| **CS-1** | `/charts` must **not** render DEX-wide **Pairs** or **Tokens**. No `charts-overview-pairs` / `charts-overview-tokens`. |
| **CS-2** | Headline Last 24h Vol / Trades are `GET /api/v1/pairs/{addr}/stats` for `activePairAddr` only. Never bind those labels to `getOverview()`. |
| **CS-3** | Pair 24h Stats sit **immediately below Find pair** and **above** `PriceChart`. TWAP stays below the chart. |
| **CS-4** | Drop the Charts overview strip (including **USTC / USD**). Oracle / census stay on `/protocol`. Do not break `GET /api/v1/overview` (**C1–C9**, #548). |
| **CS-5** | Keep **P565 / S564 / C548** formatters: `formatIndexedVolumeUsd` for USD; `formatChartsPairTokenVolume` for token vols; unpriced + trades → `—`; idle `trade_count === 0` → `$0`. Trailing-window copy (#576). Invert (#524) does not swap stats legs. |
| **CS-6** | One chrome layer: one `shell-panel` around the pair stats grid; tiles `StatBox variant="flat"`. `python3 scripts/check_chrome_nesting.py` stays green. |
| **CS-7** | Charts Leaderboard is pair-scoped. React Query key **must** include the pair. Do not fetch until a valid pair is selected. Empty copy is pair-empty (“No traders on this pair yet”). |
| **CS-8** | Unscoped `GET /api/v1/traders/leaderboard` (no `pair`) stays the DEX-wide board: same allowlisted sorts, `limit` 1…200, 60s cache. Default API sort remains `total_volume`. Required for ops and #657. |
| **CS-9** | Pair ranks **must not** use `traders.total_volume_usd` / `traders.total_realized_pnl` / `traders.best_trade_pnl`. Volume + trades from `swap_events` for that `pair_id` (lifetime-on-this-pair). Most Profit / Most Loss from `trader_positions.realized_pnl`. **Hide Best Trade** on Charts. `?pair=` + `sort=best_trade_pnl` → **400**. |
| **CS-10** | `pair=` is an indexed pair **contract address**. Unknown → **404**. Bind with sqlx `$n`. Cache key `{sort}\|{limit}\|{pair_or_-}`. 404s are not cached as `[]`. |
| **CS-11** | Hostile `/charts/:pairAddr` stays invalid-pair notice; no leaderboard/stats fetch with that string. Symbols/addresses are text-only. No `VITE_*` in retail copy. |
| **CS-12** | #215: Charts outage probes remaining indexer queries (`pairs`, pair-stats, trades, leaderboard). Banner still hides internals. |
| **CS-13** | Do not “fix” Sybil / wash ranking. Do not change candle ingest, CG/CMC, or `/protocol`. Prefer `pair=` on the existing route. |
| **CS-14** | If a shared `TraderLeaderboard` is extracted (#657): Charts passes `pairAddress`; Trader omits it. Shared React Query key must include pair (`['leaderboard', sort, pair ?? 'global']`). |
| **CS-15** | Pair aggregation is `pair_id`-selective (`idx_swaps_pair_time` / `trader_positions`). Do not seq-scan `traders` then filter in Rust. Keep `limit` clamp. Do not bind-mount indexer cargo as root. |

## Do / don’t

- **Do** pass `getLeaderboard(sort, 20, activePairAddr)` from Charts.
- **Do** 404 unknown `pair=` **before** aggregation. Empty listed pair → `[]` 200.
- **Don’t** show Pairs / Tokens / USTC census on Charts.
- **Don’t** rank a pair board with global `traders.best_trade_pnl`.
- **Don’t** cache unknown-pair 404 as empty.
- **Don’t** implement #657 (Trader global board) in this ticket.

## Regression

```bash
make verify-issue-666
```

Vitest: `ChartsPage.test.tsx` (layout + pair board), `client.test.ts` `getLeaderboard` pair query.  
Indexer: `cargo test --test api_traders --test security -- --test-threads=1`.  
Chrome: `python3 scripts/check_chrome_nesting.py`.

## Related

- [`AGENTS_FRONTEND_CHARTS_OVERVIEW.md`](./AGENTS_FRONTEND_CHARTS_OVERVIEW.md) — **C1–C9** apply to `/protocol` + `GET /overview`, not a Charts strip (#548)
- [`AGENTS_FRONTEND_CHARTS_PAIR_STATS.md`](./AGENTS_FRONTEND_CHARTS_PAIR_STATS.md) — pair 24h content (#565 / #564); placement is **CS-3**
- [`AGENTS_FRONTEND_TRADER_VOLUME_USD.md`](./AGENTS_FRONTEND_TRADER_VOLUME_USD.md) — Volume USD column (#553); Charts is pair-scoped, unscoped API stays global
- [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md) — flat tiles (#653)
- [`AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md`](./AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md) — #215
- [`AGENTS_FRONTEND_TRAILING_WINDOW.md`](./AGENTS_FRONTEND_TRAILING_WINDOW.md) — trailing 24h copy (#576)
