import type { AutoscaleInfo, LogicalRange } from 'lightweight-charts'
import type { ChartCandlePoint } from './priceChartCandles'

/** lightweight-charts `priceFormat` for a USD candlestick pane (GitLab #543). */
export type UsdCandlePriceFormat = {
  type: 'price'
  precision: number
  minMove: number
}

const MIN_USD_PRECISION = 2
const MAX_USD_PRECISION = 8

/**
 * Adaptive USD axis format from visible display-USD magnitudes.
 *
 * `minMove = 10^(floor(log10(min_visible)) - 2)` so `$1.06`, `$0.012258`, and `$0.000047`
 * do not share a fixed 2-dp format (cLUNC must not print as `0.00`). Clamped to
 * precision 2–8. Compact `T`/`K` must not be used as a price-axis formatter (P522-5).
 */
export function usdCandlePriceFormat(visibleAbsValues: number[]): UsdCandlePriceFormat {
  const positives = visibleAbsValues.filter((v) => Number.isFinite(v) && v > 0)
  if (!positives.length) {
    return { type: 'price', precision: MIN_USD_PRECISION, minMove: 0.01 }
  }
  const minVisible = Math.min(...positives)
  const mag = Math.floor(Math.log10(minVisible))
  const precision = Math.min(MAX_USD_PRECISION, Math.max(MIN_USD_PRECISION, 2 - mag))
  const minMove = 10 ** -precision
  return { type: 'price', precision, minMove }
}

export function usdCandlePriceFormatFromPoints(points: ChartCandlePoint[]): UsdCandlePriceFormat {
  const values: number[] = []
  for (const p of points) {
    values.push(p.open, p.high, p.low, p.close)
  }
  return usdCandlePriceFormat(values)
}

/**
 * Minimum `low` among candle points whose logical indices intersect the visible range.
 * Uses the same logical coordinates as `chart.timeScale().getVisibleLogicalRange()`.
 */
export function minLowInVisibleLogicalRange(points: ChartCandlePoint[], logical: LogicalRange | null): number | null {
  if (!points.length) return null
  if (!logical) {
    return Math.min(...points.map((p) => p.low))
  }
  const from = Math.max(0, Math.ceil(Number(logical.from)))
  const to = Math.min(points.length - 1, Math.floor(Number(logical.to)))
  if (from > to) return null
  let min = Infinity
  for (let i = from; i <= to; i++) {
    min = Math.min(min, points[i].low)
  }
  return Number.isFinite(min) ? min : null
}

/**
 * USD spot prices are non-negative; the chart must not auto-scale the price pane below
 * zero or below the lowest visible candle low (see GitLab #151).
 */
export function clampUsdPriceChartAutoscale(
  autoscale: AutoscaleInfo | null,
  visibleMinLow: number | null
): AutoscaleInfo | null {
  if (!autoscale?.priceRange) return autoscale
  const floor = visibleMinLow == null ? 0 : Math.max(0, visibleMinLow)
  autoscale.priceRange.minValue = Math.max(floor, autoscale.priceRange.minValue)
  return autoscale
}
