# Test gap matrix

## Existing evidence (summary)

| Area | Evidence |
|------|----------|
| Contract unit + integration | `smartcontracts/` `cargo test`; harness [`smartcontracts/tests/src/lib.rs`](../../../smartcontracts/tests/src/lib.rs) |
| Limit / hybrid contracts | [`smartcontracts/tests/src/limit_order_tests.rs`](../../../smartcontracts/tests/src/limit_order_tests.rs) |
| Property / proptest | `fuzz_tests` and related modules in `lib.rs` (~L4629+) |
| Fee discount | Documented in [`docs/testing.md`](../../testing.md) |
| Indexer lib tests | `cd indexer && cargo test --lib` |
| Indexer integration | `indexer/tests/*.rs` — security, api_route_solve, pairs, cg, cmc, orderbook LCD mock |
| Frontend unit | Vitest `frontend-dapp` |
| E2E | Playwright `frontend-dapp/e2e/*.spec.ts` — `swap.spec.ts`, `swap-tx.spec.ts`, `limit-orders.spec.ts`, pool, tiers, wrap, create-pair |

## Gaps

| Scenario | Current evidence | Missing coverage | Suggested test type | Issue | Priority |
|----------|------------------|------------------|----------------------|-------|----------|
| Hybrid swap consumes book + pool in one tx | `hybrid_pool_and_book_legs_one_swap` in [`limit_order_tests.rs`](../../../smartcontracts/tests/src/limit_order_tests.rs) (L8 sim vs execute) | Playwright hybrid UI + tx when LocalTerra seeded | E2E | DEX-P1-011 | P1 |
| Multi-hop router with `hybrid` on first hop | `router_two_hop_first_leg_hybrid_matches_simulate` in [`limit_order_tests.rs`](../../../smartcontracts/tests/src/limit_order_tests.rs) | — | cw-multi-test | DEX-P1-008 | P1 — **partial** |
| Multi-hop router ≥3 hops with `hybrid` on ≥2 legs | `router_three_hop_two_legs_hybrid_matches_simulate` in [`limit_order_tests.rs`](../../../smartcontracts/tests/src/limit_order_tests.rs); indexer `route_solve_post_three_hop_multi_leg_hybrid`, `route_solve_get_hybrid_optimize_three_hops` in [`api_route_solve.rs`](../../../indexer/tests/api_route_solve.rs) | E2E multihop hybrid tx on LocalTerra | cw-multi-test + indexer integration | [GitLab **#192**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/192) | P1 — **covered** |
| Router simulate vs execute (hybrid) | `router_simulate_swap_hybrid_field_ignored` (named in audit L8) | Assert documented in test name search | Unit/integration | covered — **document only** | P3 |
| Quote = execution (pool-only) | Many swap tests | Regression when fee_bps changes | proptest exists | — | P3 |
| Limit cancel while paused | Audit L6 | Explicit test in `limit_order_tests` | integration | verify exists — if missing add | P1 |
| Indexer parses hybrid swap attributes | Parser + `swap_events_hybrid_columns.rs` | — | integration | [GitLab **#82**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/82) / epic [**#199**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/199) | P2 — **covered** |
| Fee discount + hybrid book leg | `hybrid_book_fill_uses_taker_discounted_effective_fee_bps` | — | integration | [GitLab **#83**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/83) / **#199** | P2 — **covered** |
| Expired limit during match | `limit-orders.md` behavior | Contract test unlink + no maker transfer | integration | verify `limit_order_tests` | P2 |
| Frontend hybrid validation (max_maker_fills) | `pair.test.ts`, `router.hybrid.test.ts`, `routeOperations.test.ts` | — | Vitest | [GitLab **#84**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/84) / **#199** | P2 — **covered** |
| Gas / size limits for large `max_maker_fills` | Hard cap in dex-common | Boundary test at cap | integration | DEX-P2-020 | P3 |
| Reorg / duplicate tx | Indexer ON CONFLICT | Chaos test not in repo | manual / future | DEX-P2-016 | P2 |
| Upgrade / migrate contract | [`scripts/smoke-pool-swap.sh`](../../../scripts/smoke-pool-swap.sh) | Post-migration smoke script | manual | [GitLab **#86**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/86) / **#199** | P2 — **covered** (manual gate) |
| Pause / resume swap + limits | `pause_blocks_swap_and_place_cancel_refunds_escrow`; TradePage pause banner Vitest | E2E banner if added | integration + Vitest | [GitLab **#87**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/87) / **#199** | P2 — **covered** |
| Wrap + swap integration | `wrap-swap.spec.ts` | Extend for router path | E2E | — | P3 |

### Quick grep verification commands (for maintainers)

```bash
rg "pause_blocks_swap" smartcontracts/tests -n
rg "hybrid" smartcontracts/tests/src/limit_order_tests.rs -n
rg "hybrid" frontend-dapp/e2e -n
```

---

## CI coverage vs local

| Check | CI | Gap |
|-------|----|-----|
| `cargo llvm-cov` contracts | yes `.github/workflows/test.yml` | Does not run `build-optimized` |
| Indexer tests | yes with Postgres | Integration parallelism note [`docs/testing.md`](../../testing.md) |
| Playwright | yes | `hybrid-swap.spec.ts` asserts hybrid disclosure + doc link when LCD up |

