import type { HistogramData, Time } from 'lightweight-charts'
import type { IndexerCandle } from '@/types'
import { invertUsdNumber } from '@/utils/tradePairDisplayOrientation'

/** OHLC row passed to lightweight-charts CandlestickSeries (TradingView lightweight-charts, not the hosted widget). */
export interface ChartCandlePoint {
  time: Time
  open: number
  high: number
  low: number
  close: number
}

/** Factory USD + optional human quote-per-base for per-bar `invertUsd` (GitLab #543). */
export interface FactoryCandlePoint {
  time: Time
  usd: ChartCandlePoint
  human: ChartCandlePoint | null
}

/**
 * Client-side candle trust boundary (GitLab #226, #211, #543).
 *
 * Policy: **drop** rows rather than coerce magic values. A USD row is kept only when:
 * - `open` and `close` are non-empty strings (indexer presence check),
 * - `open_time` parses to a finite Unix second,
 * - `open`, `high`, `low`, and `close` each parse to a **finite positive** number.
 *
 * Human OHLC is optional; invert drops the bar when any human field is missing or non-positive.
 * Malformed strings (`NaN`, `Infinity`, `1e309`, non-numeric), empty `high`/`low`, `≤ 0`, and
 * invalid timestamps never reach lightweight-charts `setData`. Empty input → `[]` → chart empty state.
 */
export function parseChartFiniteNumber(raw: string): number | null {
  const n = parseFloat(raw)
  return Number.isFinite(n) ? n : null
}

function parseChartFinitePositive(raw: string | null | undefined): number | null {
  if (raw == null || raw === '') return null
  const n = parseChartFiniteNumber(raw)
  if (n == null || n <= 0) return null
  return n
}

/** Unix seconds for chart time axis; null when `open_time` is not a real instant. */
export function candleOpenTimeSeconds(openTime: string): number | null {
  const ms = new Date(openTime).getTime()
  if (!Number.isFinite(ms)) return null
  return Math.floor(ms / 1000)
}

function isValidUsdIndexerCandle(c: IndexerCandle): boolean {
  if (!c.open || !c.close) return false
  if (candleOpenTimeSeconds(c.open_time) === null) return false
  return (
    parseChartFinitePositive(c.open) !== null &&
    parseChartFinitePositive(c.high) !== null &&
    parseChartFinitePositive(c.low) !== null &&
    parseChartFinitePositive(c.close) !== null
  )
}

function parseHumanOhlc(c: IndexerCandle, time: Time): ChartCandlePoint | null {
  const open = parseChartFinitePositive(c.open_human)
  const high = parseChartFinitePositive(c.high_human)
  const low = parseChartFinitePositive(c.low_human)
  const close = parseChartFinitePositive(c.close_human)
  if (open == null || high == null || low == null || close == null) return null
  return { time, open, high, low, close }
}

function sortedValidUsdCandles(data: IndexerCandle[] | undefined): IndexerCandle[] {
  if (!data?.length) return []
  return data
    .filter(isValidUsdIndexerCandle)
    .sort((a, b) => new Date(a.open_time).getTime() - new Date(b.open_time).getTime())
}

function toUsdChartCandlePoint(c: IndexerCandle): ChartCandlePoint {
  return {
    time: candleOpenTimeSeconds(c.open_time)! as Time,
    open: parseChartFinitePositive(c.open)!,
    high: parseChartFinitePositive(c.high)!,
    low: parseChartFinitePositive(c.low)!,
    close: parseChartFinitePositive(c.close)!,
  }
}

/**
 * Maps indexer candles to factory USD + human pairs. Rows without finite **positive** USD
 * are dropped (no human-on-USD-axis fallback — GitLab #543 / P522-5).
 */
export function indexerCandlesToFactoryPoints(data: IndexerCandle[] | undefined): FactoryCandlePoint[] {
  return sortedValidUsdCandles(data).map((c) => {
    const usd = toUsdChartCandlePoint(c)
    return { time: usd.time, usd, human: parseHumanOhlc(c, usd.time) }
  })
}

/**
 * Maps indexer candles to sorted factory-USD chart points. Invalid / non-positive USD rows
 * are dropped (see module policy above).
 */
export function indexerCandlesToChartPoints(data: IndexerCandle[] | undefined): ChartCandlePoint[] {
  return indexerCandlesToFactoryPoints(data).map((p) => p.usd)
}

/**
 * Display USD of the displayed base (GitLab #543 / T524-4).
 *
 * Not inverted: factory USD. Inverted: per-bar `invertUsd(factoryUsd, human)` — **never**
 * `1/x` of a USD series (`invertOhlc` is human quote-per-base only). High/low are inverted
 * independently then swapped so `high ≥ low`. Bars missing human (or non-positive / non-finite
 * on either series) are dropped. Times are unchanged.
 */
export function applyChartDisplayInvert(points: FactoryCandlePoint[], inverted: boolean): ChartCandlePoint[] {
  if (!inverted) {
    return points.map((p) => p.usd)
  }
  const out: ChartCandlePoint[] = []
  for (const p of points) {
    const human = p.human
    if (!human) continue
    const open = invertUsdNumber(p.usd.open, human.open)
    const close = invertUsdNumber(p.usd.close, human.close)
    const high = invertUsdNumber(p.usd.high, human.high)
    const low = invertUsdNumber(p.usd.low, human.low)
    if (open == null || close == null || high == null || low == null) continue
    out.push({
      time: p.time,
      open,
      high: Math.max(high, low),
      low: Math.min(high, low),
      close,
    })
  }
  return out
}

/**
 * Quote-side volume per candle, colored by bar direction (same times as factory USD series).
 * Uses **quote** volume when non-zero; otherwise **base** volume so local / thin markets still show bars.
 * Volume is not inverted as price (GitLab #543).
 */
export function indexerCandlesToVolumeHistogramPoints(
  data: IndexerCandle[] | undefined,
  upColor: string,
  downColor: string
): HistogramData<Time>[] {
  return sortedValidUsdCandles(data).map((c) => {
    const open = parseChartFinitePositive(c.open)!
    const close = parseChartFinitePositive(c.close)!
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
