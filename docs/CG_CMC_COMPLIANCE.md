# CoinGecko & CoinMarketCap API Compliance

This document describes the CL8Y DEX's self-hosted market data API endpoints that comply with CoinGecko (CG) and CoinMarketCap (CMC) exchange integration specifications. These endpoints enable aggregators, portfolio trackers, and market data platforms to list and track the DEX.

## Table of Contents

1. [CoinGecko Endpoints (`/cg/`)](#coingecko-endpoints-cg)
2. [CoinMarketCap Endpoints (`/cmc/`)](#coinmarketcap-endpoints-cmc)
3. [GeckoTerminal (On-Chain)](#geckoterminal-on-chain)
4. [AMM Orderbook Simulation](#amm-orderbook-simulation)
5. [Listing Submission Guide](#listing-submission-guide)
6. [Related References](#related-references)

---

## CoinGecko Endpoints (`/cg/`)

Base URL: `https://<your-api-domain>/cg/`

These endpoints follow the CoinGecko exchange API specification used for DEX listings. CoinGecko's crawler will poll these endpoints to populate exchange data on coingecko.com.

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

Returns 24-hour market data for all trading pairs.

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
    "liquidity_in_usd": "0"
  }
]
```

| Field | Type | Description |
|-------|------|-------------|
| `ticker_id` | string | Pair identifier matching `/cg/pairs` |
| `base_currency` | string | Contract address (CW20) or denom (native) of base asset |
| `target_currency` | string | Contract address or denom of target asset |
| `last_price` | string | Last traded price (target per base) |
| `base_volume` | string | 24h volume denominated in base asset (raw units) |
| `target_volume` | string | 24h volume denominated in target asset (raw units) |
| `bid` | string | Simulated best bid price (last_price * 0.999) |
| `ask` | string | Simulated best ask price (last_price * 1.001) |
| `high` | string | 24h high price |
| `low` | string | 24h low price |
| `pool_id` | string | On-chain pair contract address |
| `liquidity_in_usd` | string | Pool liquidity in USD (requires external price oracle) |

**Notes:**
- `bid` and `ask` are simulated from the last trade price since AMMs don't have a traditional order book. The spread is set to 0.2% (0.1% each side).
- `liquidity_in_usd` is `"0"` unless a USD price oracle is configured.

### `GET /cg/orderbook`

Returns a simulated order book derived from the AMM constant-product curve.

**Query Parameters:**

| Parameter | Required | Default | Description |
|-----------|----------|---------|-------------|
| `ticker_id` | Yes | — | Pair identifier (e.g. `CL8Y_WLUNC`) |
| `depth` | No | 20 | Number of bid/ask levels |

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

See [AMM Orderbook Simulation](#amm-orderbook-simulation) for how levels are computed.

### `GET /cg/historical_trades`

Returns recent trades for a given pair.

**Query Parameters:**

| Parameter | Required | Default | Description |
|-----------|----------|---------|-------------|
| `ticker_id` | Yes | — | Pair identifier |
| `type` | No | both | `buy`, `sell`, or omit for both |
| `limit` | No | 100 | Max trades to return (max 200) |

**Response:**

```json
{
  "buy": [
    {
      "trade_id": 123456,
      "price": "0.00005123",
      "base_volume": "1000000",
      "target_volume": "51230",
      "trade_timestamp": 1710100000000,
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
| `trade_timestamp` | number | Unix timestamp in milliseconds |
| `type` | string | `"buy"` or `"sell"` |

**Trade direction:** A trade is classified as `"buy"` if the offer asset matches the base asset (trader is buying the target), and `"sell"` if the offer asset matches the target (trader is selling the target for the base).

---

## CoinMarketCap Endpoints (`/cmc/`)

Base URL: `https://<your-api-domain>/cmc/`

These endpoints follow the [CMC exchange API specification](https://openware.com/sdk/2.6/docs/peatio/peatio/coin-market-cap) for exchange listings.

### `GET /cmc/summary`

Overview of market data for all tickers and markets.

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
| `lowest_ask` | string | Simulated lowest ask price |
| `highest_bid` | string | Simulated highest bid price |
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

24-hour pricing and volume summary for each market pair.

**Response:**

```json
{
  "CL8Y_WLUNC": {
    "base_id": 0,
    "quote_id": 0,
    "last_price": "0.00005123",
    "base_volume": "1234567890",
    "quote_volume": "63245",
    "isFrozen": 0
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `base_id` | number | CMC unified ID of base asset |
| `quote_id` | number | CMC unified ID of quote asset |
| `last_price` | string | Last transacted price |
| `base_volume` | string | 24h volume in base currency |
| `quote_volume` | string | 24h volume in quote currency |
| `isFrozen` | number | `0` = active, `1` = frozen/disabled |

### `GET /cmc/orderbook/:market_pair`

Level 2 order book for a specific market pair.

**Path Parameters:**

| Parameter | Description |
|-----------|-------------|
| `market_pair` | Pair identifier (e.g. `CL8Y_WLUNC`) |

**Query Parameters:**

| Parameter | Required | Default | Description |
|-----------|----------|---------|-------------|
| `depth` | No | 20 | Order book depth (levels per side) |

**Response:**

```json
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
```

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

GeckoTerminal (owned by CoinGecko) crawls on-chain data directly from blockchain nodes. It does **not** require a self-hosted API.

### Requirements for GeckoTerminal Listing

1. **Network support**: Terra Classic must be supported as a network in GeckoTerminal. Check [GeckoTerminal Networks](https://api.geckoterminal.com/api/v2/networks) for current support.
2. **Factory contract**: GeckoTerminal indexes pool creation events from DEX factory contracts. The factory must emit standard events when pairs are created.
3. **Swap events**: Swap transactions must emit parseable events with amounts and assets.

### If Terra Classic Is Not Supported

If GeckoTerminal does not support Terra Classic natively, the `/cg/` endpoints above serve as a fallback for CoinGecko to list the DEX. Contact CoinGecko's listing team directly.

---

## AMM Orderbook Simulation

Since constant-product AMMs (x * y = k) don't have a traditional order book, we simulate one by walking the bonding curve at discrete quantity steps.

### Method

Given pool reserves `(R0, R1)` and constant product `k = R0 * R1`:

**Bids** (selling base for quote — price decreasing):
For each step `i` from 1 to `depth`:
1. Calculate `sell_amount = R0 * (i / depth) * 0.10` (up to 10% of reserves)
2. New `R0' = R0 + sell_amount`
3. New `R1' = k / R0'`
4. `received = R1 - R1'`
5. `effective_price = received / sell_amount`
6. Level: `[effective_price, sell_amount]`

**Asks** (buying base with quote — price increasing):
For each step `i` from 1 to `depth`:
1. Calculate `buy_amount = R0 * (i / depth) * 0.10`
2. New `R0' = R0 - buy_amount`
3. New `R1' = k / R0'`
4. `cost = R1' - R1`
5. `effective_price = cost / buy_amount`
6. Level: `[effective_price, buy_amount]`

This approach is standard across AMM DEXes listed on CoinGecko and CoinMarketCap.

---

## Listing Submission Guide

### CoinGecko

1. Ensure all `/cg/` endpoints are live and returning valid data
2. Go to [CoinGecko Request Form](https://www.coingecko.com/en/request) or contact listings@coingecko.com
3. Provide:
   - Exchange name: CL8Y DEX
   - Exchange type: Decentralized (AMM)
   - API base URL: `https://<your-api-domain>/cg/`
   - Network: Terra Classic (columbus-5)
   - Supported endpoints: `/pairs`, `/tickers`, `/orderbook`, `/historical_trades`
4. CoinGecko will crawl the endpoints and verify data quality

### CoinMarketCap

1. Ensure all `/cmc/` endpoints are live and returning valid data
2. Apply at [CoinMarketCap Exchange Listing](https://support.coinmarketcap.com/hc/en-us/articles/360043659351-Listings-Criteria)
3. Provide:
   - Exchange name and website
   - API documentation URL (link to this document)
   - API base URL: `https://<your-api-domain>/cmc/`
   - For each token, the `unified_cryptoasset_id` should be set in the assets table
4. CMC will verify endpoint compliance and data freshness

### Token-Level Listings

Individual tokens traded on the DEX can also be listed on CG/CMC:
- Use the existing `listing-api` repo for CG/CMC supply endpoints (`/api/v3/supply/:symbol`, `/cmc/circulating/:symbol`)
- Tokens need sufficient trading volume and liquidity to be listed

---

## Related References

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
