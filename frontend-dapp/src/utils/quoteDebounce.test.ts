import { describe, it, expect } from 'vitest'
import {
  assertSubmitQuotePayRawAligned,
  buildSubmitAlignedSimPayload,
  isSubmitQuoteStale,
  isSimQuoteStaleForSubmit,
} from './quoteDebounce'

describe('isSubmitQuoteStale', () => {
  it('is stale when live raw differs from debounced key', () => {
    expect(isSubmitQuoteStale('1000', '100', false, false)).toBe(true)
  })

  it('is stale when placeholder data is shown', () => {
    expect(isSubmitQuoteStale('100', '100', true, false)).toBe(true)
  })

  it('is stale while fetching for the active debounced key (#356)', () => {
    expect(isSubmitQuoteStale('100', '100', false, true)).toBe(true)
  })

  it('is fresh when amounts match, not placeholder, and not fetching', () => {
    expect(isSubmitQuoteStale('100', '100', false, false)).toBe(false)
  })
})

describe('isSimQuoteStaleForSubmit (legacy alias)', () => {
  it('defaults isFetching to false for backward compatibility', () => {
    expect(isSimQuoteStaleForSubmit('100', '100', false)).toBe(false)
    expect(isSimQuoteStaleForSubmit('100', '100', false, true)).toBe(true)
  })
})

describe('assertSubmitQuotePayRawAligned', () => {
  it('passes when pay raw matches debounced key', () => {
    expect(() => assertSubmitQuotePayRawAligned('100', '100')).not.toThrow()
  })

  it('throws when pay raw differs from debounced key', () => {
    expect(() => assertSubmitQuotePayRawAligned('1000', '100')).toThrow(/still updating/i)
  })
})

describe('buildSubmitAlignedSimPayload', () => {
  it('bundles pay raw, min received, and indexer ops from one sim snapshot', () => {
    const simData = {
      return_amount: '990000',
      indexerOperations: [{ terra_swap: {} }],
    }
    const payload = buildSubmitAlignedSimPayload('1000000', simData, 1, (amount, pct) => `${amount}-floor-${pct}`)
    expect(payload.payRaw).toBe('1000000')
    expect(payload.minReceived).toBe('990000-floor-1')
    expect(payload.simData).toBe(simData)
    expect(payload.indexerOperations).toBe(simData.indexerOperations)
  })
})
