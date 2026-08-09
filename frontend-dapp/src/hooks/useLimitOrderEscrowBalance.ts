import { useQuery, type UseQueryResult } from '@tanstack/react-query'
import { getTokenBalance } from '@/services/terraclassic/queries'
import { tokenAssetInfo } from '@/types'

/**
 * Wallet balance for CW20 (`terra1…`) or native denom (`uluna` / `uusd`).
 * Shares React Query key `['tokenBalance', address, tokenId]` with Swap/Pool/header LUNC
 * so LCD bank/CW20 balance calls are not duplicated across pages.
 */
export function useLimitOrderEscrowBalance(
  walletAddress: string | null | undefined,
  tokenId: string
): UseQueryResult<string, Error> {
  const addr = walletAddress ?? undefined
  return useQuery({
    queryKey: ['tokenBalance', addr, tokenId],
    queryFn: () => {
      if (!addr) throw new Error('No wallet')
      return getTokenBalance(addr, tokenAssetInfo(tokenId))
    },
    enabled: !!addr && !!tokenId,
    refetchInterval: 15_000,
  })
}
