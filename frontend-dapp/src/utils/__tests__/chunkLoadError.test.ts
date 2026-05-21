import { describe, expect, it } from 'vitest'
import { isChunkLoadError, isChunkLoadErrorMessage, sanitizeChunkLoadTechnicalDetail } from '../chunkLoadError'

describe('chunkLoadError', () => {
  it('detects Vite dynamic import failures', () => {
    const msg = 'TypeError: Failed to fetch dynamically imported module: http://localhost:3000/src/pages/SwapPage.tsx'
    expect(isChunkLoadErrorMessage(msg)).toBe(true)
    expect(isChunkLoadError(new Error(msg))).toBe(true)
  })

  it('detects production chunk load strings', () => {
    expect(isChunkLoadErrorMessage('Loading chunk 42 failed.')).toBe(true)
    expect(isChunkLoadErrorMessage('ChunkLoadError: Loading chunk 3 failed.')).toBe(true)
  })

  it('sanitizes dev URLs in technical details', () => {
    const raw = 'Failed to fetch dynamically imported module: http://localhost:3000/src/pages/ChartsPage.tsx'
    expect(sanitizeChunkLoadTechnicalDetail(raw)).not.toContain('localhost:3000')
    expect(sanitizeChunkLoadTechnicalDetail(raw)).toMatch(/page module/i)
  })

  it('leaves unrelated messages unchanged', () => {
    expect(sanitizeChunkLoadTechnicalDetail('Max spread assertion')).toBe('Max spread assertion')
  })
})
