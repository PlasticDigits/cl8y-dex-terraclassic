# DeFiLlama listing (TVL + volume + fees)

**Issue:** [GitLab **#631**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631) (listing + daily GET); leftover [GitLab **#687**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/687) (headline partial SUM + adapter `start`)  
**Skill:** [`skills/AGENTS_DEFILLAMA.md`](../skills/AGENTS_DEFILLAMA.md)  
**Invariants:** [`indexer-invariants.md`](./indexer-invariants.md) row **DeFiLlama UTC-day (#631)**  
**Adapter copies:** [`scripts/defillama/`](../scripts/defillama/)  
**Not this ticket:** CoinGecko / CMC crawlers — [`CG_CMC_COMPLIANCE.md`](./CG_CMC_COMPLIANCE.md)

List **CL8Y DEX** (`https://dex.cl8y.com`) on [DeFiLlama](https://defillama.com) as a Terra Classic Dexs protocol (**TVL**, **spot volume**, **fees/revenue**) and list **UST1** on the **Stablecoins** dashboard as an **unstablecoin**. Publish **USTR** reserve-token volume, fees, and hub price on the same daily API. Llama merges **open-source adapters**. Hosted `/cg/*` and `/cmc/*` do not substitute.

## Public pins (columbus-5)

| Item | Value |
|------|--------|
| dApp | `https://dex.cl8y.com` |
| Indexer | `https://indexer.dex.cl8y.com` |
| Factory | `terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea` |
| Llama chain | `terra` (Terra Classic; same as Terraport / GarudaDeFi) |
| Slug | `cl8y-dex` |
| Adapter start | `1786924800` / `2026-08-17` (first UTC day GET returns **200**; [#687](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/687)) |
| Daily API | `GET /api/v1/defillama/daily?timestamp=<unix_00:00_utc>` |
| UST1 (unstablecoin) | `terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72` |
| USTR (reserve) | `terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv` |
| ust1-window | `terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2` |

Factory pin matches [`deployments/mainnet-ust1-wrap/REGISTRY.md`](../deployments/mainnet-ust1-wrap/REGISTRY.md).

## Methodology

### TVL (on-chain only)

The TVL adapter paginates factory `Pairs { limit: 30, start_after }` and `api.add`s each pair `Pool {}` reserve (native `denom` or CW20 `contract_addr`) as a **raw** amount. Llama prices tokens.

| Include | Exclude |
|---------|---------|
| Factory-listed pair pool reserves | Indexer `total_liquidity_usd`, hub marks, CG `liquidity_in_usd` |
| Soft-launch gem pool locks (default) | LP `liquidity_token` / `total_share` (double-count) |
| cLUNC / cUSTC as `uluna` / `uusd` (1:1 wrap) | Limit-book escrow, parked dust, CMM treasury, UST1-window inventory, Venus vFDUSD |
| | Wrap-mapper native `uluna`/`uusd` inventory (pools-only DEX TVL; omit unless Llama asks) |

`timetravel: false`. Majority pool-query failure **throws** (no silent `$0`). Named wrap substitution only: **cLUNC → `uluna`**, **cUSTC → `uusd`** (1:1). Do **not** peg UST1=$1 or USTR=2.5×USTC. Other unpriced CW20s are omitted on Llama’s side.

**Expected drift vs** `GET /api/v1/overview` `total_liquidity_usd` (**#569**): different pricers + Llama omit-unpriced vs our one-sided 2× catalog. Document the gap; do not “fix” Llama TVL by posting indexer USD.

### Volume (UTC calendar day)

`dailyVolume` = `GET /api/v1/defillama/daily` `volume_usd`.

- Window is **UTC midnight → +86400**, not trailing 24h (**#576** retail windows stay separate).
- Headline = `SUM(swap_events.volume_usd)` **once per swap row** ([**L10**](./integrators-hybrid-volume.md), [#216](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/216)).
- **Exclude:** columbus-5 gem pairs ([`COLUMBUS5_GEM_ADDRESSES`](../frontend-dapp/src/utils/pairCatalogRank.ts) / [#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562)), wrap/unwrap, UST1 mint/redeem, `limit_order_fills` (already inside the parent swap), community-tax extra-debit as a separate add.
- Do **not** serve `overview.total_volume_24h_usd` as `dailyVolume`.

### Fees / revenue

Pair commission goes to `FEE_CONFIG.treasury` (CMM), **not** to LPs. `spread_amount` is **not** a fee ([PFee / L7](./indexer-invariants.md)).

| Llama dimension | CL8Y mapping |
|-----------------|--------------|
| `dailyFees` | Treasury-bound **priced SUM** (`swap_amm` + `book_take` + `limit_place` + labeled wrap/window). One unpriced source does **not** null the headline ([#687](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/687) / EFee-6). |
| `dailyRevenue` / `dailyProtocolRevenue` | Same as `dailyFees` |
| `dailySupplySideRevenue` | `0` (do not reclassify `spread_amount` or residual) |
| `dailyHoldersRevenue` | Omit |

Community-tax extra-debit is **token tax**, not a protocol DEX fee.

### UST1 unstablecoin (Stablecoins dashboard)

UST1 is a USD-**target** token minted/redeemed through the oracle window against vFDUSD. Product name is **unstablecoin**: the window rate and secondary AMM can deviate. Llama listing:

| Field | Value |
|-------|--------|
| Dashboard | [Stablecoins](https://defillama.com/stablecoins) via [peggedassets-server](https://github.com/DefiLlama/peggedassets-server) `ust1` |
| `pegType` | `peggedUSD` |
| `pegMechanism` | `crypto-backed` (vFDUSD in `ust1-window`) |
| Circulating | On-chain CW20 `token_info.total_supply` / 1e6 |
| Price | Llama / hub — **never** `$1` |

`GET /api/v1/defillama/daily` `assets.ust1` adds UTC-day DEX volume (pairs with a UST1 leg), pair + window fees, and hub `price_usd`.

### USTR (reserve token — not a stablecoin)

USTR is the CMM reserve CW20 (18 decimals). Report it on the same daily GET as `assets.ustr` (DEX volume, pair fees, hub price). Do **not** file a second pegged-asset adapter and do **not** use 2.5× USTC as a peg.

### Other Llama surfaces

| Surface | Decision |
|---------|----------|
| UST1 window issuer TVL (`cl8y-ust1`) | Optional follow-up: vFDUSD locked in the window. Category **Stablecoin Issuer**. Separate from DEX TVL. May be `$0` until Llama prices vFDUSD ([#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629)). |
| Wrap-mapper as Stablecoin Wrapper | **Do not** this ticket — would double-count vs DEX pool TVL / wrap inventory. |
| Yields / emissions / oracles-TVS / bridges | **Do not** — no incentive program; wrap is not a canonical bridge listing. |
| Treasury page | Mail `metadata@defillama.com` after adapters merge if Llama asks. |

### Indexer contract

| Status | When |
|--------|------|
| **400** | Missing / non-i64 / negative / not `00:00` UTC / future UTC day. No `from`/`to` range dump. |
| **404** | Day not in `defillama_daily_stats` yet (days before adapter `start` stay 404) |
| **200** `"0"` | Day rolled and idle (no fee events / no swaps). Volume may still be a number when fees are `"0"`. |
| **200** numeric fees | Priced treasury SUM (EFee-6). One unpriced source does not null wrap/window. |
| **200** `null` fees | Fee activity exists and **none** have `fee_usd > 0`. Never silent `"0"`. Volume may still be a number. Per-source `fees.*` may be `null` independently. |

GET is cached ≥60s and reads rollup tables only (same class as `#281` / `#586`). Refresh is the ~5 min volume loop (today + 8 prior UTC days). Days outside lookback are **not** pruned; Llama `start` is the first persisted **200** day. CORS stays `https://dex.cl8y.com`; Llama is server-side.

## Gem list

Must stay in lockstep:

1. [`frontend-dapp/src/utils/pairCatalogRank.ts`](../frontend-dapp/src/utils/pairCatalogRank.ts) `COLUMBUS5_GEM_ADDRESSES`
2. [`indexer/src/indexer/defillama.rs`](../indexer/src/indexer/defillama.rs) `COLUMBUS5_GEM_ADDRESSES`
3. [`scripts/defillama/gems.js`](../scripts/defillama/gems.js)

TVL keys by **contract/denom**, not symbol (A9). Volume exclude is by **address**.

## Upstream process

| Adapter | Upstream PR |
|---------|-------------|
| TVL `projects/cl8y-dex` | [DefiLlama-Adapters#20676](https://github.com/DefiLlama/DefiLlama-Adapters/pull/20676) |
| Volume `dexs/cl8y-dex` + fees `fees/cl8y-dex` | [dimension-adapters#8987](https://github.com/DefiLlama/dimension-adapters/pull/8987) (route is live; leftover is headline `null` / 404 vs crawler — [#687](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/687). Re-test after indexer partial SUM + `start: "2026-08-17"`.) |
| Icons / metadata | Follow-up if Llama asks |
| UST1 Stablecoins | [peggedassets-server#903](https://github.com/DefiLlama/peggedassets-server/pull/903) (`ust1`, draft). Circulating is on-chain `token_info` (no Coolify dependency). |

After merge: `https://defillama.com/protocol/cl8y-dex` should show Terra Classic + `dex.cl8y.com`.

In-repo copies: [`scripts/defillama/README.md`](../scripts/defillama/README.md). Local TVL check: `node test.js projects/cl8y-dex/index.js` in a DefiLlama-Adapters clone (~$13.3k Llama-priced cLUNC/cUSTC; unpriced CW20s omitted).

## Close is blocked

Volume includes gems or fills; TVL is sourced from indexer USD; fees include `spread_amount` or token tax; unpriced activity is published as `$0`; SSR is non-zero; adapter `start` still walks May 2026 404s.

## Related

- [#216](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/216) hybrid volume
- [#569](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/569) protocol TVL
- [#586](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/586) / [#613](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/613) / [#614](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614) treasury fees
- [#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562) production gem hide
- [#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629) token recognition (Llama pricing coverage)
- [#224](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/224) CG/CMC (do not conflate)
- [#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639) listing venue catalog (do not reopen Llama adapters there)
- [#683](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/683) economic `fee_usd` stamps (EFee-6)
- [#687](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/687) Llama fees headline partial SUM + adapter start
