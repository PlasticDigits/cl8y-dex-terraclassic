import { useMemo } from 'react'

import {
  estimateLimitOrderBatchPlaceSequenceUlunaFeesTotal,
  estimateLimitOrderPlaceSequenceUlunaFeesTotal,
} from '@/services/terraclassic/transactions'
import {
  evaluateLimitOrderEscrowPlaceGate,
  type LimitOrderEscrowPlaceGateResult,
} from '@/utils/limitOrderEscrowBalanceGate'
import { evaluateLimitOrderNativeGasPlaceGate } from '@/utils/limitOrderNativeGasBalanceGate'
import { useNativeUlunaBalance } from '@/hooks/useNativeUlunaBalance'
import { useTokenBalance } from '@/hooks/useTokenBalance'

export type LimitLadderPlaceGates = {
  escrowGate: LimitOrderEscrowPlaceGateResult
  nativeGasGate: LimitOrderEscrowPlaceGateResult
  canPlace: boolean
  inlineGate: LimitOrderEscrowPlaceGateResult
  batchMinUluna: bigint
  gasSavingsUlunaVsSeparate: bigint
  escrowBalanceQuery: {
    data: string | undefined
    isLoading: boolean
    isError: boolean
  }
  nativeUlunaQuery: {
    data: string | undefined
    isLoading: boolean
    isError: boolean
  }
}

/**
 * Escrow + native LUNC preflight for ladder/batch place (GitLab #206).
 * Uses total escrow human amount and rung count for fee envelope math.
 */
export function useLimitLadderPlaceGates(
  walletAddress: string | undefined,
  escrowToken: string,
  totalHuman: string,
  escrowDecimals: number,
  rungCount: number
): LimitLadderPlaceGates {
  const escrowBalanceQuery = useTokenBalance(walletAddress, escrowToken)
  const nativeUlunaQuery = useNativeUlunaBalance(walletAddress)

  const batchMinUluna = useMemo(() => estimateLimitOrderBatchPlaceSequenceUlunaFeesTotal(rungCount), [rungCount])
  const escrowGate = useMemo(
    () =>
      evaluateLimitOrderEscrowPlaceGate(totalHuman, escrowDecimals, {
        data: escrowBalanceQuery.data,
        isLoading: escrowBalanceQuery.isLoading,
        isError: escrowBalanceQuery.isError,
      }),
    [totalHuman, escrowDecimals, escrowBalanceQuery.data, escrowBalanceQuery.isLoading, escrowBalanceQuery.isError]
  )

  const nativeGasGate = useMemo(
    () =>
      evaluateLimitOrderNativeGasPlaceGate(
        totalHuman,
        escrowDecimals,
        {
          data: nativeUlunaQuery.data,
          isLoading: nativeUlunaQuery.isLoading,
          isError: nativeUlunaQuery.isError,
        },
        batchMinUluna
      ),
    [
      totalHuman,
      escrowDecimals,
      nativeUlunaQuery.data,
      nativeUlunaQuery.isLoading,
      nativeUlunaQuery.isError,
      batchMinUluna,
    ]
  )

  const canPlace = escrowGate.canPlaceLimit && nativeGasGate.canPlaceLimit

  const inlineGate = escrowGate.userMessage ? escrowGate : nativeGasGate

  const gasSavingsUlunaVsSeparate =
    estimateLimitOrderPlaceSequenceUlunaFeesTotal(1) * BigInt(Math.max(rungCount, 1)) - batchMinUluna

  return {
    escrowGate,
    nativeGasGate,
    canPlace,
    inlineGate,
    batchMinUluna,
    gasSavingsUlunaVsSeparate,
    escrowBalanceQuery: {
      data: escrowBalanceQuery.data,
      isLoading: escrowBalanceQuery.isLoading,
      isError: escrowBalanceQuery.isError,
    },
    nativeUlunaQuery: {
      data: nativeUlunaQuery.data,
      isLoading: nativeUlunaQuery.isLoading,
      isError: nativeUlunaQuery.isError,
    },
  }
}
