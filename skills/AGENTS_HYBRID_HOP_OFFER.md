# Agent skill: declared hybrid hop-offer partition (Forgejo #1280 / wrap #1264)

Use when changing **router/pair declared `hybrid`**, indexer **`GET /route/solve` hybrid emission**, or dApp **submit preflight** for Pattern C splits.

This is **construction drift**, not a request to drop the sum invariant and not greedy-default work ([#718](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/718)).

## Policy A (locked)

Retail **`GET /api/v1/route/solve`** emits declared `hybrid` on **hop 0 only** (the CW20 send amount). Hops **1+** are `hybrid: null` (pool-only on-chain). Joint grid may still **rank** paths using interior books; **emitted** ops and `estimated_amount_out` use the hop-0-only plan so quote ≈ execute.

**`POST /route/solve` `hybrid_by_hop`** may still declare interior splits that **partition that hop’s offer** (integrator / Advanced). Wasm keeps the hard fail.

Wrap / native BFS (`executeNativeSwap`) never copies `hybrid` / `book_input` ([#1264](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1264), **H596-7**). Do not attach leftover CW20 solver hybrid onto a wrap hop.

No columbus-5 router/pair migrate for this ticket (no rescale in `reply_swap_hop`).

## Invariants **H1280-1–H1280-8**

| Id | Rule |
|----|------|
| **H1280-1** | Declared `hybrid: Some(_)`: `pool_input + book_input` must equal that hop’s offer (`checked_add`, no wrap). Pair: `HybridSplitMismatch`. Router hop 0 / `reply_swap_hop`: `must equal hop offer amount`. |
| **H1280-2** | Retail GET emits declared hybrid on **hop 0 only**. |
| **H1280-3** | Retail GET hops 1+ are `hybrid: null` (pool-only). Do not copy hop-0 integers onto later hops. |
| **H1280-4** | POST `hybrid_by_hop` may keep interior Pattern C splits that partition that hop’s offer (**AC5**). Greedy mutex (**G11**) unchanged. `hybrid: None` stays pool-only (**G1**). |
| **H1280-5** | dApp: strip interior GET hybrid; fail closed before sign if hop-0 sum ≠ pay raw. Later-hop offers are realized `hop_output`, not quote-time `running`. |
| **H1280-6** | Wrap/native BFS hops omit `hybrid` (**H596-7**, [#1264](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1264)). |
| **H1280-7** | Do not skip `min_return` / `max_spread` / material pool-leg / greedy mutex. Do not overload `pool_input=0, book_input=offer` as “rescale for me.” |
| **H1280-8** | No wasm rescale in `reply_swap_hop`. Typed hop-offer errors stay; dApp humanizes the generic LCD string **after** the revert is rare. |

## Files

- Indexer: `retail_declared_hybrid_plan_hop0_only` in [`hybrid_route_opt.rs`](../indexer/src/api/hybrid_route_opt.rs); applied in [`best_execution.rs`](../indexer/src/api/best_execution.rs) before `apply_hybrid_by_hop`.
- Frontend: [`hybridHopOfferPartition.ts`](../frontend-dapp/src/utils/hybridHopOfferPartition.ts), [`cw20RouteSolveQuote.ts`](../frontend-dapp/src/utils/cw20RouteSolveQuote.ts), [`router.ts`](../frontend-dapp/src/services/terraclassic/router.ts) `executeMultiHopSwap`.
- Wasm (unchanged invariant): [`router/src/contract.rs`](../smartcontracts/contracts/router/src/contract.rs) `validate_hybrid_declared_split_for_no_belief`.

## Tests

```bash
make verify-issue-1280
# optional wrap pool-only + same 2.71M envelope (no columbus-5 AC1 raise):
make verify-issue-1264
cd smartcontracts && cargo test -p cl8y-dex-tests -- --test-threads=1 router_declared_split_mismatch_reverts_hop0
cd smartcontracts && cargo test -p cl8y-dex-tests -- --test-threads=1 router_two_hop_interior_hybrid_mismatch_reverts
cd indexer && cargo test --lib retail_plan_keeps_hop0
cd frontend-dapp && npm test -- src/utils/hybridHopOfferPartition.test.ts src/utils/cw20RouteSolveQuote.test.ts
```

## Cross-links

- Issues: [#1280](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1280) · [#1264](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1264)
- [`AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md) · [`AGENTS_TESTING_MULTIHOP_HYBRID.md`](./AGENTS_TESTING_MULTIHOP_HYBRID.md) · [`AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md`](./AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md) · [`AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md`](./AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md) (**H596-7**) · [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md)
- [`docs/integrators.md`](../docs/integrators.md) · [`docs/route-solver.md`](../docs/route-solver.md) · [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) **L4** / **H1280**
