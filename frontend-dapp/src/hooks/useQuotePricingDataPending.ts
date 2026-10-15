import { useState } from 'react'
import { isIndexerPricingDataPending } from '@/utils/indexerErrors'

/** Retain a data wait during its recheck, but never carry it across intents or real blockers. */
export function useQuotePricingDataPending(input: {
  intentKey: string
  currentIntent: boolean
  error: unknown
  hasValidatedQuote: boolean
  expired?: boolean
}): boolean {
  const [previous, setPrevious] = useState({ intentKey: input.intentKey, waiting: false })
  const waiting =
    input.currentIntent &&
    !input.hasValidatedQuote &&
    (isIndexerPricingDataPending(input.error) ||
      (!input.error &&
        (input.expired === true ||
          (!input.hasValidatedQuote && previous.intentKey === input.intentKey && previous.waiting))))
  // Adjust this component's state synchronously so an intent change cannot render
  // one stale waiting frame; no timer or query is driven by this presentation hook.
  if (previous.waiting !== waiting || (waiting && previous.intentKey !== input.intentKey)) {
    setPrevious({ intentKey: input.intentKey, waiting })
  }
  return waiting
}
