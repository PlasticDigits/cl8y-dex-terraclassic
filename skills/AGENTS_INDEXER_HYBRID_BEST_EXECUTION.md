# Indexer hybrid best execution + consolidated listing (GitLab #189)

Audience: third-party agents integrating Vyntrex, CG/CMC crawlers, or retail route clients against the CL8Y indexer.

## Best-execution route API

| Endpoint | When to use |
|----------|-------------|
| `GET /api/v1/route/solve?token_in=&token_out=&amount_in=` | **Default retail / integrator path** — server-chosen hybrid splits (max 3 hops) when `amount_in` is set ([GitLab **#191**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/191)). |
| `GET /api/v1/route/solve/best?token_in=&token_out=&amount_in=` | **Alias** for default hybrid GET — requires `amount_in` explicitly ([GitLab **#189**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/189)). |
| `GET /api/v1/route/solve?hybrid_optimize=true&amount_in=` | Deprecated explicit opt-in; equivalent to default GET with `amount_in`. |
| `GET /api/v1/route/solve?pool_only=true&amount_in=` | Explicit pool-only opt-out for legacy integrators (max 4 hops). |
| `POST /api/v1/route/solve` with `hybrid_by_hop` | Client-supplied splits (max 4 hops). |

Response includes `quote_kind` (`indexer_hybrid_lcd`, `indexer_hybrid_lcd_degraded`, etc.), `hybrid_notes`, and `router_operations` with merged `terra_swap.hybrid` params.

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
cd indexer && cargo test api_route_solve api_consolidated_reporting swap_events_hybrid_columns -- --test-threads=1
```

## Related invariants

[docs/indexer-invariants.md](../docs/indexer-invariants.md) — route GET hybrid default ([#191](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/191)), GET best execution ([#189](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/189)), CG/CMC consolidated reporting rows.
