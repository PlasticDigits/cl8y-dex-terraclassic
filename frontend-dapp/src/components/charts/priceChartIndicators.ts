import type { Time } from 'lightweight-charts'
import type { ChartCandlePoint } from './priceChartCandles'

/** Line points for lightweight-charts LineSeries (pane overlays). */
export interface IndicatorLinePoint {
  time: Time
  value: number
}

/**
 * Simple moving average of close prices; first point appears at index `period - 1` (inclusive).
 */
export function chartPointsToSmaLine(points: ChartCandlePoint[], period: number): IndicatorLinePoint[] {
  if (period < 1 || points.length < period) return []
  const out: IndicatorLinePoint[] = []
  for (let i = period - 1; i < points.length; i++) {
    let sum = 0
    for (let j = i - period + 1; j <= i; j++) sum += points[j].close
    out.push({ time: points[i].time, value: sum / period })
  }
  return out
}

/** Wilder RSI on closes; values align with bar indices (null where undefined). */
function rsiValues(closes: number[], period: number): (number | null)[] {
  const n = closes.length
  const result: (number | null)[] = Array.from({ length: n }, () => null)
  if (n < period + 1 || period < 2) return result

  let avgGain = 0
  let avgLoss = 0
  for (let i = 1; i <= period; i++) {
    const change = closes[i] - closes[i - 1]
    avgGain += change > 0 ? change : 0
    avgLoss += change < 0 ? -change : 0
  }
  avgGain /= period
  avgLoss /= period

  const firstIdx = period
  result[firstIdx] = avgLoss === 0 ? 100 : 100 - 100 / (1 + avgGain / avgLoss)

  for (let i = period + 1; i < n; i++) {
    const change = closes[i] - closes[i - 1]
    const gain = change > 0 ? change : 0
    const loss = change < 0 ? -change : 0
    avgGain = (avgGain * (period - 1) + gain) / period
    avgLoss = (avgLoss * (period - 1) + loss) / period
    result[i] = avgLoss === 0 ? 100 : 100 - 100 / (1 + avgGain / avgLoss)
  }
  return result
}

/** RSI line for lightweight-charts (0–100); empty if not enough history. */
export function chartPointsToRsiLine(points: ChartCandlePoint[], period = 14): IndicatorLinePoint[] {
  if (points.length < period + 1) return []
  const closes = points.map((p) => p.close)
  const values = rsiValues(closes, period)
  const out: IndicatorLinePoint[] = []
  for (let i = 0; i < points.length; i++) {
    const v = values[i]
    if (v !== null && Number.isFinite(v)) {
      out.push({ time: points[i].time, value: v })
    }
  }
  return out
}
