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

/**
 * Client-side candle trust boundary (GitLab #226, #211).
 *
 * Policy: **drop** rows rather than coerce magic values. A row is kept only when:
 * - `open` and `close` are non-empty strings (indexer presence check),
 * - `open_time` parses to a finite Unix second,
 * - `open`, `high`, `low`, and `close` each parse to a **finite** number (`Number.isFinite`).
 *
 * Malformed strings (`NaN`, `Infinity`, `1e309`, non-numeric), empty `high`/`low`, and invalid
 * timestamps never reach lightweight-charts `setData`. Empty input → `[]` → chart empty state.
 */
export function parseChartFiniteNumber(raw: string): number | null {
  const n = parseFloat(raw)
  return Number.isFinite(n) ? n : null
}

/** Unix seconds for chart time axis; null when `open_time` is not a real instant. */
export function candleOpenTimeSeconds(openTime: string): number | null {
  const ms = new Date(openTime).getTime()
  if (!Number.isFinite(ms)) return null
  return Math.floor(ms / 1000)
}

function isValidIndexerCandleForChart(c: IndexerCandle): boolean {
  if (!c.open || !c.close) return false
  if (candleOpenTimeSeconds(c.open_time) === null) return false
  return (
    parseChartFiniteNumber(c.open) !== null &&
    parseChartFiniteNumber(c.high) !== null &&
    parseChartFiniteNumber(c.low) !== null &&
    parseChartFiniteNumber(c.close) !== null
  )
}

function sortedValidCandles(data: IndexerCandle[] | undefined): IndexerCandle[] {
  if (!data?.length) return []
  return data
    .filter(isValidIndexerCandleForChart)
    .sort((a, b) => new Date(a.open_time).getTime() - new Date(b.open_time).getTime())
}

function toChartCandlePoint(c: IndexerCandle): ChartCandlePoint {
  const open = parseChartFiniteNumber(c.open)!
  const high = parseChartFiniteNumber(c.high)!
  const low = parseChartFiniteNumber(c.low)!
  const close = parseChartFiniteNumber(c.close)!
  return {
    time: candleOpenTimeSeconds(c.open_time)! as Time,
    open,
    high,
    low,
    close,
  }
}

/**
 * Maps indexer candles to sorted chart points. Invalid rows are dropped (see module policy above).
 * lightweight-charts renders a single candlestick when length === 1; empty input yields no series data.
 */
export function indexerCandlesToChartPoints(data: IndexerCandle[] | undefined): ChartCandlePoint[] {
  return sortedValidCandles(data).map(toChartCandlePoint)
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
    const open = parseChartFiniteNumber(c.open)!
    const close = parseChartFiniteNumber(c.close)!
    const vq = Math.max(0, parseChartFiniteNumber(c.volume_quote) ?? 0)
    const vb = Math.max(0, parseChartFiniteNumber(c.volume_base) ?? 0)
    const value = vq > 0 ? vq : vb
    return {
      time: candleOpenTimeSeconds(c.open_time)! as Time,
      value,
      color: close >= open ? upColor : downColor,
    }
  })
}
