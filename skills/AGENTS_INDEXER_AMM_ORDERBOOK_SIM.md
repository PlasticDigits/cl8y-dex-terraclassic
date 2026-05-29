# Indexer: AMM-simulated orderbook (CG/CMC)

Use when changing **`indexer/src/api/orderbook_sim.rs`**, **`/cg/orderbook`**, **`/cmc/orderbook/*`**, or CG/CMC compliance docs for synthetic depth.

## Do not confuse

| Surface | What it is |
|---------|------------|
| `orderbook_sim.rs` | Constant-product **curve walk** + pool `fee_bps` for listing APIs |
| Pair `orderbook` / indexer `limit-book` | On-chain **FIFO limit** resting orders |
| `tests/common/lcd_mock.rs` | Wiremock **LCD HTTP** only — not orderbook logic |

## Normative spec

- [`docs/CG_CMC_COMPLIANCE.md`](../docs/CG_CMC_COMPLIANCE.md) § AMM Orderbook Simulation
- [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) — depth ≤ 100, cache 30s per `(pair, depth, fee_bps)`
- GitLab **#210** (implementation), **#105** (stub catalog)

## Math (must stay aligned with pair pool swap)

- Integer `u128` reserves; **`ceil_div`** for new opposite reserve
- Step size: `R0 * (i / depth) * 0.10` for `i` in `1..=depth`
- Fee: `gross * fee_bps / 10000` on swap output (LCD `get_fee_config`, DB `pairs.fee_bps` fallback)

## Tests

```bash
cd indexer && cargo test orderbook_sim api_orderbook_lcd_mock -- --test-threads=1
```

Unit tests: `orderbook_sim.rs` `#[cfg(test)]`. Integration: `tests/api_orderbook_lcd_mock.rs` (wiremock LCD + Postgres).
