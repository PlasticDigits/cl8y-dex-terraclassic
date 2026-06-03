# Agent skill: Hybrid match `book_start_hint` side validation (L17 / GitLab #272)

## When to use

You touch **hybrid swap execution**, **book matching** (`match_bids` / `match_asks`), **`simulate_match_*`**, or client code that sets **`book_start_hint`** on `HybridSwapParams`.

## Invariant (L17)

- `book_start_hint` is **permissionless** on `Cw20HookMsg::Swap` — any wallet can set it.
- The matcher leg is chosen by **which token the taker sends** (`token0` → `match_bids`, `token1` → `match_asks`), not by the hint.
- **Never** start a walk from a hinted order unless `ORDERS[hint].side` equals the matcher side.
- **Never** fill, park, or debit escrow for an order whose `side` does not match the active matcher (skip and follow `next`).

Wrong-side hints must **fall back to head** silently (same UX as stale/missing id).

## Canonical docs

- Invariant **L17:** [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md)
- Product: [`docs/limit-orders.md` § Swap with Pattern C](../docs/limit-orders.md#swap-with-pattern-c-cw20hookmsgswap)
- Implementation: [`orderbook.rs`](../smartcontracts/contracts/pair/src/orderbook.rs) — `resolve_match_start_hint`, `order_on_match_side`
- Related insert hints (**L14**, #256/#265): [`skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md`](./AGENTS_LIMIT_ORDER_BATCH_LADDER.md)
- Hybrid quoting parity (**L8**): [`skills/AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md)

## Tests to run after changes

```bash
cd smartcontracts && cargo test -p cl8y-dex-pair book_start_hint_side_tests
cd smartcontracts && cargo test -p cl8y-dex-pair proptest_limits::prop_match_bids_adversarial_wrong_side_hint
cd smartcontracts && cargo test -p cl8y-dex-tests hybrid_wrong_side_book_start_hint
cd smartcontracts && cargo test -p cl8y-dex-tests match_invalid_book_start_hint_falls_back_to_head
cd smartcontracts && cargo test -p cl8y-dex-tests hybrid_same_side_book_start_hint_still_matches
```

## Do not regress

- Reintroducing existence-only hint checks (`ORDERS.may_load(h).is_some()` without `side`).
- Removing the per-step `order_on_match_side` guard (defense in depth for corrupted `next` links).
- Debiting `PENDING_ESCROW_TOKEN1` on ask fills or `PENDING_ESCROW_TOKEN0` on bid fills.
