# CoinGecko & CoinMarketCap API Compliance

This document describes the CL8Y DEX's self-hosted market data API endpoints that comply with CoinGecko (CG) and CoinMarketCap (CMC) **exchange listing** specifications. These endpoints enable aggregators, portfolio trackers, and market data platforms to list and track the DEX.

**Not DeFiLlama.** Llama does not poll `/cg/*` or `/cmc/*`. TVL/volume/fees listing is [`docs/DEFILLAMA.md`](./DEFILLAMA.md) (`GET /api/v1/defillama/daily` + on-chain factory adapters, [GitLab #631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631)). CG `liquidity_in_usd` is AMM v2 pool TVL from the `pair_liquidity_usd` stamp ([#685](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/685)) — still **never** publish it as Llama TVL (Llama reads on-chain `Pool {}` only).

**Last verified:** GitLab [**#685**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/685) listing field truthfulness (`make verify-issue-685`). `#224` remains the closed spec-matrix pass (timestamps / wrappers). Live handlers in [`indexer/src/api/cg.rs`](../indexer/src/api/cg.rs) and [`cmc.rs`](../indexer/src/api/cmc.rs); OpenAPI (utoipa) on the indexer Swagger UI. When this markdown and code disagree, **code wins** until a doc PR lands.

## Table of Contents

1. [Not CoinGecko Pro API v3](#not-coingecko-pro-api-v3)
2. [Spec compliance matrix](#spec-compliance-matrix)
3. [CoinGecko Endpoints (`/cg/`)](#coingecko-endpoints-cg)
4. [CoinMarketCap Endpoints (`/cmc/`)](#coinmarketcap-endpoints-cmc)
5. [GeckoTerminal (On-Chain)](#geckoterminal-on-chain)
6. [Hybrid Orderbook Simulation](#hybrid-orderbook-simulation)
7. [AMM Orderbook Simulation](#amm-orderbook-simulation)
8. [Listing Submission Guide](#listing-submission-guide)
9. [Compliance verification checklist](#compliance-verification-checklist)
10. [Related References](#related-references)

---

## Not CoinGecko Pro API v3

> **Warning:** [CoinGecko Pro API v3](https://docs.coingecko.com/) (`/api/v3/...` on `api.coingecko.com`) is the **aggregator consumer API**. CL8Y implements the **self-hosted exchange integration** surface (`/cg/*`, `/cmc/*`) that listing crawlers poll on **your** API domain — the same class as [Kujira FIN](https://docs.kujira.app/dapps-and-infrastructure/fin/coingecko-api.md) and [Openware Peatio CMC](https://openware.com/sdk/2.6/docs/peatio/peatio/coin-market-cap). Do not point integrators or listing forms at Pro v3 paths.

Keplr **Add Token** name/logo is a different listing: [keplr-contract-registry](https://github.com/chainapsis/keplr-contract-registry) under `cosmos/columbus` ([#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629)). CL8Y’s CoinGecko id `ceramicliberty-com` is still BSC-only; that does not change `/cg/*`. Pack: [`listings/keplr-contract-registry/`](./listings/keplr-contract-registry/). Exchange **form** submit + other wallets: [listing venue catalog](./listings/README.md) ([#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639)).

---

## Spec compliance matrix

| Endpoint | Official ref | CL8Y path | Match | Notes |
|----------|--------------|-----------|-------|-------|
| Pairs | [Kujira FIN CG](https://docs.kujira.app/dapps-and-infrastructure/fin/coingecko-api.md) | `GET /cg/pairs` | Partial | Kujira wraps `{ "pairs": [...] }`; CL8Y returns a **top-level JSON array** (simpler for crawlers). Fields `ticker_id`, `base`, `target`, `pool_id` match. Gems omitted (#685 / **L639-2**). |
| Tickers | Kujira FIN CG | `GET /cg/tickers` | Partial | Kujira wraps `{ "tickers": [...] }`; CL8Y returns a **top-level array**. `liquidity_in_usd` is AMM TVL stamp (#685). Standard volume fields are **consolidated** swap totals; optional `cl8y_extensions` ([#189](#consolidated-hybrid--pool-only-reporting-gitlab-189)). Gems omitted (**L639-2**). |
| Orderbook | Kujira FIN CG | `GET /cg/orderbook` | Yes | `timestamp` = JSON **number**, Unix **ms**; `bids`/`asks` = `[price, qty]` strings, sorted. **Hybrid-simulated** depth ([#220](#hybrid-orderbook-simulation)). Query `depth` = **total** levels (Openware split [#221](#amm-orderbook-simulation)), not Kujira per-side semantics. |
| Historical trades | Kujira FIN CG | `GET /cg/historical_trades` | Yes | `trade_timestamp` = JSON **number**, Unix **seconds**; grouped `buy` / `sell` arrays. |
| Summary | [Openware CMC](https://openware.com/sdk/2.6/docs/peatio/peatio/coin-market-cap) | `GET /cmc/summary` | Yes | Array of market rows; optional `cl8y_extensions` on summary rows when indexed. |
| Assets | Openware CMC | `GET /cmc/assets` | Partial | Openware shows array-of-maps; CL8Y returns one **object** keyed by symbol (equivalent data). |
| Ticker | Openware CMC | `GET /cmc/ticker` | Partial | One **object** keyed by `BASE_QUOTE` (not array-of-maps). `base_id` / `quote_id` are numeric CMC ids (else `0`); contracts live in `cl8y_*_address`. `isFrozen` is `"1"` when F6-frozen. |
| Orderbook | Openware CMC | `GET /cmc/orderbook/:market_pair` | Yes | Root **array** with **one** book object ([#223](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/223)); `timestamp` = Unix **seconds** (intentional delta: Openware text says ms, CL8Y aligns with `/cmc/trades` seconds — see [#222](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/222)). `/cg/orderbook` remains a **single object** (out of scope for #223). |
| Trades | Openware CMC | `GET /cmc/trades/:market_pair` | Partial | `timestamp` = Unix **seconds** (Openware example text says ms; CL8Y uses seconds consistently on CMC trade feeds). |

Path prefix: CL8Y serves `/cg/` and `/cmc/` instead of upstream `/api/coingecko/` or `/api/v2/coinmarketcap/` — configure listing forms with your API base + these prefixes.

Agent playbook: [`skills/AGENTS_INDEXER_CG_CMC_LISTING.md`](../skills/AGENTS_INDEXER_CG_CMC_LISTING.md) (**L685**, [#685](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/685)). Depth sim: [`skills/AGENTS_INDEXER_AMM_ORDERBOOK_SIM.md`](../skills/AGENTS_INDEXER_AMM_ORDERBOOK_SIM.md). Indexer invariants: [`indexer-invariants.md`](./indexer-invariants.md).

---

## CoinGecko Endpoints (`/cg/`)

Base URL: `https://<your-api-domain>/cg/`

These endpoints follow the [Kujira FIN CoinGecko integration](https://docs.kujira.app/dapps-and-infrastructure/fin/coingecko-api.md) shape used for Terra-ecosystem DEX listings. CoinGecko's crawler will poll these endpoints to populate exchange data on coingecko.com.

### `GET /cg/pairs`

Returns all available trading pairs on the DEX.

**Response:**

```json
[
  {
    "ticker_id": "CL8Y_WLUNC",
    "base": "CL8Y",
    "target": "WLUNC",
    "pool_id": "terra1abc...xyz"
  }
]
```

| Field | Type | Description |
|-------|------|-------------|
| `ticker_id` | string | Identifier: `{BASE_SYMBOL}_{TARGET_SYMBOL}` |
| `base` | string | Symbol of the base asset |
| `target` | string | Symbol of the target (quote) asset |
| `pool_id` | string | On-chain pair contract address |

### `GET /cg/tickers`

Returns 24-hour market data for trading pairs, ranked by 24h quote volume (highest first).

**Query parameters (GitLab #288):**

| Param | Default | Max | Description |
|-------|---------|-----|-------------|
| `limit` | `100` | `1000` | Number of pairs returned |
| `offset` | `0` | `10000` | Pagination offset (deep offset → **400**) |

Responses are cached **60s** per `(limit, offset)`; stats are loaded with set-based queries (not per-pair N+1).

**Response:**

```json
[
  {
    "ticker_id": "CL8Y_WLUNC",
    "base_currency": "terra1cl8y_contract_addr",
    "target_currency": "terra1wlunc_contract_addr",
    "last_price": "0.00005123",
    "base_volume": "1234567890",
    "target_volume": "63245",
    "bid": "0.00005118",
    "ask": "0.00005128",
    "high": "0.00005500",
    "low": "0.00004900",
    "pool_id": "terra1abc...xyz",
    "liquidity_in_usd": "12345.67"
  }
]
```

| Field | Type | Description |
|-------|------|-------------|
| `ticker_id` | string | Pair identifier matching `/cg/pairs` |
| `base_currency` | string | Contract address (CW20) or denom (native) of base asset |
| `target_currency` | string | Contract address or denom of target asset |
| `last_price` | string | Last traded price (human quote per base) |
| `base_volume` | string | 24h volume denominated in base asset (raw units) |
| `target_volume` | string | 24h volume denominated in target asset (raw units) |
| `bid` | string | Reserve-implied mid ± pair `fee_bps` when `pair_reserves` exist; otherwise `last_price` both sides (omit-equivalent). **Not** the hybrid book. |
| `ask` | string | Same rule as `bid` (ask side). |
| `high` | string | 24h high price |
| `low` | string | 24h low price |
| `pool_id` | string | On-chain pair contract address |
| `liquidity_in_usd` | string | AMM v2 pool TVL USD from `pair_liquidity_usd` / `protocol_pair_tvl` ([#655](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/655) / [#685](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/685)). Unpriced / stale → `"0"`. |

**Notes:**
- `liquidity_in_usd` is **not** 24h volume. Never `$1` UST1, never `2.5×` USTR, never book escrow. Llama must not ingest this field ([#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631)).
- `bid` / `ask` on the **list** path are computed from SQL stamps only (no LCD N+1, no `f64 * 0.999`). Hybrid book best is `/cg/orderbook`.
- Gem / ALPHA / USTRIX / SpaceUSD pairs are omitted (**L639-2**), lockstep with `/gt`.
- Duplicate `SYMBOL_SYMBOL` ticker ids are not resolved silently — `/cg/pairs` still lists both `pool_id`s; orderbook/trades for that `ticker_id` return **404**.

**On-chain limit orders:** CG/CMC `orderbook` depth is **hybrid-simulated** (AMM curve walk + resting FIFO limits merged for listing). It is **not** a live CEX L2 feed or guaranteed fill quote. Resting maker orders are also available via indexer `limit-book` and on-chain queries — see [limit-orders.md](./limit-orders.md) (GitLab **#220**).

### Consolidated hybrid + pool-only reporting (GitLab #189)

CL8Y v2 pairs may settle swaps through the **pool only** or through a **hybrid** (pool + limit book) in a **single** `swap_events` row. Listing endpoints report **consolidated** volumes — there is **no double-count** with separate `limit_order_fills` rows:

| Field | Semantics |
|-------|-----------|
| `base_volume` / `target_volume` (tickers) | 24h totals bucket `offer_amount` / `return_amount` by pair **base/quote** orientation (`asset_0` / `asset_1`), not raw offer/return columns ([#466](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/466)). |
| `base_volume` / `quote_volume` (CMC summary) | Same oriented totals. |
| `price` (trades) | **Human quote per base** after orientation ([#466](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/466)) and decimal scale ([#522](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522)): `raw × 10^(decimals_base − decimals_quote)`. CG/CMC `last_price` stays this quote-currency figure, not `price_usd`. |
| `cl8y_extensions` (tickers + CMC summary) | Optional attribution block: `hybrid_trade_count_24h`, `pool_only_trade_count_24h`, `book_leg_volume_quote_24h`, `pool_leg_volume_quote_24h`. Safe for aggregators to ignore. |
| `pool_leg_volume` / `book_leg_volume` (trades) | Present on hybrid swaps when indexed; sum to the trade's consolidated `target_volume` / `quote_volume`. |

Example ticker extension:

```json
"cl8y_extensions": {
  "consolidated": true,
  "hybrid_trade_count_24h": "12",
  "pool_only_trade_count_24h": "340",
  "book_leg_volume_quote_24h": "5500000",
  "pool_leg_volume_quote_24h": "42000000"
}
```

Hybrid best-execution routing for integrators: [`GET /api/v1/route/solve/best`](./integrators.md#route-discovery-and-quotes-l8). Terraport field mapping: [integrators.md § Vyntrex](./integrators.md#vyntrex--terraport-hybrid-event-mapping-gitlab-189).

**Volume reconciliation (required for listings):** [integrators-hybrid-volume.md](./integrators-hybrid-volume.md) — headline 24h volume, leg attribution, anti-patterns, and API mapping ([GitLab #216](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/216)). Indexer invariant **L10**: [indexer-invariants.md](./indexer-invariants.md).

### `GET /cg/orderbook`

Returns a **hybrid-simulated** order book: constant-product pool levels plus resting on-chain limit orders (merged, price-sorted, capped at `depth`). Set env `ORDERBOOK_HYBRID=0` for pool-only rollback.

**Query Parameters:**

| Parameter | Required | Default | Description |
|-----------|----------|---------|-------------|
| `ticker_id` | Yes | — | Pair identifier (e.g. `CL8Y_WLUNC`) |
| `depth` | No | 20 | Total levels across the book (split evenly: `depth=100` → 50 bids + 50 asks; max 100 total). [Openware/CMC](https://openware.com/sdk/2.6/docs/peatio/peatio/coin-market-cap); GitLab **#221** |

**Response:**

```json
{
  "ticker_id": "CL8Y_WLUNC",
  "timestamp": 1710100000000,
  "bids": [
    ["0.00005120", "10000000"],
    ["0.00005100", "25000000"]
  ],
  "asks": [
    ["0.00005126", "10000000"],
    ["0.00005150", "25000000"]
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `ticker_id` | string | Pair identifier |
| `timestamp` | number | Unix timestamp in milliseconds |
| `bids` | array | `[[price, quantity], ...]` — buy orders sorted high to low |
| `asks` | array | `[[price, quantity], ...]` — sell orders sorted low to high |

See [Hybrid Orderbook Simulation](#hybrid-orderbook-simulation) (pool + limits) and [AMM Orderbook Simulation](#amm-orderbook-simulation) (pool leg only).

### `GET /cg/historical_trades`

Returns recent trades for a given pair.

**Query Parameters:**

| Parameter | Required | Default | Description |
|-----------|----------|---------|-------------|
| `ticker_id` | Yes | — | Pair identifier |
| `type` | No | both | `buy`, `sell`, or omit for both |
| `limit` | No | 100 | Max trades to return (max **500**) |

**Response:**

```json
{
  "buy": [
    {
      "trade_id": 123456,
      "price": "0.00005123",
      "base_volume": "1000000",
      "target_volume": "51230",
      "trade_timestamp": 1710100000,
      "type": "buy"
    }
  ],
  "sell": []
}
```

| Field | Type | Description |
|-------|------|-------------|
| `trade_id` | number | Unique trade ID (database primary key) |
| `price` | string | Executed price |
| `base_volume` | string | Amount in base asset (raw units) |
| `target_volume` | string | Amount in target asset (raw units) |
| `trade_timestamp` | number | Unix timestamp in **seconds** |
| `type` | string | `"buy"` or `"sell"` |

**Trade direction:** A trade is `"buy"` when the trader **offers quote** (`asset_1`) to **buy base** (`asset_0`). `"sell"` is offer = base. This matches CEX convention (buy the base). GitLab #685.

---

## CoinMarketCap Endpoints (`/cmc/`)

Base URL: `https://<your-api-domain>/cmc/`

These endpoints follow the [CMC exchange API specification](https://openware.com/sdk/2.6/docs/peatio/peatio/coin-market-cap) for exchange listings.

### `GET /cmc/summary`

Overview of market data for tickers and markets, ranked by 24h quote volume (highest first). Same `limit` / `offset` pagination as `/cg/tickers` (default `limit=100`; GitLab #288).

**Response:**

```json
[
  {
    "trading_pairs": "CL8Y_WLUNC",
    "base_currency": "CL8Y",
    "quote_currency": "WLUNC",
    "last_price": "0.00005123",
    "lowest_ask": "0.00005128",
    "highest_bid": "0.00005118",
    "base_volume": "1234567890",
    "quote_volume": "63245",
    "price_change_percent_24h": "2.5",
    "highest_price_24h": "0.00005500",
    "lowest_price_24h": "0.00004900"
  }
]
```

| Field | Type | Description |
|-------|------|-------------|
| `trading_pairs` | string | Pair identifier with `_` delimiter |
| `base_currency` | string | Base asset symbol |
| `quote_currency` | string | Quote asset symbol |
| `last_price` | string | Last transacted price |
| `lowest_ask` | string | Reserve-implied mid ± `fee_bps` (same rule as CG ticker `ask`) |
| `highest_bid` | string | Reserve-implied mid ± `fee_bps` (same rule as CG ticker `bid`) |
| `base_volume` | string | 24h volume in base currency (raw units) |
| `quote_volume` | string | 24h volume in quote currency (raw units) |
| `price_change_percent_24h` | string | 24h price change percentage |
| `highest_price_24h` | string | 24h high |
| `lowest_price_24h` | string | 24h low |

### `GET /cmc/assets`

Detailed summary for each available currency on the DEX.

**Response:**

```json
{
  "CL8Y": {
    "name": "CL8Y Token",
    "unified_cryptoasset_id": 0,
    "can_withdraw": true,
    "can_deposit": true,
    "min_withdraw": "0"
  },
  "WLUNC": {
    "name": "Wrapped LUNC",
    "unified_cryptoasset_id": 0,
    "can_withdraw": true,
    "can_deposit": true,
    "min_withdraw": "0"
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Full name of the cryptocurrency |
| `unified_cryptoasset_id` | number | CoinMarketCap unified ID (set via `cmc_id` in assets table, 0 if not mapped) |
| `can_withdraw` | boolean | Always `true` (DEX — users control their own tokens) |
| `can_deposit` | boolean | Always `true` |
| `min_withdraw` | string | Minimum withdrawal amount (always `"0"` for a DEX) |

**Note:** The `unified_cryptoasset_id` must be manually configured in the `assets` table (`cmc_id` column) by looking up each token's ID on [CoinMarketCap](https://coinmarketcap.com/). Set to `0` for tokens not yet listed on CMC.

### `GET /cmc/ticker`

24-hour pricing and volume summary per market pair (object keyed by `BASE_QUOTE`). Same `limit` / `offset` pagination as `/cg/tickers` (default `limit=100`; GitLab #288).

**Response:**

```json
{
    "CL8Y_WLUNC": {
    "base_id": 0,
    "quote_id": 0,
    "last_price": "0.00005123",
    "base_volume": "1234567890",
    "quote_volume": "63245",
    "isFrozen": "0",
    "cl8y_base_address": "terra1cl8y_contract_addr",
    "cl8y_quote_address": "terra1wlunc_contract_addr"
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `base_id` | number | CMC unified ID of base asset (`assets.cmc_id`, else `0`). **Not** a contract string. |
| `quote_id` | number | CMC unified ID of quote asset (`assets.cmc_id`, else `0`). **Not** a contract string. |
| `last_price` | string | Last transacted price |
| `base_volume` | string | 24h volume in base currency |
| `quote_volume` | string | 24h volume in quote currency |
| `isFrozen` | string | `"0"` = active, `"1"` = F6 `code_id_frozen` ([#585](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/585)) |
| `cl8y_base_address` | string | Additive: CW20 contract or native denom of the base leg |
| `cl8y_quote_address` | string | Additive: CW20 contract or native denom of the quote leg |

### `GET /cmc/orderbook/:market_pair`

Level 2 order book for a specific market pair.

**Path Parameters:**

| Parameter | Description |
|-----------|-------------|
| `market_pair` | Pair identifier (e.g. `CL8Y_WLUNC`) |

**Query Parameters:**

| Parameter | Required | Default | Description |
|-----------|----------|---------|-------------|
| `depth` | No | 20 | Total levels across the book (split evenly per side; max 100 total). GitLab **#221** |

**Response:** (Openware [array wrapper](https://openware.com/sdk/2.6/docs/peatio/peatio/coin-market-cap) — **exactly one** object per request; GitLab **#223**)

```json
[
  {
    "timestamp": 1710100000,
    "bids": [
      ["0.00005120", "10000000"],
      ["0.00005100", "25000000"]
    ],
    "asks": [
      ["0.00005126", "10000000"],
      ["0.00005150", "25000000"]
    ]
  }
]
```

| Field | Type | Description |
|-------|------|-------------|
| (root) | array | Single-element array per Openware Peatio CMC module |
| `timestamp` | number | Unix timestamp in **seconds** (same unit as `/cmc/trades`; GitLab **#222**, **#224**) |
| `bids` | array | `[[price, quantity], ...]` — buy orders sorted high to low |
| `asks` | array | `[[price, quantity], ...]` — sell orders sorted low to high |

Hybrid-simulated depth — not on-chain FIFO L2: [limit-orders.md](./limit-orders.md), [Hybrid Orderbook Simulation](#hybrid-orderbook-simulation).

### `GET /cmc/trades/:market_pair`

Recent trades for a specific market pair.

**Path Parameters:**

| Parameter | Description |
|-----------|-------------|
| `market_pair` | Pair identifier (e.g. `CL8Y_WLUNC`) |

**Response:**

```json
[
  {
    "trade_id": 123456,
    "price": "0.00005123",
    "base_volume": "1000000",
    "quote_volume": "51230",
    "timestamp": 1710100000,
    "type": "buy"
  }
]
```

---

## GeckoTerminal (On-Chain)

Terra Classic is **not** in GeckoTerminal `GET /api/v2/networks`, so Uniswap-V2 auto-detect is unavailable. Non-EVM listing uses the [Integration API](https://docs.google.com/document/d/1ufjAJUa6rGO9PBGJGwfBMn-XMk9NE0ow3_iMYrS3drk) on **our** indexer ([#646](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/646)):

| Path | Notes |
|------|--------|
| `GET /gt/latest-block` | `last_indexed_height` |
| `GET /gt/asset?id=` | CW20 or `uluna` / `uusd` |
| `GET /gt/pair?id=` | `dexKey` = `cl8y` |
| `GET /gt/events?fromBlock=&toBlock=` | Swaps + join/exit; gems excluded |

Base: `https://indexer.dex.cl8y.com/gt`. Do **not** point GeckoTerminal at `/cg/*` (CoinGecko exchange shape). Form pack: [`listings/forms/geckoterminal.md`](./listings/forms/geckoterminal.md). Verify: `make verify-issue-646`.

---

## Hybrid Orderbook Simulation

GitLab **#220**. `/cg/orderbook` and `/cmc/orderbook/*` return **hybrid-simulated** depth:

1. **Pool leg** — constant-product curve walk ([`orderbook_sim.rs`](../indexer/src/api/orderbook_sim.rs), **#210**).
2. **Limit leg** — up to `levels_per_side(depth)` resting orders per side from LCD (`order_book_head` + `limit_order` pagination, same caps as [`limit-book`](./limit-orders.md)).
3. **Merge** — concatenate pool + limit levels, **sort** (bids: price descending; asks: ascending), **sum quantities** at identical price strings, **truncate** to `levels_per_side(depth)` per side (**#221** total-depth split).

**Disclosure:** Listing crawlers must treat levels as **indicative simulation** — execution may require on-chain hybrid swap or limit matching; not all displayed size is immediately marketable at the printed price.

**Rollback:** `ORDERBOOK_HYBRID=0` (or `false`) disables limit merge (pool-only, pre-#220 behavior).

**Cache:** 30s per `(pair, depth, fee_bps, bid_head, ask_head)` — see [`indexer-invariants.md`](./indexer-invariants.md).

Implementation: [`hybrid_orderbook_sim.rs`](../indexer/src/api/hybrid_orderbook_sim.rs), wired from [`orderbook_sim.rs`](../indexer/src/api/orderbook_sim.rs). Agent playbook: [`skills/AGENTS_INDEXER_AMM_ORDERBOOK_SIM.md`](../skills/AGENTS_INDEXER_AMM_ORDERBOOK_SIM.md).

### Listing QA (hybrid orderbook)

- [ ] `/cg/orderbook` and `/cmc/orderbook/*` for a pair with resting limits include limit prices in the merged ladder.
- [ ] Pair with empty limit book matches pool-only depth (modulo merge sort).
- [ ] `depth` capped at 100; repeat requests within 30s do not amplify LCD (cache).
- [ ] Product copy states **hybrid-simulated**, not live CEX L2.

---

## AMM Orderbook Simulation

The **pool leg** of hybrid orderbook depth. Constant-product AMMs (x × y = k) do not have a traditional FIFO book; the curve walk below supplies pool levels only. Full CG/CMC responses also merge resting limits when hybrid mode is on (**#220**). On-chain FIFO book: [`limit-orders.md`](./limit-orders.md), indexer `limit-book`. Pool math: [`orderbook_sim.rs`](../indexer/src/api/orderbook_sim.rs) (**#210**, Openware total-depth split **#221**). Invariants: [`indexer-invariants.md`](./indexer-invariants.md).

**Conventions:** `asset_0` = **base**, `asset_1` = **quote**. Levels are `[price, quantity]` decimal strings (smallest on-chain units). `price` = quote per base after pool fee.

**Depth (Openware / CMC exchange integration, GitLab #221):** Query `depth` is the **total** number of levels across the book, split **evenly per side** with integer floor (`levels_per_side = max(1, depth / 2)`). Examples: default `20` → 10 bids + 10 asks; `depth=100` (cap) → 50+50; `depth=1` → 1+1. Odd totals drop the remainder (e.g. `21` → 10+10). This differs from Kujira FIN’s per-side semantics. Cap: **100** total; cache key uses the **requested** `depth` query value (not per-side count); TTL **30s** per `(pair, depth, fee_bps)`.

### Method

Given pool reserves `(R0, R1)`, `k = R0 * R1`, and pool `fee_bps` (indexer `pairs.fee_bps` when the pair is indexed, else LCD `get_fee_config`):

For each step `i` from **1** to `levels_per_side` (derived from query `depth` as above):

- `step_amount = R0 * (i / levels_per_side) * 0.10` (integer division; up to **10%** of base reserves at `i = levels_per_side`)

**Bids** (sell base → receive quote; effective price **decreases** with size):

1. `sell_amount = step_amount`
2. `R0' = R0 + sell_amount`
3. `R1' = ceil_div(k, R0')` (pair-style ceiling division — pool-favorable)
4. `gross_quote = R1 - R1'`
5. `net_quote = gross_quote - (gross_quote * fee_bps / 10000)`
6. `effective_price = net_quote / sell_amount`
7. Level: `[effective_price, sell_amount]`

**Asks** (buy base → pay quote; effective price **increases** with size):

1. `buy_amount = step_amount` (must be `< R0`)
2. `R0' = R0 - buy_amount`
3. `R1' = ceil_div(k, R0')`
4. `gross_quote_cost = R1' - R1`
5. `net_base = buy_amount - (buy_amount * fee_bps / 10000)` (fee on base output, same as pair pool swap)
6. `effective_price = gross_quote_cost / net_base`
7. Level: `[effective_price, buy_amount]`

This matches the pair contract pool leg (`ceil_div`, commission on gross output). Trader-specific fee discounts are **not** applied on public orderbook endpoints.

---

## Listing Submission Guide

### CoinGecko

1. Ensure all `/cg/` endpoints are live and returning valid data
2. Go to [CoinGecko Partners Platform](https://partner.coingecko.com/request-form/new) → Decentralized Spot Exchange (account + captcha). Field pack: [`listings/forms/coingecko-exchange.md`](./listings/forms/coingecko-exchange.md) ([#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639)).
3. Provide:
   - Exchange name: CL8Y DEX
   - Exchange type: Decentralized (AMM)
   - API base URL: `https://indexer.dex.cl8y.com/cg/` (not CoinGecko Pro v3)
   - Network: Terra Classic (columbus-5)
   - Supported endpoints: `/pairs`, `/tickers`, `/orderbook`, `/historical_trades`
4. CoinGecko will crawl the endpoints and verify data quality

### CoinMarketCap

1. Ensure all `/cmc/` endpoints are live and returning valid data
2. Apply at [coinmarketcap.com/request](https://coinmarketcap.com/request/) → Add exchange. Criteria: [Listings Criteria](https://support.coinmarketcap.com/hc/en-us/articles/360043659351-Listings-Criteria) (60-day operation). Field pack: [`listings/forms/coinmarketcap-exchange.md`](./listings/forms/coinmarketcap-exchange.md) ([#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639)). CMC DexScan is **not** the Terra Classic path.
3. Provide:
   - Exchange name and website (`https://dex.cl8y.com`, books visible logged-out)
   - API documentation URL (link to this document)
   - API base URL: `https://indexer.dex.cl8y.com/cmc/` (not CoinGecko Pro v3)
   - Factory: `terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea`
   - For each token, the `unified_cryptoasset_id` should be set in the assets table once CMC ids exist
4. CMC will verify endpoint compliance and data freshness

### Token-Level Listings

Individual tokens traded on the DEX can also be listed on CG/CMC:
- Use the existing `listing-api` repo for CG/CMC supply endpoints (`/api/v3/supply/:symbol`, `/cmc/circulating/:symbol`)
- Tokens need sufficient trading volume and liquidity to be listed

---

## Listing compliance sign-off

Before submitting exchange listings, confirm:

- [ ] Headline 24h volume uses consolidated `swap_events` / ticker `base_volume` & `target_volume` (not `limit_order_fills` sums).
- [ ] Hybrid leg fields understood as **ask-side attribution**; `cl8y_extensions` leg sums are not added to standard volume fields.
- [ ] Team has read [integrators-hybrid-volume.md](./integrators-hybrid-volume.md) and [indexer-invariants.md](./indexer-invariants.md) **L10**.

---

## Compliance verification checklist

Use after deploy or indexer release (GitLab **#224**, CMC orderbook array **#223**):

| # | Check | Command / expectation |
|---|--------|------------------------|
| 1 | `/cg/pairs` fields | `curl -s …/cg/pairs \| jq '.[0] \| keys'` → `ticker_id`, `base`, `target`, `pool_id` |
| 2 | `/cg/tickers` + extensions | Consolidated volumes; `cl8y_extensions` present when hybrid indexed |
| 3 | `/cg/orderbook` | `timestamp` is number, magnitude ~1.7e12 (ms); bids descending, asks ascending |
| 4 | `/cg/historical_trades` | `trade_timestamp` seconds (~1.7e9) |
| 5 | `/cmc/summary` | Array of rows; Openware field names |
| 6 | `/cmc/orderbook/:pair` | `jq 'type'` → `"array"`; `length == 1`; `timestamp` seconds; `depth` total cap 100 ([#223](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/223)) |
| 7 | `/cmc/trades/:pair` | Array of trades; `timestamp` seconds |
| 8 | Not Pro API v3 | Listing form uses your `/cg` + `/cmc` base URL only |
| 9 | Simulated book disclosure | Product copy references hybrid-sim vs `limit-book` |
| 10 | CI | `cd indexer && cargo test --test api_orderbook_lcd_mock --test api_cg --test api_cmc --test api_cg_cmc_listing` · `make verify-issue-685` |

---

## Token metadata human review {#token-metadata-human-review}

Indexer token rows (`logo_url`, symbol, decimals) are **operator-curated** before they appear in the retail dApp ([#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378) / M-09). Listing a new CW20 requires human review of:

1. Confirm the CW20 contract address matches the intended asset (factory whitelist / governance records).
2. Verify symbol/name against an independent source (on-chain `token_info`, official project site).
3. Only approve `logo_url` values on **allowlisted HTTPS hosts** ([`tokenLogoAllowlist.ts`](../frontend-dapp/src/utils/tokenLogoAllowlist.ts) — e.g. `gitlab.com`, CoinGecko/CMC CDN, IPFS gateways). Reject arbitrary third-party image hosts.
4. No look-alike tickers vs major assets without explicit governance approval.
5. Re-review on metadata changes; remove or blank `logo_url` on delist.

The dApp does **not** add extra token detail to swap confirmation — logos use the allowlist with blockie fallback. Security context: [Security model § Off-chain trust boundaries](./security-model.md#off-chain-trust-boundaries-frontend).

---

## Related References

- [integrators-hybrid-volume.md](./integrators-hybrid-volume.md) — volume reconciliation guide (#216)
- [`skills/AGENTS_INDEXER_CG_CMC_LISTING.md`](../skills/AGENTS_INDEXER_CG_CMC_LISTING.md) — listing field truthfulness ([#685](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/685))
- [`skills/AGENTS_INDEXER_AMM_ORDERBOOK_SIM.md`](../skills/AGENTS_INDEXER_AMM_ORDERBOOK_SIM.md) — agent playbook for CG/CMC depth ([#224](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/224), CMC array wrapper [#223](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/223))
- [`skills/AGENTS_TESTING_P2_EPIC.md`](../skills/AGENTS_TESTING_P2_EPIC.md) — indexer integration test matrix
- [`gaps/GAP_1780023683.md`](../gaps/GAP_1780023683.md) — gap matrix (orderbook sim row)
- **`listing-api` repo** — Existing CoinGecko/CMC token supply API endpoints (implemented)
- **`krchange-dapp/PROPOSAL_FEAT_001.md`** — CG/CMC/GeckoTerminal compatibility API proposal for KRChange
- **`ustr-cmm/plans/DEX_PLAN.md` Section 8** — CoinGecko-compatible API specification for UST1 DEX
- **[CMC Exchange Integration Spec](https://openware.com/sdk/2.6/docs/peatio/peatio/coin-market-cap)** — Official CMC endpoint format reference
- **[GeckoTerminal API Docs](https://api.geckoterminal.com/docs/index.html)** — On-chain DEX data API by CoinGecko

---

## Rate Limits

| Tier | Limit | Description |
|------|-------|-------------|
| Public | 60 req/min | Default for unauthenticated requests |
| Aggregator | Unlimited | CoinGecko/CMC crawlers (whitelist by IP) |

Rate limiting is enforced at the API gateway level. CoinGecko and CMC crawler IPs should be whitelisted for unthrottled access.
