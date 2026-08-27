# QA — GitLab #684 GeckoTerminal `/gt/events` post-event reserves

Verify (Postgres): `make verify-issue-684`

Playbook: [`skills/AGENTS_INDEXER_GT_EVENT_RESERVES.md`](../../../skills/AGENTS_INDEXER_GT_EVENT_RESERVES.md) · parent adapters [`scripts/geckoterminal/README.md`](../../../scripts/geckoterminal/README.md) · invariants **R684-1–R684-8**.

## Automated

- Migration adds nullable `reserve_0` / `reserve_1` on `swap_events` and `liquidity_events`
- Ingest reconstructs pool-leg deltas (hybrid book does not move reserves)
- `GET /gt/events` reads those columns; NULL → `"0"`; no `pair_reserves` JOIN
- Backfill reverse-applies from the current snapshot (oldest ≠ tip when swaps exist)
- Gems omitted; span > 2000 / inverted range **400**
- `#646` routes / form pack still green (`make verify-issue-646`)

## Manual / operator (not this verify)

1. Coolify: apply `20260827160000_gt_event_post_reserves.sql`, redeploy indexer.
2. Run `cl8y-dex-indexer backfill-gt-event-reserves`.
3. Pick a factory pair with ≥2 pool swaps in a ≤2000-block window and confirm `reserves` change across events. Do **not** paste integrator chat.

## Out of scope here

- Pair wasm `reserve_0` / `reserve_1` attrs (optional gold-standard; parser already accepts them).
- `/cg/tickers` `liquidity_in_usd` ([#685](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/685)).
- GeckoTerminal form resubmit.
- Protocol / pool list TVL.
