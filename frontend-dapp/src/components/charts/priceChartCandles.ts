import type { Time } from 'lightweight-charts'
import type { IndexerCandle } from '@/types'

/** OHLC row passed to lightweight-charts CandlestickSeries (TradingView lightweight-charts, not the hosted widget). */
export interface ChartCandlePoint {
  time: Time
  open: number
  high: number
  low: number
  close: number
}

/**
 * Maps indexer candles to sorted chart points. Rows without both `open` and `close` are dropped.
 * lightweight-charts renders a single candlestick when length === 1; empty input yields no series data.
 */
export function indexerCandlesToChartPoints(data: IndexerCandle[] | undefined): ChartCandlePoint[] {
  if (!data?.length) return []
  return data
    .filter((c) => c.open && c.close)
    .map((c) => ({
      time: Math.floor(new Date(c.open_time).getTime() / 1000) as Time,
      open: parseFloat(c.open),
      high: parseFloat(c.high),
      low: parseFloat(c.low),
      close: parseFloat(c.close),
    }))
    .sort((a, b) => (a.time as number) - (b.time as number))
}
