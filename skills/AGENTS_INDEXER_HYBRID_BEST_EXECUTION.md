# Indexer hybrid best execution + consolidated listing (GitLab #189, #209)

Audience: third-party agents integrating Vyntrex, CG/CMC crawlers, or retail route clients against the CL8Y indexer.

## Best-execution route API

| Endpoint | When to use |
|----------|-------------|
| `GET /api/v1/route/solve?token_in=&token_out=&amount_in=` | **Default retail / integrator path** — **global best execution** (`solver_version`: `global_v1` or `global_v2`): top-5 paths by hop count, joint hybrid splits, max **4 hops** when `amount_in` is set ([#209](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/209), [#191](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/191), [#323](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/323)). Optional **`trader`** / **`sender`** for CL8Y fee-tier quote parity — pair `HybridSimulation` discount math ([#238](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/238)); indexer/frontend wiring ([#245](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/245)). See [`AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md). |
| `GET /api/v1/route/solve/best?token_in=&token_out=&amount_in=` | **Alias** — requires `amount_in` ([#189](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/189)). Accepts same optional **`trader`** / **`sender`**. |
| `GET /api/v1/route/solve?hybrid_optimize=true&amount_in=` | Deprecated explicit opt-in; equivalent to default GET with `amount_in`. |
| `GET /api/v1/route/solve?pool_only=true&amount_in=` | Pool-only opt-out (max 4 hops, no global solver). |
| `POST /api/v1/route/solve` with `hybrid_by_hop` | **Override** splits (max 4 hops, first BFS path); not the global optimizer. Optional body **`trader`** / **`sender`** forwarded to LCD sim ([#245](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/245)). |

### Response fields (#209)

| Field | Meaning |
|-------|---------|
| `solver_version` | `global_v1` (LCD grid) or `global_v2` (Postgres mirror grid, `ROUTE_SOLVER_DB_HYBRID=1`) |
| `paths_considered` | Simple paths evaluated (≤ 5) |
| `optimality_scope` | Human-readable bound (not unbounded “global optimal”) |
| `lcd_hybrid_queries` | LCD `HybridSimulation` calls during optimization (legacy / per-hop fallback) |
| `db_hybrid_queries` | Postgres mirror grid evals (`global_v2`) |
| `fidelity_check` | `passed` \| `drift` \| `skipped` — router sim vs mirror grid (#319) |
| `hybrid_notes` | Degradation + liability boundary |
| `quote_kind` | `indexer_hybrid_db`, `indexer_hybrid_db_degraded`, `indexer_pool_db`, or legacy `*_lcd` kinds |

Read `optimality_scope` before marketing “best price” — optimality is **within documented search bounds** only ([ADR 0002](../docs/adr/0002-global-best-execution-route-solver.md), full guide: [route-solver.md](../docs/route-solver.md)).

## Terraport-compatible swap events

Hybrid swaps emit baseline Terraport attrs plus CL8Y extensions. Mapping table: [docs/integrators.md](../docs/integrators.md#vyntrex--terraport-hybrid-event-mapping-gitlab-189).

Key invariant: **`return_amount` is the consolidated total**; do not add `limit_order_fills` volume on top of swap totals.

## CG/CMC consolidated reporting

- Standard volume fields = consolidated totals from `swap_events`.
- Optional `cl8y_extensions` on `/cg/tickers` and `/cmc/summary`.
- Optional `pool_leg_volume` / `book_leg_volume` on trade endpoints when hybrid columns indexed.

Full spec: [docs/CG_CMC_COMPLIANCE.md](../docs/CG_CMC_COMPLIANCE.md#consolidated-hybrid--pool-only-reporting-gitlab-189).

## Zero-reserve pairs (GitLab #369)

Under `ROUTE_SOLVER_DB_HYBRID=1`, indexed pairs with **zero** `pair_reserves` on either side are normal (unfunded `create_pair`). The global solver must **not** return **502** when a longer candidate path touches such a pair but a **shorter funded path** exists (e.g. direct pool). Behavior:

- Mirror load marks `0/0` reserves as `EmptyPool` freshness → per-hop LCD fallback (same degradation bucket as missing mirror).
- Path candidates that cannot simulate any hop (zero pool + no viable book) are **skipped**; the request succeeds on the best remaining path or **404** when every enumerated path is unusable.

Regression: `route_solve_db_hybrid_skips_zero_reserve_path_candidate` in `indexer/tests/api_route_solve_db_hybrid.rs`.

## Tests to run after changes

```bash
cd indexer && cargo test --test api_route_solve --test api_route_solve_db_hybrid --test db_orderbook_mirror -- --test-threads=1
```

Multi-path regression: `route_solve_global_picks_best_path_not_shortest`, `route_solve_global_response_metadata_contract`.

Distant-pair latency + progress (#485): [`AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md`](./AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md); `make verify-issue-485`.

## Rate limits and LCD budgets ([#239](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/239))

- `GET /route/solve` and `/route/solve/best` are **LCD-heavy** (stricter **10 RPS** per IP by default).
- Hybrid optimization is bounded by **`LCD_HYBRID_SIM_BUDGET`**; LCD failures return generic **502** (`Upstream LCD query failed`), not raw `LcdError` text.
- Agent playbook: [`AGENTS_INDEXER_API_LCD_SECURITY.md`](./AGENTS_INDEXER_API_LCD_SECURITY.md).

## `book_start_hint` on optimized hops ([#332](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/332))

When `solver_version` is **`global_v2`** and a hop has `book_input > 0` with a **fresh** Postgres mirror, `optimize_hop_hybrid` sets `book_start_hint` to the first live resting order on the taker's match side (from `resting_limit_orders`, expired rows filtered). Stale/missing mirror → `null` (LCD fallback grid omits hint). Pool-only hops → `null`. The same hint is forwarded to mirror `HybridSimulation` math and LCD fallback is hint-free.

Side safety: hint rows must match bid/ask for the offer token; corrupt wrong-side mirror rows are skipped. On-chain **L17** ([#272](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/272)) still validates at execute — see [`AGENTS_BOOK_MATCH_HINT_SECURITY.md`](./AGENTS_BOOK_MATCH_HINT_SECURITY.md).

Regression: `route_solve_db_hybrid_book_start_hint_paths` in `indexer/tests/api_route_solve_db_hybrid.rs`.

## Zero-reserve path candidates ([#369](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/369))

Unfunded pairs (`pair_reserves` with `reserve_0 = reserve_1 = 0`) are normal after `create_pair` until LP is added. Under `ROUTE_SOLVER_DB_HYBRID=1`, a path candidate that includes a zero-reserve hop must be **skipped** during concurrent evaluation — not fail the whole `GET /route/solve` with **502** "Route mirror simulation failed" when a direct pool route is viable.

Regression: `route_solve_db_hybrid_skips_zero_reserve_path_candidate` in `indexer/tests/api_route_solve_db_hybrid.rs`; `make verify-issue-369`.

## Related invariants

[docs/indexer-invariants.md](../docs/indexer-invariants.md) — route GET global best execution ([#209](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/209)), GET best ([#189](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/189)), hybrid GET cache tier ([#283](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/283)), CG/CMC consolidated reporting rows.
