import {
  clearStaleChunkReloadGuard,
  isChunkLoadError,
  isChunkLoadErrorMessage,
  reloadOnceOnStaleChunk,
  reloadSameOriginDocument,
  sanitizeChunkLoadTechnicalDetail,
  STALE_CHUNK_RELOAD_STORAGE_KEY,
  staleChunkReloadIo,
  wouldAutoReloadOnStaleChunk,
} from '../chunkLoadError'

const CHROME_PROD = 'Failed to fetch dynamically imported module: https://dex.cl8y.com/assets/PoolPage-BrgV5Tp1.js'
const FIREFOX_PROD = 'error loading dynamically imported module: https://dex.cl8y.com/assets/PoolPage-BrgV5Tp1.js'
const SAFARI = 'Importing a module script failed.'
const WEBPACK = 'ChunkLoadError: Loading chunk 3 failed.'

describe('chunkLoadError', () => {
  beforeEach(() => {
    sessionStorage.clear()
    vi.stubGlobal('navigator', { ...navigator, onLine: true })
    vi.spyOn(staleChunkReloadIo, 'reloadDocument').mockImplementation(() => {})
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
    sessionStorage.clear()
  })

  it('detects Vite dynamic import failures', () => {
    const msg = 'TypeError: Failed to fetch dynamically imported module: http://localhost:3000/src/pages/SwapPage.tsx'
    expect(isChunkLoadErrorMessage(msg)).toBe(true)
    expect(isChunkLoadError(new Error(msg))).toBe(true)
  })

  it('detects Chrome, Firefox, Safari, and webpack production chunk strings (GitLab #706)', () => {
    expect(isChunkLoadErrorMessage(CHROME_PROD)).toBe(true)
    expect(isChunkLoadError(new TypeError(CHROME_PROD))).toBe(true)
    expect(isChunkLoadErrorMessage(FIREFOX_PROD)).toBe(true)
    expect(isChunkLoadErrorMessage(SAFARI)).toBe(true)
    expect(isChunkLoadErrorMessage(WEBPACK)).toBe(true)
    expect(isChunkLoadErrorMessage('Loading chunk 42 failed.')).toBe(true)
  })

  it('does not classify indexer Failed to fetch, LCD timeouts, Max spread, or user reject', () => {
    expect(isChunkLoadErrorMessage('TypeError: Failed to fetch')).toBe(false)
    expect(isChunkLoadError(new TypeError('Failed to fetch'))).toBe(false)
    expect(isChunkLoadErrorMessage('LCD request timed out')).toBe(false)
    expect(isChunkLoadErrorMessage('Max spread assertion')).toBe(false)
    expect(isChunkLoadErrorMessage('User rejected the request')).toBe(false)
  })

  it('sanitizes production hashed asset URLs in technical details', () => {
    expect(sanitizeChunkLoadTechnicalDetail(CHROME_PROD)).not.toContain('dex.cl8y.com')
    expect(sanitizeChunkLoadTechnicalDetail(CHROME_PROD)).not.toContain('PoolPage-BrgV5Tp1')
    expect(sanitizeChunkLoadTechnicalDetail(CHROME_PROD)).toMatch(/page module/i)
    expect(sanitizeChunkLoadTechnicalDetail(FIREFOX_PROD)).not.toContain('https://')
  })

  it('sanitizes dev URLs in technical details', () => {
    const raw = 'Failed to fetch dynamically imported module: http://localhost:3000/src/pages/ChartsPage.tsx'
    expect(sanitizeChunkLoadTechnicalDetail(raw)).not.toContain('localhost:3000')
    expect(sanitizeChunkLoadTechnicalDetail(raw)).toMatch(/page module/i)
  })

  it('leaves unrelated messages unchanged', () => {
    expect(sanitizeChunkLoadTechnicalDetail('Max spread assertion')).toBe('Max spread assertion')
  })

  it('reloadOnceOnStaleChunk: online first chunk error sets storage and reloads', () => {
    const err = new TypeError(CHROME_PROD)
    expect(reloadOnceOnStaleChunk(err)).toBe(true)
    expect(sessionStorage.getItem(STALE_CHUNK_RELOAD_STORAGE_KEY)).toBe('1')
    expect(staleChunkReloadIo.reloadDocument).toHaveBeenCalledTimes(1)
  })

  it('reloadOnceOnStaleChunk: storage already set does not reload', () => {
    sessionStorage.setItem(STALE_CHUNK_RELOAD_STORAGE_KEY, '1')
    expect(reloadOnceOnStaleChunk(new TypeError(CHROME_PROD))).toBe(false)
    expect(staleChunkReloadIo.reloadDocument).not.toHaveBeenCalled()
  })

  it('reloadOnceOnStaleChunk: offline does not reload or write storage', () => {
    vi.stubGlobal('navigator', { ...navigator, onLine: false })
    expect(wouldAutoReloadOnStaleChunk(new TypeError(CHROME_PROD))).toBe(false)
    expect(reloadOnceOnStaleChunk(new TypeError(CHROME_PROD))).toBe(false)
    expect(sessionStorage.getItem(STALE_CHUNK_RELOAD_STORAGE_KEY)).toBeNull()
    expect(staleChunkReloadIo.reloadDocument).not.toHaveBeenCalled()
  })

  it('reloadOnceOnStaleChunk: non-chunk Error does not reload', () => {
    expect(reloadOnceOnStaleChunk(new Error('Max spread assertion'))).toBe(false)
    expect(staleChunkReloadIo.reloadDocument).not.toHaveBeenCalled()
    expect(sessionStorage.getItem(STALE_CHUNK_RELOAD_STORAGE_KEY)).toBeNull()
  })

  it('reloadOnceOnStaleChunk: evil module URL never becomes location.href', () => {
    const hrefBefore = window.location.href
    const err = new TypeError('Failed to fetch dynamically imported module: https://evil.example/assets/x.js')
    expect(reloadOnceOnStaleChunk(err)).toBe(true)
    expect(staleChunkReloadIo.reloadDocument).toHaveBeenCalledTimes(1)
    expect(window.location.href).toBe(hrefBefore)
    expect(window.location.href).not.toContain('evil.example')
  })

  it('reloadOnceOnStaleChunk: sessionStorage throw skips reload (fail-safe)', () => {
    const setItem = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('quota')
    })
    expect(reloadOnceOnStaleChunk(new TypeError(CHROME_PROD))).toBe(false)
    expect(staleChunkReloadIo.reloadDocument).not.toHaveBeenCalled()
    setItem.mockRestore()
  })

  it('clearStaleChunkReloadGuard removes the key after a successful mount', () => {
    sessionStorage.setItem(STALE_CHUNK_RELOAD_STORAGE_KEY, '1')
    clearStaleChunkReloadGuard()
    expect(sessionStorage.getItem(STALE_CHUNK_RELOAD_STORAGE_KEY)).toBeNull()
  })

  it('reloadSameOriginDocument calls location.reload only', () => {
    reloadSameOriginDocument()
    expect(staleChunkReloadIo.reloadDocument).toHaveBeenCalledTimes(1)
  })
})
