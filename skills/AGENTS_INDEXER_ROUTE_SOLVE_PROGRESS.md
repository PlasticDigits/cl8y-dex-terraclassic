# Indexer route-solve progress + distant-pair latency (GitLab #485)

Audience: third-party agents working on Swap quoting UX, indexer hybrid solve latency, or progress polling.

Companion docs: [`docs/route-solver.md`](../docs/route-solver.md) (progress poll + cache TTLs), [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (#485 row), [`AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md`](./AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md), [`AGENTS_FRONTEND_SWAP_QUOTE_REFETCH.md`](./AGENTS_FRONTEND_SWAP_QUOTE_REFETCH.md).

## Problem class

Distant CW20 pairs (no direct pool) can take many seconds on cold hybrid `GET /api/v1/route/solve`. #484 stopped the frontend from cancel/restarting those quotes, but traders still saw a static **Calculating…**. #485 adds:

1. **Latency:** reuse a TTL-cached token graph, longer multi-hop response cache, stage timing logs — target cold solves **&lt;15s** p95 when DB-hybrid is enabled.
2. **Progress:** advisory `GET /api/v1/route/solve/progress` so the UI can show **Searching x of y…** ~1 Hz.

## Invariants

| Invariant | Meaning |
|-----------|---------|
| **Progress is advisory** | Never a quote. No `estimated_amount_out`. On-chain `min_receive` / `max_spread` remain authoritative. |
| **Same cache key isolation** | Progress key = hybrid cache key (`solver_version`, tokens, amount bucket, maker-fills bucket, **`discount_bps`**). No tier cross-talk (#283/#324). |
| **No LCD amplification** | Progress polls do **not** run hybrid grids or router sims. Endpoint is on the **standard** rate-limit router (not LCD-heavy). |
| **Nested solves silent** | Slippage token-price solves (`route_slippage`) pass `progress_key: None`. |
| **Optimality bounds unchanged** | Do not shrink `MAX_PATH_CANDIDATES` / hop cap / grid without bumping `solver_version` + docs. Empty-book pool-only short-circuit (#493) is allowed without a version bump — see [`AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md`](./AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md) § Empty-book grid short-circuit. |
| **#484 / #496 still hold** | Progress is display-only; `simQuoteRefetchInterval`, `shouldShowSimReceiveCalculating` (same-key keep-previous vs pay-change loading), `isSubmitQuoteStale` unchanged. |
| **No flash on fast quotes** | Frontend shows progress only after ~500ms and non-idle stage. |

## Code map

| Concern | Location |
|---------|----------|
| Graph snapshot (15s TTL) | [`route_graph.rs`](../indexer/src/api/route_graph.rs) |
| Progress registry + HTTP | [`route_solve_progress.rs`](../indexer/src/api/route_solve_progress.rs) |
| Cache TTL 12s / 90s | [`route_solver.rs`](../indexer/src/api/route_solver.rs) — `ROUTE_CACHE_TTL`, `ROUTE_CACHE_TTL_DISTANT` |
| Stage timings + progress updates | [`best_execution.rs`](../indexer/src/api/best_execution.rs) |
| Frontend poll + labels | [`useRouteSolveProgress.ts`](../frontend-dapp/src/hooks/useRouteSolveProgress.ts), [`routeSolveProgress.ts`](../frontend-dapp/src/utils/routeSolveProgress.ts) |
| Swap wiring | [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) |
| Client | [`client.ts`](../frontend-dapp/src/services/indexer/client.ts) — `getRouteSolveProgress` |

## Poll contract

```
GET /api/v1/route/solve/progress?token_in=&token_out=&amount_in=&trader=&max_maker_fills=
```

| Field | Notes |
|-------|-------|
| `stage` | `idle` \| `graph_load` \| `enumerating` \| `loading_mirrors` \| `evaluating` \| `simulating` \| `enriching` \| `done` \| `cached` \| `error` |
| `done` / `total` | Honest counters (pairs during mirror preload; paths during evaluate) |
| `label` | Render as **text**, never HTML |
| `cache_hit` | True when hybrid body came from cache |

## Do / don’t

- **Do** poll ~1 Hz while Swap `getRouteSolve` is in flight; abort with the sim AbortSignal.
- **Do** keep Trade market pair-scoped **Quoting…** for now (Trade uses `GET /route/solve` by default after [#501](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/501), but still shows static Quoting… — wiring `useRouteSolveProgress` on `/trade` is an optional follow-up).
- **Don’t** invent “found better route” claims beyond `optimality_scope`.
- **Don’t** lower `INDEXER_ROUTE_SOLVE_TIMEOUT_MS` until distant-pair p95 is proven &lt;15s.
- **Don’t** put progress on the LCD-heavy governor.

## Regression checklist

1. Unit: `cargo test --lib route_solve_progress route_graph cache_ttl`
2. Integration: `route_solve_progress_*` in `api_route_solve` (+ cache tier tests still pass)
3. Frontend: `routeSolveProgress.test.ts`, `client.test.ts` progress URL
4. Docs: `python3 scripts/check_route_solver_docs.py`
5. Smoke: `make verify-issue-485`
