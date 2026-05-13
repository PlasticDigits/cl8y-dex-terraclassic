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
 */
export const TRADE_INDEXER_OUTAGE_BANNER_LEAD =
  'Chart, tape, order book depth, and most swap/limit ticket features need a reachable indexer'

export const TRADE_INDEXER_OUTAGE_BANNER_TAIL =
  'Expect those surfaces to be unavailable or degraded until the indexer is back or VITE_INDEXER_URL is corrected. Limit price reference (buy-below / sell-above) can fall back to AMM pool reserves via your wallet LCD when indexed tape is missing ([GitLab #166](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/166)).'
