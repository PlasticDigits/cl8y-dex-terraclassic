/** Indexer responded 404 — pair/route is not in the indexer catalog (distinct from outage). */
export function isIndexerPairNotFoundError(err: unknown): boolean {
  return err instanceof Error && /Indexer API error:\s*404\b/.test(err.message)
}

/** True when the indexer HTTP API is unreachable or returns a non-OK status (not a logical 404). */
export function isIndexerUnavailableError(err: unknown): boolean {
  if (!err || !(err instanceof Error)) return false
  if (isIndexerPairNotFoundError(err)) return false
  const m = err.message
  if (m.includes('Failed to fetch') || m.includes('NetworkError') || m.includes('AbortError')) return true
  if (m.includes('Indexer API error:')) return true
  return false
}
