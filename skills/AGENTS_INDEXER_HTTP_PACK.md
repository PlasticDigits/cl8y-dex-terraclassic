# Agent playbook: indexer HTTP pack (#1204)

Use when an agent or integrator needs copy-paste curls for swaps tape, pools, treasury fees, hook burns, or volume windows.

This pack does **not** add routes, query params, ingest, or frontend copy.

**Pack:** [`docs/indexer-http.md`](../docs/indexer-http.md)  
**Issue:** [Forgejo **#1204**](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1204)  
**Invariants:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **HTTP pack #1204**, plus **L10**)  
**Execute path (not this pack):** [#707 pair-direct vs solver guidance](../docs/integrators.md#pair-swap-pool-only-vs-best-execution-forgejo-707)
**Verify:** `make verify-issue-1204`

## Invariants (I1204-1–I1204-8)

| ID | Rule |
|----|------|
| **I1204-1** | One pack: `docs/indexer-http.md`, linked from `docs/README.md` and the top of `docs/integrators.md`. |
| **I1204-2** | Five families stay in the path table and the example curls: swaps tape, pools, fees, burns/hooks, volume windows. |
| **I1204-3** | Three clocks stay labeled: trailing `24h\|7d\|30d`, UTC series `protocol/volume/daily` and `protocol/fees/daily`, DeFiLlama UTC day. |
| **I1204-4** | Burns are `GET /api/v1/hooks` (`after_swap_burn`). Treasury fees are `GET /api/v1/protocol/fees`. Terra Classic burn tax is neither. No `/burns` route. |
| **I1204-5** | Raw volume fields are not human USD. `null` is unpriced, `"0"` is idle, a decimal string is priced USD. **L10:** do not SUM `limit_order_fills` into pair volume. |
| **I1204-6** | `ApiDoc.paths` includes `/health`, `/api/v1/health/fee-discount`, and `/api/v1/compliance/blacklist-check`. No new Axum routes. |
| **I1204-7** | Placeholders only (`$INDEXER`, `$PAIR`, `$TOKEN`, `$TRADER`, `$HOOK`). No secrets, no wallet dumps. LCD-heavy (`route/solve`, blacklist-check) stays in the appendix as one shot. |
| **I1204-8** | `make verify-issue-1204` fails if a five-surface path leaves the markdown pack or `ApiDoc` JSON. |

## Rules of thumb

1. History and quotes are HTTP. On-chain execute stays on #707.
2. Do not invent `/burns`, `/swaps`, or `/api/v1/listings`.
3. Do not document a loop of `GET /api/v1/route/solve` or `/limit-book`.
4. Official pool list is `GET /api/v1/pairs`, not `/gt`.

## Verification

```bash
make verify-issue-1204
```

The harness greps the pack and runs `openapi_pack_includes_five_surfaces_and_health`. Postgres integration tests (`security`, `api_hooks`, `api_pairs`, protocol fees/volume) stay the behavior suite; this ticket does not change their SQL.
