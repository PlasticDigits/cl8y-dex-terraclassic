import { describe, it, expect } from 'vitest'
import { detectMarketDataOutage } from '../marketDataOutage'

describe('detectMarketDataOutage', () => {
  const ok = { isError: false, error: null }
  const down = { isError: true, error: new Error('Indexer API error: 502') }
  const notFound = { isError: true, error: new Error('Indexer API error: 404 Not Found') }
  const routeSimFailed = { isError: true, error: new Error('Indexer API error: 400 Bad Request') }

  it('ORs transport errors and ignores 404', () => {
    expect(detectMarketDataOutage(ok, ok)).toBe(false)
    expect(detectMarketDataOutage(ok, down)).toBe(true)
    expect(detectMarketDataOutage(down, ok)).toBe(true)
    expect(detectMarketDataOutage(notFound)).toBe(false)
  })

  it('ignores 400 route-simulation failures (GitLab #326)', () => {
    expect(detectMarketDataOutage(routeSimFailed)).toBe(false)
    expect(detectMarketDataOutage(ok, routeSimFailed)).toBe(false)
  })
})
