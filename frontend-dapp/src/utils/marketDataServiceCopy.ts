/**
 * Retail copy for market-data service outage banners (GitLab #174, #215).
 * Trade-specific panel strings and limit-reference tail live in `indexerTradeOutageCopy.ts`.
 */

export const MARKET_DATA_SERVICE_OUTAGE_TITLE = 'Market data service unavailable.'

export const CHARTS_MARKET_DATA_OUTAGE_LEAD =
  'Charts, pair analytics, and leaderboards may be limited until the service recovers.'

export const TRADER_MARKET_DATA_OUTAGE_LEAD =
  'Trader profiles, positions, and trade history may be limited until the service recovers.'

export const PORTFOLIO_MARKET_DATA_OUTAGE_LEAD =
  'Portfolio summary, open positions, and recent swaps may be limited until the service recovers.'

export const POOL_MARKET_DATA_OUTAGE_LEAD =
  'Pool listings and indexer-backed pair metadata may be limited until the service recovers.'

export const PROTOCOL_MARKET_DATA_OUTAGE_LEAD =
  'Oracle reference pricing and hook activity may be limited until the service recovers.'

/** Standalone `/limits` route — book, tape, placements ([GitLab #218](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/218)). */
export const LIMITS_MARKET_DATA_OUTAGE_LEAD =
  'Order book depth, recent trades, and indexed placements may be limited until the service recovers.'

/** Swap route (`/` and `/swap`) — hybrid routing and indexer-backed quotes ([GitLab #241](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/241)). */
export const SWAP_MARKET_DATA_OUTAGE_LEAD =
  'Swap routing quotes and hybrid path optimization may be limited until the service recovers.'

/** Calm retry guidance when indexer returns HTTP 429 (SEC-E04 / GitLab #426). */
export const INDEXER_RATE_LIMIT_RETRY_MESSAGE =
  'Too many requests were sent in a short time. Wait a moment and try again.'
