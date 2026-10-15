import { renderHook } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { IndexerApiError } from '@/utils/routeQuoteErrors'
import { useQuotePricingDataPending } from './useQuotePricingDataPending'

const initial = {
  intentKey: 'UST1->KENA:1',
  currentIntent: true,
  error: new IndexerApiError(503, 'missing_observations') as unknown,
  hasValidatedQuote: false,
}

describe('pricing-data wait ownership', () => {
  it('retains the wait while rechecking and clears only when a usable quote arrives', () => {
    const { result, rerender } = renderHook(useQuotePricingDataPending, { initialProps: initial })
    expect(result.current).toBe(true)
    rerender({ ...initial, error: null })
    expect(result.current).toBe(true)
    rerender({ ...initial, error: null, hasValidatedQuote: true })
    expect(result.current).toBe(false)
  })
  it('clears waiting immediately on amount/side intent changes or cancellation', () => {
    const { result, rerender } = renderHook(useQuotePricingDataPending, { initialProps: initial })
    rerender({ ...initial, intentKey: 'KENA->UST1:1', error: null })
    expect(result.current).toBe(false)
    rerender(initial)
    expect(result.current).toBe(true)
    rerender({ ...initial, currentIntent: false })
    expect(result.current).toBe(false)
  })
  it.each(['missing_observations', 'stale_observations'] as const)(
    'does not replace a matching unexpired validated display during %s refresh',
    (reason) => {
      const { result, rerender } = renderHook(useQuotePricingDataPending, {
        initialProps: { ...initial, hasValidatedQuote: true, error: new IndexerApiError(503, reason) },
      })
      expect(result.current).toBe(false)
      rerender({ ...initial, hasValidatedQuote: false, error: new IndexerApiError(503, reason) })
      expect(result.current).toBe(true)
    }
  )
  it.each(['amount_too_small', 'protection', 'unsupported_semantics'] as const)(
    'a real %s blocker supersedes the wait, including expired cached data',
    (reason) => {
      const input = { ...initial, expired: true }
      const { result, rerender } = renderHook(useQuotePricingDataPending, { initialProps: input })
      rerender({ ...input, error: new IndexerApiError(422, reason) })
      expect(result.current).toBe(false)
    }
  )
})
