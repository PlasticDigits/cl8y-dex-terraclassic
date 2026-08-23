import { useQuery } from '@tanstack/react-query'
import { getChainContractInfo } from '@/services/terraclassic/queries'
import { queryCommunityTaxConfig, queryCommunityTaxIsExempt } from '@/services/terraclassic/communityTaxToken'
import { useWalletStore } from '@/hooks/useWallet'
import { COMMUNITY_TAX_CODE_ID, isCommunityTaxEnabled } from '@/utils/constants'
import { isValidTerraBech32Address } from '@/utils/terraAddressValidation'
import { effectiveExtraDebitSellBps } from '@/utils/taxPreviewMaxSpend'

/** LCD sell_bps when pay token is the community tax template (#593 / #609). Null otherwise. */
export function useCommunityTaxSellBps(tokenAddr: string | null | undefined) {
  const wallet = useWalletStore((s) => s.address)
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
  const walletOk = !!wallet && isValidTerraBech32Address(wallet)
  const exempt = useQuery({
    queryKey: ['communityTaxIsExempt', tokenAddr, wallet],
    queryFn: () => queryCommunityTaxIsExempt(tokenAddr!, wallet!),
    enabled: enabled && isTax && walletOk,
    staleTime: 15_000,
  })
  return {
    sellBps: isTax ? effectiveExtraDebitSellBps(cfg.data?.sell_bps ?? null, exempt.data?.manager) : null,
    isTaxToken: isTax,
    isLoading: info.isLoading || (isTax && cfg.isLoading),
  }
}
