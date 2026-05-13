/**
 * User-facing copy when the Trade route detects indexer unavailability
 * (`isIndexerUnavailableError` on the pair metadata query).
 *
 * **Product invariant (GitLab #164):** Depth, tape, candles, hybrid routing, and
 * several ticket guards go through indexer HTTP APIs (book pages proxy LCD via the
 * indexer). The banner must not imply the order book or swap/limit ticket remain
 * fully usable “on chain” while those panels show indexer/LCD errors.
 */
export const TRADE_INDEXER_OUTAGE_BANNER_LEAD =
  'Chart, tape, order book depth, and most swap/limit ticket features need a reachable indexer'

export const TRADE_INDEXER_OUTAGE_BANNER_TAIL =
  'Expect those surfaces to be unavailable or degraded until the indexer is back or VITE_INDEXER_URL is corrected.'
