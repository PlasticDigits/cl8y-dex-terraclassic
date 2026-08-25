# DeFiLlama listing (TVL + volume + fees)

**Issue:** [GitLab **#631**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631)  
**Skill:** [`skills/AGENTS_DEFILLAMA.md`](../skills/AGENTS_DEFILLAMA.md)  
**Invariants:** [`indexer-invariants.md`](./indexer-invariants.md) row **DeFiLlama UTC-day (#631)**  
**Adapter copies:** [`scripts/defillama/`](../scripts/defillama/)  
**Not this ticket:** CoinGecko / CMC crawlers — [`CG_CMC_COMPLIANCE.md`](./CG_CMC_COMPLIANCE.md)

List **CL8Y DEX** (`https://dex.cl8y.com`) on [DeFiLlama](https://defillama.com) as a Terra Classic Dexs protocol: **TVL**, **spot volume**, and **fees/revenue**. Llama merges **open-source adapters**. Hosted `/cg/*` and `/cmc/*` do not substitute.

## Public pins (columbus-5)

| Item | Value |
|------|--------|
| dApp | `https://dex.cl8y.com` |
| Indexer | `https://indexer.dex.cl8y.com` |
| Factory | `terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea` |
| Llama chain | `terra` (Terra Classic; same as Terraport / GarudaDeFi) |
| Slug | `cl8y-dex` |
| Adapter start | `1777593600` (2026-05-01 00:00 UTC) |
| Daily API | `GET /api/v1/defillama/daily?timestamp=<unix_00:00_utc>` |

Factory pin matches [`deployments/mainnet-ust1-wrap/REGISTRY.md`](../deployments/mainnet-ust1-wrap/REGISTRY.md).

## Methodology

### TVL (on-chain only)

The TVL adapter paginates factory `Pairs { limit: 30, start_after }` and `api.add`s each pair `Pool {}` reserve (native `denom` or CW20 `contract_addr`) as a **raw** amount. Llama prices tokens.

| Include | Exclude |
|---------|---------|
| Factory-listed pair pool reserves | Indexer `total_liquidity_usd`, hub marks, CG `liquidity_in_usd` |
| Soft-launch gem pool locks (default) | LP `liquidity_token` / `total_share` (double-count) |
| | Limit-book escrow, parked dust, CMM treasury, UST1-window inventory, Venus vFDUSD |
| | Wrap-mapper native `uluna`/`uusd` (pools-only DEX TVL; omit unless Llama asks) |

`timetravel: false`. Majority pool-query failure **throws** (no silent `$0`). No `misrepresentedTokens` pegs (no UST1=$1, no USTR=2.5×USTC). Unpriced CW20s are omitted on Llama’s side.

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
| `dailyFees` | Treasury-bound: `swap_amm` + `book_take` + `limit_place` + labeled wrap/window |
| `dailyRevenue` / `dailyProtocolRevenue` | Same as `dailyFees` |
| `dailySupplySideRevenue` | `0` (do not reclassify `spread_amount`) |
| `dailyHoldersRevenue` | Omit |

Community-tax extra-debit is **token tax**, not a protocol DEX fee.

### Indexer contract

| Status | When |
|--------|------|
| **400** | Missing / non-i64 / negative / not `00:00` UTC / future UTC day. No `from`/`to` range dump. |
| **404** | Day not in `defillama_daily_stats` yet |
| **200** `"0"` | Day rolled and idle |
| **200** `null` | Day rolled, activity exists, USD unpriced (fail closed — never silent `"0"`) |

GET is cached ≥60s and reads rollup tables only (same class as `#281` / `#586`). Refresh is the ~5 min volume loop (today + 8 prior UTC days). CORS stays `https://dex.cl8y.com`; Llama is server-side.

## Gem list

Must stay in lockstep:

1. [`frontend-dapp/src/utils/pairCatalogRank.ts`](../frontend-dapp/src/utils/pairCatalogRank.ts) `COLUMBUS5_GEM_ADDRESSES`
2. [`indexer/src/indexer/defillama.rs`](../indexer/src/indexer/defillama.rs) `COLUMBUS5_GEM_ADDRESSES`
3. [`scripts/defillama/gems.js`](../scripts/defillama/gems.js)

TVL keys by **contract/denom**, not symbol (A9). Volume exclude is by **address**.

## Upstream process

1. TVL PR → [DefiLlama-Adapters](https://github.com/DefiLlama/DefiLlama-Adapters) `projects/cl8y-dex/`
2. Volume PR → [dimension-adapters](https://github.com/DefiLlama/dimension-adapters) `dexs/cl8y-dex/`
3. Fees PR → same repo `fees/cl8y-dex/`
4. Icons / metadata if Llama asks
5. After merge: `https://defillama.com/protocol/cl8y-dex` should show Terra Classic + `dex.cl8y.com`

In-repo copies and submission notes: [`scripts/defillama/README.md`](../scripts/defillama/README.md). **This MR does not open the GitHub PRs** (operator follow-up).

## Close is blocked

Volume includes gems or fills; TVL is sourced from indexer USD; fees include `spread_amount` or token tax.

## Related

- [#216](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/216) hybrid volume
- [#569](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/569) protocol TVL
- [#586](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/586) / [#613](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/613) / [#614](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614) treasury fees
- [#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562) production gem hide
- [#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629) token recognition (Llama pricing coverage)
- [#224](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/224) CG/CMC (do not conflate)
