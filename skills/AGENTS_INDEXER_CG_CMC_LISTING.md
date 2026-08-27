# Agent playbook: CG/CMC listing field truthfulness (GitLab #685)

Audience: third-party agents changing `/cg/tickers`, `/cg/pairs`, `/cmc/summary`, `/cmc/ticker`, `/cmc/assets`, or listing docs.

**Issue:** [GitLab **#685**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/685)  
**Compliance table:** [`docs/CG_CMC_COMPLIANCE.md`](../docs/CG_CMC_COMPLIANCE.md)  
**Invariants:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) row **CG/CMC listing field truthfulness (#685)**  
**Gem exclude parent:** [`AGENTS_LISTINGS.md`](./AGENTS_LISTINGS.md) **L639-2** · [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md) **#562**  
**TVL stamp:** [`AGENTS_INDEXER_PAIR_LIQUIDITY_USD.md`](./AGENTS_INDEXER_PAIR_LIQUIDITY_USD.md) **#655**  
**Llama (do not mix):** [`AGENTS_DEFILLAMA.md`](./AGENTS_DEFILLAMA.md) **#631**  
**GT (lockstep exclude, not this JSON):** `make verify-issue-646` · [#646](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/646)

`#224` stays **closed** (timestamps / wrappers). Do not reopen it for TVL vs volume.

## Problem class

Listing crawlers treat CoinGecko `liquidity_in_usd` as **pool TVL**. The indexer used to emit 24h `volume_usd` under that name, fake a 20 bps bid/ask with `f64 * 0.999`, leak gems onto `/cg/pairs`, always report `isFrozen=0`, and collide `SYMBOL_SYMBOL` ticker ids onto the first pair.

## Invariants (L685-1–L685-8)

| ID | Rule |
|----|------|
| **L685-1** | Keep the JSON name `liquidity_in_usd`. Value = `pair_liquidity_usd` / `protocol_pair_tvl` stamp. Unpriced → `"0"`. Never 24h volume, `$1` UST1, `2.5×` USTR, vFDUSD, or book escrow. |
| **L685-2** | GET is a rollup read. Do not live-join `pair_reserves ⋈` oracles on `/cg/tickers`. 60s aggregator cache (#288) stays. Invalid `offset` → **400**. |
| **L685-3** | Ticker bid/ask: reserve-implied mid ± `fee_bps` (BigDecimal) when reserves exist, else `last_price` both sides. Never `f64 * 0.999`. No LCD N+1 on the list path. Hybrid book is `/cg/orderbook`. |
| **L685-4** | `/cmc/ticker` `isFrozen` = `"1"` when `code_id_frozen`. Frozen non-gem pairs stay listed. |
| **L685-5** | CMC `base_id` / `quote_id` are **numbers** (`assets.cmc_id` or `0`). Contracts/denoms are additive `cl8y_base_address` / `cl8y_quote_address`. |
| **L685-6** | Gem / ALPHA / USTRIX / SpaceUSD omitted from `/cg/pairs`, tickers, `/cmc/summary`, `/cmc/ticker`, `/cmc/assets`. Same helper as `/gt` ([`listing_exclude.rs`](../indexer/src/indexer/listing_exclude.rs)). |
| **L685-7** | Duplicate `ticker_id` is skipped (orderbook/trades **404**), not first-insert. `/cmc/assets` symbol collisions prefer the economic hub pin. |
| **L685-8** | Trades: buy = offer quote (buy base). `historical_trades` `limit` max **500**. Additive JSON; do not wrap `/cg/tickers` in `{ tickers: [] }`. |

## Do / don’t

- **Do** JOIN `pair_liquidity_usd` in `load_aggregator_pairs`.
- **Do** keep `/gt` exclude lockstep when adding a pin (gems + ALPHA / USTRIX / SpaceUSD).
- **Don’t** copy `stats.volume_usd` into `liquidity_in_usd`.
- **Don’t** LCD-simulate the book per ticker row (A7 / A12).
- **Don’t** point DeFiLlama at this field (**L631-1**).
- **Don’t** reopen `#224`, `#631`, or `#684` (GT event reserves) here.

## Canonical code

| File | Role |
|------|------|
| `indexer/src/indexer/listing_exclude.rs` | Shared gem/ALPHA/USTRIX/SpaceUSD helper |
| `indexer/src/api/listing_spread.rs` | TVL string + bid/ask + CMC numeric ids |
| `indexer/src/api/aggregator_snapshot.rs` | Set-based load + TVL/reserve stamps + gem filter |
| `indexer/src/api/cg.rs` | Tickers / pairs |
| `indexer/src/api/cmc.rs` | Summary bid/ask, ticker freeze/ids, assets |
| `indexer/src/api/gt.rs` | Re-exports `is_excluded_cw20` |

## Verify

```bash
make setup-indexer-postgres   # if needed
make verify-issue-685
```

Related: `make verify-issue-655` · `make verify-issue-646` · `make verify-issue-639`. `#224` remains closed.

## Related

- [`docs/CG_CMC_COMPLIANCE.md`](../docs/CG_CMC_COMPLIANCE.md)
- [`docs/listings/forms/coingecko-exchange.md`](../docs/listings/forms/coingecko-exchange.md)
- [`AGENTS_INDEXER_AMM_ORDERBOOK_SIM.md`](./AGENTS_INDEXER_AMM_ORDERBOOK_SIM.md) — `/cg/orderbook` hybrid depth
- GT historical reserves: [#684](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/684)
