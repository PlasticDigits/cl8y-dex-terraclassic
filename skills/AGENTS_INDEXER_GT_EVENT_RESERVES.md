# Agent playbook: GeckoTerminal `/gt/events` post-event reserves (GitLab #684)

Audience: third-party agents changing indexer `/gt/*`, swap ingest, or `pair_reserves`.

**Issue:** [GitLab **#684**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/684)  
**Parent:** [#646](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/646) adapters (`make verify-issue-646`)  
**Catalog:** [`AGENTS_LISTINGS.md`](./AGENTS_LISTINGS.md) (**L639-2**)  
**Invariants:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (GeckoTerminal #684)

## Problem class

`GET /gt/events` stamped the **live** `pair_reserves` snapshot on every swap/join/exit. Integrators saw identical `reserves.asset0` / `asset1` across blocks even when `asset0In` / `asset1Out` proved the pool moved.

## Do / don’t

- **Do** persist post-event AMM `RESERVES` on `swap_events.reserve_0/1` and `liquidity_events.reserve_0/1` at ingest.
- **Do** apply only the **pool** leg: `pool_input = offer − limit_book_offer_consumed` (legacy NULL hybrid = whole offer). `gross_output = pool_return + pool commission`. Book-only (`pool_return = 0`, `book_return > 0`) leaves reserves unchanged.
- **Do** serve those columns from `/gt/events`. NULL → `"0"` (never today’s snapshot).
- **Do** backfill with reverse-apply from current `pair_reserves` (`cl8y-dex-indexer backfill-gt-event-reserves`). Invert deltas newest → oldest.
- **Don’t** JOIN `pair_reserves` on the `/gt/events` GET path. That table stays the solver’s **current** LCD mirror.
- **Don’t** treat `return_amount` (pool+book) as the reserve drain.
- **Don’t** LCD `Pool {}` per event on GET or stamp the live snapshot onto every historical row.
- **Don’t** emit USD, `$1` UST1, `2.5×` USTR, hub marks, or TVL on `/gt/events`.
- **Don’t** leak gems / ALPHA / USTRIX / SpaceUSD (**L639-2**).
- **Don’t** “fix” `/cg/tickers` `liquidity_in_usd` here ([#685](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/685)).
- **Don’t** emit pair wasm `reserve_*` from a non-pair `_contract_address` (**#285**).

## Canonical code

| File | Role |
|------|------|
| `indexer/migrations/20260827160000_gt_event_post_reserves.sql` | Nullable `reserve_0/1` on swap + liquidity tape |
| `indexer/src/indexer/gt_event_reserves.rs` | Pool-leg math, last-persisted lookup, reverse-apply backfill |
| `indexer/src/indexer/parser.rs` | Persist after each swap/join/exit (in-block cache) |
| `indexer/src/api/gt.rs` | GET reads row columns; `"0"` if NULL |
| `indexer/src/main.rs` | `backfill-gt-event-reserves` |

## Invariants (R684-1–R684-8)

1. **R684-1** — Event `reserves` = AMM `RESERVES` after that event (factory asset_0 / asset_1, decimalized like #646). Not book escrow, donations, wrap inventory, or sweep.
2. **R684-2** — Hybrid book notional does not move reserves. Book-only swaps keep the previous post-event pair state.
3. **R684-3** — `GET /gt/events` does not SELECT `pair_reserves`. Solver snapshot loop unchanged.
4. **R684-4** — GET stays cheap: max 2000-block span, `i64` binds, inverted/oversize range **400**. No history walk to rebuild reserves per request.
5. **R684-5** — Missing columns emit `"0"` / `"0"`. Backfill must invert; copying the tip snapshot onto every row is forbidden.
6. **R684-6** — Gems stay 404 / omitted. `dexKey` = `cl8y`. Orientation is factory indices, not offer/ask.
7. **R684-7** — Reorg: `--cleanup-derived` deletes tape rows at `height >= H` (same as other event tables). Cursor-only rewind + `ON CONFLICT DO NOTHING` keeps existing reserves — use cleanup when canonical txs changed.
8. **R684-8** — Optional wasm `reserve_0` / `reserve_1` are gold-standard when present on the pair’s `_contract_address`. Indexer works without a factory migrate.

## Ops

After indexer migrate on Coolify:

```bash
cl8y-dex-indexer backfill-gt-event-reserves
```

No dApp change. No GeckoTerminal form resubmit unless the vendor asks.

Live ingest does **not** LCD-seed the first event on a pair (that would stamp current pool onto catch-up heights). First events stay NULL until backfill; afterward ingest applies forward from last persisted + in-block cache.

## Verify

```bash
make setup-indexer-postgres   # if indexer/.env missing
make verify-issue-684
make verify-issue-646         # routes / form pack still green
```

Related: `make verify-issue-639`.
