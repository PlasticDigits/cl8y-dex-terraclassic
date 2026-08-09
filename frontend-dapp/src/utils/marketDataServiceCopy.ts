/**
 * Retail copy for market-data service outage banners (GitLab #174, #215).
 * Trade-specific panel strings and limit-reference tail live in `indexerTradeOutageCopy.ts`.
 */

export const MARKET_DATA_SERVICE_OUTAGE_TITLE = 'Market data service unavailable.'

export const CHARTS_MARKET_DATA_OUTAGE_LEAD = 'Charts and analytics may be limited.'

export const TRADER_MARKET_DATA_OUTAGE_LEAD = 'Trader profiles and history may be limited.'

export const PORTFOLIO_MARKET_DATA_OUTAGE_LEAD = 'Portfolio data may be limited.'

export const POOL_MARKET_DATA_OUTAGE_LEAD = 'Pool listings may be limited.'

export const PROTOCOL_MARKET_DATA_OUTAGE_LEAD = 'Oracle pricing may be limited.'

/** Standalone `/limits` route — book, tape, placements ([GitLab #218](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/218)). */
export const LIMITS_MARKET_DATA_OUTAGE_LEAD = 'Book, tape, and limits may be limited.'

/** Swap route (`/` and `/swap`) — hybrid routing and indexer-backed quotes ([GitLab #241](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/241)). */
export const SWAP_MARKET_DATA_OUTAGE_LEAD = 'Swap quotes may be limited.'

/** Calm retry guidance when indexer returns HTTP 429 (SEC-E04 / GitLab #426). */
export const INDEXER_RATE_LIMIT_RETRY_MESSAGE =
  'Too many requests were sent in a short time. Wait a moment and try again.'

/** Inline alert when the wrap-mapper daily rate limit blocks a wrap/unwrap (SEC-I05 / GitLab #463). */
export const WRAP_RATE_LIMIT_EXCEEDED_MESSAGE =
  'Daily wrap limit reached for this amount. Wait and try again later, or reduce the amount.'

/** Submit CTA when wrap-mapper config / pause / rate-limit LCD queries fail closed (#507). */
export const WRAP_CONFIG_UNAVAILABLE_CTA = 'Wrap config unavailable'

/** Submit CTA when `VITE_TREASURY_ADDRESS` ≠ on-chain wrap-mapper `config.treasury` (#507 / W2). */
export const WRAP_TREASURY_MISCONFIGURED_CTA = 'Wrap treasury misconfigured'
