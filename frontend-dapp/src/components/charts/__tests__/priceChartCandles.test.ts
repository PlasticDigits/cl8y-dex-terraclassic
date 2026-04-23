import { describe, it, expect } from 'vitest'
import { indexerCandlesToChartPoints } from '../priceChartCandles'
import type { IndexerCandle } from '@/types'

function row(overrides: Partial<IndexerCandle> = {}): IndexerCandle {
  return {
    open_time: '2024-01-01T12:00:00.000Z',
    open: '1',
    high: '1.1',
    low: '0.9',
    close: '1.05',
    volume_base: '100',
    volume_quote: '105',
    trade_count: 3,
    ...overrides,
  }
}

describe('indexerCandlesToChartPoints', () => {
  it('returns empty array for undefined or empty input', () => {
    expect(indexerCandlesToChartPoints(undefined)).toEqual([])
    expect(indexerCandlesToChartPoints([])).toEqual([])
  })

  it('drops rows without open and close', () => {
    expect(indexerCandlesToChartPoints([row({ open: '', close: '' })])).toEqual([])
  })

  it('sorts by time ascending', () => {
    const a = row({ open_time: '2024-01-02T12:00:00.000Z', open: '2', close: '2' })
    const b = row({ open_time: '2024-01-01T12:00:00.000Z', open: '1', close: '1' })
    const pts = indexerCandlesToChartPoints([a, b])
    expect(pts).toHaveLength(2)
    expect(pts[0].open).toBe(1)
    expect(pts[1].open).toBe(2)
  })
})
