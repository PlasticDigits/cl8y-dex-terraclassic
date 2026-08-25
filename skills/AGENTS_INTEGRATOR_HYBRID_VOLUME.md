# Agent skill: integrator hybrid volume reconciliation

**Audience:** third-party agents building CG/CMC/Vyntrex integrations or auditing indexer volume.

**GitLab:** [#216](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/216)

## Rule

Headline volume = **one** `swap_events` row per taker swap (`offer_amount` / `return_amount`). Never add `limit_order_fills` on top for the same `tx_hash`.

DeFiLlama `dailyVolume` uses this rule on a UTC calendar day (`GET /api/v1/defillama/daily`) — [`AGENTS_DEFILLAMA.md`](./AGENTS_DEFILLAMA.md) / [#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631).

## Fill ↔ swap linkage (#316)

`limit_order_fills.swap_event_id` resolves via per-pair **`swap_index`** (on-chain wasm attr since #331 when present; else parser walk ordinal), not `MIN(swap_events.id)`. Multi-swap same-pair txs: each fill links to the swap that produced it. See [integrators-hybrid-volume.md § Fill ↔ swap linkage](../docs/integrators-hybrid-volume.md#fill--swap-linkage-swap_event_id).

## Tests to run

```bash
cd indexer
cargo test swap_events_hybrid_columns api_consolidated_reporting api_integrator_hybrid_volume limit_fill_swap_linkage --tests
cargo test parse_limit_order_fills_assigns_swap_index --lib
```

## Key files

| Area | Path |
|------|------|
| Reconciliation guide | [docs/integrators-hybrid-volume.md](../docs/integrators-hybrid-volume.md) |
| Leg volume helper | [indexer/src/api/consolidated_stats.rs](../indexer/src/api/consolidated_stats.rs) `hybrid_leg_volumes` |
| 24h breakdown SQL | [indexer/src/db/queries/swap_events.rs](../indexer/src/db/queries/swap_events.rs) `get_24h_hybrid_breakdown` |
| Internal trades API | [indexer/src/api/pairs.rs](../indexer/src/api/pairs.rs) `TradeResponse` |
| CG/CMC | [indexer/src/api/cg.rs](../indexer/src/api/cg.rs), [cmc.rs](../indexer/src/api/cmc.rs) |

## Canonical docs

- [docs/integrators-hybrid-volume.md](../docs/integrators-hybrid-volume.md)
- [docs/indexer-invariants.md](../docs/indexer-invariants.md) — **L10**
- [docs/CG_CMC_COMPLIANCE.md](../docs/CG_CMC_COMPLIANCE.md)

## Related skills

- [AGENTS_HOOK_COMMISSION.md](./AGENTS_HOOK_COMMISSION.md) — L7 hook vs swap attr fees
- [AGENTS_TESTING_P2_EPIC.md](./AGENTS_TESTING_P2_EPIC.md) — hybrid column integration tests
