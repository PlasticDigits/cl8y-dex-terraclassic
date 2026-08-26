# GeckoTerminal Non-EVM adapters (GitLab #646)

Live Integration API on the indexer. Base URL submitted on the listing form: `https://indexer.dex.cl8y.com/gt`.

Spec: [GeckoTerminal Integration API Standards](https://docs.google.com/document/d/1ufjAJUa6rGO9PBGJGwfBMn-XMk9NE0ow3_iMYrS3drk).

| Path | Role |
|------|------|
| `GET /gt/latest-block` | Last indexed height (not chain tip if ingest lags) |
| `GET /gt/asset?id=` | CW20 contract or native denom (`uluna` / `uusd`) |
| `GET /gt/pair?id=` | Factory pair. `dexKey` = `cl8y`. 404 for gems / ALPHA / USTRIX / SpaceUSD |
| `GET /gt/events?fromBlock=&toBlock=` | Inclusive block range (max 2000). Swaps + join/exit |

Handlers: [`indexer/src/api/gt.rs`](../../indexer/src/api/gt.rs). Form pack: [`docs/listings/forms/geckoterminal.md`](../../docs/listings/forms/geckoterminal.md). Verify: `make verify-issue-646`.

`/cg/*` is CoinGecko **exchange** shape. Do not point GeckoTerminal at `/cg/`.
