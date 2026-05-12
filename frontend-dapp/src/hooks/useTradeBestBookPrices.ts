import { useQueries } from '@tanstack/react-query'
import { getPairLimitBookPage } from '@/services/indexer/client'

/**
 * Best bid / ask prices from the first page of the indexer limit book (same ordering as on-chain head walk).
 */
export function useTradeBestBookPrices(pairAddr: string) {
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

  const bestBid = bidQ.data?.orders?.[0]?.price ?? null
  const bestAsk = askQ.data?.orders?.[0]?.price ?? null
  const isLoading = bidQ.isLoading || askQ.isLoading
  const isError = bidQ.isError || askQ.isError

  return { bestBid, bestAsk, isLoading, isError, refetch: () => void Promise.all([bidQ.refetch(), askQ.refetch()]) }
}
