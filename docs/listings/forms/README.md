# Listing form drafts (GitLab #639)

Agent-ready field packs for public listing forms. These are **drafts** — a human still clicks captcha / logs in / submits.

Exchange forms must use **our** indexer hosts ([**L639-3**](../README.md)):

- CoinGecko crawler: `https://indexer.dex.cl8y.com/cg/`
- CoinMarketCap crawler: `https://indexer.dex.cl8y.com/cmc/`

Never point a form at CoinGecko Pro v3 (`api.coingecko.com`). Compliance shape: [`docs/CG_CMC_COMPLIANCE.md`](../../CG_CMC_COMPLIANCE.md) ([#224](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/224)).

| Draft | Venue | Child order |
|-------|-------|-------------|
| [coingecko-exchange.md](./coingecko-exchange.md) | CoinGecko Decentralized Spot Exchange | 4 |
| [coingecko-terra-classic-platform.md](./coingecko-terra-classic-platform.md) | Add Terra Classic CW20 to `ceramicliberty-com` | 5 |
| [coinmarketcap-exchange.md](./coinmarketcap-exchange.md) | CMC Add exchange | 6 |

Token listings for UST1 / USTR / wraps wait until the DEX page is visible (**L639-5**: UST1 is an unstablecoin, never `$1`).
