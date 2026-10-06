import { useQuery } from '@tanstack/react-query'
import { getOverview } from '@/services/indexer/client'
import { protocolOverviewPollInterval } from '@/utils/protocolLiquidityQuality'

export function useProtocolOverviewQuery() {
  return useQuery({
    queryKey: ['indexer-overview'],
    queryFn: getOverview,
    refetchInterval: (query) => protocolOverviewPollInterval(query.state.data),
    retry: false,
  })
}
