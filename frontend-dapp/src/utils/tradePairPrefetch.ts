import type { QueryClient } from '@tanstack/react-query'
import { getCandles, getPair, getPairLimitBookPage, getTrades } from '@/services/indexer/client'
import { LIMIT_BOOK_UI_PAGE_SIZE, limitBookPageQueryKey } from '@/utils/limitBookPagination'
import { isTradePairRouteParam } from '@/utils/tradePairRoute'

const DEFAULT_CHART_INTERVAL = '1h'
const TRADES_LIMIT = 80

/** Warm indexer + chart + book queries before or right after a `/trade` pair switch (GitLab #180). */
export function prefetchTradePairWorkspace(
  queryClient: QueryClient,
  pairAddr: string,
  chartInterval: string = DEFAULT_CHART_INTERVAL
): void {
  if (!isTradePairRouteParam(pairAddr)) return

  const pairOpts = { queryKey: ['indexer-pair-trade', pairAddr] as const, staleTime: 60_000 }
  void queryClient.prefetchQuery({
    ...pairOpts,
    queryFn: () => getPair(pairAddr),
  })

  void queryClient.prefetchQuery({
    queryKey: ['candles', pairAddr, chartInterval],
    queryFn: () => getCandles(pairAddr, chartInterval),
  })

  void queryClient.prefetchQuery({
    queryKey: ['pair-trades-trade', pairAddr],
    queryFn: () => getTrades(pairAddr, TRADES_LIMIT),
  })

  for (const side of ['bid', 'ask'] as const) {
    void queryClient.prefetchQuery({
      queryKey: limitBookPageQueryKey(pairAddr, side),
      queryFn: () =>
        getPairLimitBookPage(pairAddr, side, {
          limit: LIMIT_BOOK_UI_PAGE_SIZE,
          afterOrderId: undefined,
        }),
      staleTime: 10_000,
    })
  }
}
