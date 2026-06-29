import { MARKET_DATA_SERVICE_OUTAGE_TITLE } from '@/utils/marketDataServiceCopy'

/**
 * User-facing copy when the Trade route detects indexer unavailability
 * (`isIndexerUnavailableError` on the pair metadata query).
 *
 * **Product invariant (GitLab #164):** Depth, tape, candles, hybrid routing, and
 * several ticket guards go through indexer HTTP APIs (book pages proxy LCD via the
 * indexer). The banner must not imply the order book or swap/limit ticket remain
 * fully usable “on chain” while those panels show indexer/LCD errors.
 *
 * **GitLab #166:** limit *reference price* for the place gate may use on-chain pool
 * reserves (LCD) when tape is missing; the banner tail states this explicitly.
 *
 * **GitLab #174:** retail copy must not expose `VITE_INDEXER_URL`, hostnames, or
 * “indexer” jargon — use “market data service” and plain-language recovery hints.
 *
 * **GitLab #165:** each trade workspace panel (book, tape, chart) must explain what
 * is missing when the market data service is down — never leave sections blank.
 *
 * **GitLab #427 (SEC-E05):** outage copy must distinguish **data/display unavailable**
 * from **funds at risk** — on-chain balances, LP shares, and limit escrows stay safe.
 */
export const TRADE_INDEXER_OUTAGE_BANNER_TITLE = MARKET_DATA_SERVICE_OUTAGE_TITLE

export const TRADE_INDEXER_OUTAGE_BANNER_LEAD =
  'Chart, tape, order book depth, and most swap/limit ticket features may be limited until the service recovers. Your on-chain wallet balances, LP shares, and limit order escrows are unaffected — only what you see here may be limited.'

export const TRADE_INDEXER_OUTAGE_BANNER_TAIL =
  'Limit price reference (buy-below / sell-above) can still use on-chain pool reserves via your wallet when recent trade history is missing.'

export const TRADE_PANEL_BOOK_UNAVAILABLE = 'Order book depth is unavailable while the market data service is down.'

export const TRADE_PANEL_TAPE_UNAVAILABLE = 'Recent trades are unavailable while the market data service is down.'

export const TRADE_PANEL_CHART_UNAVAILABLE = 'Price chart is unavailable while the market data service is down.'
