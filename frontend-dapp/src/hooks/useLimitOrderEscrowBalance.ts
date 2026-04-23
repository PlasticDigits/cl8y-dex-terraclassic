import { useQuery, type UseQueryResult } from '@tanstack/react-query'
import { getTokenBalance } from '@/services/terraclassic/queries'
import { tokenAssetInfo } from '@/types'

/**
 * Escrow token CW20 balance for limit place (same query key as swap so cache is shared after allowance).
 */
export function useLimitOrderEscrowBalance(
  walletAddress: string | null | undefined,
  escrowTokenAddr: string
): UseQueryResult<string, Error> {
  const addr = walletAddress ?? undefined
  return useQuery({
    queryKey: ['tokenBalance', addr, escrowTokenAddr],
    queryFn: () => {
      if (!addr) throw new Error('No wallet')
      return getTokenBalance(addr, tokenAssetInfo(escrowTokenAddr))
    },
    enabled: !!addr && escrowTokenAddr.startsWith('terra1'),
    refetchInterval: 15_000,
  })
}
