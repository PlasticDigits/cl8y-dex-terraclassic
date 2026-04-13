# ADR 0001: Hybrid quoting (L8) and V1 routing scope

## Status

Accepted

## Context

Router and pair legacy queries were pool-only while Pattern C execution consumed the on-chain FIFO book, so quotes could diverge from settlement (invariant L8).

## Decision

1. **Forward / reverse hybrid quotes:** Add pair queries `HybridSimulation` and `HybridReverseSimulation` (read-only book walk + pool leg). The router uses them whenever `SwapOperation::TerraSwap.hybrid` is set; legs must sum to the per-hop simulated amount. Reverse hybrid scales `pool_input` / `book_input` as ratio weights to find the minimum total offer achieving the target net output (binary search over total offer).
2. **Indexer routing scope:** `GET /api/v1/route/solve` remains **pool-only** (`terra_swap.hybrid: null`) for backward compatibility and simple clients. **`POST /api/v1/route/solve`** accepts an optional `hybrid_by_hop` array (one optional hybrid payload per hop, aligned with BFS order). The indexer merges those into `router_operations` and, when `amount_in` and `ROUTER_ADDRESS` are set, runs the same LCD `simulate_swap_operations` query the chain uses for that message shape—so estimates can include the book leg when the client supplies valid per-hop splits. The dApp may still compute splits off-chain; the POST path is the supported **server-assisted merge + quote** for integrators that already know their split.

## Consequences

- CosmWasm queries do not persist book expiry cleanup; simulation skips expired orders without unlinking storage, matching execute only when the visible walk is unchanged.
- `max_spread` / `belief_price` checks on execute use hybrid **total** gross output (pool gross + book net to taker) for the no-belief denominator; belief path uses full input vs book net + pool gross.

## Links

- `docs/limit-orders.md`
- `docs/contracts-security-audit.md` (L8)
