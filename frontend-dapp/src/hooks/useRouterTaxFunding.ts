import { useQuery } from '@tanstack/react-query'
import { queryRouterTaxFundingVersion } from '@/services/terraclassic/routeTaxFunding'
import { ROUTER_CONTRACT_ADDRESS } from '@/utils/constants'

/** Router version controls gross-budget Max before the first amount quote exists. */
export function useRouterTaxFunding() {
  return useQuery({
    queryKey: ['routerTaxFunding', ROUTER_CONTRACT_ADDRESS],
    queryFn: ({ signal }) => queryRouterTaxFundingVersion(signal).then((version) => version ?? 0),
    enabled: !!ROUTER_CONTRACT_ADDRESS,
    staleTime: 5_000,
    retry: false,
  })
}
