# ADR 0001: Hybrid quoting (L8) and V1 routing scope

## Status

Accepted

## Context

Router and pair legacy `Simulation` queries were pool-only while Pattern C execution consumed the on-chain FIFO book, so quotes could diverge from settlement (invariant L8). Legacy queries were removed in [GitLab #190](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/190); all quotes use hybrid queries.

## Decision

1. **Forward / reverse hybrid quotes:** Pair queries `HybridSimulation` and `HybridReverseSimulation` (read-only book walk + pool leg) are the **only** quote paths ([#190](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/190)). Pool-only: `pool_only_hybrid_params(offer)` / `pool_only_hybrid_template()`. Optional query fields **`trader`** / **`sender`**: when `trader` is omitted, quotes use full pair `fee_bps`; when set, CL8Y fee-tier discount matches execute (read-only — no deregister side-effects; GitLab **#238**). Helpers: `hybrid_simulation_with_trader` / `hybrid_simulation_undiscounted` in `dex_common::pair`. The router always uses hybrid queries; when `terra_swap.hybrid` is unset it passes pool-only params. Router `SimulateSwapOperations` accepts the same optional `trader` / `sender` and forwards them to each hop. When `hybrid` is set, legs must sum to the per-hop simulated amount. Reverse hybrid scales `pool_input` / `book_input` as ratio weights; search is seeded from pool CP reverse math and capped at 32 full sim calls per query ([GitLab **#257**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/257), [`hybrid_reverse.rs`](../smartcontracts/contracts/pair/src/hybrid_reverse.rs)).
2. **Indexer routing scope:** `GET /api/v1/route/solve` defaults to **global best execution** when `amount_in` is set (top-K paths + joint hybrid per hop, max **4 hops**; GitLab **#209**, **#323**, [ADR 0002](./0002-global-best-execution-route-solver.md)). Use **`pool_only=true`** (or `hybrid_optimize=false`) for legacy pool-only ops (**max 4 hops**, `hybrid: null`). Without `amount_in`, GET returns route discovery only (first BFS path, pool-only ops). `GET /api/v1/route/solve/best` (GitLab **#189**) is an alias for the same engine when `amount_in` is required explicitly. All indexer LCD pair quotes use **`hybrid_simulation`** only. **`POST /api/v1/route/solve`** accepts an optional `hybrid_by_hop` array (one optional hybrid payload per hop, aligned with BFS order, **max 4 hops**). The indexer merges those into `router_operations` and, when `amount_in` and `ROUTER_ADDRESS` are set, runs the same LCD `simulate_swap_operations` query the chain uses for that message shape. Integrators can use **GET** (default best execution), **`GET /best`**, or **POST** when supplying their own `hybrid_by_hop`.

## Consequences

- CosmWasm queries do not persist book expiry cleanup; simulation skips expired orders without unlinking storage, matching execute only when the visible walk is unchanged.
- `max_spread` / `belief_price` checks on execute use hybrid **total** gross output (pool gross + book net to taker) for the no-belief denominator; belief path uses full input vs book net + pool gross. Shared implementation: [`dex_common::max_spread`](../smartcontracts/packages/dex-common/src/max_spread.rs) (invariant **L9**, GitLab **#197**).
- **Greedy book-first ([GitLab #708](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/708)):** opt-in `greedy: { max_maker_fills, book_start_hint }` on pair `Swap` / router `TerraSwap`. Distinct from Pattern C (`HybridSwapParams`). **`hybrid: None` stays pool-only (G1)** — not greedy-by-default. Not an on-chain split search (**G4**). Official dApp remains on `GET /route/solve`. Invariants **G1–G14**: [`skills/AGENTS_GREEDY_BOOK_FIRST.md`](../../skills/AGENTS_GREEDY_BOOK_FIRST.md).

## Links

- [ADR 0002 — global best execution](./0002-global-best-execution-route-solver.md) (GitLab #209)
- [GitLab #238 — hybrid sim fee-discount parity](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/238)
- [GitLab #708 — greedy book-first](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/708)
- `docs/limit-orders.md`
- `docs/contracts-security-audit.md` (L8, G1–G14)
