import { describe, it, expect } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useSubmitAlignedSimQuote } from './useSubmitAlignedSimQuote'

describe('useSubmitAlignedSimQuote', () => {
  const baseSimData = { return_amount: '990000' }

  it('marks quote stale while fetching for the debounced key (#356)', () => {
    const { result } = renderHook(() =>
      useSubmitAlignedSimQuote({
        rawInputAmount: '1000000',
        debouncedRawInputAmount: '1000000',
        simQuery: {
          data: baseSimData,
          isLoading: false,
          isError: false,
          isFetching: true,
          isPlaceholderData: false,
        },
        slippageTolerance: 1,
      })
    )
    expect(result.current.isQuoteStale).toBe(true)
    expect(result.current.isSubmitReady).toBe(false)
  })

  it('is submit-ready when amounts match and quote settled', () => {
    const { result } = renderHook(() =>
      useSubmitAlignedSimQuote({
        rawInputAmount: '1000000',
        debouncedRawInputAmount: '1000000',
        simQuery: {
          data: baseSimData,
          isLoading: false,
          isError: false,
          isFetching: false,
          isPlaceholderData: false,
        },
        slippageTolerance: 1,
      })
    )
    expect(result.current.isQuoteStale).toBe(false)
    expect(result.current.isSubmitReady).toBe(true)
    expect(result.current.submitPayRaw).toBe('1000000')
    expect(result.current.minReceived).toBeTruthy()
  })

  it('blocks submit when live raw differs from debounced key', () => {
    const { result } = renderHook(() =>
      useSubmitAlignedSimQuote({
        rawInputAmount: '10000000',
        debouncedRawInputAmount: '1000000',
        simQuery: {
          data: baseSimData,
          isLoading: false,
          isError: false,
          isFetching: false,
          isPlaceholderData: false,
        },
        slippageTolerance: 1,
      })
    )
    expect(result.current.isSubmitReady).toBe(false)
  })

  it('blocks submit when live book leg differs from snapshotted hybrid (#360)', () => {
    const { result } = renderHook(() =>
      useSubmitAlignedSimQuote({
        rawInputAmount: '1000000',
        debouncedRawInputAmount: '1000000',
        simQuery: {
          data: baseSimData,
          isLoading: false,
          isError: false,
          isFetching: false,
          isPlaceholderData: false,
        },
        slippageTolerance: 1,
        hybrid: {
          enabled: true,
          live: { bookInputHuman: '5', hybridMaxMakers: 8 },
          snapshotted: { bookInputHuman: '2', hybridMaxMakers: 8 },
        },
      })
    )
    expect(result.current.isQuoteStale).toBe(true)
    expect(result.current.isSubmitReady).toBe(false)
    expect(result.current.snapshottedHybrid).toEqual({ bookInputHuman: '2', hybridMaxMakers: 8 })
  })

  it('builds submit payload via buildSubmitAlignedSimPayload (#360)', () => {
    const { result } = renderHook(() =>
      useSubmitAlignedSimQuote({
        rawInputAmount: '1000000',
        debouncedRawInputAmount: '1000000',
        simQuery: {
          data: { return_amount: '990000', indexerOperations: [{ terra_swap: {} }] },
          isLoading: false,
          isError: false,
          isFetching: false,
          isPlaceholderData: false,
        },
        slippageTolerance: 1,
      })
    )
    expect(result.current.submitPayload?.payRaw).toBe('1000000')
    expect(result.current.submitPayload?.minReceived).toBe(result.current.minReceived)
    expect(result.current.submitPayload?.indexerOperations).toEqual([{ terra_swap: {} }])
  })
})
