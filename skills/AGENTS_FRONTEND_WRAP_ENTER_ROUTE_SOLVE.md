# Agent skill: native wrap-enter / unwrap-exit GET `/route/solve` (Forgejo #1218)

Use when changing **Swap** (`/`) native `uluna` / `uusd` quote or submit, wrap-mapper substitution, indexer top-K path fill, or **H596-7** (pool-only wrap execute).

Issue **#1218 is implemented**. Client maps wrap-mapper CW20s **before** `GET /api/v1/route/solve`. Indexer still rejects raw `token_in=uluna` (one mapping site). Execute stays pool-only.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [Forgejo **#1218**](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1218) | Wrap-enter must not BFS-first a ~100% hop |
| [`wrapMappedSolvePair`](../frontend-dapp/src/utils/nativeWrapRouteSolve.ts) | `uluna`→cLUNC / `uusd`→cUSTC |
| [`quoteCw20ViaRouteSolve`](../frontend-dapp/src/utils/cw20RouteSolveQuote.ts) `omitAllHybrid` | Quote = pool-only execute |
| [`executeNativeSwap`](../frontend-dapp/src/services/terraclassic/router.ts) | Solver hops; no second BFS |
| [`path_unusable_for_top_k`](../indexer/src/api/db_orderbook_sim.rs) | Skip ~100% spread while filling K |
| [ADR 0002](../docs/adr/0002-global-best-execution-route-solver.md) | Skip-unusable amendment |
| [`AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md`](./AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md) | **H596-7** |
| [`AGENTS_HYBRID_HOP_OFFER.md`](./AGENTS_HYBRID_HOP_OFFER.md) | **H1280-6** |
| [`AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](./AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md) | One Route row from submit ops |

## Invariants **H1218-1–H1218-8**

1. **H1218-1 — client wrap map.** Non-direct native pay and/or native receive resolve wrap CW20s, then `quoteCw20ViaRouteSolve`. Direct 1:1 wrap/unwrap never calls `/route/solve`.
2. **H1218-2 — post-wrap `amount_in`.** Wrap-enter solve uses `netCw20AfterNativeWrap`. Unwrap-exit You Receive nets mapper unwrap fee + InstantWithdraw burn tax.
3. **H1218-3 — quote = submit hops.** `executeNativeSwap(..., operations)` uses this quote tick’s solver hops. Do not re-run `findRouteWithNativeSupport` when ops are present.
4. **H1218-4 — execute pool-only.** Strip all `hybrid` / `book_input` (**H596-7**). `wrap_deposit` then CW20 send; `unwrap_output` iff native ask.
5. **H1218-5 — one mapping site.** Indexer GET without substitution still 400s `token_in=uluna`. Do not also wrap-map in the GET handler.
6. **H1218-6 — skip-unusable then fill K.** Do not keep ~100% hop-spread / `hop_sim_implausible` candidates while filling `MAX_PATH_CANDIDATES`. Continue until K usable or hop cap / `PATH_ENUM_POOL` exhausted. Winner remains max `estimated_amount_out_net`. Do **not** bump `MAX_PATH_CANDIDATES` or hop cap as the primary fix.
7. **H1218-7 — degradation is not theater.** Indexer 5xx may fall back to client BFS (**H596-5**). If fallback worst-hop ~100%, submit stays blocked. Do not submit BFS when this quote tick already had solver ops.
8. **H1218-8 — one Route row.** Display wrap prefix + solver hops (not BFS 2-hop while ops are wrap-then-cUSTC). Trade market stays CW20 pair-scoped (out of scope unless a native denom appears).

## Rules of thumb

- Do **not** attach hop-0 book to wrap execute “to fix ranking.”
- Do **not** fold [#1257](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1257) USTR→USDT or [#1264](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1264) gas envelopes into this change.
- Identity is contract / denom, not ticker ([#715](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/715)).

## Verify

```bash
make verify-issue-1218
```
