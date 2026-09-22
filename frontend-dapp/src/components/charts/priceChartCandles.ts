import type { HistogramData, Time } from 'lightweight-charts'
import type { IndexerCandle } from '@/types'
import { isIndexerCatalogLeg } from '@/utils/pairPriceUsd'
import { invertUsdNumber } from '@/utils/tradePairDisplayOrientation'
import { fromRawAmount, isPairLegDecimals } from '@/utils/formatAmount'

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

export type PricePaneKind = 'usd' | 'human' | 'subject'

export interface ChartLegRef {
  symbol?: string | null
  denom?: string | null
  contract_addr?: string | null
  contractAddr?: string | null
}

export interface PricePanePlot {
  points: ChartCandlePoint[]
  kind: PricePaneKind
}

function positiveOhlc(
  time: Time,
  openRaw: string | null | undefined,
  highRaw: string | null | undefined,
  lowRaw: string | null | undefined,
  closeRaw: string | null | undefined
): ChartCandlePoint | null {
  const open = parseChartFinitePositive(openRaw)
  const high = parseChartFinitePositive(highRaw)
  const low = parseChartFinitePositive(lowRaw)
  const close = parseChartFinitePositive(closeRaw)
  if (open == null || high == null || low == null || close == null) return null
  return { time, open, high, low, close }
}

function mulPositive(a: number, b: number): number | null {
  const n = a * b
  if (!Number.isFinite(n) || n <= 0) return null
  return n
}

/**
 * USD of `asset_0` from a stored USD-of-`asset_1` bar: open×open_human, close×close_human,
 * stored high×human low, stored low×human high, then high = max and low = min (#1315).
 */
export function subjectUsdTimesHuman(stored: ChartCandlePoint, human: ChartCandlePoint): ChartCandlePoint | null {
  const open = mulPositive(stored.open, human.open)
  const close = mulPositive(stored.close, human.close)
  const highRaw = mulPositive(stored.high, human.low)
  const lowRaw = mulPositive(stored.low, human.high)
  if (open == null || close == null || highRaw == null || lowRaw == null) return null
  return {
    time: stored.time,
    open,
    close,
    high: Math.max(highRaw, lowRaw),
    low: Math.min(highRaw, lowRaw),
  }
}

function neitherCatalogPair(asset0?: ChartLegRef | null, asset1?: ChartLegRef | null): boolean {
  if (asset0 == null || asset1 == null) return false
  return !isIndexerCatalogLeg(asset0) && !isIndexerCatalogLeg(asset1)
}

/**
 * Price-pane bars (#1315).
 *
 * Neither-catalog pairs plot human quote-per-base and do not run `invertUsd`.
 * `usd_leg=asset_1` plots `subject_*` when the displayed token is `asset_1`, and
 * `subject × human` (crossed wicks) when the displayed token is `asset_0`. That series
 * does not go through `applyChartDisplayInvert`. Other catalog bars stay factory USD of
 * `asset_0` (empty when only `*_human` is present).
 */
export function plotPricePaneCandles(
  data: IndexerCandle[] | undefined,
  displayInverted: boolean,
  asset0?: ChartLegRef | null,
  asset1?: ChartLegRef | null
): PricePanePlot {
  if (!data?.length) return { points: [], kind: neitherCatalogPair(asset0, asset1) ? 'human' : 'usd' }
  const rows = [...data].sort((a, b) => new Date(a.open_time).getTime() - new Date(b.open_time).getTime())
  if (neitherCatalogPair(asset0, asset1)) {
    const points: ChartCandlePoint[] = []
    for (const c of rows) {
      const timeSec = candleOpenTimeSeconds(c.open_time)
      if (timeSec == null) continue
      const human = parseHumanOhlc(c, timeSec as Time)
      if (human) points.push(human)
    }
    return { points, kind: 'human' }
  }

  const points: ChartCandlePoint[] = []
  let sawSubject = false
  for (const c of rows) {
    const timeSec = candleOpenTimeSeconds(c.open_time)
    if (timeSec == null) continue
    const time = timeSec as Time
    if (c.usd_leg === 'asset_1') {
      const stored = positiveOhlc(time, c.subject_open, c.subject_high, c.subject_low, c.subject_close)
      if (!stored) continue
      if (displayInverted) {
        points.push(stored)
        sawSubject = true
        continue
      }
      const human = parseHumanOhlc(c, time)
      if (!human) continue
      const crossed = subjectUsdTimesHuman(stored, human)
      if (!crossed) continue
      points.push(crossed)
      sawSubject = true
      continue
    }
    if (!isValidUsdIndexerCandle(c)) continue
    const factory: FactoryCandlePoint = {
      time,
      usd: toUsdChartCandlePoint(c),
      human: parseHumanOhlc(c, time),
    }
    const plotted = applyChartDisplayInvert([factory], displayInverted)
    if (plotted[0]) points.push(plotted[0])
  }
  return { points, kind: sawSubject ? 'subject' : 'usd' }
}

/**
 * Quote-side volume per candle, colored by bar direction (same times as factory USD series).
 * Uses **quote** volume when non-zero; otherwise **base** volume so local / thin markets still show bars.
 * Volume is not inverted as price (GitLab #543 **C543-8**).
 *
 * When `scale` is set, raw indexer integers are divided by `10^decimals` (GitLab #564).
 * Missing / out-of-range decimals, non-integer raw, or non-finite human values drop that bar.
 * Omit `scale` only in fixtures that already use human-sized integers.
 */
export type CandleVolumeScale = {
  quoteDecimals: number
  baseDecimals: number
}

function scaleRawCandleVolume(raw: string | undefined, decimals: number): number | null {
  if (!isPairLegDecimals(decimals)) return null
  if (raw == null || raw === '') return 0
  if (typeof raw !== 'string' || raw.length > 78 || /[<>]/.test(raw)) return null
  let n: bigint
  try {
    n = BigInt(raw)
  } catch {
    return null
  }
  if (n < 0n) return null
  if (n === 0n) return 0
  const human = fromRawAmount(raw, decimals)
  const v = Number(human)
  if (!Number.isFinite(v) || v < 0) return null
  return v
}

export function indexerCandlesToVolumeHistogramPoints(
  data: IndexerCandle[] | undefined,
  upColor: string,
  downColor: string,
  scale?: CandleVolumeScale
): HistogramData<Time>[] {
  const out: HistogramData<Time>[] = []
  for (const c of sortedValidUsdCandles(data)) {
    const open = parseChartFinitePositive(c.open)!
    const close = parseChartFinitePositive(c.close)!
    let value: number
    if (scale) {
      const quoteRaw = c.volume_quote
      const quoteIsZero = quoteRaw == null || quoteRaw === '' || quoteRaw === '0'
      if (!quoteIsZero) {
        const vq = scaleRawCandleVolume(quoteRaw, scale.quoteDecimals)
        if (vq == null) continue
        value = vq
      } else {
        const vb = scaleRawCandleVolume(c.volume_base, scale.baseDecimals)
        if (vb == null) continue
        value = vb
      }
      if (!Number.isFinite(value)) continue
    } else {
      const vq = Math.max(0, parseChartFiniteNumber(c.volume_quote) ?? 0)
      const vb = Math.max(0, parseChartFiniteNumber(c.volume_base) ?? 0)
      value = vq > 0 ? vq : vb
      if (!Number.isFinite(value)) continue
    }
    out.push({
      time: candleOpenTimeSeconds(c.open_time)! as Time,
      value,
      color: close >= open ? upColor : downColor,
    })
  }
  return out
}

function candleVolumeValue(c: IndexerCandle, scale?: CandleVolumeScale): number | null {
  if (scale) {
    const quoteRaw = c.volume_quote
    const quoteIsZero = quoteRaw == null || quoteRaw === '' || quoteRaw === '0'
    if (!quoteIsZero) return scaleRawCandleVolume(quoteRaw, scale.quoteDecimals)
    return scaleRawCandleVolume(c.volume_base, scale.baseDecimals)
  }
  const vq = Math.max(0, parseChartFiniteNumber(c.volume_quote) ?? 0)
  const vb = Math.max(0, parseChartFiniteNumber(c.volume_base) ?? 0)
  const value = vq > 0 ? vq : vb
  return Number.isFinite(value) ? value : null
}

/** Volume bars for a plotted human or subject series, colored by the plotted open/close. */
export function volumeHistogramForPlottedPoints(
  data: IndexerCandle[] | undefined,
  points: ChartCandlePoint[],
  upColor: string,
  downColor: string,
  scale?: CandleVolumeScale
): HistogramData<Time>[] {
  if (!data?.length || points.length === 0) return []
  const byTime = new Map<number, IndexerCandle>()
  for (const c of data) {
    const t = candleOpenTimeSeconds(c.open_time)
    if (t != null) byTime.set(t, c)
  }
  const out: HistogramData<Time>[] = []
  for (const point of points) {
    const t = typeof point.time === 'number' ? point.time : null
    if (t == null) continue
    const candle = byTime.get(t)
    if (!candle) continue
    const value = candleVolumeValue(candle, scale)
    if (value == null || !Number.isFinite(value)) continue
    out.push({
      time: point.time,
      value,
      color: point.close >= point.open ? upColor : downColor,
    })
  }
  return out
}
