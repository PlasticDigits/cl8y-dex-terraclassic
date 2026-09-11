# Agent playbook: Limit reprice time-at-level FIFO (#1227)

Audience: third-party agents touching **`UpdateLimitOrderPrice`**, pair book insert/match, indexer book snapshot / `db_orderbook_sim`, or `/trade` price-only **Edit**.

**Issue:** [Forgejo **#1227**](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1227)  
**Invariant:** **L23** in [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md)  
**Product:** [`docs/limit-orders.md` § Ordering](../docs/limit-orders.md#ordering-composite-key-fifo)

## Problem class

`UpdateLimitOrderPrice` keeps `order_id` (**#247**: no cancel+replace, no second maker fee, no CW20). Equal-price insert used to compare that preserved id, so a globally older maker who later quoted `P` sorted **ahead** of makers already resting at `P`. Price priority across *different* prices was already correct. Closed [#424](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/424) documented the leapfrog as by-design; **#1227 is the matcher fix**. Do not reopen #424 or #247.

## Invariants (R1227-1–R1227-8)

| ID | Rule |
|----|------|
| **R1227-1** | Storage / cancel / Edit identity stays the original **`order_id`**. Relink does **not** mint a new id or punch holes in `ORDER_NEXT_ID`. |
| **R1227-2** | After unlink, insert as a **new arrival at that price**: positioning key `(price, RELINK_EQUAL_PRICE_SORT_ID)` (`u64::MAX`). Match / `simulate_match_*` walk the DLL (not a re-sort by id). |
| **R1227-3** | Same-price update (`new_price == old_price`) is a **no-op**. Repeat `P' → P → P'` cannot grind to the head of `P`. |
| **R1227-4** | Cross-price order is unchanged: a strictly better price is still closer to the head than worse resters. |
| **R1227-5** | New placements and [#266](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/266) equal-price **batch** rungs still insert by ascending `order_id`. `reserve_order_id_block` never assigns `u64::MAX`. |
| **R1227-6** | **L14** hints stay advisory. `hint_after_order_id` pointing at an earlier same-price maker cannot place the jumper *before* that maker. |
| **R1227-7** | Failed relink (`LimitInsertStepsExceeded`, pause, non-owner, expired, blacklist) is all-or-nothing: still linked at the old price with the same `remaining`. **L1:** success does not change `remaining` or `PENDING_ESCROW_*`. No CW20 on the update tx. |
| **R1227-8** | Indexer snapshot stores LCD walk order as **`resting_limit_orders.walk_index`**. `get_pair_resting_book` / `db_orderbook_sim` **ORDER BY walk_index** — never rebuild equal-price FIFO from `order_id ASC` (**L8**). |

## Do / don’t

- **Do** keep `#247` gas/UX: owner-only price relink, same row on Edit.
- **Do** treat arrival-at-level as the FIFO key after reprice; place-time id is identity only.
- **Don’t** charge a second maker fee or force cancel+place in the dApp while the chain msg exists.
- **Don’t** sort Postgres mirror by `(price, order_id)` after a snapshot that already walked the DLL.
- **Don’t** reopen #424 / #247 or invert #266 batch id assignment.

## Canonical code

| File | Role |
|------|------|
| [`orderbook.rs`](../smartcontracts/contracts/pair/src/orderbook.rs) | `RELINK_EQUAL_PRICE_SORT_ID`, `relink_limit_order_price`, `link_*_order_at_id(..., sort_id)` |
| [`contract.rs`](../smartcontracts/contracts/pair/src/contract.rs) | `execute_update_limit_order_price` (owner / pause / expiry / band) |
| [`resting_orders.rs`](../indexer/src/db/queries/resting_orders.rs) | `walk_index` insert + `ORDER BY walk_index` |
| [`20260911120000_resting_orders_walk_index.sql`](../indexer/migrations/20260911120000_resting_orders_walk_index.sql) | Schema |
| [`pair.ts`](../frontend-dapp/src/services/terraclassic/pair.ts) | `updateLimitOrderPrice` — still same `order_id` |

## Regression

```bash
make verify-issue-1227
```

```bash
cd smartcontracts && cargo test -p cl8y-dex-pair fifo_after_update_limit_order_price --quiet
cd smartcontracts && cargo test -p cl8y-dex-tests fifo_after_update_limit_order_price fifo_two_bids_same_price --quiet
```

Indexer (Postgres): `resting_book_walk_index_preserves_reprice_fifo` in `db_orderbook_mirror`.

## Related

- [`AGENTS_LIMIT_ORDER_BATCH_LADDER.md`](./AGENTS_LIMIT_ORDER_BATCH_LADDER.md) — #266 batch ids stay ascending; relink of one rung still joins the tail
- [`AGENTS_BOOK_MATCH_HINT_SECURITY.md`](./AGENTS_BOOK_MATCH_HINT_SECURITY.md) — **L14** / **L17** hints cannot invert FIFO
- [`AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md`](./AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md) — Edit still one tx, same id (**T11**)
- [`AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md) — **L8** sim = execute
- [`AGENTS_INDEXER_AMM_ORDERBOOK_SIM.md`](./AGENTS_INDEXER_AMM_ORDERBOOK_SIM.md) — CG/CMC synthetic depth is **not** the on-chain FIFO walk
- [`docs/runbooks/book-snapshot-mirror.md`](../docs/runbooks/book-snapshot-mirror.md) — snapshot walk order
