import { TradesTable, RetryError, Skeleton } from '@/components/ui'
import { TradeMarketDataUnavailableNotice } from '@/components/trade/TradeMarketDataUnavailableNotice'
import { TRADE_PANEL_TAPE_UNAVAILABLE } from '@/utils/indexerTradeOutageCopy'
import { isIndexerUnavailableError } from '@/utils/indexerErrors'
import { getErrorMessage } from '@/utils/humanizeUserFacingError'
import type { IndexerPair, IndexerTrade } from '@/types'
import type { UseQueryResult } from '@tanstack/react-query'

type TradeRecentTradesSectionProps = {
  pairRouteReady: boolean
  tradesQuery: Pick<UseQueryResult<IndexerTrade[]>, 'isLoading' | 'isError' | 'error' | 'data' | 'refetch'>
  activePair: IndexerPair | undefined
  formatTimeFn: (iso: string) => string
  skeletonHeight: string
}

export function TradeRecentTradesSection({
  pairRouteReady,
  tradesQuery,
  activePair,
  formatTimeFn,
  skeletonHeight,
}: TradeRecentTradesSectionProps) {
  const tapeIndexerOutage = tradesQuery.isError && isIndexerUnavailableError(tradesQuery.error)

  return (
    <>
      <h2 className="text-xs font-semibold uppercase tracking-wide mb-2 shrink-0" style={{ color: 'var(--ink-dim)' }}>
        Recent trades
      </h2>
      {pairRouteReady && tradesQuery.isLoading && !tradesQuery.isError && <Skeleton height={skeletonHeight} />}
      {tapeIndexerOutage && (
        <TradeMarketDataUnavailableNotice message={TRADE_PANEL_TAPE_UNAVAILABLE} data-testid="trade-tape-unavailable" />
      )}
      {tradesQuery.isError && !tapeIndexerOutage && (
        <RetryError message={getErrorMessage(tradesQuery.error)} onRetry={() => void tradesQuery.refetch()} />
      )}
      {tradesQuery.data && !tradesQuery.isError && (
        <TradesTable
          trades={tradesQuery.data}
          formatTimeFn={formatTimeFn}
          activePair={activePair}
          ariaLabel="Recent trades"
        />
      )}
    </>
  )
}
