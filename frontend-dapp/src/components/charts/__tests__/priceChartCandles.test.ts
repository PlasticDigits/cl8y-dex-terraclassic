import { describe, it, expect } from 'vitest'
import {
  applyChartDisplayInvert,
  candleOpenTimeSeconds,
  indexerCandlesToChartPoints,
  indexerCandlesToVolumeHistogramPoints,
  parseChartFiniteNumber,
} from '../priceChartCandles'
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

function assertAllFiniteOHLC(pts: ReturnType<typeof indexerCandlesToChartPoints>) {
  for (const p of pts) {
    expect(Number.isFinite(p.open)).toBe(true)
    expect(Number.isFinite(p.high)).toBe(true)
    expect(Number.isFinite(p.low)).toBe(true)
    expect(Number.isFinite(p.close)).toBe(true)
    expect(Number.isFinite(p.time as number)).toBe(true)
  }
}

describe('parseChartFiniteNumber', () => {
  it('returns finite parses and null for non-finite', () => {
    expect(parseChartFiniteNumber('1.5')).toBe(1.5)
    expect(parseChartFiniteNumber('abc')).toBeNull()
    expect(parseChartFiniteNumber('NaN')).toBeNull()
    expect(parseChartFiniteNumber('Infinity')).toBeNull()
    expect(parseChartFiniteNumber('1e309')).toBeNull()
    expect(parseChartFiniteNumber('')).toBeNull()
  })
})

describe('candleOpenTimeSeconds', () => {
  it('returns unix seconds for valid ISO times and null for garbage', () => {
    expect(candleOpenTimeSeconds('2024-01-01T12:00:00.000Z')).toBe(1704110400)
    expect(candleOpenTimeSeconds('not-a-date')).toBeNull()
    expect(candleOpenTimeSeconds('<script>alert(1)</script>')).toBeNull()
  })
})

describe('indexerCandlesToChartPoints', () => {
  it('returns empty array for undefined or empty input', () => {
    expect(indexerCandlesToChartPoints(undefined)).toEqual([])
    expect(indexerCandlesToChartPoints([])).toEqual([])
  })

  it('drops rows without open and close', () => {
    expect(indexerCandlesToChartPoints([row({ open: '', close: '' })])).toEqual([])
  })

  it('sorts by time ascending', () => {
    const a = row({ open_time: '2024-01-02T12:00:00.000Z', open: '2', close: '2', high: '2', low: '2' })
    const b = row({ open_time: '2024-01-01T12:00:00.000Z', open: '1', close: '1', high: '1', low: '1' })
    const pts = indexerCandlesToChartPoints([a, b])
    expect(pts).toHaveLength(2)
    expect(pts[0].open).toBe(1)
    expect(pts[1].open).toBe(2)
    assertAllFiniteOHLC(pts)
  })

  it('drops non-numeric open or close', () => {
    expect(indexerCandlesToChartPoints([row({ open: 'abc', close: '1' })])).toEqual([])
    expect(indexerCandlesToChartPoints([row({ open: '1', close: 'xyz' })])).toEqual([])
  })

  it('drops NaN and Infinity string OHLC', () => {
    expect(indexerCandlesToChartPoints([row({ close: 'NaN' })])).toEqual([])
    expect(indexerCandlesToChartPoints([row({ high: 'Infinity' })])).toEqual([])
  })

  it('drops extreme float strings', () => {
    expect(indexerCandlesToChartPoints([row({ open: '1e309', close: '1', high: '1', low: '1' })])).toEqual([])
  })

  it('drops rows with missing or invalid high/low', () => {
    expect(indexerCandlesToChartPoints([row({ high: '', low: '0.9' })])).toEqual([])
    expect(indexerCandlesToChartPoints([row({ high: 'bad', low: '0.9' })])).toEqual([])
  })

  it('drops invalid open_time', () => {
    expect(indexerCandlesToChartPoints([row({ open_time: 'invalid' })])).toEqual([])
  })

  it('keeps only valid rows in a mixed batch', () => {
    const pts = indexerCandlesToChartPoints([
      row({ open: '1', close: '1', high: '1', low: '1' }),
      row({ open: 'bad', close: '1' }),
      row({ open: '3', close: '3', high: '3', low: '3', open_time: '2024-01-02T12:00:00.000Z' }),
    ])
    expect(pts).toHaveLength(2)
    assertAllFiniteOHLC(pts)
  })

  it('stable-sorts duplicate open_time without throwing', () => {
    const t = '2024-01-01T12:00:00.000Z'
    const pts = indexerCandlesToChartPoints([
      row({ open: '2', close: '2', high: '2', low: '2', open_time: t }),
      row({ open: '1', close: '1', high: '1', low: '1', open_time: t }),
    ])
    expect(pts).toHaveLength(2)
    expect(pts[0].time).toBe(pts[1].time)
    assertAllFiniteOHLC(pts)
  })

  it('drops runtime-weird open values without throwing', () => {
    const weird = row({ open: '1', close: '1' })
    ;(weird as { open: unknown }).open = { toString: () => '1' } as unknown as string
    expect(() => indexerCandlesToChartPoints([weird])).not.toThrow()
    assertAllFiniteOHLC(indexerCandlesToChartPoints([weird]))
  })
})

describe('indexerCandlesToVolumeHistogramPoints', () => {
  it('maps quote volume and colors from open vs close', () => {
    const pts = indexerCandlesToVolumeHistogramPoints(
      [
        row({ open: '1', close: '1.1', volume_quote: '50' }),
        row({
          open_time: '2024-01-02T12:00:00.000Z',
          open: '2',
          close: '1.5',
          high: '2',
          low: '1',
          volume_quote: '12',
        }),
      ],
      '#00ff00',
      '#ff0000'
    )
    expect(pts).toHaveLength(2)
    expect(pts[0].value).toBe(50)
    expect(pts[0].color).toBe('#00ff00')
    expect(pts[1].value).toBe(12)
    expect(pts[1].color).toBe('#ff0000')
  })

  it('falls back to base volume when quote is zero', () => {
    const pts = indexerCandlesToVolumeHistogramPoints(
      [row({ open: '1', close: '1.1', volume_quote: '0', volume_base: '42' })],
      '#00ff00',
      '#ff0000'
    )
    expect(pts[0]?.value).toBe(42)
  })

  it('applyChartDisplayInvert reciprocates and drops dust/zero (GitLab #524 / H5)', () => {
    const pts = indexerCandlesToChartPoints([row({ open: '2', high: '4', low: '1', close: '3' })])
    const inv = applyChartDisplayInvert(pts, true)
    expect(inv).toHaveLength(1)
    expect(inv[0].open).toBeCloseTo(0.5)
    expect(inv[0].high).toBeCloseTo(1)
    expect(inv[0].low).toBeCloseTo(0.25)
    expect(inv[0].close).toBeCloseTo(1 / 3)
    expect(applyChartDisplayInvert(pts, false)).toEqual(pts)
    expect(applyChartDisplayInvert([{ time: 1 as never, open: 0, high: 1, low: 1, close: 1 }], true)).toEqual([])
  })

  it('omits volume for rows that fail OHLC validation', () => {
    const pts = indexerCandlesToVolumeHistogramPoints(
      [row({ open: '', close: '' }), row({ open: '1', close: '1.1' })],
      '#00ff00',
      '#ff0000'
    )
    expect(pts).toHaveLength(1)
  })
})
