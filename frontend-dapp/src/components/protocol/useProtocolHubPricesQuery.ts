import { useQuery } from '@tanstack/react-query'
import { getHubPrices } from '@/services/indexer/client'

export function useProtocolHubPricesQuery() {
  return useQuery({
    queryKey: ['indexer-hub-prices'],
    queryFn: getHubPrices,
    refetchInterval: 30_000,
    retry: false,
  })
}
