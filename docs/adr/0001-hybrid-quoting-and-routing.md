# ADR 0001: Hybrid quoting (L8) and V1 routing scope

## Status

Accepted

## Context

Router and pair legacy queries were pool-only while Pattern C execution consumed the on-chain FIFO book, so quotes could diverge from settlement (invariant L8).

## Decision

1. **Forward / reverse hybrid quotes:** Add pair queries `HybridSimulation` and `HybridReverseSimulation` (read-only book walk + pool leg). The router uses them whenever `SwapOperation::TerraSwap.hybrid` is set; legs must sum to the per-hop simulated amount. Reverse hybrid scales `pool_input` / `book_input` as ratio weights to find the minimum total offer achieving the target net output (binary search over total offer).
2. **Indexer routing scope:** `GET /api/v1/route/solve/best` (GitLab **#189**) is the retail / Vyntrex **best-execution** entry point: requires `amount_in`, always runs hybrid optimization (max 3 hops). `GET /api/v1/route/solve` defaults to **pool-only** ops (`terra_swap.hybrid: null`) for backward compatibility; opt-in **`hybrid_optimize=true`** (with `amount_in`) or **`pool_only=true`** (explicit pool-only when `amount_in` is set) preserve legacy integrator contracts. **`POST /api/v1/route/solve`** accepts an optional `hybrid_by_hop` array (one optional hybrid payload per hop, aligned with BFS order, **max 4 hops**). The indexer merges those into `router_operations` and, when `amount_in` and `ROUTER_ADDRESS` are set, runs the same LCD `simulate_swap_operations` query the chain uses for that message shape. Integrators should prefer **`GET /route/solve/best`** for server-chosen splits or **POST** when supplying their own `hybrid_by_hop`.

## Consequences

- CosmWasm queries do not persist book expiry cleanup; simulation skips expired orders without unlinking storage, matching execute only when the visible walk is unchanged.
- `max_spread` / `belief_price` checks on execute use hybrid **total** gross output (pool gross + book net to taker) for the no-belief denominator; belief path uses full input vs book net + pool gross.

## Links

- `docs/limit-orders.md`
- `docs/contracts-security-audit.md` (L8)
