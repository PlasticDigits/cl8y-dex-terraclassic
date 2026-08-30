/**
 * Lazy route chunk failures — offline navigation (GitLab #172) and stale Coolify
 * hashed assets after a frontend roll (GitLab #706).
 *
 * Classifier matches **browser** dynamic-import / chunk-load strings only. Generic
 * `Failed to fetch` (indexer/LCD) must stay on the fetch branch in
 * `humanizeOffChainError.ts`. Residual: an untrusted `error.message` that copies a
 * browser chunk phrase could trigger a one-shot reload — same UI-trust caveat as #145.
 */

const CHUNK_LOAD_PATTERNS =
  /dynamically imported module|importing a module script failed|chunkloaderror|loading chunk \d+ failed/i

export const CHUNK_LOAD_ROUTE_HEADLINE = 'Page unavailable'

/** Retail copy when a lazy-loaded route chunk cannot be fetched (offline or stale deploy). */
export const CHUNK_LOAD_ROUTE_MESSAGE =
  'This page could not load. You may be offline or the app was updated — check your connection and try again.'

/** Brief fallback while a one-shot document reload picks up a new HTML shell (#706). */
export const CHUNK_LOAD_UPDATING_MESSAGE = 'Updating…'

/** sessionStorage guard so a broken live deploy cannot infinite-reload. */
export const STALE_CHUNK_RELOAD_STORAGE_KEY = 'cl8y-dex-stale-chunk-reload'

export function isChunkLoadErrorMessage(message: string): boolean {
  return CHUNK_LOAD_PATTERNS.test(message.trim())
}

export function isChunkLoadError(err: unknown): boolean {
  if (err instanceof Error) return isChunkLoadErrorMessage(err.message)
  if (typeof err === 'string') return isChunkLoadErrorMessage(err)
  return false
}

/** Scrub module URLs (dev-server and production hashed assets) from technical-details blocks. */
export function sanitizeChunkLoadTechnicalDetail(message: string): string {
  const trimmed = message.trim()
  if (!isChunkLoadErrorMessage(trimmed)) return trimmed
  const withoutUrls = trimmed.replace(/https?:\/\/\S+/g, '[module]')
  if (/dynamically imported module/i.test(withoutUrls)) {
    return 'Failed to load page module (network unavailable or stale cache).'
  }
  return withoutUrls
}

function sessionStorageGet(key: string): string | null {
  try {
    if (typeof sessionStorage === 'undefined') return null
    return sessionStorage.getItem(key)
  } catch {
    return null
  }
}

function sessionStorageSet(key: string, value: string): boolean {
  try {
    sessionStorage.setItem(key, value)
    return true
  } catch {
    return false
  }
}

function sessionStorageRemove(key: string): void {
  try {
    sessionStorage.removeItem(key)
  } catch {
    // fail-safe: blocked / quota — do not throw into the error UI
  }
}

/** `navigator.onLine === false` must not auto-reload (#172 offline Try Again). */
export function isBrowserOnline(): boolean {
  if (typeof navigator === 'undefined') return true
  return navigator.onLine !== false
}

/**
 * True when a chunk error should trigger a one-shot same-origin document reload.
 * Does not write storage or call `location.reload`.
 */
export function wouldAutoReloadOnStaleChunk(error: unknown): boolean {
  if (!isChunkLoadError(error)) return false
  if (!isBrowserOnline()) return false
  if (sessionStorageGet(STALE_CHUNK_RELOAD_STORAGE_KEY)) return false
  return true
}

/**
 * Same-origin document reload. Vitest spies `staleChunkReloadIo.reloadDocument`
 * so tests never assign `window.location` (open-redirect surface stays production-only).
 */
export const staleChunkReloadIo = {
  reloadDocument(): void {
    window.location.reload()
  },
}

/** Same-origin document reload only. Never assign a failed module URL (open redirect). */
export function reloadSameOriginDocument(): void {
  staleChunkReloadIo.reloadDocument()
}

/**
 * One-shot reload for a stale hashed chunk while the tab is online.
 * Returns true only when `reload` was invoked.
 */
export function reloadOnceOnStaleChunk(error: unknown): boolean {
  if (!wouldAutoReloadOnStaleChunk(error)) return false
  if (!sessionStorageSet(STALE_CHUNK_RELOAD_STORAGE_KEY, '1')) return false
  reloadSameOriginDocument()
  return true
}

/** Clear the guard after a lazy page successfully mounts so a later deploy can recover again. */
export function clearStaleChunkReloadGuard(): void {
  sessionStorageRemove(STALE_CHUNK_RELOAD_STORAGE_KEY)
}
