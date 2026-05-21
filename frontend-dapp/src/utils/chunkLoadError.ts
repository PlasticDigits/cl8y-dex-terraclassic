/**
 * Lazy route chunk failures (offline navigation, stale deploy) — GitLab #172.
 */

const CHUNK_LOAD_PATTERNS =
  /dynamically imported module|importing a module script failed|chunkloaderror|loading chunk \d+ failed/i

export const CHUNK_LOAD_ROUTE_HEADLINE = 'Page unavailable'

/** Retail copy when a lazy-loaded route chunk cannot be fetched. */
export const CHUNK_LOAD_ROUTE_MESSAGE =
  'This page could not load. You may be offline or the app was updated — check your connection and try again.'

export function isChunkLoadErrorMessage(message: string): boolean {
  return CHUNK_LOAD_PATTERNS.test(message.trim())
}

export function isChunkLoadError(err: unknown): boolean {
  if (err instanceof Error) return isChunkLoadErrorMessage(err.message)
  if (typeof err === 'string') return isChunkLoadErrorMessage(err)
  return false
}

/** Scrub dev-server URLs from technical-details blocks. */
export function sanitizeChunkLoadTechnicalDetail(message: string): string {
  const trimmed = message.trim()
  if (!isChunkLoadErrorMessage(trimmed)) return trimmed
  const withoutUrls = trimmed.replace(/https?:\/\/\S+/g, '[module]')
  if (/dynamically imported module/i.test(withoutUrls)) {
    return 'Failed to load page module (network unavailable or stale cache).'
  }
  return withoutUrls
}
