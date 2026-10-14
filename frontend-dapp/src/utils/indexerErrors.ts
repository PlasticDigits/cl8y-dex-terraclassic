import { IndexerApiError } from '@/utils/routeQuoteErrors'

/** Cached price publication is pending; this is neither a transport outage nor a trade blocker. */
export function isIndexerPricingDataPending(err: unknown): boolean {
  return (
    err instanceof IndexerApiError &&
    err.status === 503 &&
    (err.quoteReason === 'missing_observations' || err.quoteReason === 'stale_observations')
  )
}

/** Indexer responded 404 — pair/route is not in the indexer catalog (distinct from outage). */
export function isIndexerPairNotFoundError(err: unknown): boolean {
  return err instanceof Error && /Indexer API error:\s*404\b/.test(err.message)
}

/** Indexer responded 429 — rate limited (client error, not transport outage). */
export function isIndexerRateLimitError(err: unknown): boolean {
  return err instanceof Error && /Indexer API error:\s*429\b/.test(err.message)
}

/** Indexer responded 4xx — semantic/client error from a reachable service (not transport outage). */
export function isIndexerClientError(err: unknown): boolean {
  return err instanceof Error && /Indexer API error:\s*4\d{2}\b/.test(err.message)
}

/** Authoritative route/guard rejection (#1390); never downgrade to a client fallback quote. */
export function isIndexerRouteValidationError(err: unknown): boolean {
  return err instanceof Error && /Indexer API error:\s*422\b/.test(err.message)
}

/** Known local pricing failures recover only after intent or observation generation changes. */
export function isIndexerLocalQuoteRejection(err: unknown): boolean {
  return (
    isIndexerRouteValidationError(err) ||
    (err instanceof IndexerApiError && err.status === 503 && err.quoteReason != null)
  )
}

/** A missing/stale publication cannot revoke an otherwise unexpired exact-intent
 * display. It still blocks signing and interval retries through the query error.
 * Every other typed rejection remains a visible, immediate revocation. */
export function isIndexerQuoteDisplayRevoked(err: unknown): boolean {
  return isIndexerLocalQuoteRejection(err) && !isIndexerPricingDataPending(err)
}

/** Transport outage, distinct from authoritative local observation/accounting outcomes (#1473). */
export function isIndexerUnavailableError(err: unknown): boolean {
  if (!err || !(err instanceof Error)) return false
  if (err instanceof IndexerApiError && err.quoteReason != null) return false
  if (isIndexerPairNotFoundError(err)) return false
  if (isIndexerClientError(err)) return false
  const m = err.message
  if (m.includes('Failed to fetch') || m.includes('NetworkError') || m.includes('AbortError')) return true
  if (m.includes('Indexer API error:')) return true
  return false
}
