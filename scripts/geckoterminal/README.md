# GeckoTerminal Non-EVM adapters (GitLab #646)

Live Integration API on the indexer. Base URL submitted on the listing form: `https://indexer.dex.cl8y.com/gt`.

Spec: [GeckoTerminal Integration API Standards](https://docs.google.com/document/d/1ufjAJUa6rGO9PBGJGwfBMn-XMk9NE0ow3_iMYrS3drk).

| Path | Role |
|------|------|
| `GET /gt/latest-block` | Last indexed height (not chain tip if ingest lags) |
| `GET /gt/asset?id=` | CW20 contract or native denom (`uluna` / `uusd`) |
| `GET /gt/pair?id=` | Factory pair or eligible listing-only native projection. `dexKey` = `cl8y`. 404 for gems / ALPHA / USTRIX / SpaceUSD |
| `GET /gt/events?fromBlock=&toBlock=` | Inclusive block range (max 2000). **Factory** swap+join/exit rows ≤ **5000** (`MAX_GT_EVENT_ROWS`); over-cap is **400** before projections. Under-cap responses preserve all factory rows and append eligible projected swaps ([#694](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/694)). Swaps + join/exit. `reserves` are **post-event AMM state** persisted at ingest ([#684](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/684)), not the live `pair_reserves` snapshot. Legacy missing factory reserve history emits `"0"`; unavailable projected source/custody history omits the affected projected swap. |

Handlers: [`indexer/src/api/gt.rs`](../../indexer/src/api/gt.rs). Math / backfill: [`indexer/src/indexer/gt_event_reserves.rs`](../../indexer/src/indexer/gt_event_reserves.rs). Form pack: [`docs/listings/forms/geckoterminal.md`](../../docs/listings/forms/geckoterminal.md). Verify: `make verify-issue-646` · `make verify-issue-684` · `make verify-issue-694`. Skills: [`AGENTS_INDEXER_GT_EVENT_RESERVES.md`](../../skills/AGENTS_INDEXER_GT_EVENT_RESERVES.md), [`AGENTS_INDEXER_API4_PER_REQUEST.md`](../../skills/AGENTS_INDEXER_API4_PER_REQUEST.md).

After indexer migrate: `cl8y-dex-indexer backfill-gt-event-reserves` (reverse-apply from current `pair_reserves`; never copy the tip onto every historical row).

`/cg/*` is CoinGecko **exchange** shape. Do not point GeckoTerminal at `/cg/`.

Listing-only native markets use opaque `cl8y-virtual:` ids, never on-chain
factory addresses. Their swaps retain real indexed source transactions; mapper
principal requires height-attested custody at the configured treasury. Missing
backing or exact pinned source omits the affected projection. Factory reserves
and economic accounting remain unchanged. See the
[projection contract](../../docs/CG_CMC_COMPLIANCE.md#listing-only-native-markets-1437),
[LV1437 agent playbook](../../skills/AGENTS_LISTING_VIRTUAL_PAIRS.md) and
[verification checklist](../../docs/qa/issue-1437/README.md).
