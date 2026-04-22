# Architecture & capability gap matrix

**Last reviewed:** 2026-04-22. This matrix is a **historical snapshot** from the review bundle; **authoritative** backlog and epics live in [GitLab](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues) (see [ISSUE_BACKLOG.md](./ISSUE_BACKLOG.md), [GLAB_ISSUES.md](./GLAB_ISSUES.md)).

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
| Quote vs hybrid execution | P | L8: pool-only `Simulation` without `hybrid`; with `hybrid`, router/pair use hybrid sim queries — integrators must pass `hybrid` when quoting Pattern C | Y | N | N | Y | high | contracts |
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
| Order book HTTP API | P | Indexer proxies head + shallow book to LCD ([`limit-orders.md`](../../limit-orders.md)); deep/paginated API **#102** | Y | N | P | P | medium | backend |
| User-visible order state | P | `LimitOrdersPage.tsx` shows **shallow** book via indexer; not full depth until **#102** | Y | N | P | P | medium | frontend |
| Pause + cancel semantics | E | L6 blocks cancel when paused | N | N | P | P | medium | docs |

## Hybrid

| Capability | Status | Evidence | Issue? | Bv2 | Blimit | Bhybrid | Sev | Owner |
|------------|--------|----------|--------|-----|--------|---------|-----|-------|
| Hybrid execution (Pattern C) | E | `HybridSwapParams`, `execute_swap` | N | N | N | N | note | contracts |
| Hybrid route selection | P | `GET hybrid_optimize` + dApp default; `POST hybrid_by_hop`; manual/advanced in Settings — **#101** | N | N | N | P | high | backend |
| Best execution logic | P | Sequential per-hop indexer optimizer (not global); fuller product gate **#108** | N | N | N | P | medium | product |
| Fallback routing | P | user can set book leg 0 | N | N | N | P | low | frontend |
| Router forward `hybrid` | E | `router/src/contract.rs` | N | N | N | N | note | contracts |
| Simulation includes book | M | L8 | Y | N | N | Y | high | contracts |
| Indexer hybrid-aware quotes | P | `GET hybrid_optimize` + `POST hybrid_by_hop` + LCD sim | N | N | N | P | high | backend |

## Infra, security, ops

| Capability | Status | Evidence | Issue? | Bv2 | Blimit | Bhybrid | Sev | Owner |
|------------|--------|----------|--------|-----|--------|---------|-----|-------|
| Pause / emergency controls | E | factory `SetPairPaused` | N | N | P | P | medium | contracts |
| Upgrade workflow (Wasm migration) | U | deployment guide; not automated in repo | Y | Y | Y | Y | medium | infra |
| Governance scope documented | E | `security-model.md`, audit doc | N | N | N | N | note | docs |
| Deployment reproducibility | P | optimizer vs CI wasm | Y | Y | N | N | high | infra |
| Test coverage (contracts) | E | `smartcontracts/tests`, proptest | N | N | N | N | note | testing |
| Test coverage (hybrid E2E) | P | `hybrid-swap.spec.ts` (book leg, wasm fill, quote source); conditional skips; stricter gates **#103** / **#79**; multihop hybrid parity optional | N | N | N | P | high | testing |
| Monitoring / alerting | M | not defined in repo | Y | Y | Y | Y | medium | infra |
| Runbooks | P | docs scattered | Y | Y | Y | Y | low | docs |
| Indexer observability | P | `tracing`, no dashboard defs | Y | P | P | P | medium | backend |
| User-visible fees/slippage (swap) | P | hybrid fee disclosure weak | Y | N | N | Y | medium | frontend |

## Reference issue IDs

See [ISSUE_BACKLOG.md](./ISSUE_BACKLOG.md) for DEX-P* items mapping.

---

**Footnote:** Rows reflect capabilities at **last reviewed** date above. For **current** scope (deep book, default hybrid routing, E2E gates), use GitLab issues (e.g. **#101**, **#102**, **#103**, **#108**) rather than treating this file as a live dashboard.
