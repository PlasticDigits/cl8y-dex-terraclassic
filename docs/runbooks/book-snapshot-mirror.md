# Book snapshot mirror (Phase 1b)

GitLab [#322](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/322) — background writer for `pair_reserves` and `resting_limit_orders` (Phase 1a schema).

## What runs

`run_book_snapshot_loop` in [`indexer/src/indexer/book_snapshot.rs`](../../indexer/src/indexer/book_snapshot.rs) is spawned from [`poller.rs`](../../indexer/src/indexer/poller.rs) alongside the oracle and tier-sync loops. Each cycle:

1. Reads latest block height (best-effort).
2. For every row in `pairs`, queries LCD for pool reserves, fee config, and the full resting book (bid + ask FIFO walks).
3. Upserts `pair_reserves` and atomically replaces `resting_limit_orders` per pair.

Per-pair LCD failures are logged and skipped; the pair keeps its last good snapshot.

## Configuration

| Env | Default | Meaning |
|-----|---------|---------|
| `BOOK_SNAPSHOT_INTERVAL_MS` | `10000` | Target cadence between full snapshot cycles |

## Freshness contract (Phase 1c)

Constants in `book_snapshot.rs`:

| Constant | Value | Use |
|----------|-------|-----|
| `BOOK_SNAPSHOT_DEFAULT_INTERVAL_MS` | 10_000 | Documented default cadence |
| `BOOK_SNAPSHOT_STALENESS_TOLERANCE_CYCLES` | 2 | TTL multiplier (one missed cycle) |
| `BOOK_SNAPSHOT_MAX_STALENESS_MS` | 20_000 | Import in Phase 1c: treat older `snapshot_at` as stale |

**Degrade-not-error:** missing reserves row, empty book, or stale `snapshot_at` → Phase 1c falls back to LCD or marks the quote degraded; never hard-fail the whole solve.

**Block lag:** compare cycle `block_height` on mirror rows vs current chain head.

## LCD budget

Per cycle (documented upper bound):

```
1 + (pairs × 4) + total_resting_orders
```

Fixed four calls per pair: `pool`, `get_fee_config`, `order_book_head` (bid), `order_book_head` (ask), plus one `limit_order` per resting order. See `book_snapshot_lcd_budget()`.

## Tests

- Unit: `cargo test -p cl8y-dex-indexer book_snapshot::tests`
- Integration (Postgres): `cargo test -p cl8y-dex-indexer --test book_snapshot_loop`

## Related

- Phase 1a schema: [#279](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/279)
- Phase 1c consumer: [#319](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/319)
- Indexer invariants: [`docs/indexer-invariants.md`](../indexer-invariants.md)
