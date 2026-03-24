/** True when the indexer HTTP API is unreachable or returns a non-OK status (not a logical 404). */
export function isIndexerUnavailableError(err: unknown): boolean {
  if (!err || !(err instanceof Error)) return false
  const m = err.message
  if (m.includes('Failed to fetch') || m.includes('NetworkError') || m.includes('AbortError')) return true
  if (/Indexer API error:\s*404/.test(m)) return false
  if (m.includes('Indexer API error:')) return true
  return false
}
