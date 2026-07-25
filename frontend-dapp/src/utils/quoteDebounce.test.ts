import { describe, it, expect } from 'vitest'
import {
  assertSubmitHybridAligned,
  assertSubmitQuotePayRawAligned,
  buildSubmitAlignedSimPayload,
  isSubmitQuoteStale,
  isSimQuoteStaleForSubmit,
  shouldShowSimReceiveCalculating,
  simQuoteRefetchInterval,
  SIM_QUOTE_REFETCH_INTERVAL_MS,
} from './quoteDebounce'

describe('simQuoteRefetchInterval (#484)', () => {
  it('returns false while a fetch is in flight so overlapping cancel/restart cannot pile up', () => {
    expect(simQuoteRefetchInterval({ state: { fetchStatus: 'fetching' } })).toBe(false)
  })

  it('returns the standard interval when idle or paused', () => {
    expect(simQuoteRefetchInterval({ state: { fetchStatus: 'idle' } })).toBe(SIM_QUOTE_REFETCH_INTERVAL_MS)
    expect(simQuoteRefetchInterval({ state: { fetchStatus: 'paused' } })).toBe(SIM_QUOTE_REFETCH_INTERVAL_MS)
  })
})

describe('shouldShowSimReceiveCalculating (#484 / #496)', () => {
  it('shows Calculating only when fetching and no settled quote exists (#484)', () => {
    expect(shouldShowSimReceiveCalculating(true, false)).toBe(true)
    expect(shouldShowSimReceiveCalculating(true, true)).toBe(false)
    expect(shouldShowSimReceiveCalculating(false, false)).toBe(false)
    expect(shouldShowSimReceiveCalculating(false, true)).toBe(false)
  })

  it('keeps prior amount during same-input background refetch (#484)', () => {
    expect(shouldShowSimReceiveCalculating(true, true, false, false)).toBe(false)
  })

  it('shows Calculating when keepPreviousData placeholder is for a prior query key (#496)', () => {
    expect(shouldShowSimReceiveCalculating(true, false, true, false)).toBe(true)
    // Even if caller passed a truthy "has data" flag, placeholder means prior-key stale.
    expect(shouldShowSimReceiveCalculating(true, true, true, false)).toBe(true)
    expect(shouldShowSimReceiveCalculating(false, true, true, false)).toBe(true)
  })

  it('shows Calculating while typed pay raw differs from debounced sim key (#496)', () => {
    expect(shouldShowSimReceiveCalculating(false, true, false, true)).toBe(true)
    expect(shouldShowSimReceiveCalculating(true, true, false, true)).toBe(true)
  })
})

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

  it('is stale when live book leg differs from snapshotted book while pay is stable (#360)', () => {
    expect(
      isSubmitQuoteStale('100', '100', false, false, {
        enabled: true,
        live: { bookInputHuman: '5', hybridMaxMakers: 8 },
        snapshotted: { bookInputHuman: '2', hybridMaxMakers: 8 },
      })
    ).toBe(true)
  })

  it('is stale when live max makers differs from snapshotted while pay and book are stable (#360)', () => {
    expect(
      isSubmitQuoteStale('100', '100', false, false, {
        enabled: true,
        live: { bookInputHuman: '2', hybridMaxMakers: 4 },
        snapshotted: { bookInputHuman: '2', hybridMaxMakers: 8 },
      })
    ).toBe(true)
  })

  it('is fresh when hybrid snapshotted fields match live (#360)', () => {
    expect(
      isSubmitQuoteStale('100', '100', false, false, {
        enabled: true,
        live: { bookInputHuman: '2', hybridMaxMakers: 8 },
        snapshotted: { bookInputHuman: '2', hybridMaxMakers: 8 },
      })
    ).toBe(false)
  })
})

describe('isSimQuoteStaleForSubmit (legacy alias)', () => {
  it('defaults isFetching to false for backward compatibility', () => {
    expect(isSimQuoteStaleForSubmit('100', '100', false)).toBe(false)
    expect(isSimQuoteStaleForSubmit('100', '100', false, true)).toBe(true)
  })
})

describe('assertSubmitHybridAligned', () => {
  it('passes when book leg and max makers match snapshotted values', () => {
    expect(() =>
      assertSubmitHybridAligned(
        { bookInputHuman: '2', hybridMaxMakers: 8 },
        { bookInputHuman: '2', hybridMaxMakers: 8 }
      )
    ).not.toThrow()
  })

  it('throws when book leg differs from snapshotted values', () => {
    expect(() =>
      assertSubmitHybridAligned(
        { bookInputHuman: '5', hybridMaxMakers: 8 },
        { bookInputHuman: '2', hybridMaxMakers: 8 }
      )
    ).toThrow(/still updating/i)
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
