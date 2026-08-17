import { useQuery } from '@tanstack/react-query'
import { getPairFeeConfig } from '@/services/terraclassic/settings'
import { getTraderDiscount } from '@/services/terraclassic/feeDiscount'
import { getPairDiscountRegistry } from '@/services/terraclassic/pairDiscountRegistry'
import { FEE_DISCOUNT_CONTRACT_ADDRESS } from '@/utils/constants'
import { pairFeeDiscountApplies } from '@/utils/pairDiscountRegistry'
import { effectiveSwapFeeBps, makerPlacementFeeBps, resolveLimitDiscountBps } from '@/utils/limitOrderFeeSummary'

export function useLimitOrderMakerFeeRates(pairAddr: string, walletAddress: string | undefined) {
  const feeConfigQuery = useQuery({
    queryKey: ['pairFeeConfig', pairAddr],
    queryFn: () => getPairFeeConfig(pairAddr),
    enabled: pairAddr.startsWith('terra1'),
    staleTime: 60_000,
  })

  const pairRegistryQuery = useQuery({
    queryKey: ['pairDiscountRegistry', pairAddr],
    queryFn: () => getPairDiscountRegistry(pairAddr),
    enabled: pairAddr.startsWith('terra1') && !!FEE_DISCOUNT_CONTRACT_ADDRESS,
    staleTime: 60_000,
    retry: false,
  })

  const pairDiscountApplies = pairFeeDiscountApplies(pairRegistryQuery.data, FEE_DISCOUNT_CONTRACT_ADDRESS)

  const discountQuery = useQuery({
    queryKey: ['traderDiscount', walletAddress],
    queryFn: () => getTraderDiscount(walletAddress!),
    enabled: pairDiscountApplies && !!walletAddress?.startsWith('terra1'),
    staleTime: 60_000,
  })

  const limitDiscount = pairDiscountApplies
    ? resolveLimitDiscountBps(discountQuery.data?.discount_bps, discountQuery.data?.limit_discount_bps)
    : 0

  const effective = feeConfigQuery.data != null ? effectiveSwapFeeBps(feeConfigQuery.data.fee_bps, limitDiscount) : null

  const makerPlacement = effective != null ? makerPlacementFeeBps(effective) : null

  const feeLoading =
    feeConfigQuery.isLoading ||
    (!!pairAddr.startsWith('terra1') && !!FEE_DISCOUNT_CONTRACT_ADDRESS && pairRegistryQuery.isLoading) ||
    (pairDiscountApplies && !!walletAddress?.startsWith('terra1') && discountQuery.isLoading)

  const feeError = feeConfigQuery.isError || discountQuery.isError

  return {
    effectiveFeeBps: effective,
    makerPlacementFeeBps: makerPlacement,
    pairDiscountApplies,
    feeLoading,
    feeError,
  }
}
