import { useMemo } from 'react'
import { useQueries, useQueryClient, type InfiniteData } from '@tanstack/react-query'
import { getPairLimitBookPage } from '@/services/indexer/client'
import type { IndexerLimitBookPageResponse } from '@/types'
import { limitBookPageQueryKey } from '@/utils/limitBookPagination'
import { scaleRawLimitPriceForDisplay, type LimitPriceDecimals } from '@/utils/limitOrderPriceScale'

function bestPriceFromLimitBookCache(
  queryClient: ReturnType<typeof useQueryClient>,
  pairAddr: string,
  side: 'bid' | 'ask'
): string | null {
  const cached = queryClient.getQueryData<InfiniteData<IndexerLimitBookPageResponse>>(
    limitBookPageQueryKey(pairAddr, side)
  )
  return cached?.pages?.[0]?.orders?.[0]?.price ?? null
}

/**
 * Best bid / ask prices from the first page of the indexer limit book (same ordering as on-chain head walk).
 * Falls back to the paginated `limitBookPage` query cache when the dedicated head fetch has not resolved yet
 * ([#385](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/385)).
 */
export function useTradeBestBookPrices(pairAddr: string, limitPriceScale?: LimitPriceDecimals | null) {
  const queryClient = useQueryClient()
  const [bidQ, askQ] = useQueries({
    queries: [
      {
        queryKey: ['tradeBestBook', pairAddr, 'bid'],
        queryFn: () => getPairLimitBookPage(pairAddr, 'bid', { limit: 1 }),
        enabled: pairAddr.startsWith('terra1'),
        staleTime: 5000,
      },
      {
        queryKey: ['tradeBestBook', pairAddr, 'ask'],
        queryFn: () => getPairLimitBookPage(pairAddr, 'ask', { limit: 1 }),
        enabled: pairAddr.startsWith('terra1'),
        staleTime: 5000,
      },
    ],
  })

  const bestBidFromQuery = bidQ.data?.orders?.[0]?.price ?? null
  const bestAskFromQuery = askQ.data?.orders?.[0]?.price ?? null

  const bestBidFromCache = useMemo(
    () => (bestBidFromQuery ? null : bestPriceFromLimitBookCache(queryClient, pairAddr, 'bid')),
    [bestBidFromQuery, queryClient, pairAddr]
  )
  const bestAskFromCache = useMemo(
    () => (bestAskFromQuery ? null : bestPriceFromLimitBookCache(queryClient, pairAddr, 'ask')),
    [bestAskFromQuery, queryClient, pairAddr]
  )

  const bestBidRaw = bestBidFromQuery ?? bestBidFromCache
  const bestAskRaw = bestAskFromQuery ?? bestAskFromCache
  const bestBid = bestBidRaw ? scaleRawLimitPriceForDisplay(bestBidRaw, limitPriceScale) : null
  const bestAsk = bestAskRaw ? scaleRawLimitPriceForDisplay(bestAskRaw, limitPriceScale) : null
  const isLoading = bidQ.isLoading || askQ.isLoading
  const isError = bidQ.isError || askQ.isError

  return { bestBid, bestAsk, isLoading, isError, refetch: () => void Promise.all([bidQ.refetch(), askQ.refetch()]) }
}
