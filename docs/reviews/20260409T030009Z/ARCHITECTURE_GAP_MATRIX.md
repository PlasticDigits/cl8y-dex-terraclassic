# Architecture & capability gap matrix

Legend: **E** = exists, **P** = partial, **M** = missing, **U** = unclear, **N/A** = not applicable to this repo.  
**Bv2** / **Blimit** / **Bhybrid** = launch blocker for that mode (Y/N).  
**Sev** = critical / high / medium / low / note.

## Cross-cutting

| Capability | Status | Evidence | Issue? | Bv2 | Blimit | Bhybrid | Sev | Owner |
|------------|--------|----------|--------|-----|--------|---------|-----|-------|
| Monorepo contracts + router + tests | E | `smartcontracts/` | N | N | N | N | note | contracts |
| Indexer + HTTP API | E | `indexer/` | N | N | N | N | note | backend |
| Frontend dApp | E | `frontend-dapp/` | N | N | N | N | note | frontend |

## Swaps (v2 = pool-only)

| Capability | Status | Evidence | Issue? | Bv2 | Blimit | Bhybrid | Sev | Owner |
|------------|--------|----------|--------|-----|--------|---------|-----|-------|
| v2 swap execution (pair) | E | `pair/src/contract.rs` `execute_swap` | N | N | N | N | note | contracts |
| v2 multi-hop (router) | E | `router/src/contract.rs` | N | N | N | N | note | contracts |
| Quote generation (pool Simulation) | E | pair queries | N | N | N | Y | high | contracts |
| Quote vs hybrid execution | P | L8 audit; router ignores `hybrid` in simulate | Y | N | N | Y | high | contracts |
| Price impact / slippage (`max_spread`, `belief_price`) | E | pair swap | N | N | N | P | medium | contracts |
| Fee accounting (pool) | E | `execute_swap` commission | N | N | N | P | medium | contracts |
| Fee discount (registry + trusted router) | E | `fee-discount`, pair query | N | N | N | N | note | contracts |
| Destination-chain execution | N/A | single-chain | N | — | — | — | note | — |
| Liquidity sourcing (AMM reserves) | E | pair reserves | N | N | N | N | note | contracts |
| Replay protection (chain) | E | Cosmos tx + account seq | N | N | N | N | note | infra |

## Limit orders

| Capability | Status | Evidence | Issue? | Bv2 | Blimit | Bhybrid | Sev | Owner |
|------------|--------|----------|--------|-----|--------|---------|-----|-------|
| Limit order creation | E | `PlaceLimitOrder`, `orderbook.rs` | N | N | N | N | note | contracts |
| Limit order cancellation | E | `CancelLimitOrder` | N | N | N | N | note | contracts |
| Order persistence (on-chain DLL + escrow) | E | `state.rs`, orderbook | N | N | N | N | note | contracts |
| Order expiration | E | `expires_at` in match path | N | N | N | N | note | contracts |
| Order matching (FIFO) | E | `match_bids` / `match_asks` | N | N | N | N | note | contracts |
| Off-chain vs on-chain validation | E | only on-chain authoritative | N | N | N | N | note | product |
| Signature/domain separation | U | CW20 send auth; no EIP-712 | N | N | N | N | note | contracts |
| Partial fills | E | remaining decremented | N | N | N | N | note | contracts |
| Order book HTTP API | M | `limit-orders.md`: LCD only | Y | N | Y | P | high | backend |
| User-visible order state | P | `LimitOrdersPage.tsx`; no live book | Y | N | Y | P | medium | frontend |
| Pause + cancel semantics | E | L6 blocks cancel when paused | N | N | P | P | medium | docs |

## Hybrid

| Capability | Status | Evidence | Issue? | Bv2 | Blimit | Bhybrid | Sev | Owner |
|------------|--------|----------|--------|-----|--------|---------|-----|-------|
| Hybrid execution (Pattern C) | E | `HybridSwapParams`, `execute_swap` | N | N | N | N | note | contracts |
| Hybrid route selection | M | manual UI; `route_solver` `hybrid: null` | Y | N | N | Y | high | backend |
| Best execution logic | M | not implemented server-side | Y | N | N | Y | high | product |
| Fallback routing | P | user can set book leg 0 | N | N | N | P | low | frontend |
| Router forward `hybrid` | E | `router/src/contract.rs` | N | N | N | N | note | contracts |
| Simulation includes book | M | L8 | Y | N | N | Y | high | contracts |
| Indexer hybrid-aware quotes | M | `route_solver.rs` header | Y | N | N | Y | high | backend |

## Infra, security, ops

| Capability | Status | Evidence | Issue? | Bv2 | Blimit | Bhybrid | Sev | Owner |
|------------|--------|----------|--------|-----|--------|---------|-----|-------|
| Pause / emergency controls | E | factory `SetPairPaused` | N | N | P | P | medium | contracts |
| Upgrade workflow (Wasm migration) | U | deployment guide; not automated in repo | Y | Y | Y | Y | medium | infra |
| Governance scope documented | E | `security-model.md`, audit doc | N | N | N | N | note | docs |
| Deployment reproducibility | P | optimizer vs CI wasm | Y | Y | N | N | high | infra |
| Test coverage (contracts) | E | `smartcontracts/tests`, proptest | N | N | N | N | note | testing |
| Test coverage (hybrid E2E) | M | `swap.spec.ts` no hybrid fill | Y | N | N | Y | high | testing |
| Monitoring / alerting | M | not defined in repo | Y | Y | Y | Y | medium | infra |
| Runbooks | P | docs scattered | Y | Y | Y | Y | low | docs |
| Indexer observability | P | `tracing`, no dashboard defs | Y | P | P | P | medium | backend |
| User-visible fees/slippage (swap) | P | hybrid fee disclosure weak | Y | N | N | Y | medium | frontend |

## Reference issue IDs

See [ISSUE_BACKLOG.md](./ISSUE_BACKLOG.md) for DEX-P* items mapping.
