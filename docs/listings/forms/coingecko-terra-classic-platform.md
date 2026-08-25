# CoinGecko Terra Classic platform draft (GitLab #639 child 5)

**Form:** [Partners Platform](https://partner.coingecko.com/request-form/new) → **Update coin / add contract**.  
**Human gate:** account + captcha.

Goal: Keplr / Cosmostation `coinGeckoId: ceramicliberty-com` actually prices the **columbus-5** CW20. CoinGecko **does** have asset platform id `terra` = Terra Classic (`GET /api/v3/asset_platforms`). Live `ceramicliberty-com` platforms = **`binance-smart-chain` only**.

## Fields

| Field | Value |
|-------|--------|
| Existing CoinGecko id | `ceramicliberty-com` |
| Request | Add Terra Classic (`terra` / columbus-5) contract — **do not** create a second id |
| CW20 | `terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3` |
| Decimals | **18** (not a stale tokenlist `6`) |
| BSC contract (already listed) | `0x8f452a1fdd388a45e1080992eff051b4dd9048d2` |
| dApp | `https://dex.cl8y.com` |
| Logo | [`tokenlist/images/CL8Y.png`](../../../tokenlist/images/CL8Y.png) |

## Do / don’t

- **Do** keep one economic id. **Don’t** invent `cl8y-terra` or similar (**L639-5**).
- **Don’t** add UST1 / USTR / wraps on this form — they have **no** CG id yet (child 9, after volume).
- **Don’t** advertise UST1 as `$1` or USTR as a stablecoin.
- Keplr Job 2 USD stays blocked until this platform add lands ([#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629) **K629-7**).
