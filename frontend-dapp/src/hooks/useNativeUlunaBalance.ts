import type { UseQueryResult } from '@tanstack/react-query'
import { useTokenBalance } from '@/hooks/useTokenBalance'

/**
 * Bank uluna balance for the connected wallet (same `tokenBalance` cache key as swap/pool/wrap).
 */
export function useNativeUlunaBalance(walletAddress: string | null | undefined): UseQueryResult<string, Error> {
  return useTokenBalance(walletAddress, 'uluna')
}
