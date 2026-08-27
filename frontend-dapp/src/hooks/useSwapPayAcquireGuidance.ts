import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { getUst1EffectiveSwap } from '@/services/terraclassic/ust1Window'
import { getTokenBalance } from '@/services/terraclassic/queries'
import {
  isNativeWrapEnabled,
  isUst1WindowEnabled,
  NATIVE_WRAPPED_PAIRS,
  UST1_TOKEN_ADDRESS,
  UST1_WINDOW_CONTRACT_ADDRESS,
  VFDUSD_TOKEN_ADDRESS,
} from '@/utils/constants'
import { tryParseBigInt } from '@/utils/decimalAmountInput'
import {
  evaluateSwapPayAcquireGuidance,
  isUst1PayAsset,
  type SwapPayAcquireGuidance,
} from '@/utils/swapPayAcquireGuidance'

const WRAPPED_PAY_ASSETS = new Set(Object.values(NATIVE_WRAPPED_PAIRS).filter(Boolean))

export function useSwapPayAcquireGuidance(input: {
  walletConnected: boolean
  address: string | null
  hasPositivePay: boolean
  hasSettledQuote: boolean
  payAsset: string
  paySymbol: string
  payDecimals: number
  payRaw: bigint | null
  payBalanceRaw: bigint | null
  expectedSlippagePct: number | null
}): SwapPayAcquireGuidance {
  const payIsUst1 = isUst1PayAsset(input.payAsset, UST1_TOKEN_ADDRESS)
  const windowEnabled = isUst1WindowEnabled()

  const windowQuery = useQuery({
    queryKey: ['ust1EffectiveSwap', UST1_WINDOW_CONTRACT_ADDRESS],
    queryFn: getUst1EffectiveSwap,
    enabled: windowEnabled && payIsUst1,
    staleTime: 15_000,
    refetchInterval: 30_000,
  })

  const vfdusdQuery = useQuery({
    queryKey: ['tokenBalance', input.address, VFDUSD_TOKEN_ADDRESS],
    queryFn: () => {
      if (!input.address || !VFDUSD_TOKEN_ADDRESS) throw new Error('Missing params')
      return getTokenBalance(input.address, { token: { contract_addr: VFDUSD_TOKEN_ADDRESS } })
    },
    enabled: input.walletConnected && !!input.address && !!VFDUSD_TOKEN_ADDRESS && payIsUst1,
    staleTime: 15_000,
  })

  const nowSec = Math.floor(Date.now() / 1000)

  return useMemo(
    () =>
      evaluateSwapPayAcquireGuidance({
        walletConnected: input.walletConnected,
        hasPositivePay: input.hasPositivePay,
        hasSettledQuote: input.hasSettledQuote,
        payAsset: input.payAsset,
        paySymbol: input.paySymbol,
        payDecimals: input.payDecimals,
        payRaw: input.payRaw,
        payBalanceRaw: input.payBalanceRaw,
        vfdusdBalanceRaw: tryParseBigInt(vfdusdQuery.data ?? ''),
        ust1TokenAddress: UST1_TOKEN_ADDRESS,
        windowEnabled,
        windowView: windowQuery.data ?? null,
        windowViewError: windowQuery.isError,
        wrapEnabled: isNativeWrapEnabled(),
        wrappedPayAssets: WRAPPED_PAY_ASSETS,
        expectedSlippagePct: input.expectedSlippagePct,
        nowSec,
      }),
    [
      input.walletConnected,
      input.hasPositivePay,
      input.hasSettledQuote,
      input.payAsset,
      input.paySymbol,
      input.payDecimals,
      input.payRaw,
      input.payBalanceRaw,
      input.expectedSlippagePct,
      vfdusdQuery.data,
      windowEnabled,
      windowQuery.data,
      windowQuery.isError,
      nowSec,
    ]
  )
}
