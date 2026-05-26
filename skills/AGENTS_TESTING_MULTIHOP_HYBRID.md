# Agent playbook: multihop hybrid regression tests

**Issue:** [GitLab **#192**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/192)  
**Invariant:** **L8** — hybrid quotes must match execution when `hybrid` is set on router ops ([`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md)).

## What this covers

Regression tests for **≥3-hop** router paths where **≥2 legs** carry non-zero `hybrid` / limit-book consumption. Without these, router composition and indexer `hybrid_by_hop` merge can drift silently.

## Contract integration (cw-multi-test)

| Test | File | Notes |
|------|------|-------|
| `router_two_hop_first_leg_hybrid_matches_simulate` | [`smartcontracts/tests/src/limit_order_tests.rs`](../smartcontracts/tests/src/limit_order_tests.rs) | Hybrid on hop 1 only |
| `router_three_hop_two_legs_hybrid_matches_simulate` | same | A→B→C→D; hybrid on hops 1 & 2; hop 3 pool-only |

**Harness helpers:**

- `setup_router_abc_env` — A/B + B/C pairs ([`smartcontracts/tests/src/lib.rs`](../smartcontracts/tests/src/lib.rs))
- `setup_router_abcd_env` — adds C/D pair for 3-hop paths

**Building per-hop hybrid splits:** each hop’s `pool_input + book_input` must equal that hop’s offer amount. For hop *n > 1*, simulate hops `1..n` first (router `SimulateSwapOperations`) to learn the intermediate offer, then assign splits (see `hybrid_params_split` in `limit_order_tests.rs`).

**Run:**

```bash
cd smartcontracts
cargo test router_three_hop_two_legs_hybrid_matches_simulate
cargo test router_two_hop_first_leg_hybrid_matches_simulate
```

## Indexer integration (Postgres + Wiremock LCD)

| Test | File | Seed |
|------|------|------|
| `route_solve_post_three_hop_multi_leg_hybrid` | [`indexer/tests/api_route_solve.rs`](../indexer/tests/api_route_solve.rs) | `seed_route_solve_3hop` |
| `route_solve_get_hybrid_optimize_three_hops` | same | `seed_route_solve_3hop` |

**Run (serialized — shared DB):**

```bash
cd indexer
export TEST_DATABASE_URL="${TEST_DATABASE_URL:-postgres://postgres:postgres@127.0.0.1:5432/dex_indexer_test}"
cargo test --test api_route_solve three_hop -j 1 -- --test-threads=1
```

LCD stubs: [`indexer/tests/common/lcd_mock.rs`](../indexer/tests/common/lcd_mock.rs) (`start_smart_query_data_mock`, `start_hybrid_route_optimizer_mock`).

## Docs cross-links

- [`docs/testing.md`](../docs/testing.md) — test types and commands
- [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) — route solve HTTP semantics
- [`docs/limit-orders.md`](../docs/limit-orders.md) — Pattern C / L8 quoting scope

## When to extend

Add cases here (not ad-hoc one-offs) when changing:

- Router multi-hop `hybrid` forwarding or simulate loop
- Indexer `apply_hybrid_by_hop` or `hybrid_optimize` hop cap (currently **3 hops** on GET optimize, **4 hops** on POST discovery)
- Frontend/indexer merge of `hybrid_by_hop` for routes with **>2** hybrid legs
