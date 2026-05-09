import { describe, it, expect } from 'vitest'
import { chartPointsToRsiLine, chartPointsToSmaLine } from '../priceChartIndicators'
import type { ChartCandlePoint } from '../priceChartCandles'

function pt(t: number, close: number, open = close): ChartCandlePoint {
  return {
    time: t as ChartCandlePoint['time'],
    open,
    high: Math.max(open, close) + 0.01,
    low: Math.min(open, close) - 0.01,
    close,
  }
}

describe('chartPointsToSmaLine', () => {
  it('returns empty when not enough bars', () => {
    expect(chartPointsToSmaLine([pt(1, 10)], 7)).toEqual([])
  })

  it('computes SMA(3) on closes', () => {
    const pts = [pt(1, 10), pt(2, 12), pt(3, 14)]
    const sma = chartPointsToSmaLine(pts, 3)
    expect(sma).toHaveLength(1)
    expect(sma[0]).toEqual({ time: 3 as ChartCandlePoint['time'], value: 12 })
  })
})

describe('chartPointsToRsiLine', () => {
  it('returns empty when fewer than period + 1 closes', () => {
    const pts = Array.from({ length: 14 }, (_, i) => pt(i + 1, 100 + i))
    expect(chartPointsToRsiLine(pts, 14)).toEqual([])
  })

  it('produces bounded RSI values for a simple uptrend', () => {
    const pts = Array.from({ length: 30 }, (_, i) => pt(i + 1, 100 + i * 0.5))
    const rsi = chartPointsToRsiLine(pts, 14)
    expect(rsi.length).toBeGreaterThan(0)
    for (const p of rsi) {
      expect(p.value).toBeGreaterThanOrEqual(0)
      expect(p.value).toBeLessThanOrEqual(100)
    }
  })
})
