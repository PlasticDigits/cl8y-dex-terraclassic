import { formatPairPrice } from '@/utils/formatAmount'
import { pairStatsUsdField } from '@/utils/pairPriceUsd'

/** Latest indexed trade USD only (#1512). Historical candle closes remain on the chart. */
export function resolveTradeChartHeadlineUsd(tapeLastPriceUsd: string | null | undefined): string | null {
  const usd = pairStatsUsdField(tapeLastPriceUsd)
  return usd == null ? null : formatPairPrice(usd, 6)
}
