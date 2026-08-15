import { describe, it, expect, vi } from 'vitest'
import type { HistogramData, Time } from 'lightweight-charts'
import type { ChartCandlePoint } from '../priceChartCandles'
import type { IndicatorLinePoint } from '../priceChartIndicators'
import { syncCandleSeriesData, syncHistogramSeriesData, syncLineSeriesData } from '../priceChartLightweightSeriesSync'

function candle(time: number, close: number): ChartCandlePoint {
  return { time: time as Time, open: close, high: close + 1, low: close - 1, close }
}

function mockSeries() {
  return {
    setData: vi.fn(),
    update: vi.fn(),
  }
}

describe('priceChartLightweightSeriesSync (GitLab #336)', () => {
  it('setData on first load', () => {
    const series = mockSeries()
    const next = [candle(1, 10), candle(2, 11)]
    const result = syncCandleSeriesData(series, [], next)
    expect(series.setData).toHaveBeenCalledWith(next)
    expect(series.update).not.toHaveBeenCalled()
    expect(result).toBe(next)
  })

  it('update tail bar when only the last candle changes', () => {
    const series = mockSeries()
    const previous = [candle(1, 10), candle(2, 11)]
    const next = [candle(1, 10), candle(2, 12)]
    const result = syncCandleSeriesData(series, previous, next)
    expect(series.setData).not.toHaveBeenCalled()
    expect(series.update).toHaveBeenCalledTimes(1)
    expect(series.update).toHaveBeenCalledWith(next[1])
    expect(result).toBe(next)
  })

  it('update appended bars without setData', () => {
    const series = mockSeries()
    const previous = [candle(1, 10), candle(2, 11)]
    const next = [candle(1, 10), candle(2, 11), candle(3, 12)]
    syncCandleSeriesData(series, previous, next)
    expect(series.setData).not.toHaveBeenCalled()
    expect(series.update).toHaveBeenCalledTimes(1)
    expect(series.update).toHaveBeenCalledWith(next[2])
  })

  it('setData when first bar time changes (interval switch)', () => {
    const series = mockSeries()
    const previous = [candle(1, 10), candle(2, 11)]
    const next = [candle(100, 20), candle(101, 21)]
    syncCandleSeriesData(series, previous, next)
    expect(series.setData).toHaveBeenCalledWith(next)
    expect(series.update).not.toHaveBeenCalled()
  })

  it('setData when history is truncated', () => {
    const series = mockSeries()
    const previous = [candle(1, 10), candle(2, 11), candle(3, 12)]
    const next = [candle(1, 10), candle(2, 11)]
    syncCandleSeriesData(series, previous, next)
    expect(series.setData).toHaveBeenCalledWith(next)
    expect(series.update).not.toHaveBeenCalled()
  })

  it('setData when invert rewrites historical OHLC at the same times (GitLab #524)', () => {
    const series = mockSeries()
    const previous = [candle(1, 10), candle(2, 11), candle(3, 12)]
    const next = [candle(1, 0.1), candle(2, 1 / 11), candle(3, 1 / 12)]
    syncCandleSeriesData(series, previous, next)
    expect(series.setData).toHaveBeenCalledWith(next)
    expect(series.update).not.toHaveBeenCalled()
  })

  it('update last bar then append when only the tail changed', () => {
    const series = mockSeries()
    const previous = [candle(1, 10), candle(2, 11)]
    const next = [candle(1, 10), candle(2, 12), candle(3, 13)]
    syncCandleSeriesData(series, previous, next)
    expect(series.setData).not.toHaveBeenCalled()
    expect(series.update).toHaveBeenCalledTimes(2)
    expect(series.update).toHaveBeenNthCalledWith(1, next[1])
    expect(series.update).toHaveBeenNthCalledWith(2, next[2])
  })

  it('no-op when data is unchanged', () => {
    const series = mockSeries()
    const previous = [candle(1, 10), candle(2, 11)]
    const result = syncCandleSeriesData(series, previous, previous)
    expect(series.setData).not.toHaveBeenCalled()
    expect(series.update).not.toHaveBeenCalled()
    expect(result).toBe(previous)
  })

  it('syncHistogramSeriesData updates changed volume bar', () => {
    const series = mockSeries()
    const previous: HistogramData<Time>[] = [
      { time: 1 as Time, value: 5, color: '#22c55e' },
      { time: 2 as Time, value: 6, color: '#ef4444' },
    ]
    const next: HistogramData<Time>[] = [
      { time: 1 as Time, value: 5, color: '#22c55e' },
      { time: 2 as Time, value: 7, color: '#ef4444' },
    ]
    syncHistogramSeriesData(series, previous, next)
    expect(series.update).toHaveBeenCalledWith(next[1])
    expect(series.setData).not.toHaveBeenCalled()
  })

  it('syncLineSeriesData updates indicator tail', () => {
    const series = mockSeries()
    const previous: IndicatorLinePoint[] = [
      { time: 1 as Time, value: 1.1 },
      { time: 2 as Time, value: 1.2 },
    ]
    const next: IndicatorLinePoint[] = [
      { time: 1 as Time, value: 1.1 },
      { time: 2 as Time, value: 1.25 },
    ]
    syncLineSeriesData(series, previous, next)
    expect(series.update).toHaveBeenCalledWith(next[1])
    expect(series.setData).not.toHaveBeenCalled()
  })
})
