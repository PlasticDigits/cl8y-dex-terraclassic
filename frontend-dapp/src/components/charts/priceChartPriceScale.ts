import type { AutoscaleInfo, LogicalRange } from 'lightweight-charts'
import type { ChartCandlePoint } from './priceChartCandles'

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
