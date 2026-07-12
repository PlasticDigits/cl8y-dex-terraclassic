import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { getTraderDiscount, getRegistration } from '@/services/terraclassic/feeDiscount'
import { getFeeDiscountHealth } from '@/services/indexer/client'
import { FEE_DISCOUNT_CONTRACT_ADDRESS } from '@/utils/constants'
import { useWalletStore } from '@/hooks/useWallet'
import {
  resolveFeeDiscountRegistryStatus,
  shouldShowFeeDiscountRegistryWarning,
  type FeeDiscountRegistryStatus,
  type FeeDiscountRegistryWarningInput,
} from '@/utils/feeDiscountRegistryWarning'

/**
 * Shared fee-discount registry queries + status for Swap and Pool (GitLab #374 / #476).
 * React Query keys match prior page-local wiring so caches stay shared across surfaces.
 */
export function useFeeDiscountRegistryStatus() {
  const address = useWalletStore((s) => s.address)
  const feeDiscountConfigured = !!FEE_DISCOUNT_CONTRACT_ADDRESS

  const discountQuery = useQuery({
    queryKey: ['traderDiscount', address],
    queryFn: () => {
      if (!address) throw new Error('No address')
      return getTraderDiscount(address)
    },
    enabled: !!address && feeDiscountConfigured,
    staleTime: 15_000,
  })

  const registrationQuery = useQuery({
    queryKey: ['feeDiscountRegistration', address],
    queryFn: () => {
      if (!address) throw new Error('No address')
      return getRegistration(address)
    },
    enabled: !!address && feeDiscountConfigured,
    staleTime: 15_000,
  })

  const feeDiscountHealthQuery = useQuery({
    queryKey: ['feeDiscountHealth'],
    queryFn: getFeeDiscountHealth,
    enabled: feeDiscountConfigured,
    staleTime: 30_000,
    refetchInterval: 30_000,
    retry: false,
  })

  const feeDiscountRegistryInput: FeeDiscountRegistryWarningInput = useMemo(
    () => ({
      feeDiscountContractConfigured: feeDiscountConfigured,
      registration: registrationQuery.data,
      discount: discountQuery.data,
      registrationQueryError: registrationQuery.isError,
      discountQueryError: discountQuery.isError,
      indexerHealth: feeDiscountHealthQuery.isSuccess ? feeDiscountHealthQuery.data : null,
    }),
    [
      feeDiscountConfigured,
      registrationQuery.data,
      discountQuery.data,
      registrationQuery.isError,
      discountQuery.isError,
      feeDiscountHealthQuery.isSuccess,
      feeDiscountHealthQuery.data,
    ]
  )

  const feeDiscountRegistryStatus: FeeDiscountRegistryStatus = useMemo(
    () => resolveFeeDiscountRegistryStatus(feeDiscountRegistryInput),
    [feeDiscountRegistryInput]
  )

  const showFeeDiscountRegistryWarning = shouldShowFeeDiscountRegistryWarning(feeDiscountRegistryInput)

  return {
    discountBps: discountQuery.data?.discount_bps as number | undefined,
    discountQuery,
    registrationQuery,
    feeDiscountRegistryStatus,
    showFeeDiscountRegistryWarning,
    feeDiscountConfigured,
  }
}
