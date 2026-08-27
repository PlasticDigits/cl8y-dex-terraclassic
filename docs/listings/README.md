# Listing venue catalog (GitLab #639)

Parent tracker for **places that can list CL8Y DEX stats, tokens, and wallet recognition** the same way we already do DeFiLlama ([#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631)) and Keplr ([#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629)): **GitHub PRs**, **public listing forms**, or **in-repo agent skills**.

This ticket does **not** implement a new indexer API. Machine-readable pins: [`catalog.json`](./catalog.json). Agent playbook: [`skills/AGENTS_LISTINGS.md`](../../skills/AGENTS_LISTINGS.md). Verify: `make verify-issue-639`.

Do **not** reopen DeFiLlama adapter work here, and do **not** treat `/cg/*` / `/cmc/*` compliance ([#224](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/224)) as a listing.

**Verified 2026-08-25** against live APIs, official docs, and upstream repos. Agentic means an agent can open the PR, vendor the adapter, or fill the form from pins below. Login/captcha/Telegram/Discord are human gates.

## Invariants (L639-1–L639-8)

| ID | Rule |
|----|------|
| **L639-1** | Catalog + go/no-go only. **No** new indexer endpoint here. Children are **one venue per issue**. |
| **L639-2** | Permanent six CW20s only — same addresses, decimals, and logos as Keplr **K629-2** / **K629-3**. **Exclude** `#562` gems, ALPHA, USTRIX, SpaceUSD, and community-tax templates. |
| **L639-3** | Exchange forms point at **`https://indexer.dex.cl8y.com/cg/`** and **`/cmc/`**. Never CoinGecko Pro v3 (`api.coingecko.com`). |
| **L639-4** | Owned surfaces stay on their issues: Llama **#631**, Keplr **#629**, `/cg` `/cmc` shape **#224**. Do not reopen those here. |
| **L639-5** | UST1 is an **unstablecoin** (never advertise `$1`). USTR is **not** a stablecoin. Do not invent a second CoinGecko id for CL8Y (`ceramicliberty-com` is BSC-only today). |
| **L639-6** | Skip Coinhall, DexScreener, LuncScan Telegram, Token Terminal, CMC DexScan, a Leap-owned CW20 repo, and `terra-money/assets`. |
| **L639-7** | Do **not** open upstream PRs from this repo’s CI. PR venues are operator/agent GitHub work; forms may need a human captcha/login click. |
| **L639-8** | This README + [`catalog.json`](./catalog.json) + [`skills/AGENTS_LISTINGS.md`](../../skills/AGENTS_LISTINGS.md) + `make verify-issue-639`. |

## Public pins (columbus-5)

| Item | Value |
|------|--------|
| dApp | `https://dex.cl8y.com` |
| Indexer | `https://indexer.dex.cl8y.com` |
| Factory | `terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea` |
| CG exchange API | `https://indexer.dex.cl8y.com/cg/` (`/pairs`, `/tickers`, `/orderbook`, `/historical_trades`) |
| CMC exchange API | `https://indexer.dex.cl8y.com/cmc/` (`/summary`, `/assets`, `/ticker`, `/orderbook/:pair`, `/trades/:pair`) |
| Llama daily | `GET /api/v1/defillama/daily?timestamp=<unix_00:00_utc>` ([#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631)) |
| CoinGecko id (BSC only today) | `ceramicliberty-com` → `0x8f452a1fdd388a45e1080992eff051b4dd9048d2` |

Permanent economic CW20s (same set as Keplr **K629-2** — **no gems**):

| Token | Decimals | Contract | Notes |
|-------|----------|----------|--------|
| CL8Y | 18 | `terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3` | CG id exists; Terra Classic platform **missing** |
| UST1 | 6 | `terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72` | No CoinGecko id. **Unstablecoin** — never `$1` |
| USTR | 18 | `terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv` | Already in Keplr + Hexxagon as **USTC Repeg**. Not a stablecoin |
| cLUNC | 6 | `terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg` | |
| cUSTC | 6 | `terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch` | |
| vFDUSD | 6 | `terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3` | UST1-window mint asset |

Logos: [`tokenlist/images/`](../../tokenlist/images). Keplr pack: [`keplr-contract-registry/`](./keplr-contract-registry/).

## Already in flight (do not duplicate)

| Surface | Issue | Upstream | Status |
|---------|-------|----------|--------|
| DeFiLlama TVL / volume / fees + UST1 unstablecoin | [#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631) / leftover [#687](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/687) | [Adapters#20676](https://github.com/DefiLlama/DefiLlama-Adapters/pull/20676), [dimension-adapters#8987](https://github.com/DefiLlama/dimension-adapters/pull/8987), [peggedassets-server#903](https://github.com/DefiLlama/peggedassets-server/pull/903) | Route live; leftover is fees `null`/404 vs crawler (`make verify-issue-687`). Close on Llama merge |
| Keplr Add Token (name + logo) | [#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629) | [keplr-contract-registry#132](https://github.com/chainapsis/keplr-contract-registry/pull/132) | Pack + `make verify-issue-629`; USTR already live |
| Self-hosted `/cg/*` `/cmc/*` shape | [#224](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/224) | [`docs/CG_CMC_COMPLIANCE.md`](../CG_CMC_COMPLIANCE.md) | **API ready**. Submitting the **exchange listing forms** is still open (this ticket’s children). |

## Agentic go list

Legend: **PR** = GitHub pull request. **Form** = public request form (agent drafts/fills; human may click captcha / log in). **Skill** = vendor adapter + playbook in this repo like DeFiLlama.

### 1. Wallets / token recognition — PR, high confidence

| Venue | What it unlocks | How |
|-------|-----------------|-----|
| **Cosmostation** (+ Mintscan token metadata) | CW20 name / logo / optional `coinGeckoId` | Pack: [#640](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/640) / !431. **Blocked:** [cosmostation/chainlist](https://github.com/cosmostation/chainlist) is **archived** (2026-07); Cosmostation wallet service ends **2026-09-01**. Folder would have been **`terra`**. |
| **Galaxy Station / Hexxagon** | Token + optional DEX pair display | Pack: [#641](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/641) / !432. PR to [hexxagon-io/chain-registry](https://github.com/hexxagon-io/chain-registry) `cw20/tokens/mainnet/terra.js` (+ `cw20/dex_pairs` if they want CL8Y pools). **USTR** already there. Live append: [PR #68](https://github.com/hexxagon-io/chain-registry/pull/68). |
| **cosmos/chain-registry** | Cosmos Directory, Leap custom-token metadata, explorers | PR `terra/assetlist.json` + images; `type_asset: cw20`. This is Leap’s real path — Leap has **no** public CW20 registry of its own. |
| **Trust Wallet assets** | Logo / info in Trust Wallet | PR to [trustwallet/assets](https://github.com/trustwallet/assets). **Paid** merge. Only if product asks. |

Do **not** PR `terra-money/assets` as the live Station list — Hexxagon replaced that for Galaxy Station.

### 2. DEX / stats aggregators — form + existing APIs

Indexer already speaks the crawler dialects. Child tasks are **submit + chase**, not new endpoints. Field drafts: [`forms/`](./forms/).

| Venue | How |
|-------|-----|
| **CoinGecko exchange** | [Partners Platform](https://partner.coingecko.com/request-form/new) → Decentralized Spot Exchange. Point at `/cg/`. **Human gate:** account + captcha. Pack: [`forms/coingecko-exchange.md`](./forms/coingecko-exchange.md). |
| **CoinGecko token — Terra Classic platform** | Update coin / add contract on existing `ceramicliberty-com`. Pack: [`forms/coingecko-terra-classic-platform.md`](./forms/coingecko-terra-classic-platform.md). |
| **CoinMarketCap exchange** | [coinmarketcap.com/request](https://coinmarketcap.com/request/) → Add exchange. Factory + five `/cmc/*` URLs. **60-day** operation. Pack: [`forms/coinmarketcap-exchange.md`](./forms/coinmarketcap-exchange.md). CMC **DexScan** is **not** the path. |
| **CoinMarketCap tokens** | Same hub → Add cryptoasset after volume exists. UST1 is an **unstablecoin**. |
| **GeckoTerminal** | Terra Classic is **not** in `GET https://api.geckoterminal.com/api/v2/networks`. **DEX Addition** form submitted 2026-08-26. Live Non-EVM adapters: `GET https://indexer.dex.cl8y.com/gt/{latest-block,asset,pair,events}` ([#646](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/646), [`scripts/geckoterminal/`](../../scripts/geckoterminal/)). Event `reserves` = post-event AMM state ([#684](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/684)). Pack: [`forms/geckoterminal.md`](./forms/geckoterminal.md). Not Uniswap-V2 auto-detect. |
| **CoinPaprika / Coinranking / LiveCoinWatch / CoinCodex** | Public token forms. Coinranking already has CL8Y with a **stale** Classic address `terra1u9c7…` — fix that contract. |

### 3. Skip (document only)

| Venue | Verdict |
|-------|---------|
| **LuncScan** | Telegram to Nueng. Human ops. Agent may prepare a CA/logo pack. |
| **Coinhall** | **Skip.** No public DEX-add PR; Terra Classic is being deprecated. |
| **DexScreener** | **Skip** until they add Classic. Discord is not an agent skill. |
| **Leap** | Covered by the chain-registry PR. Do not invent a Leap repo. |
| **Token Terminal / CMC DexScan / generic EVM screeners** | No Terra Classic DEX platform. **Skip.** |

## Suggested child tasks (open separately)

Do these as **one venue per issue** so an agent can close them. Suggested order is in [`catalog.json`](./catalog.json) `children`.

Each child should: vendor the JSON/adapter in-repo (Keplr/Llama pattern), add a short skill, and `make verify-issue-*` for the pack. **Do not** open upstream PRs from CI.

## Agent skill shape (when a child starts)

Copy [#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631) / [#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629):

- `docs/listings/<venue>/` with the exact files to copy
- `skills/AGENTS_<VENUE>.md` with invariants (decimals, gem exclude, no invented pegs)
- Pins from the table above
- Close blocked if gems / wrong decimals / BSC-only contract shipped as Classic

## Do / don’t

- **Do** reuse Keplr decimals and gem exclude.
- **Do** point exchange forms at **our** `/cg` and `/cmc` hosts, not CoinGecko Pro v3.
- **Do** treat Llama TVL/volume as [#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631) only.
- **Don’t** list USTR as a stablecoin or peg UST1 to $1.
- **Don’t** submit ALPHA / USTRIX / SpaceUSD / community-tax templates / `#562` gems.
- **Don’t** spend agent time on Coinhall, DexScreener, or LuncScan Telegram as if they were PRs.

## Related

- [#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631) DeFiLlama — [`docs/DEFILLAMA.md`](../DEFILLAMA.md) · [`skills/AGENTS_DEFILLAMA.md`](../../skills/AGENTS_DEFILLAMA.md)
- [#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629) Keplr CW20 — [`keplr-contract-registry/`](./keplr-contract-registry/) · [`skills/AGENTS_KEPLR_CW20_REGISTRY.md`](../../skills/AGENTS_KEPLR_CW20_REGISTRY.md)
- [#224](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/224) `/cg` `/cmc` — [`docs/CG_CMC_COMPLIANCE.md`](../CG_CMC_COMPLIANCE.md)
- [#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562) gem hide
- [#640](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/640) Cosmostation (archived) — [`cosmostation/`](./cosmostation/) · [`AGENTS_COSMOSTATION.md`](../../skills/AGENTS_COSMOSTATION.md)
- [#641](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/641) Hexxagon — [`hexxagon/`](./hexxagon/) · [`AGENTS_HEXXAGON.md`](../../skills/AGENTS_HEXXAGON.md)
- Integrators: [`docs/integrators.md`](../integrators.md#listing-venue-catalog-gitlab-639)
- QA: [`docs/qa/issue-639/README.md`](../qa/issue-639/README.md)
