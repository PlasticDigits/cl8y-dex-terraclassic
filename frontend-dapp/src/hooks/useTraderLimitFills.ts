import { useQuery } from '@tanstack/react-query'
import { getTraderLimitFills } from '@/services/indexer/client'

/** Wallet-scoped fills on one pair — used to classify LCD `Unknown` vs fill (#530 / L21). */
export function useTraderLimitFills(walletAddress: string | undefined, pairAddr: string) {
  return useQuery({
    queryKey: ['limitFills', pairAddr, walletAddress],
    queryFn: () => getTraderLimitFills(walletAddress!, { pair: pairAddr, limit: 200 }),
    enabled: !!walletAddress && pairAddr.startsWith('terra1'),
    staleTime: 10_000,
  })
}
