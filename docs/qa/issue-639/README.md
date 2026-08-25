# QA — GitLab #639 listing venue catalog

Verify (no chain): `make verify-issue-639`

Playbook: [`skills/AGENTS_LISTINGS.md`](../../../skills/AGENTS_LISTINGS.md) · catalog [`docs/listings/`](../../listings/) · invariants **L639-1–L639-8**.

## Automated

- `catalog.json` pins match the Keplr permanent-six pack (addresses + decimals)
- Gems / ALPHA / USTRIX / SpaceUSD excluded
- Exchange form drafts point at `indexer.dex.cl8y.com` `/cg` + `/cmc` (not CoinGecko Pro v3)
- Skip list + owned surfaces (#631 / #629 / #224) documented
- UST1 unstablecoin / USTR not-a-stablecoin language present
- Skill + AGENTS.md + integrators + testing cross-links

## Manual / operator (not this verify)

1. Open **one child issue per venue** (order in `docs/listings/catalog.json` `children`).
2. Do **not** submit Coinhall, DexScreener, or LuncScan Telegram as if they were PRs.
3. Human clicks captcha / login on CoinGecko and CMC forms.
4. Do **not** open upstream GitHub PRs from CI.

## Out of scope here

- New indexer endpoints.
- Reopening DeFiLlama (#631) or Keplr (#629) adapter/pack work.
- Vendoring Cosmostation / Hexxagon / chain-registry JSON (those are children).
- Paying Trust Wallet’s merge fee.
