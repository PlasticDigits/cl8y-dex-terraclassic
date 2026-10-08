# GeckoTerminal DEX/Chain listing form (GitLab #639 child 7 / #646)

**Landing:** [List your DEX & Chain](https://about.geckoterminal.com/dex-chain-listing)  
**Form:** [Google Form](https://docs.google.com/forms/d/e/1FAIpQLSeOAlwKZKTj6LFJfukUWSYS9XUIwtvITX9W5C5ZgUIC4NgX0Q/viewform) ([forms.gle](https://forms.gle/zBLCAEv69b8YoFxV6))  
**Non-EVM spec:** [GeckoTerminal Integration API Standards](https://docs.google.com/document/d/1ufjAJUa6rGO9PBGJGwfBMn-XMk9NE0ow3_iMYrS3drk) (`/latest-block`, `/asset`, `/pair`, `/events`)  
**Human gate:** none on this form (no Google login required). Express Listing is **USD 15,000** — default **No**.  
**Historical submission:** 2026-08-26 as **DEX Addition** + **Non-EVM** + **Not a Fork**, Express **No**. Confirmation: “Your response has been recorded.” PIC email `contact@ceramicliberty.com` / Telegram `@ceramicliberty`.

**Latest submission:** 2026-10-08 01:54:54 UTC, regular DEX Addition on existing
network `terra`, Non-EVM, Not a Fork, Express No. Google confirmed “Your response
has been recorded.” See the [rollout and submission receipt](../../qa/issue-1437/production-listing.md)
for the deployed revision, live retest, submitted scope and historical skips.
Listing approval remains pending.

Terra Classic is present in GeckoTerminal as network **`terra`** (name **TerraClassic**), verified on 2026-10-08. Request **DEX Addition** on that existing network with **Non-EVM** selected. Do **not** treat `/cg/*` as already listed on GT ([#224](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/224) is CoinGecko exchange shape).

## Fields (DEX Addition path)

| Field | Value |
|-------|--------|
| Email | `contact@ceramicliberty.com` |
| Telegram Handle (PIC) | `@ceramicliberty` |
| Request | **DEX Addition** |
| Network Name | Terra Classic (`columbus-5`) |
| DEX Name | CL8Y DEX |
| DEX Logo | `https://git.cl8y.com/code/cl8y-dex-terraclassic/raw/branch/main/tokenlist/images/CL8Y.png` (public PNG) |
| DEX URL | `https://dex.cl8y.com` |
| Unique description | CL8Y DEX is a Terra Classic CosmWasm AMM with an on-chain hybrid limit book. One swap can take pool plus resting limits; native LUNC/USTC wrap to cLUNC/cUSTC. |
| StablePool / highest liquidity | UST1/cUSTC pair `terra1ceprjsxp86ggftf5e38wwt34l83e5gq7penkdnv4wsatkwcs8v6qccw55f` — [Finder](https://finder.terraclassic.community/columbus-5/address/terra1ceprjsxp86ggftf5e38wwt34l83e5gq7penkdnv4wsatkwcs8v6qccw55f). Highest positive USD liquidity among real CG factory tickers at 2026-10-08 readback; recheck before using. No native/stable pool is asserted. UST1 is an **unstablecoin** (never `$1`). USTR is **not** a stablecoin. |
| DEX Pool Page / Analytics | `https://dex.cl8y.com/charts/terra1ceprjsxp86ggftf5e38wwt34l83e5gq7penkdnv4wsatkwcs8v6qccw55f` |
| Official documentation | `https://git.cl8y.com/code/cl8y-dex-terraclassic/src/branch/main/docs/integrators.md` |
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
Please list CL8Y DEX on the existing TerraClassic network, GeckoTerminal network id terra (chain columbus-5).

Explorer: https://finder.terraclassic.community
RPC (https): https://terra-classic-rpc.publicnode.com
LCD: https://terra-classic-lcd.publicnode.com
Native ticker: LUNC (denom uluna). Chain logo: https://git.cl8y.com/code/cl8y-dex-terraclassic/raw/branch/main/tokenlist/images/LUNC.png

Factory: terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea
Router: terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw
dApp: https://dex.cl8y.com
Indexer: https://indexer.dex.cl8y.com

Permanent economic pairs only (ignore soft-launch gems such as EMBER/CORAL in /cg/pairs):
- UST1/USTR terra16vxrhpvpcucu05y0nr862vf9hnqeh274uaff4s7hz4n0ea74006qf5hgqy
- UST1/cUSTC terra1ceprjsxp86ggftf5e38wwt34l83e5gq7penkdnv4wsatkwcs8v6qccw55f
- cLUNC/UST1 terra1su5363453fj326u4t0kqar30f35cm3n0dc9yksg379u6875z350s4mm7h4
- cLUNC/cUSTC terra15rl8g308yzzt5kxu4skgwlahrvm8adyv0s2cupsmvte0akgs2ttsszau38

Supported listing-only native pricing edges (#1437; include only edges published by live readback):
- cLUNC/LUNC cl8y-virtual:clunc-uluna
- cUSTC/USTC cl8y-virtual:custc-uusd
- LUNC/USTC cl8y-virtual:uluna-uusd
- LUNC/USDT cl8y-virtual:uluna-usdt
- cLUNC/USTC cl8y-virtual:clunc-uusd
These are opaque adapter ids, not factory contracts. /gt/events translates real pinned-mapper wrap/unwrap principal and exact pinned-source pool swaps into the existing swap shape. Native legs are uluna/uusd. USDT is pinned terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4 (18 decimals). Missing backing/source omits an edge. Factory/economic accounting excludes these projections.

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
- GT adapters are live at `https://indexer.dex.cl8y.com/gt` after indexer redeploy (`make verify-issue-646`). Event `reserves` are post-event AMM state ([#684](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/684), `make verify-issue-684`). Not `/cg/*`.

Before submitting, verify the deployed revision, read the live CG/GT responses, and
feature the highest-liquidity **real factory pool** at the time of submission.
The dated featured pool above is not evidence of a later ranking.
List only currently published native projections; missing custody or source
legitimately omits an edge. Form confirmation proves receipt, not listing approval.
See the
[projection contract](../../CG_CMC_COMPLIANCE.md#listing-only-native-markets-1437) and
[agent verification checklist](../../qa/issue-1437/README.md).
