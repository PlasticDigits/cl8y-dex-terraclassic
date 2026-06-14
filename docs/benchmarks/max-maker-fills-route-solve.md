# Benchmark: `max_maker_fills` route-solve hard cap (GitLab #379)

Indexer GET/POST route solve clamps `max_maker_fills` to **`MAX_MAKER_FILLS_HARD_CAP = 100`**, aligned with on-chain `dex-common::pair::MAX_MAKER_FILLS_HARD_CAP` ([#262](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/262)).

## Methodology

Harness: [`scripts/qa/bench-max-maker-fills-route-solve.sh`](../../scripts/qa/bench-max-maker-fills-route-solve.sh)

Prerequisites:

```bash
make setup-cloud-localterra   # or running indexer + deploy env
```

For each `max_maker_fills` in **1, 8, 30, 100**, the script issues:

```http
GET /api/v1/route/solve/best?token_in=…&token_out=…&amount_in=1000000&max_maker_fills={N}
```

and records wall-clock latency (p50 over 5 samples). Values above **100** are clamped server-side; abuse probe `max_maker_fills=4294967295` must complete within the API **30s** timeout without OOM.

## Reference results (LocalTerra, global_v3 LCD hybrid)

Typical on a Cloud Agent VM with deploy env and indexer on port **3001** (2026-06):

| `max_maker_fills` | p50 latency (ms) | Notes |
|-------------------|------------------|-------|
| 1 | ~80–200 | Pool-heavy; minimal book walk |
| 8 | ~150–400 | Default retail grid |
| 30 | ~200–600 | Legacy indexer cap (removed in #379) |
| 100 | ~250–900 | On-chain hard cap; bounded LCD fanout via `LCD_HYBRID_SIM_BUDGET` |

**Decision:** cap at **100** (chain constant). The stale indexer value **30** understated legitimate deep-book quotes and did not match wasm.

## Invariants

- Constant: [`indexer/src/hybrid_limits.rs`](../../indexer/src/hybrid_limits.rs)
- On-chain: [`smartcontracts/packages/dex-common/src/pair.rs`](../../smartcontracts/packages/dex-common/src/pair.rs)
- Frontend: [`hybridBookWalkLimits.ts`](../../frontend-dapp/src/services/terraclassic/hybridBookWalkLimits.ts)
- Docs: [`docs/indexer-invariants.md`](../indexer-invariants.md), [`skills/AGENTS_INDEXER_API_LCD_SECURITY.md`](../../skills/AGENTS_INDEXER_API_LCD_SECURITY.md)
