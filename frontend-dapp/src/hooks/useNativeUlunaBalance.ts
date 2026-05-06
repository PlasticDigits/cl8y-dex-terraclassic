import { useQuery, type UseQueryResult } from '@tanstack/react-query'
import { getTokenBalance } from '@/services/terraclassic/queries'

/**
 * Bank uluna balance for the connected wallet (same `tokenBalance` cache key shape as swap/pool).
 */
export function useNativeUlunaBalance(walletAddress: string | null | undefined): UseQueryResult<string, Error> {
  const addr = walletAddress ?? undefined
  return useQuery({
    queryKey: ['tokenBalance', addr, 'uluna'],
    queryFn: () => {
      if (!addr) throw new Error('No wallet')
      return getTokenBalance(addr, { native_token: { denom: 'uluna' } })
    },
    enabled: !!addr,
    refetchInterval: 15_000,
  })
}
