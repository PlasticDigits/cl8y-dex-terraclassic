# ADR 0001: Hybrid quoting (L8) and V1 routing scope

## Status

Accepted

## Context

Router and pair legacy queries were pool-only while Pattern C execution consumed the on-chain FIFO book, so quotes could diverge from settlement (invariant L8).

## Decision

1. **Forward / reverse hybrid quotes:** Add pair queries `HybridSimulation` and `HybridReverseSimulation` (read-only book walk + pool leg). The router uses them whenever `SwapOperation::TerraSwap.hybrid` is set; legs must sum to the per-hop simulated amount. Reverse hybrid scales `pool_input` / `book_input` as ratio weights to find the minimum total offer achieving the target net output (binary search over total offer).
2. **Indexer routing scope:** `GET /api/v1/route/solve` defaults to **pool-only** ops (`terra_swap.hybrid: null`) for backward compatibility. Opt-in **`hybrid_optimize=true`** (with `amount_in`) runs indexer-side per-hop split search (pair `HybridSimulation`), **caps the path at 3 hops**, merges optimized `hybrid` into `router_operations`, and may return `estimated_amount_out` from router LCD simulation. **`POST /api/v1/route/solve`** accepts an optional `hybrid_by_hop` array (one optional hybrid payload per hop, aligned with BFS order, **max 4 hops**). The indexer merges those into `router_operations` and, when `amount_in` and `ROUTER_ADDRESS` are set, runs the same LCD `simulate_swap_operations` query the chain uses for that message shape. Integrators can rely on **GET hybrid_optimize** for server-chosen splits or **POST** when supplying their own `hybrid_by_hop`.

## Consequences

- CosmWasm queries do not persist book expiry cleanup; simulation skips expired orders without unlinking storage, matching execute only when the visible walk is unchanged.
- `max_spread` / `belief_price` checks on execute use hybrid **total** gross output (pool gross + book net to taker) for the no-belief denominator; belief path uses full input vs book net + pool gross.

## Links

- `docs/limit-orders.md`
- `docs/contracts-security-audit.md` (L8)
