import type { HistogramData, Time } from 'lightweight-charts'
import type { IndexerCandle } from '@/types'

/** OHLC row passed to lightweight-charts CandlestickSeries (TradingView lightweight-charts, not the hosted widget). */
export interface ChartCandlePoint {
  time: Time
  open: number
  high: number
  low: number
  close: number
}

function sortedValidCandles(data: IndexerCandle[] | undefined): IndexerCandle[] {
  if (!data?.length) return []
  return data
    .filter((c) => c.open && c.close)
    .sort((a, b) => new Date(a.open_time).getTime() - new Date(b.open_time).getTime())
}

/**
 * Maps indexer candles to sorted chart points. Rows without both `open` and `close` are dropped.
 * lightweight-charts renders a single candlestick when length === 1; empty input yields no series data.
 */
export function indexerCandlesToChartPoints(data: IndexerCandle[] | undefined): ChartCandlePoint[] {
  return sortedValidCandles(data).map((c) => ({
    time: Math.floor(new Date(c.open_time).getTime() / 1000) as Time,
    open: parseFloat(c.open),
    high: parseFloat(c.high),
    low: parseFloat(c.low),
    close: parseFloat(c.close),
  }))
}

/**
 * Quote-side volume per candle, colored by bar direction (same times as OHLC series).
 * Uses **quote** volume when non-zero; otherwise **base** volume so local / thin markets still show bars.
 */
export function indexerCandlesToVolumeHistogramPoints(
  data: IndexerCandle[] | undefined,
  upColor: string,
  downColor: string
): HistogramData<Time>[] {
  return sortedValidCandles(data).map((c) => {
    const open = parseFloat(c.open)
    const close = parseFloat(c.close)
    const vq = Math.max(0, parseFloat(c.volume_quote) || 0)
    const vb = Math.max(0, parseFloat(c.volume_base) || 0)
    const value = vq > 0 ? vq : vb
    return {
      time: Math.floor(new Date(c.open_time).getTime() / 1000) as Time,
      value,
      color: close >= open ? upColor : downColor,
    }
  })
}
