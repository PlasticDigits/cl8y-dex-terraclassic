# ADR 0001: Hybrid quoting (L8) and V1 routing scope

## Status

Accepted

## Context

Router and pair legacy queries were pool-only while Pattern C execution consumed the on-chain FIFO book, so quotes could diverge from settlement (invariant L8).

## Decision

1. **Forward / reverse hybrid quotes:** Add pair queries `HybridSimulation` and `HybridReverseSimulation` (read-only book walk + pool leg). The router uses them whenever `SwapOperation::TerraSwap.hybrid` is set; legs must sum to the per-hop simulated amount. Reverse hybrid scales `pool_input` / `book_input` as ratio weights to find the minimum total offer achieving the target net output (binary search over total offer).
2. **V1 routing scope:** User-controlled split in the dApp remains the supported launch path; indexer `route/solve` continues to return `hybrid: null` until a future ADR defines server-assisted splits.

## Consequences

- CosmWasm queries do not persist book expiry cleanup; simulation skips expired orders without unlinking storage, matching execute only when the visible walk is unchanged.
- `max_spread` / `belief_price` checks on execute use hybrid **total** gross output (pool gross + book net to taker) for the no-belief denominator; belief path uses full input vs book net + pool gross.

## Links

- `docs/limit-orders.md`
- `docs/contracts-security-audit.md` (L8)
