import { useWalletStore } from '@/hooks/useWallet'
import { getFeeDiscountHealth } from '@/services/indexer/client'
import { getRegistration, getTraderDiscount } from '@/services/terraclassic/feeDiscount'
import { getPairDiscountRegistry } from '@/services/terraclassic/pairDiscountRegistry'
import { FEE_DISCOUNT_CONTRACT_ADDRESS } from '@/utils/constants'
import {
  resolveFeeDiscountRegistryStatus,
  shouldShowFeeDiscountRegistryWarning,
  type FeeDiscountRegistryStatus,
  type FeeDiscountRegistryWarningInput,
} from '@/utils/feeDiscountRegistryWarning'
import { advertisedDiscountBps, pairFeeDiscountApplies } from '@/utils/pairDiscountRegistry'
import { useQuery } from '@tanstack/react-query'
import { useMemo } from 'react'

/**
 * Shared fee-discount registry queries + status for Swap and Pool (GitLab #374 / #476 / #537).
 * React Query keys match prior page-local wiring so caches stay shared across surfaces.
 *
 * Pass `pairAddr` to gate advertised `discountBps` on that pair's `DISCOUNT_REGISTRY` (I14).
 */
export function useFeeDiscountRegistryStatus(pairAddr?: string, options?: { enabled?: boolean }) {
  const address = useWalletStore((s) => s.address)
  const feeDiscountConfigured = !!FEE_DISCOUNT_CONTRACT_ADDRESS
  const pairQueryAddr = pairAddr?.startsWith('terra1') ? pairAddr : undefined

  const discountQuery = useQuery({
    queryKey: ['traderDiscount', address],
    queryFn: () => {
      if (!address) throw new Error('No address')
      return getTraderDiscount(address)
    },
    enabled: options?.enabled !== false && !!address && feeDiscountConfigured,
    staleTime: 15_000,
  })

  const registrationQuery = useQuery({
    queryKey: ['feeDiscountRegistration', address],
    queryFn: () => {
      if (!address) throw new Error('No address')
      return getRegistration(address)
    },
    enabled: options?.enabled !== false && !!address && feeDiscountConfigured,
    staleTime: 15_000,
  })

  const pairRegistryQuery = useQuery({
    queryKey: ['pairDiscountRegistry', pairQueryAddr],
    queryFn: () => getPairDiscountRegistry(pairQueryAddr!),
    enabled: options?.enabled !== false && !!pairQueryAddr && feeDiscountConfigured,
    staleTime: 60_000,
    retry: false,
  })

  const feeDiscountHealthQuery = useQuery({
    queryKey: ['feeDiscountHealth'],
    queryFn: getFeeDiscountHealth,
    enabled: options?.enabled !== false && feeDiscountConfigured,
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

  const pairRegistry = pairRegistryQuery.isSuccess ? (pairRegistryQuery.data ?? null) : undefined
  const pairDiscountApplies = pairFeeDiscountApplies(pairRegistry, FEE_DISCOUNT_CONTRACT_ADDRESS)
  const discountBps = advertisedDiscountBps(
    discountQuery.data?.discount_bps,
    pairRegistry,
    FEE_DISCOUNT_CONTRACT_ADDRESS
  )

  return {
    discountBps,
    pairDiscountApplies,
    pairRegistryQuery,
    discountQuery,
    registrationQuery,
    feeDiscountRegistryStatus,
    showFeeDiscountRegistryWarning,
    feeDiscountConfigured,
  }
}
