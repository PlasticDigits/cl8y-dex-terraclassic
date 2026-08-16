# Agent skill: Hybrid match `book_start_hint` side validation (L17 / GitLab #272)

## When to use

You touch **hybrid swap execution**, **book matching** (`match_bids` / `match_asks`), **`simulate_match_*`**, or client code that sets **`book_start_hint`** on `HybridSwapParams`.

## Invariant (L17)

- `book_start_hint` is **permissionless** on `Cw20HookMsg::Swap` — any wallet can set it.
- The matcher leg is chosen by **which token the taker sends** (`token0` → `match_bids`, `token1` → `match_asks`), not by the hint.
- **Never** start a walk from a hinted order unless `ORDERS[hint].side` equals the matcher side.
- **Never** fill, park, or debit escrow for an order whose `side` does not match the active matcher (skip and follow `next`).

Wrong-side hints must **fall back to head** silently (same UX as stale/missing id).

## Invariant (L18 / GitLab #470)

- After computing `cost = floor(fill × price)` (and the too-expensive shrink loop), if **`cost == 0`** while **`fill > 0`**, **skip** the order — do not debit maker escrow or credit a zero counter-leg payout.
- Apply symmetrically in **`match_bids`**, **`match_asks`**, **`simulate_match_*`**, and indexer **`db_orderbook_sim`** so quotes match execute (**L8**).

## Invariant (L20 / GitLab #467, decimals-normalized #529)

- **`validate_limit_order_price(price, decimals0, decimals1)`** in `dex-common::limit_placement` gates batch placement, ladder expansion, and **`UpdateLimitOrderPrice`**: **[`MIN_LIMIT_PRICE`, `MAX_LIMIT_PRICE`]** = **[1e-9, 1e9]** apply to the **human-scale** price `raw × 10^(dec0 − dec1)` ([#529](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/529)). Execution keeps raw token1-units/token0-units.
- **`match_bids` / `match_asks` / `simulate_match_*`:** on `checked_mul_floor` overflow for `1/price` or `fill × price`, **skip** the maker (`continue`) — do not revert the whole swap (legacy rows predating the band).
- Constants: [`MIN_LIMIT_PRICE`](../smartcontracts/packages/dex-common/src/limit_placement.rs), [`MAX_LIMIT_PRICE`](../smartcontracts/packages/dex-common/src/limit_placement.rs). Playbook: [`AGENTS_LIMIT_PRICE_DECIMALS.md`](./AGENTS_LIMIT_PRICE_DECIMALS.md).

## Expired head-clog mitigation ([#289](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/289))

When the book head is a long **expired** prefix, a head-only hybrid walk can hit **`MAX_SCAN_STEPS` (500)** before live liquidity. **Integrators should set `book_start_hint` to the first live order on the matcher side** (bid hint for `match_bids`, ask hint for `match_asks`) so `resolve_match_start_hint` starts past the clog. Keepers use resumable **`CleanLimitBook`** (**#274**). The indexer **`global_v2`** route optimizer emits this hint automatically when the Postgres mirror is fresh ([#332](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/332)); LCD-only (`global_v1`) and stale-mirror fallbacks still use `null`.

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
cd smartcontracts && cargo test -p cl8y-dex-tests match_asks_skips_zero_cost_fill_sub_unity_price
cd smartcontracts && cargo test -p cl8y-dex-tests match_bids_skips_zero_cost_fill_sub_unity_price
cd smartcontracts && cargo test -p cl8y-dex-tests place_limit_order_dust_price_rejected
cd smartcontracts && cargo test -p cl8y-dex-tests dust_ask_brick_attack_prevented_valid_ask_still_fills
cd smartcontracts && cargo test -p cl8y-dex-pair limit_price_band_tests
make verify-issue-467
make verify-issue-529
```

## Do not regress

- Reintroducing existence-only hint checks (`ORDERS.may_load(h).is_some()` without `side`).
- Removing the per-step `order_on_match_side` guard (defense in depth for corrupted `next` links).
- Debiting `PENDING_ESCROW_TOKEN1` on ask fills or `PENDING_ESCROW_TOKEN0` on bid fills.
