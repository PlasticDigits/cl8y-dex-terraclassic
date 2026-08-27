# GeckoTerminal Non-EVM adapters (GitLab #646)

Live Integration API on the indexer. Base URL submitted on the listing form: `https://indexer.dex.cl8y.com/gt`.

Spec: [GeckoTerminal Integration API Standards](https://docs.google.com/document/d/1ufjAJUa6rGO9PBGJGwfBMn-XMk9NE0ow3_iMYrS3drk).

| Path | Role |
|------|------|
| `GET /gt/latest-block` | Last indexed height (not chain tip if ingest lags) |
| `GET /gt/asset?id=` | CW20 contract or native denom (`uluna` / `uusd`) |
| `GET /gt/pair?id=` | Factory pair. `dexKey` = `cl8y`. 404 for gems / ALPHA / USTRIX / SpaceUSD |
| `GET /gt/events?fromBlock=&toBlock=` | Inclusive block range (max 2000). Swaps + join/exit. `reserves` are **post-event AMM state** persisted at ingest ([#684](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/684)), not the live `pair_reserves` snapshot. Missing history emits `"0"`. |

Handlers: [`indexer/src/api/gt.rs`](../../indexer/src/api/gt.rs). Math / backfill: [`indexer/src/indexer/gt_event_reserves.rs`](../../indexer/src/indexer/gt_event_reserves.rs). Form pack: [`docs/listings/forms/geckoterminal.md`](../../docs/listings/forms/geckoterminal.md). Verify: `make verify-issue-646` · `make verify-issue-684`. Skill: [`skills/AGENTS_INDEXER_GT_EVENT_RESERVES.md`](../../skills/AGENTS_INDEXER_GT_EVENT_RESERVES.md).

After indexer migrate: `cl8y-dex-indexer backfill-gt-event-reserves` (reverse-apply from current `pair_reserves`; never copy the tip onto every historical row).

`/cg/*` is CoinGecko **exchange** shape. Do not point GeckoTerminal at `/cg/`.
