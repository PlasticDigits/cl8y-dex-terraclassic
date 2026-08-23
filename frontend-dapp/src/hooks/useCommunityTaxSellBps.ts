import { useQuery } from '@tanstack/react-query'
import { getChainContractInfo } from '@/services/terraclassic/queries'
import { queryCommunityTaxConfig } from '@/services/terraclassic/communityTaxToken'
import { COMMUNITY_TAX_CODE_ID, isCommunityTaxEnabled } from '@/utils/constants'
import { isValidTerraBech32Address } from '@/utils/terraAddressValidation'

/** LCD sell_bps when pay token is the community tax template (#593). Null otherwise. */
export function useCommunityTaxSellBps(tokenAddr: string | null | undefined) {
  const enabled = isCommunityTaxEnabled() && !!tokenAddr && isValidTerraBech32Address(tokenAddr)
  const info = useQuery({
    queryKey: ['communityTaxCodeId', tokenAddr],
    queryFn: () => getChainContractInfo(tokenAddr!),
    enabled,
    staleTime: 60_000,
  })
  const isTax = info.data?.code_id === COMMUNITY_TAX_CODE_ID
  const cfg = useQuery({
    queryKey: ['communityTaxSellBps', tokenAddr],
    queryFn: () => queryCommunityTaxConfig(tokenAddr!),
    enabled: enabled && isTax,
    staleTime: 30_000,
  })
  return {
    sellBps: isTax ? (cfg.data?.sell_bps ?? null) : null,
    isTaxToken: isTax,
    isLoading: info.isLoading || (isTax && cfg.isLoading),
  }
}
