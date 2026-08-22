# ADR 0002: Global best-execution route solver (indexer)

## Status

Accepted (GitLab [#209](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/209))

## Context

The indexer previously chose the **first** BFS shortest path and ran a **sequential** per-hop hybrid grid (`hybrid_route_opt::optimize_multihop_hybrid`). A longer path or different split schedule could yield higher `estimated_amount_out` on the same LCD snapshot. Clients saw `hybrid_notes` admitting non-global optimality without a single “best execution” contract.

## Decision

1. **Path search:** Enumerate up to **5** simple paths ordered by hop count (`route_paths::find_paths_top_k`), capped by GET **3 hops** (unchanged from ADR 0001 / #191).
2. **Hybrid search:** For each candidate path, run **joint** optimization: sequential baseline plus **2** coordinate-descent passes over per-hop `book_input` grids (`hybrid_route_opt::optimize_multihop_hybrid_joint`, 17 grid points per hop).
3. **Winner selection:** Compare router `simulate_swap_operations` `estimated_amount_out` when `ROUTER_ADDRESS` is set; otherwise compare hybrid simulation totals. Highest out wins (deterministic tie-break: later path index does not replace equal out).
4. **API metadata:** Hybrid GET responses include `solver_version` (`global_v1`), `paths_considered`, `optimality_scope`, `lcd_hybrid_queries`, and updated `hybrid_notes`. Cache keys include `solver_version` and amount bucket.
5. **Liability:** Solver remains **advisory**; clients set `max_spread` / min receive on execute. `optimality_scope` states bounds explicitly (not “globally optimal on all possible paths”).
6. **POST unchanged:** `POST /api/v1/route/solve` keeps BFS discovery (max 4 hops) and optional `hybrid_by_hop` for integrator overrides.

## Consequences

- LCD load scales with paths × hops × grid; bounded by constants in `best_execution.rs` (`MAX_PATH_CANDIDATES`, `LCD_HYBRID_SIM_BUDGET`).
- Degraded hops (`indexer_hybrid_lcd_degraded`) still pool-only fallback per hop (#190).
- Discovery GET without `amount_in` still uses first BFS path only (no hybrid solver).

## Amendments

- **#323** (2026): Raised default hybrid GET hop cap from **3 → 4** (`GET_DEFAULT_MAX_HOPS`) after #319 (DB mirror pricing). Matches `GET_POOL_ONLY_MAX_HOPS`, POST BFS, and on-chain `MAX_HOPS`. `LCD_HYBRID_SIM_BUDGET` re-derived to **1700** (`5×4×85`). See [route-solver.md](../route-solver.md).
- **#501** (2026): Retail `/trade` Market ticket defaults to the same **GET** best-execution path as Swap (no client-side split search; Pattern C remains caller-declared on-chain with solver-filled params). Advanced manual book leg keeps **POST** `hybrid_by_hop`. Shared helper: [`cw20RouteSolveQuote.ts`](../../frontend-dapp/src/utils/cw20RouteSolveQuote.ts). Docs: [`limit-orders.md`](../limit-orders.md#swap-ui-hybrid-vs-pool-only-estimates), [`skills/AGENTS_HYBRID_QUOTING.md`](../../skills/AGENTS_HYBRID_QUOTING.md).
- **#596** (2026): Official dApp **always** uses that GET path — no retail hybrid opt-in/opt-out. Pool-only remains indexer `pool_only=true` for integrators / custom frontends. Skill: [`AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md`](../../skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md).

## Links

- [ADR 0001](./0001-hybrid-quoting-and-routing.md)
- [route-solver.md](../route-solver.md) — in-depth pipeline, glossary, optimization theory, and shipped constants (GitLab [#310](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/310))
- [indexer-invariants.md](../indexer-invariants.md) — route GET hybrid / best execution rows
- [skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md](../../skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md)
- [skills/AGENTS_HYBRID_QUOTING.md](../../skills/AGENTS_HYBRID_QUOTING.md) — Swap + Trade GET default (#501); always-on hybrid (#596)
- [skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md](../../skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md) — no retail pool-only opt-out (#596)
- [gaps/GAP_1780023683.md](../../gaps/GAP_1780023683.md)
