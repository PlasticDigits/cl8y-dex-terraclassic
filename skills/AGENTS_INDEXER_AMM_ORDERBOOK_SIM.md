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
- [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) — query `depth` ≤ 100 **total** (Openware), cache 30s per `(pair, depth, fee_bps)`
- GitLab **#210** (implementation), **#221** (Openware total-depth split), **#105** (stub catalog)

## Depth query (Openware / CMC — #221)

- `cap_orderbook_depth` / `levels_per_side` in `orderbook_sim.rs`
- `depth=100` → **50** bids + **50** asks; default **20** → **10+10**; `depth=1` → **1+1**
- Cache keys use **requested** total `depth`, not per-side count
- Do **not** use Kujira FIN per-side semantics on CG/CMC listing endpoints

## Math (must stay aligned with pair pool swap)

- Integer `u128` reserves; **`ceil_div`** for new opposite reserve
- Step size: `R0 * (i / levels_per_side) * 0.10` for `i` in `1..=levels_per_side` (from query `depth`)
- Fee: `gross * fee_bps / 10000` on swap output (LCD `get_fee_config`, DB `pairs.fee_bps` fallback)

## Tests

```bash
cd indexer && cargo test orderbook_sim api_orderbook_lcd_mock -- --test-threads=1
```

Unit tests: `orderbook_sim.rs` `#[cfg(test)]`. Integration: `tests/api_orderbook_lcd_mock.rs` (wiremock LCD + Postgres).
