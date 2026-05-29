import { MarketDataLoadingStatus } from '@/components/common/MarketDataLoadingStatus'

/** Immediate feedback while indexer chart/book queries load after a pair change (GitLab #180). */
export function TradePairSwitchStatus({ pairLabel }: { pairLabel?: string }) {
  return (
    <MarketDataLoadingStatus
      testId="trade-pair-switch-loading"
      label={`Loading market data${pairLabel ? ` for ${pairLabel}` : ''}…`}
    />
  )
}
