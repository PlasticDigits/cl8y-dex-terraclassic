import { describe, it, expect } from 'vitest'
import type { Time } from 'lightweight-charts'
import { formatNum } from '@/utils/formatAmount'
import { resolveTradeChartHeadlineUsd } from '../chartHeadlinePrice'
import type { ChartCandlePoint } from '../priceChartCandles'

describe('resolveTradeChartHeadlineUsd', () => {
  const points: ChartCandlePoint[] = [
    { time: 1 as Time, open: 1, high: 2, low: 0.5, close: 1.5 },
    { time: 2 as Time, open: 1.5, high: 2, low: 1, close: 2 },
  ]

  it('prefers tape price over last candle close', () => {
    expect(resolveTradeChartHeadlineUsd('12.345678', points)).toBe(formatNum('12.345678', 6))
  })

  it('falls back to last candle close when tape is absent', () => {
    expect(resolveTradeChartHeadlineUsd(undefined, points)).toBe(formatNum(2, 6))
  })

  it('returns null when there is no tape and no candles', () => {
    expect(resolveTradeChartHeadlineUsd(undefined, [])).toBe(null)
    expect(resolveTradeChartHeadlineUsd('', [])).toBe(null)
  })
})
