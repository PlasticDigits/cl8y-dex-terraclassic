import { useQuery } from '@tanstack/react-query'
import { getOverview } from '@/services/indexer/client'

export function useProtocolOverviewQuery() {
  return useQuery({
    queryKey: ['indexer-overview'],
    queryFn: getOverview,
    refetchInterval: 60_000,
    retry: false,
  })
}
