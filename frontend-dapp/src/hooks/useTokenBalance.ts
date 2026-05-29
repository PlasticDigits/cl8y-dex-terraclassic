import { useQuery, type UseQueryResult } from '@tanstack/react-query'
import { getTokenBalance } from '@/services/terraclassic/queries'
import { tokenAssetInfo } from '@/types'

/**
 * CW20 balance for a connected wallet (shared `tokenBalance` cache key with swap/pool/limit flows).
 */
export function useTokenBalance(
  walletAddress: string | null | undefined,
  tokenContractAddr: string
): UseQueryResult<string, Error> {
  const addr = walletAddress ?? undefined
  return useQuery({
    queryKey: ['tokenBalance', addr, tokenContractAddr],
    queryFn: () => {
      if (!addr) throw new Error('No wallet')
      return getTokenBalance(addr, tokenAssetInfo(tokenContractAddr))
    },
    enabled: !!addr && tokenContractAddr.startsWith('terra1'),
    refetchInterval: 15_000,
  })
}
