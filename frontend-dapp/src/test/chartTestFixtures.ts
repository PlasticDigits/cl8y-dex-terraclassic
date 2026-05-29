import type { Time } from 'lightweight-charts'
import type { ChartCandlePoint } from '@/components/charts/priceChartCandles'
import type { IndexerCandle } from '@/types'
import {
  indexerCandlesToChartPoints,
  indexerCandlesToVolumeHistogramPoints,
} from '@/components/charts/priceChartCandles'
import { chartPointsToRsiLine, chartPointsToSmaLine } from '@/components/charts/priceChartIndicators'

const UP = '#22c55e'
const DOWN = '#ef4444'

/** UTC second timestamps for deterministic chart fixtures. */
export function chartTimeUtc(year: number, month: number, day: number, hour = 0): Time {
  return Math.floor(Date.UTC(year, month - 1, day, hour, 0, 0) / 1000) as Time
}

export function makeChartCandlePoints(count: number, start: Time = chartTimeUtc(2024, 1, 1)): ChartCandlePoint[] {
  const startSec = Number(start)
  return Array.from({ length: count }, (_, i) => {
    const base = 1 + i * 0.01
    return {
      time: (startSec + i * 3600) as Time,
      open: base,
      high: base + 0.05,
      low: Math.max(0.01, base - 0.03),
      close: base + 0.02,
    }
  })
}

export function indexerCandleRow(overrides: Partial<IndexerCandle> = {}): IndexerCandle {
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

export function volumePointsFromIndexer(rows: IndexerCandle[]) {
  return indexerCandlesToVolumeHistogramPoints(rows, UP, DOWN)
}

export function chartBundleFromCandles(count: number) {
  const candlePoints = makeChartCandlePoints(count)
  const volumePoints = candlePoints.map((p, i) => ({
    time: p.time,
    value: 50 + i,
    color: UP,
  }))
  return {
    candlePoints,
    volumePoints,
    sma7Points: chartPointsToSmaLine(candlePoints, 7),
    sma25Points: chartPointsToSmaLine(candlePoints, 25),
    rsiPoints: chartPointsToRsiLine(candlePoints, 14),
  }
}

export function chartBundleFromIndexerRows(rows: IndexerCandle[]) {
  const candlePoints = indexerCandlesToChartPoints(rows)
  const volumePoints = indexerCandlesToVolumeHistogramPoints(rows, UP, DOWN)
  return {
    candlePoints,
    volumePoints,
    sma7Points: chartPointsToSmaLine(candlePoints, 7),
    sma25Points: chartPointsToSmaLine(candlePoints, 25),
    rsiPoints: chartPointsToRsiLine(candlePoints, 14),
  }
}
