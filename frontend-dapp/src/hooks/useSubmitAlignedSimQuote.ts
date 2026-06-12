import { useMemo } from 'react'
import type { UseQueryResult } from '@tanstack/react-query'
import { applySlippagePercentFloor } from '@/utils/rawAmountMath'
import {
  buildSubmitAlignedSimPayload,
  isSubmitQuoteStale,
  type SubmitAlignedSimPayload,
  type SubmitHybridSnapshot,
} from '@/utils/quoteDebounce'

export type SubmitAlignedSimQuote<T extends { return_amount: string }> = {
  /** Raw pay amount aligned with the debounced sim query key — use for on-chain submit. */
  submitPayRaw: string
  simData: T | undefined
  minReceived: string | null
  submitPayload: SubmitAlignedSimPayload<T> | undefined
  /** Snapshotted hybrid book leg + max makers aligned with the debounced sim key (#360). */
  snapshottedHybrid: SubmitHybridSnapshot | undefined
  isQuoteStale: boolean
  isSubmitReady: boolean
}

/**
 * Single snapshot for submit: debounced pay size + matching sim result (#356, #360).
 * Display and mutation must both consume this object when submit is allowed.
 */
export function useSubmitAlignedSimQuote<T extends { return_amount: string; indexerOperations?: unknown }>(params: {
  rawInputAmount: string
  debouncedRawInputAmount: string
  simQuery: Pick<UseQueryResult<T>, 'data' | 'isLoading' | 'isError' | 'isFetching' | 'isPlaceholderData'>
  slippageTolerance: number
  /** Extra gate (e.g. route slippage block, price impact) — merged into isSubmitReady. */
  extraSubmitBlocked?: boolean
  hybrid?: {
    enabled: boolean
    live: SubmitHybridSnapshot
    snapshotted: SubmitHybridSnapshot
  }
}): SubmitAlignedSimQuote<T> {
  const {
    rawInputAmount,
    debouncedRawInputAmount,
    simQuery,
    slippageTolerance,
    extraSubmitBlocked = false,
    hybrid,
  } = params

  const isQuoteStale = isSubmitQuoteStale(
    rawInputAmount,
    debouncedRawInputAmount,
    simQuery.isPlaceholderData,
    simQuery.isFetching,
    hybrid
  )

  const simData = simQuery.isError ? undefined : simQuery.data

  const submitPayload = useMemo(
    () =>
      simData
        ? buildSubmitAlignedSimPayload(debouncedRawInputAmount, simData, slippageTolerance, applySlippagePercentFloor)
        : undefined,
    [simData, debouncedRawInputAmount, slippageTolerance]
  )

  const minReceived = submitPayload?.minReceived ?? null

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
    submitPayload,
    snapshottedHybrid: hybrid?.enabled ? hybrid.snapshotted : undefined,
    isQuoteStale,
    isSubmitReady,
  }
}
