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
 */
export const TRADE_INDEXER_OUTAGE_BANNER_TITLE = 'Market data service unavailable.'

export const TRADE_INDEXER_OUTAGE_BANNER_LEAD =
  'Chart, tape, order book depth, and most swap/limit ticket features may be limited until the service recovers.'

export const TRADE_INDEXER_OUTAGE_BANNER_TAIL =
  'Limit price reference (buy-below / sell-above) can still use on-chain pool reserves via your wallet when recent trade history is missing.'
