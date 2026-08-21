import { useMemo } from 'react'
import { useQueries } from '@tanstack/react-query'
import { probePairCodeIdFreeze } from '@/services/terraclassic/assetCodeIdFreeze'

export interface UsePairCodeIdFreezeParams {
  pairAddress?: string | null
  pairAddresses?: string[] | null
  /** Indexer `code_id_frozen` hint (catalog / pair detail). */
  indexerHintFrozen?: boolean
  enabled?: boolean
}

function resolveAddresses(pairAddress?: string | null, pairAddresses?: string[] | null): string[] {
  const fromList = pairAddresses?.filter((p) => p.startsWith('terra1')) ?? []
  if (fromList.length > 0) return [...new Set(fromList)]
  if (pairAddress?.startsWith('terra1')) return [pairAddress]
  return []
}

/** LCD F6 freeze probe; `isFrozen` is true when any hop is frozen or the indexer hint is set. */
export function usePairCodeIdFreeze({
  pairAddress,
  pairAddresses,
  indexerHintFrozen = false,
  enabled = true,
}: UsePairCodeIdFreezeParams) {
  const addresses = useMemo(() => resolveAddresses(pairAddress, pairAddresses), [pairAddress, pairAddresses])

  const queries = useQueries({
    queries: addresses.map((addr) => ({
      queryKey: ['pairCodeIdFreeze', addr] as const,
      queryFn: () => probePairCodeIdFreeze(addr),
      enabled: enabled && addresses.length > 0,
      staleTime: 15_000,
      retry: false,
    })),
  })

  const lcdFrozen = queries.some((q) => q.data?.frozen === true)
  const isFrozen = indexerHintFrozen || lcdFrozen

  return {
    addresses,
    queries,
    isFrozen,
    isLoading: queries.some((q) => q.isLoading),
    isFetching: queries.some((q) => q.isFetching),
    isError: queries.some((q) => q.isError),
  }
}
