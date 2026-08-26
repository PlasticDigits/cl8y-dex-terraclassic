# Agent playbook: Trader page global leaderboard (GitLab #657)

Audience: third-party agents editing `/trader`, `/trader/:address`, Charts **Leaderboard**, or `GET /api/v1/traders/leaderboard`.

**Issue:** [GitLab **#657**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/657)  
**Invariants:** [`docs/frontend.md` § Trader profile](../docs/frontend.md#trader-profile-indexer) (**TL-1–TL-10**); USD column is still [**T553-1–T553-6**](./AGENTS_FRONTEND_TRADER_VOLUME_USD.md)  
**Verify:** `make verify-issue-657`

## Problem class

**Trader** in More was a paste-an-address dead-end. The only retail rank table lived at the bottom of `/charts`. Duplicate markup would fork **T553** (raw `total_volume` printing `10,000,000T`). Extract one component; mount it on both surfaces.

`/portfolio` is wallet-home ([#212](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/212)) — **do not** add the board there.

## Do / don’t

- **Do** use [`TraderLeaderboard`](../frontend-dapp/src/components/trader/TraderLeaderboard.tsx) as the last sibling in `TraderPage` (outside `{trader && (…)}`).
- **Do** share React Query key `['leaderboard', sort, pair ?? 'global']`. Global: `getLeaderboard(sort, 20)`. Charts: `getLeaderboard(sort, 20, pair)`.
- **Do** format Volume with [`formatIndexedVolumeUsd`](../frontend-dapp/src/utils/chartsOverviewStats.ts) on `total_volume_usd`. Unpriced → `—`. Idle (`total_trades === 0`) → `$0`.
- **Do** keep heading **Leaderboard**. Global has four tabs (Volume / Best Trade / Most Profit / Most Loss). Charts pair board hides **Best Trade** ([#666](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/666) **CS-9**).
- **Do** skip rows that fail `isValidTerraAddress`. Links are `Link` to `/trader/${encodeURIComponent(addr)}`. Rank `#` is displayed server order — do not re-sort.
- **Don’t** nest the board in `app-footer-shell`, mobile nav, or Trade History’s panel.
- **Don’t** add `/portfolio` ranks, rolling 24h/7d/30d columns, `total_fees_paid`, or “you are rank N” when the wallet is not in the top 20.
- **Don’t** change the unscoped indexer API, `#280` TTL, default API sort (`total_volume` raw), or `limit` clamp. Pair-scoped Charts ranks are [#666](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/666) — pass `pairAddress`; do not fork USD math.
- **Don’t** claim anti-Sybil (POS-02). Optional docs note only — no retail lecture ([#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/489)).

## Invariants (TL-1–TL-10)

| ID | Rule |
|----|------|
| **TL-1** | Last section of Trader page content: search → profile/empty/error/outage → (when loaded) stats → positions → trade history → **leaderboard** → Layout footer. |
| **TL-2** | Board on **both** `/trader` and `/trader/:address`, including loading, 404, and profile outage. Own loading / empty / `RetryError`. |
| **TL-3** | Default `total_volume_usd`, `limit=20`, `refetchInterval: 30_000`, shared query key including pair (`global` when omitted). Global board has four tabs; Charts pair board hides Best Trade (**CS-9**). |
| **TL-4** | **T553-1 / T553-5:** never display raw `total_volume` as Volume. |
| **TL-5** | One chrome layer: one `shell-panel-strong`. No nested `shell-panel*` / per-row `card-glass`. `python3 scripts/check_chrome_nesting.py` stays green. |
| **TL-6** | Heading **Leaderboard**. No Sybil lecture, no “see also Charts”, no `VITE_*` / host in retail copy. |
| **TL-7** | No indexer / contract change. |
| **TL-8** | `/portfolio` unchanged. Links stay `/trader/{addr}`. Address text via `shortenAddress` (no `dangerouslySetInnerHTML`). |
| **TL-9** | Optional: `aria-current="page"` on the current `:address` row when it is in the top 20. Do not invent rank N. |
| **TL-10** | `#126` / `#215` / `#177` stay. Board tab state may reset on address remount. |

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/components/trader/TraderLeaderboard.tsx` | Tabs + query + table + empty/error |
| `frontend-dapp/src/components/trader/traderLeaderboard.ts` | Tab keys, row filter, P&L field map |
| `frontend-dapp/src/pages/TraderPage.tsx` | Mount as last child; `highlightAddress={traderAddr}` |
| `frontend-dapp/src/pages/ChartsPage.tsx` | Thin wrapper after Recent Trades |
| `frontend-dapp/src/pages/PortfolioPage.tsx` | Must **not** import the board |

## Regression

```bash
make verify-issue-657
```

Also keep `make verify-issue-553` and `make verify-issue-653` green. Vitest: `TraderLeaderboard.test.tsx`, `TraderPage.test.tsx`, `ChartsPage.test.tsx` #553, `PortfolioPage.test.tsx` (no board). Playwright: `e2e/trader-page.spec.ts` (5 workers, no `e2e-tx`) when LocalTerra is up — dedicated Vite `:3173` (indexer CORS).

## Related

- [`AGENTS_FRONTEND_TRADER_VOLUME_USD.md`](./AGENTS_FRONTEND_TRADER_VOLUME_USD.md) — USD column / sort (**T553**)
- [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md) — one chrome layer (**C653**)
- [`AGENTS_FRONTEND_PORTFOLIO.md`](./AGENTS_FRONTEND_PORTFOLIO.md) — wallet-home; no board
- [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — no Sybil lecture
- [`AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md`](./AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md) — profile outage vs board `RetryError`
- [#666](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/666) — Charts passes `pairAddress`; hide Best Trade; empty copy is pair-empty
- [#665](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/665) — trader share-link (separate)
- Post-merge leftover: [#673](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/673) / `make verify-issue-673` / [`AGENTS_POST_MERGE_OPS_673.md`](./AGENTS_POST_MERGE_OPS_673.md)
