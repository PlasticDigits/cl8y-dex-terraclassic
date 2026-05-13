import { useQuery } from '@tanstack/react-query'
import { getPairFeeConfig } from '@/services/terraclassic/settings'
import { getTraderDiscount } from '@/services/terraclassic/feeDiscount'
import { effectiveSwapFeeBps, makerPlacementFeeBps } from '@/utils/limitOrderFeeSummary'

export function useLimitOrderMakerFeeRates(pairAddr: string, walletAddress: string | undefined) {
  const feeConfigQuery = useQuery({
    queryKey: ['pairFeeConfig', pairAddr],
    queryFn: () => getPairFeeConfig(pairAddr),
    enabled: pairAddr.startsWith('terra1'),
    staleTime: 60_000,
  })

  const discountQuery = useQuery({
    queryKey: ['traderDiscount', walletAddress],
    queryFn: () => getTraderDiscount(walletAddress!),
    enabled: !!walletAddress?.startsWith('terra1'),
    staleTime: 60_000,
  })

  const effective =
    feeConfigQuery.data != null
      ? effectiveSwapFeeBps(feeConfigQuery.data.fee_bps, discountQuery.data?.discount_bps)
      : null

  const makerPlacement = effective != null ? makerPlacementFeeBps(effective) : null

  const feeLoading =
    feeConfigQuery.isLoading || (!!walletAddress?.startsWith('terra1') && discountQuery.isLoading)

  const feeError = feeConfigQuery.isError || discountQuery.isError

  return {
    effectiveFeeBps: effective,
    makerPlacementFeeBps: makerPlacement,
    feeLoading,
    feeError,
  }
}
