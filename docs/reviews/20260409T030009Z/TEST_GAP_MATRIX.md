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
| Multi-hop router with `hybrid` on first hop | `router_two_hop_first_leg_hybrid_matches_simulate` in [`limit_order_tests.rs`](../../../smartcontracts/tests/src/limit_order_tests.rs) | Optional: second hop hybrid variant | cw-multi-test | DEX-P1-008 | P1 |
| Router simulate vs execute (hybrid) | `router_simulate_swap_hybrid_field_ignored` (named in audit L8) | Assert documented in test name search | Unit/integration | covered — **document only** | P3 |
| Quote = execution (pool-only) | Many swap tests | Regression when fee_bps changes | proptest exists | — | P3 |
| Limit cancel while paused | Audit L6 | Explicit test in `limit_order_tests` | integration | verify exists — if missing add | P1 |
| Indexer parses hybrid swap attributes | Parser unit tests | Swap row has `book_return_amount` when present | integration | DEX-P2-017 | P2 |
| Fee discount + hybrid book leg | Discount tests + orderbook | Combined: discounted `effective_fee_bps` on match_bids/asks | integration | DEX-P2-018 | P2 |
| Expired limit during match | `limit-orders.md` behavior | Contract test unlink + no maker transfer | integration | verify `limit_order_tests` | P2 |
| Frontend hybrid validation (max_maker_fills) | UI state | Unit test for message shape | Vitest | DEX-P2-019 | P2 |
| Gas / size limits for large `max_maker_fills` | Hard cap in dex-common | Boundary test at cap | integration | DEX-P2-020 | P3 |
| Reorg / duplicate tx | Indexer ON CONFLICT | Chaos test not in repo | manual / future | DEX-P2-016 | P2 |
| Upgrade / migrate contract | None | Post-migration smoke script | manual | DEX-P2-021 | P2 |
| Pause / resume swap + limits | `limit_order_tests` pause | E2E banner if added | E2E | DEX-P2-022 | P2 |
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

