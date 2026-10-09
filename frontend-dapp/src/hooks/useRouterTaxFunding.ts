import { queryRouterTaxFundingVersion } from '@/services/terraclassic/routeTaxFunding'
import { ROUTER_CONTRACT_ADDRESS } from '@/utils/constants'
import { useQuery } from '@tanstack/react-query'

/** Router version controls gross-budget Max before the first amount quote exists. */
export function useRouterTaxFunding(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: ['routerTaxFunding', ROUTER_CONTRACT_ADDRESS],
    queryFn: ({ signal }) => queryRouterTaxFundingVersion(signal).then((version) => version ?? 0),
    enabled: options?.enabled !== false && !!ROUTER_CONTRACT_ADDRESS,
    staleTime: 5_000,
    retry: false,
  })
}
