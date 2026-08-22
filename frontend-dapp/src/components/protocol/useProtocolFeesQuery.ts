import { useQuery } from '@tanstack/react-query'
import { getProtocolFees } from '@/services/indexer/client'

export function useProtocolFeesQuery() {
  return useQuery({
    queryKey: ['indexer-protocol-fees', '24h'],
    queryFn: () => getProtocolFees('24h'),
    refetchInterval: 60_000,
    retry: false,
  })
}
