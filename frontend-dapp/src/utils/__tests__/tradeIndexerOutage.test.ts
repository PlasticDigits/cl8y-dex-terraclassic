import { describe, it, expect } from 'vitest'
import { detectTradeIndexerOutage } from '../tradeIndexerOutage'

describe('detectTradeIndexerOutage', () => {
  it('returns true when any workspace query has an indexer transport error (GitLab #165)', () => {
    const ok = { isError: false, error: null }
    const pairDown = { isError: true, error: new Error('Indexer API error: 502 Bad Gateway') }
    const tapeDown = { isError: true, error: new Error('Failed to fetch') }

    expect(detectTradeIndexerOutage(ok, ok)).toBe(false)
    expect(detectTradeIndexerOutage(ok, tapeDown)).toBe(true)
    expect(detectTradeIndexerOutage(pairDown, ok)).toBe(true)
  })

  it('ignores indexer 404 pair misses', () => {
    const notFound = { isError: true, error: new Error('Indexer API error: 404 Not Found') }
    expect(detectTradeIndexerOutage(notFound)).toBe(false)
  })
})
