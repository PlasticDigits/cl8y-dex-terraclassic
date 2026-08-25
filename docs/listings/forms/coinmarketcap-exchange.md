# CoinMarketCap exchange form draft (GitLab #639 child 6)

**Form:** [coinmarketcap.com/request](https://coinmarketcap.com/request/) → **Add exchange**.  
**Criteria:** [Listings Criteria](https://support.coinmarketcap.com/hc/en-us/articles/360043659351-Listings-Criteria) (**60-day** operation).  
**Human gate:** human submits. Agent fills.

CMC **DexScan** does **not** list Terra Classic — do not treat DexScan as the path (**L639-6**).

## Fields

| Field | Value |
|-------|--------|
| Exchange name | CL8Y DEX |
| Website (books visible logged-out) | `https://dex.cl8y.com` |
| Network | Terra Classic (`columbus-5`) |
| Factory | `terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea` |
| API base | `https://indexer.dex.cl8y.com/cmc/` |
| Summary | `https://indexer.dex.cl8y.com/cmc/summary` |
| Assets | `https://indexer.dex.cl8y.com/cmc/assets` |
| Ticker | `https://indexer.dex.cl8y.com/cmc/ticker` |
| Orderbook | `https://indexer.dex.cl8y.com/cmc/orderbook/:pair` |
| Trades | `https://indexer.dex.cl8y.com/cmc/trades/:pair` |
| Spec notes | [`docs/CG_CMC_COMPLIANCE.md`](../../CG_CMC_COMPLIANCE.md) |

## Do / don’t

- **Do** use the five `/cmc/*` URLs on **our** indexer. **Don’t** point at CoinGecko Pro v3 (**L639-3**).
- **Do** disclose hybrid-simulated orderbook depth.
- Token `unified_cryptoasset_id` / `cmc_id` waits on child 9 (after a tradable market + supply story).
- **Don’t** reopen `#224` API work; this child is submit + chase only.
