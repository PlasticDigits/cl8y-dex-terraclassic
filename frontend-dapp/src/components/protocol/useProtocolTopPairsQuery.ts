import { useQuery } from '@tanstack/react-query'
import { getProtocolTopPairs } from '@/services/indexer/client'

export function useProtocolTopPairsQuery() {
  return useQuery({
    queryKey: ['indexer-protocol-top-pairs'],
    queryFn: getProtocolTopPairs,
    refetchInterval: 60_000,
    retry: false,
  })
}

export function isProtocolTopPairsUnavailable(err: unknown): boolean {
  if (!(err instanceof Error)) return false
  return /Indexer API error: 404/.test(err.message) || /Indexer API error: 501/.test(err.message)
}
