import { describe, it, expect } from 'vitest'
import { detectSwapIndexerOutage } from './swapIndexerOutage'

describe('detectSwapIndexerOutage', () => {
  it('returns true when sim query fails with indexer transport error', () => {
    expect(
      detectSwapIndexerOutage({
        isError: true,
        error: new Error('Indexer API error: 502 Bad Gateway'),
      })
    ).toBe(true)
  })

  it('returns true when sim succeeded but indexer transport failed during quote', () => {
    expect(detectSwapIndexerOutage({ isError: false, error: null }, { indexerTransportFailed: true })).toBe(true)
  })

  it('returns false for logical sim errors unrelated to indexer transport', () => {
    expect(
      detectSwapIndexerOutage({
        isError: true,
        error: new Error('No route found'),
      })
    ).toBe(false)
  })

  it('returns false for route/solve 400 (healthy indexer, simulation failed) (GitLab #326)', () => {
    expect(
      detectSwapIndexerOutage({
        isError: true,
        error: new Error('Indexer API error: 400 Bad Request'),
      })
    ).toBe(false)
  })

  it('returns false for indexer 429 rate limit (SEC-E04 / GitLab #426)', () => {
    expect(
      detectSwapIndexerOutage({
        isError: true,
        error: new Error('Indexer API error: 429 Too Many Requests'),
      })
    ).toBe(false)
  })
})
