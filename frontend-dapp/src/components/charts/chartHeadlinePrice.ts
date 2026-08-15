import { formatNum } from '@/utils/formatAmount'
import type { ChartCandlePoint } from './priceChartCandles'

/**
 * Prominent USD headline for the trade/charts candle panel: prefer latest tape USD
 * (`tapeLastPriceUsd` from `resolveTapeLastPriceUsd` / indexer `price_usd` — GitLab #522),
 * else the last candle close for the selected interval (GitLab #149).
 * Do not pass raw `trades[].price` (human quote-per-base or unscaled integers).
 */
export function resolveTradeChartHeadlineUsd(
  tapeLastPriceUsd: string | null | undefined,
  chartPoints: ChartCandlePoint[]
): string | null {
  const raw = tapeLastPriceUsd?.trim()
  if (raw) {
    const n = parseFloat(raw)
    if (!Number.isNaN(n)) return formatNum(raw, 6)
  }
  if (chartPoints.length > 0) {
    const close = chartPoints[chartPoints.length - 1].close
    return formatNum(close, 6)
  }
  return null
}
