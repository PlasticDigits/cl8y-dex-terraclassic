# Agent playbook: indexer API4 per-request caps (GitLab #694)

Audience: third-party agents changing indexer `/gt/events`, `route/solve/progress`, or `compliance/blacklist-check`.

**Issue:** [GitLab **#694**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/694)  
**Audit:** `INTERNAL_GROK46_1787908099` findings **RE-01**, **RE-02**, **RE-03** (OWASP API4).  
**Companions:** [`AGENTS_INDEXER_API_LCD_SECURITY.md`](./AGENTS_INDEXER_API_LCD_SECURITY.md) (H6/H7 request-count governors), [`AGENTS_INDEXER_GT_EVENT_RESERVES.md`](./AGENTS_INDEXER_GT_EVENT_RESERVES.md) (**R684**), [`AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md`](./AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md) (#485), [`AGENTS_SUSPICIOUS_ACTIVITY_QUERIES.md`](./AGENTS_SUSPICIOUS_ACTIVITY_QUERIES.md).  
**Invariants:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (#694 rows).

## Problem class

Request-count governors (`RATE_LIMIT_RPS=60`, `RATE_LIMIT_LCD_HEAVY_RPS=10`) miss **work per request**. A busy 2k-block `/gt/events` window can return multi-MB JSON. Progress polls with `trader` can LCD `GetDiscount` at 1 Hz. Unbounded `tokens`/`pairs` lists amplify factory `BlacklistCheck`.

## Invariants (A694-1–A694-8)

| ID | Meaning |
|----|---------|
| **A694-1** | `/gt/events` combined swap+liq rows ≤ **`MAX_GT_EVENT_ROWS` (5000)**. Over-cap → **400** `event count exceeds 5000` (no truncation). Block span still **2000** → 400. |
| **A694-2** | `/gt/events` GET does **not** `SELECT` `pair_reserves` (still **R684-3**). Reserves stay post-event columns. |
| **A694-3** | `GET /api/v1/route/solve/progress` is on **`lcd_heavy_router`** (same 10 RPS as `/route/solve`). Still advisory — no quote, does not gate submit (#484). |
| **A694-4** | `resolve_discount_bps` caches per `(trader, sender)` for **12s** (`DISCOUNT_BPS_CACHE_TTL` = route-solve TTL). Absent trader/sender → **0**, no LCD. |
| **A694-5** | `GET /api/v1/compliance/blacklist-check` is on **`lcd_heavy_router`**. `tokens` ≤ **16**, `pairs` ≤ **8** after trim/empty-drop. Oversize → **400** (fail-closed, no silent truncate). Factory LCD errors stay sanitized **502**. |
| **A694-6** | Do **not** trust `X-Forwarded-For` / `CF-Connecting-IP`. Keep `PeerIpKeyExtractor`. Prod RPS clamp and CORS allowlist unchanged. |
| **A694-7** | dApp progress omits `trader` when the parent quote already resolved discount (`discount_bps` query param). Back off after consecutive poll failures. Do not rewrite `refetchInterval`s. |
| **A694-8** | Gem exclusion (**L639-2**), hybrid solve cache isolation, and Peer-IP governors stay. No new write APIs. No per-swap FoT math. |

## Code map

| Concern | Location |
|---------|----------|
| Event row cap + COUNT | [`gt.rs`](../indexer/src/api/gt.rs) `MAX_GT_EVENT_ROWS` |
| Router membership | [`api/mod.rs`](../indexer/src/api/mod.rs) `lcd_heavy_router` |
| Discount cache | [`route_solver.rs`](../indexer/src/api/route_solver.rs) `resolve_discount_bps` |
| Progress handler | [`route_solve_progress.rs`](../indexer/src/api/route_solve_progress.rs) |
| Blacklist list caps | [`compliance.rs`](../indexer/src/api/compliance.rs) |
| dApp poll | [`useRouteSolveProgress.ts`](../frontend-dapp/src/hooks/useRouteSolveProgress.ts), [`routeSolveProgress.ts`](../frontend-dapp/src/utils/routeSolveProgress.ts) |

## Do / don’t

- **Do** COUNT (or equivalent) before materializing `/gt/events` rows.
- **Do** return **400** for over-cap GT windows and oversize blacklist lists.
- **Do** keep progress advisory; `You Receive` / Place / Swap stay ungated.
- **Don’t** JOIN `pair_reserves` on GET `/gt/events`.
- **Don’t** fail-open blacklist by truncating lists.
- **Don’t** disable or zero rate limits on non-loopback without `ALLOW_ZERO_RATE_LIMITS=1`.

## Verify

```bash
make setup-indexer-postgres   # if indexer/.env missing
make verify-issue-694
```
