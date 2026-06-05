# Indexer hybrid best execution + consolidated listing (GitLab #189, #209)

Audience: third-party agents integrating Vyntrex, CG/CMC crawlers, or retail route clients against the CL8Y indexer.

## Best-execution route API

| Endpoint | When to use |
|----------|-------------|
| `GET /api/v1/route/solve?token_in=&token_out=&amount_in=` | **Default retail / integrator path** — **global best execution** (`solver_version`: `global_v1`): top-5 paths by hop count, joint hybrid splits, max **3 hops** when `amount_in` is set ([#209](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/209), [#191](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/191)). Optional **`trader`** / **`sender`** for CL8Y fee-tier quote parity — pair `HybridSimulation` discount math ([#238](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/238)); indexer/frontend wiring ([#245](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/245)). See [`AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md). |
| `GET /api/v1/route/solve/best?token_in=&token_out=&amount_in=` | **Alias** — requires `amount_in` ([#189](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/189)). Accepts same optional **`trader`** / **`sender`**. |
| `GET /api/v1/route/solve?hybrid_optimize=true&amount_in=` | Deprecated explicit opt-in; equivalent to default GET with `amount_in`. |
| `GET /api/v1/route/solve?pool_only=true&amount_in=` | Pool-only opt-out (max 4 hops, no global solver). |
| `POST /api/v1/route/solve` with `hybrid_by_hop` | **Override** splits (max 4 hops, first BFS path); not the global optimizer. Optional body **`trader`** / **`sender`** forwarded to LCD sim ([#245](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/245)). |

### Response fields (#209)

| Field | Meaning |
|-------|---------|
| `solver_version` | e.g. `global_v1` |
| `paths_considered` | Simple paths evaluated (≤ 5) |
| `optimality_scope` | Human-readable bound (not unbounded “global optimal”) |
| `lcd_hybrid_queries` | Approximate pair-level `HybridSimulation` call count |
| `hybrid_notes` | Degradation + liability boundary |
| `quote_kind` | `indexer_hybrid_lcd`, `indexer_hybrid_lcd_degraded`, etc. |

Read `optimality_scope` before marketing “best price” — optimality is **within documented search bounds** only ([ADR 0002](../docs/adr/0002-global-best-execution-route-solver.md), full guide: [route-solver.md](../docs/route-solver.md)).

## Terraport-compatible swap events

Hybrid swaps emit baseline Terraport attrs plus CL8Y extensions. Mapping table: [docs/integrators.md](../docs/integrators.md#vyntrex--terraport-hybrid-event-mapping-gitlab-189).

Key invariant: **`return_amount` is the consolidated total**; do not add `limit_order_fills` volume on top of swap totals.

## CG/CMC consolidated reporting

- Standard volume fields = consolidated totals from `swap_events`.
- Optional `cl8y_extensions` on `/cg/tickers` and `/cmc/summary`.
- Optional `pool_leg_volume` / `book_leg_volume` on trade endpoints when hybrid columns indexed.

Full spec: [docs/CG_CMC_COMPLIANCE.md](../docs/CG_CMC_COMPLIANCE.md#consolidated-hybrid--pool-only-reporting-gitlab-189).

## Tests to run after changes

```bash
cd indexer && cargo test --test api_route_solve -- --test-threads=1
```

Multi-path regression: `route_solve_global_picks_best_path_not_shortest`, `route_solve_global_response_metadata_contract`.

## Rate limits and LCD budgets ([#239](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/239))

- `GET /route/solve` and `/route/solve/best` are **LCD-heavy** (stricter **10 RPS** per IP by default).
- Hybrid optimization is bounded by **`LCD_HYBRID_SIM_BUDGET`**; LCD failures return generic **502** (`Upstream LCD query failed`), not raw `LcdError` text.
- Agent playbook: [`AGENTS_INDEXER_API_LCD_SECURITY.md`](./AGENTS_INDEXER_API_LCD_SECURITY.md).

## Related invariants

[docs/indexer-invariants.md](../docs/indexer-invariants.md) — route GET global best execution ([#209](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/209)), GET best ([#189](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/189)), hybrid GET cache tier ([#283](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/283)), CG/CMC consolidated reporting rows.
