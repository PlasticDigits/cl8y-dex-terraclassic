import type { HistogramData, ISeriesApi, SeriesType, Time } from 'lightweight-charts'
import type { ChartCandlePoint } from './priceChartCandles'
import type { IndicatorLinePoint } from './priceChartIndicators'

type SeriesApi<T extends SeriesType> = Pick<ISeriesApi<T>, 'setData' | 'update'>

function candlePointEqual(a: ChartCandlePoint, b: ChartCandlePoint): boolean {
  return a.time === b.time && a.open === b.open && a.high === b.high && a.low === b.low && a.close === b.close
}

function histogramPointEqual(a: HistogramData<Time>, b: HistogramData<Time>): boolean {
  return a.time === b.time && a.value === b.value && a.color === b.color
}

function linePointEqual(a: IndicatorLinePoint, b: IndicatorLinePoint): boolean {
  return a.time === b.time && a.value === b.value
}

/**
 * Apply candle/volume/indicator data without resetting the time-scale viewport.
 *
 * - First load or structural change (interval switch, truncated history): `setData`.
 * - Background refetch with same prefix: `update` from the first changed bar onward.
 */
export function syncCandleSeriesData(
  series: SeriesApi<'Candlestick'>,
  previous: ChartCandlePoint[],
  next: ChartCandlePoint[]
): ChartCandlePoint[] {
  return syncTimedSeries(series, previous, next, candlePointEqual)
}

export function syncHistogramSeriesData(
  series: SeriesApi<'Histogram'>,
  previous: HistogramData<Time>[],
  next: HistogramData<Time>[]
): HistogramData<Time>[] {
  return syncTimedSeries(series, previous, next, histogramPointEqual)
}

export function syncLineSeriesData(
  series: SeriesApi<'Line'>,
  previous: IndicatorLinePoint[],
  next: IndicatorLinePoint[]
): IndicatorLinePoint[] {
  return syncTimedSeries(series, previous, next, linePointEqual)
}

function syncTimedSeries<T extends { time: Time }>(
  series: SeriesApi<'Candlestick' | 'Histogram' | 'Line'>,
  previous: T[],
  next: T[],
  equal: (a: T, b: T) => boolean
): T[] {
  if (next.length === 0) {
    series.setData([])
    return []
  }

  if (previous.length === 0) {
    series.setData(next)
    return next
  }

  const structuralChange = previous[0]?.time !== next[0]?.time || next.length < previous.length

  if (structuralChange) {
    series.setData(next)
    return next
  }

  let firstChanged = previous.length
  const commonLen = Math.min(previous.length, next.length)
  for (let i = 0; i < commonLen; i++) {
    if (!equal(previous[i]!, next[i]!)) {
      firstChanged = i
      break
    }
  }

  if (firstChanged === previous.length && next.length === previous.length) {
    return previous
  }

  for (let i = firstChanged; i < next.length; i++) {
    series.update(next[i]!)
  }

  return next
}
