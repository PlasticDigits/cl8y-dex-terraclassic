# CoinGecko exchange form draft (GitLab #639 child 4)

**Form:** [Partners Platform](https://partner.coingecko.com/request-form/new) → **Decentralized Spot Exchange**.  
**Guide:** [How to Use the CoinGecko Exchange Listing Request Form](https://support.coingecko.com/hc/en-us/articles/4497658821273-Guide-How-to-Use-the-CoinGecko-Exchange-Listing-Request-Form).  
**Human gate:** CoinGecko account + captcha. Agent fills; human submits.

Do **not** treat `/cg/*` compliance ([#224](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/224)) as already listed. This form is the missing submit step.

## Fields

| Field | Value |
|-------|--------|
| Exchange name | CL8Y DEX |
| Exchange type | Decentralized spot (AMM + hybrid limit book) |
| Website | `https://dex.cl8y.com` |
| Network | Terra Classic (`columbus-5`) |
| Factory | `terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea` |
| API base | `https://indexer.dex.cl8y.com/cg/` |
| Pairs | `https://indexer.dex.cl8y.com/cg/pairs` |
| Tickers | `https://indexer.dex.cl8y.com/cg/tickers` |
| Orderbook | `https://indexer.dex.cl8y.com/cg/orderbook` |
| Historical trades | `https://indexer.dex.cl8y.com/cg/historical_trades` |
| Spec notes | [`docs/CG_CMC_COMPLIANCE.md`](../../CG_CMC_COMPLIANCE.md) |

## Do / don’t

- **Do** use the indexer host above. **Don’t** paste `api.coingecko.com` / Pro v3 paths (**L639-3**).
- **Do** disclose hybrid-simulated orderbook depth ([#220](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/220)).
- **Don’t** submit gems / ALPHA / USTRIX / SpaceUSD. `/cg/pairs` and `/cg/tickers` omit them ([#685](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/685), **L639-2**).
- **Don’t** treat `liquidity_in_usd` as 24h volume — it is AMM v2 TVL (`pair_liquidity_usd`). Skill: [`skills/AGENTS_INDEXER_CG_CMC_LISTING.md`](../../../skills/AGENTS_INDEXER_CG_CMC_LISTING.md).
- **Don’t** reopen DeFiLlama ([#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631)) or Keplr ([#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629)) here.
