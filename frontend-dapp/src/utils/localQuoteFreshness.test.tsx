import { act, renderHook } from '@testing-library/react'
import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useSubmitAlignedSimQuote } from '@/hooks/useSubmitAlignedSimQuote'
import { IndexerApiError } from '@/utils/routeQuoteErrors'
import { LOCAL_ROUTE_UNAVAILABLE } from '@/utils/localRouteQuote'
import { isExpiredLocalQuote, localQuoteRefetchInterval, localQuoteStaleTime } from './localQuoteFreshness'

const initialTime = 1_800_000_000_000
const quote = (expiry: number) => ({ return_amount: '100', executionMetadata: { expires_at_ms: expiry } })

describe('original local quote expiry', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(initialTime)
  })
  afterEach(() => vi.useRealTimers())

  it('fetches an expired revisited amount immediately even inside the global ten-second cache freshness', async () => {
    const client = new QueryClient({
      defaultOptions: { queries: { staleTime: 10_000, retry: false, gcTime: Infinity } },
    })
    client.setQueryData(['quote', 'old'], quote(initialTime + 2_000), { updatedAt: initialTime })
    client.setQueryData(['quote', 'current'], quote(initialTime + 8_000), { updatedAt: initialTime })
    vi.setSystemTime(initialTime + 3_000)
    const fetchQuote = vi.fn(async () => quote(Date.now() + 2_000))
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    )
    const { result, rerender, unmount } = renderHook(
      (amount: string) =>
        useQuery({ queryKey: ['quote', amount], queryFn: fetchQuote, staleTime: localQuoteStaleTime }),
      { initialProps: 'current', wrapper }
    )
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1)
    })
    expect(fetchQuote).not.toHaveBeenCalled()
    rerender('old')
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1)
    })
    expect(fetchQuote).toHaveBeenCalledTimes(1)
    expect(isExpiredLocalQuote(result.current.data)).toBe(false)
    unmount()
    client.clear()
  })

  it('refreshes active local arithmetic at original expiry without renewing cached metadata', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } })
    const original = quote(initialTime + 2_000)
    client.setQueryData(['active'], original, { updatedAt: initialTime })
    const fetchQuote = vi.fn(async () => quote(Date.now() + 2_000))
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    )
    const { result, unmount } = renderHook(
      () =>
        useQuery({
          queryKey: ['active'],
          queryFn: fetchQuote,
          staleTime: localQuoteStaleTime,
          refetchInterval: localQuoteRefetchInterval,
        }),
      { wrapper }
    )
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_999)
    })
    expect(fetchQuote).not.toHaveBeenCalled()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(2)
    })
    expect(fetchQuote).toHaveBeenCalledTimes(1)
    expect(isExpiredLocalQuote(result.current.data)).toBe(false)
    expect(original.executionMetadata.expires_at_ms).toBe(initialTime + 2_000)
    unmount()
    client.clear()
  })

  it.each(['Failed to fetch', LOCAL_ROUTE_UNAVAILABLE])(
    'does not accelerate failed refreshes after expiry: %s',
    async (message) => {
      const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } })
      client.setQueryData(['failed-refresh'], quote(initialTime + 2_000), { updatedAt: initialTime })
      const fetchQuote = vi.fn(async () => {
        throw new Error(message)
      })
      const wrapper = ({ children }: { children: ReactNode }) => (
        <QueryClientProvider client={client}>{children}</QueryClientProvider>
      )
      const { result, unmount } = renderHook(
        () =>
          useQuery({
            queryKey: ['failed-refresh'],
            queryFn: fetchQuote,
            staleTime: localQuoteStaleTime,
            refetchInterval: localQuoteRefetchInterval,
          }),
        { wrapper }
      )
      await act(async () => {
        await vi.advanceTimersByTimeAsync(2_001)
      })
      expect(fetchQuote).toHaveBeenCalledTimes(1)
      expect(result.current.isError).toBe(true)
      expect(isExpiredLocalQuote(result.current.data)).toBe(true)
      await act(async () => {
        await vi.advanceTimersByTimeAsync(8_000)
      })
      expect(fetchQuote).toHaveBeenCalledTimes(1)
      await act(async () => {
        await vi.advanceTimersByTimeAsync(2_000)
      })
      expect(fetchQuote).toHaveBeenCalledTimes(2)
      unmount()
      client.clear()
    }
  )

  it('removes the active expired quote and its signing payload at the original deadline', async () => {
    const data = quote(initialTime + 2_000)
    const { result, unmount } = renderHook(() =>
      useSubmitAlignedSimQuote({
        rawInputAmount: '100',
        debouncedRawInputAmount: '100',
        slippageTolerance: 5,
        simQuery: { data, isLoading: false, isError: false, isFetching: false, isPlaceholderData: false },
      })
    )
    expect(result.current.isSubmitReady).toBe(true)
    await act(async () => {
      await vi.advanceTimersByTimeAsync(2_001)
    })
    expect(result.current.isQuoteStale).toBe(true)
    expect(result.current.simData).toBeUndefined()
    expect(result.current.submitPayload).toBeUndefined()
    expect(result.current.isSubmitReady).toBe(false)
    expect(data.executionMetadata.expires_at_ms).toBe(initialTime + 2_000)
    unmount()
  })

  it('uses the receive quote original expiry and keeps refresh bounded without retrying known missing data', () => {
    const nested = { offerAmount: '100', quote: quote(initialTime + 2_000) }
    expect(localQuoteStaleTime({ state: { data: nested, dataUpdatedAt: initialTime } })).toBe(2_000)
    expect(localQuoteRefetchInterval({ state: { data: nested, fetchStatus: 'idle' } })).toBe(2_000)
    vi.setSystemTime(initialTime + 1_999)
    expect(localQuoteRefetchInterval({ state: { data: nested, fetchStatus: 'idle' } })).toBe(1_000)
    expect(localQuoteRefetchInterval({ state: { data: nested, fetchStatus: 'fetching' } })).toBe(false)
    expect(
      localQuoteRefetchInterval({
        state: { data: nested, fetchStatus: 'idle', error: new IndexerApiError(503, 'missing_observations') },
      })
    ).toBe(false)
    expect(isExpiredLocalQuote(nested, initialTime + 2_000)).toBe(true)
    expect(isExpiredLocalQuote({ executionMetadata: { expires_at_ms: 'future' } })).toBe(true)
  })
})
