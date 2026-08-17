import { useMemo } from 'react'
import { useQueries } from '@tanstack/react-query'
import { getTokenBalance } from '@/services/terraclassic/queries'
import { tokenAssetInfo } from '@/types'
import { isNativeDenom } from '@/types'
import { isNativeWrapEnabled } from '@/utils/constants'

/** Factory tokens plus native denoms when wrap env is set (retail add picker universe). */
export function retailAddTokenCandidates(factoryTokenIds: string[]): string[] {
  const ids = [...factoryTokenIds]
  if (isNativeWrapEnabled()) {
    if (!ids.includes('uluna')) ids.unshift('uluna')
    if (!ids.includes('uusd')) ids.splice(1, 0, 'uusd')
  }
  return [...new Set(ids.filter(Boolean))]
}

/**
 * Wallet holdings `> 0` when connected (Z533-1 / U2). Disconnected returns the candidate
 * universe so pickers stay visible (U1).
 */
export function usePositiveWalletTokens(address: string | null | undefined, candidates: string[]) {
  const queries = useQueries({
    queries: candidates.map((tokenId) => ({
      queryKey: ['tokenBalance', address, tokenId] as const,
      queryFn: () => getTokenBalance(address!, tokenAssetInfo(tokenId)),
      enabled: !!address && !!tokenId,
      staleTime: 15_000,
    })),
  })

  return useMemo(() => {
    if (!address) {
      return { tokenIds: candidates, empty: false, loading: false }
    }
    const loading = queries.some((q) => q.isLoading)
    const tokenIds = candidates.filter((_, i) => {
      const raw = queries[i]?.data
      if (!raw || raw === '0') return false
      try {
        return BigInt(raw) > 0n
      } catch {
        return false
      }
    })
    return { tokenIds, empty: !loading && tokenIds.length === 0, loading }
  }, [address, candidates, queries])
}

export function isNativeAddToken(tokenId: string): boolean {
  return isNativeDenom(tokenId)
}
