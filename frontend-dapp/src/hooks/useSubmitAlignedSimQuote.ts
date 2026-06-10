import { useMemo } from 'react'
import type { UseQueryResult } from '@tanstack/react-query'
import { applySlippagePercentFloor } from '@/utils/rawAmountMath'
import { isSubmitQuoteStale } from '@/utils/quoteDebounce'

export type SubmitAlignedSimQuote<T extends { return_amount: string }> = {
  /** Raw pay amount aligned with the debounced sim query key — use for on-chain submit. */
  submitPayRaw: string
  simData: T | undefined
  minReceived: string | null
  isQuoteStale: boolean
  isSubmitReady: boolean
}

/**
 * Single snapshot for submit: debounced pay size + matching sim result (#356).
 * Display and mutation must both consume this object when submit is allowed.
 */
export function useSubmitAlignedSimQuote<T extends { return_amount: string }>(params: {
  rawInputAmount: string
  debouncedRawInputAmount: string
  simQuery: Pick<UseQueryResult<T>, 'data' | 'isLoading' | 'isError' | 'isFetching' | 'isPlaceholderData'>
  slippageTolerance: number
  /** Extra gate (e.g. route slippage block, price impact) — merged into isSubmitReady. */
  extraSubmitBlocked?: boolean
}): SubmitAlignedSimQuote<T> {
  const { rawInputAmount, debouncedRawInputAmount, simQuery, slippageTolerance, extraSubmitBlocked = false } = params

  const isQuoteStale = isSubmitQuoteStale(
    rawInputAmount,
    debouncedRawInputAmount,
    simQuery.isPlaceholderData,
    simQuery.isFetching
  )

  const simData = simQuery.isError ? undefined : simQuery.data

  const minReceived = useMemo(
    () => (simData?.return_amount ? applySlippagePercentFloor(simData.return_amount, slippageTolerance) : null),
    [simData?.return_amount, slippageTolerance]
  )

  const isSubmitReady =
    !isQuoteStale &&
    !simQuery.isLoading &&
    !simQuery.isError &&
    !!simData &&
    debouncedRawInputAmount !== '0' &&
    !extraSubmitBlocked

  return {
    submitPayRaw: debouncedRawInputAmount,
    simData,
    minReceived,
    isQuoteStale,
    isSubmitReady,
  }
}
