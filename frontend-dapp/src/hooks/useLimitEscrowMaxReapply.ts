import { useEffect } from 'react'
import type { UseQueryResult } from '@tanstack/react-query'
import { computeMaxSpendableHumanAmount } from '@/utils/maxSpendableAmount'
import type { LimitEscrowAmountSource } from '@/hooks/useLimitOrderForm'

type Params = {
  escrowAmountSource: LimitEscrowAmountSource
  balanceQuery: UseQueryResult<string, Error>
  escrowDecimals: number
  assetIsNativeUluna: boolean
  limitPlaceRungCount?: number
  setLimitEscrowAmountFromMaxReapply: (human: string) => void
}

/** Re-apply Max after Bid/Ask switch using shared gas-aware compute ([GitLab #213](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/213)). */
export function useLimitEscrowMaxReapply({
  escrowAmountSource,
  balanceQuery,
  escrowDecimals,
  assetIsNativeUluna,
  limitPlaceRungCount,
  setLimitEscrowAmountFromMaxReapply,
}: Params) {
  useEffect(() => {
    if (escrowAmountSource !== 'max') return
    if (balanceQuery.isLoading || balanceQuery.isError || !balanceQuery.data) return
    const { human } = computeMaxSpendableHumanAmount({
      balanceRaw: balanceQuery.data,
      decimals: escrowDecimals,
      assetIsNativeUluna,
      context: 'limit_place',
      limitPlaceRungCount,
    })
    setLimitEscrowAmountFromMaxReapply(human)
  }, [
    escrowAmountSource,
    balanceQuery.isLoading,
    balanceQuery.isError,
    balanceQuery.data,
    escrowDecimals,
    assetIsNativeUluna,
    limitPlaceRungCount,
    setLimitEscrowAmountFromMaxReapply,
  ])
}
