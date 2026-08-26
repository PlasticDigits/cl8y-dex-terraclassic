# GeckoTerminal DEX/Chain listing form (GitLab #639 child 7 / #646)

**Landing:** [List your DEX & Chain](https://about.geckoterminal.com/dex-chain-listing)  
**Form:** [Google Form](https://docs.google.com/forms/d/e/1FAIpQLSeOAlwKZKTj6LFJfukUWSYS9XUIwtvITX9W5C5ZgUIC4NgX0Q/viewform) ([forms.gle](https://forms.gle/zBLCAEv69b8YoFxV6))  
**Non-EVM spec:** [GeckoTerminal Integration API Standards](https://docs.google.com/document/d/1ufjAJUa6rGO9PBGJGwfBMn-XMk9NE0ow3_iMYrS3drk) (`/latest-block`, `/asset`, `/pair`, `/events`)  
**Human gate:** none on this form (no Google login required). Express Listing is **USD 15,000** — default **No**.  
**Submitted:** 2026-08-26 as **DEX Addition** + **Non-EVM** + **Not a Fork**, Express **No**. Confirmation: “Your response has been recorded.” PIC email `contact@ceramicliberty.com` / Telegram `@ceramicliberty`.

Terra Classic is **not** in GeckoTerminal `GET /api/v2/networks`. This is **DEX Addition** + **Non-EVM**, not Uniswap-V2 auto-detect. Do **not** treat `/cg/*` as already listed on GT ([#224](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/224) is CoinGecko exchange shape).

## Fields (DEX Addition path)

| Field | Value |
|-------|--------|
| Email | `contact@ceramicliberty.com` |
| Telegram Handle (PIC) | `@ceramicliberty` |
| Request | **DEX Addition** |
| Network Name | Terra Classic (`columbus-5`) |
| DEX Name | CL8Y DEX |
| DEX Logo | `https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/raw/main/tokenlist/images/CL8Y.png` (Imgur preferred; GitLab raw is public PNG) |
| DEX URL | `https://dex.cl8y.com` |
| Unique description | CL8Y DEX is a Terra Classic CosmWasm AMM with an on-chain hybrid limit book. One swap can take pool plus resting limits; native LUNC/USTC wrap to cLUNC/cUSTC. |
| StablePool / highest liquidity | UST1/USTR pair `terra16vxrhpvpcucu05y0nr862vf9hnqeh274uaff4s7hz4n0ea74006qf5hgqy` — [Finder](https://finder.terraclassic.community/columbus-5/address/terra16vxrhpvpcucu05y0nr862vf9hnqeh274uaff4s7hz4n0ea74006qf5hgqy). No native/stable like BUSD-WETH; this is the highest-liquidity factory pair. UST1 is an **unstablecoin** (never `$1`). USTR is **not** a stablecoin. |
| DEX Pool Page / Analytics | `https://dex.cl8y.com/charts/terra16vxrhpvpcucu05y0nr862vf9hnqeh274uaff4s7hz4n0ea74006qf5hgqy` |
| Official documentation | `https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/blob/main/docs/integrators.md` |
| DEX Twitter/X | `https://x.com/ceramictoken` |
| EVM compatible? | **Non-EVM** |
| Fork | **Not a Fork** |
| Protocol if not a fork | TerraSwap-style CosmWasm constant-product AMM (`x * y = k`) plus an on-chain hybrid limit order book. Factory creates pairs; router executes multi-hop and wrap. Not an EVM Uniswap fork. |
| Factory | `terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea` |
| Router | `terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw` |
| DEX Adapter documentation | GT spec: `https://docs.google.com/document/d/1ufjAJUa6rGO9PBGJGwfBMn-XMk9NE0ow3_iMYrS3drk`. Live adapters: [`scripts/geckoterminal/README.md`](../../../scripts/geckoterminal/README.md). CoinGecko-shaped `/cg/` stays a different crawler. |
| DEX Adapter Base URL | `https://indexer.dex.cl8y.com/gt` |
| DEX Key | `cl8y` |
| Express Listing (USD 15,000)? | **No** |
| Follow GeckoTerminal on X? | **No** |
| Willing to do-follow linkback? | **Yes** |

### Additional Remark (paste)

```
Please add Terra Classic (columbus-5) as a GeckoTerminal network and list CL8Y DEX on it. Classic is not in GET /api/v2/networks today.

Explorer: https://finder.terraclassic.community
RPC (https): https://terra-classic-rpc.publicnode.com
LCD: https://terra-classic-lcd.publicnode.com
Native ticker: LUNC (denom uluna). Chain logo: https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/raw/main/tokenlist/images/LUNC.png

Factory: terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea
Router: terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw
dApp: https://dex.cl8y.com
Indexer: https://indexer.dex.cl8y.com

Permanent economic pairs only (ignore soft-launch gems such as EMBER/CORAL in /cg/pairs):
- UST1/USTR terra16vxrhpvpcucu05y0nr862vf9hnqeh274uaff4s7hz4n0ea74006qf5hgqy
- UST1/cUSTC terra1ceprjsxp86ggftf5e38wwt34l83e5gq7penkdnv4wsatkwcs8v6qccw55f
- cLUNC/UST1 terra1su5363453fj326u4t0kqar30f35cm3n0dc9yksg379u6875z350s4mm7h4
- cLUNC/cUSTC terra15rl8g308yzzt5kxu4skgwlahrvm8adyv0s2cupsmvte0akgs2ttsszau38

UST1 is an unstablecoin — never advertise $1. USTR is USTC Repeg, not a stablecoin.
CL8Y CoinGecko id ceramicliberty-com is BSC-only today; columbus-5 CW20 is terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3 (18 decimals).

Contact: contact@ceramicliberty.com / Telegram @ceramicliberty / https://x.com/ceramictoken
```

## Do / don’t

- **Do** select **DEX Addition** + **Non-EVM** + **Not a Fork**. **Don’t** pick Uniswap V2.
- **Do** point Adapter Base URL at **our** indexer (`indexer.dex.cl8y.com`). **Don’t** paste CoinGecko Pro v3 (**L639-3**).
- **Do** decline Express unless product pays USD 15,000.
- **Don’t** submit gems / ALPHA / USTRIX / SpaceUSD as the featured pool (**L639-2**).
- **Don’t** advertise UST1 as `$1` or USTR as a stablecoin (**L639-5**).
- GT adapters are live at `https://indexer.dex.cl8y.com/gt` after indexer redeploy (`make verify-issue-646`). Not `/cg/*`.
