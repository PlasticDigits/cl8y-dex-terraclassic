import { useMemo } from 'react'
import { useQueries } from '@tanstack/react-query'
import { getPairPaused } from '@/services/terraclassic/pair'

export interface UsePairPausedParams {
  pairAddress?: string | null
  pairAddresses?: string[] | null
  enabled?: boolean
}

function resolvePairPausedAddresses(pairAddress?: string | null, pairAddresses?: string[] | null): string[] {
  const fromList = pairAddresses?.filter((p) => p.startsWith('terra1')) ?? []
  if (fromList.length > 0) return [...new Set(fromList)]
  if (pairAddress?.startsWith('terra1')) return [pairAddress]
  return []
}

/** LCD `is_paused` for one or more pairs; `isPaused` is true when any queried pair is paused (L6). */
export function usePairPaused({ pairAddress, pairAddresses, enabled = true }: UsePairPausedParams) {
  const addresses = useMemo(() => resolvePairPausedAddresses(pairAddress, pairAddresses), [pairAddress, pairAddresses])

  const queries = useQueries({
    queries: addresses.map((addr) => ({
      queryKey: ['pairPaused', addr] as const,
      queryFn: () => getPairPaused(addr),
      enabled: enabled && addresses.length > 0,
      staleTime: 15_000,
    })),
  })

  const isPaused = queries.some((q) => q.data?.paused === true)
  const isLoading = queries.some((q) => q.isLoading)
  const isFetching = queries.some((q) => q.isFetching)
  const isError = queries.some((q) => q.isError)

  return {
    addresses,
    queries,
    isPaused,
    isLoading,
    isFetching,
    isError,
  }
}
