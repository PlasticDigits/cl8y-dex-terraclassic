import { act, renderHook } from '@testing-library/react'
import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { createRoot } from 'react-dom/client'
import { flushSync } from 'react-dom'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useSubmitAlignedSimQuote } from '@/hooks/useSubmitAlignedSimQuote'
import { useLocalQuoteExpired } from '@/hooks/useLocalQuoteExpired'
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

  it('starts a bounded refresh before original expiry without renewing cached metadata', async () => {
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
      await vi.advanceTimersByTimeAsync(999)
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

  it('keeps the ten-second refresh deadline during frequent unrelated observer renders (#1499)', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } })
    client.setQueryData(['render-churn'], quote(initialTime + 20_000), { updatedAt: initialTime })
    const fetchQuote = vi.fn(async () => quote(Date.now() + 20_000))
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    )
    const { rerender, unmount } = renderHook(
      () =>
        useQuery({
          queryKey: ['render-churn'],
          queryFn: fetchQuote,
          staleTime: localQuoteStaleTime,
          refetchInterval: localQuoteRefetchInterval,
        }),
      { initialProps: 0, wrapper }
    )
    for (let tick = 1; tick <= 99; tick++) {
      await act(async () => {
        await vi.advanceTimersByTimeAsync(100)
      })
      rerender(tick)
    }
    expect(fetchQuote).not.toHaveBeenCalled()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(101)
    })
    expect(fetchQuote).toHaveBeenCalledTimes(1)
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
      await vi.advanceTimersByTimeAsync(1_999)
    })
    expect(result.current.simData).toBe(data)
    expect(result.current.isSubmitReady).toBe(true)
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1)
    })
    expect(result.current.isQuoteStale).toBe(true)
    expect(result.current.simData).toBeUndefined()
    expect(result.current.submitPayload).toBeUndefined()
    expect(result.current.isSubmitReady).toBe(false)
    expect(data.executionMetadata.expires_at_ms).toBe(initialTime + 2_000)
    unmount()
  })

  it('commits expiry before the timer returns rather than leaving executable DOM queued for React (#1505)', () => {
    const data = quote(initialTime + 2_000)
    let expiryTimer: (() => void) | undefined
    const timer = vi.spyOn(window, 'setTimeout').mockImplementation((callback) => {
      expiryTimer = callback as () => void
      return 1
    })
    const container = document.createElement('div')
    const root = createRoot(container)
    const actEnvironment = globalThis.IS_REACT_ACT_ENVIRONMENT
    globalThis.IS_REACT_ACT_ENVIRONMENT = false
    function Snapshot() {
      const snapshot = useSubmitAlignedSimQuote({
        rawInputAmount: '100',
        debouncedRawInputAmount: '100',
        slippageTolerance: 5,
        simQuery: { data, isLoading: false, isError: false, isFetching: false, isPlaceholderData: false },
      })
      return <output>{snapshot.submitPayload ? snapshot.simData?.return_amount : 'expired'}</output>
    }
    try {
      flushSync(() => root.render(<Snapshot />))
      expect(container.textContent).toBe('100')
      expect(expiryTimer).toBeDefined()
      vi.setSystemTime(initialTime + 2_000)
      expiryTimer!()
      // No act()/frame/task flush here: the original deadline itself must revoke DOM/payload.
      expect(container.textContent).toBe('expired')
      expect(data.executionMetadata.expires_at_ms).toBe(initialTime + 2_000)
    } finally {
      flushSync(() => root.unmount())
      globalThis.IS_REACT_ACT_ENVIRONMENT = actEnvironment
      timer.mockRestore()
    }
  })

  it('keeps the original display during early refresh, expires during a slow response, then adopts only new provenance', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } })
    const original = quote(initialTime + 2_000)
    client.setQueryData(['slow-refresh'], original, { updatedAt: initialTime })
    let finish: ((value: ReturnType<typeof quote>) => void) | undefined
    const fetchQuote = vi.fn(
      () =>
        new Promise<ReturnType<typeof quote>>((resolve) => {
          finish = resolve
        })
    )
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    )
    const { result, unmount } = renderHook(
      () => {
        const query = useQuery({
          queryKey: ['slow-refresh'],
          queryFn: fetchQuote,
          staleTime: localQuoteStaleTime,
          refetchInterval: localQuoteRefetchInterval,
        })
        return useSubmitAlignedSimQuote({
          rawInputAmount: '100',
          debouncedRawInputAmount: '100',
          slippageTolerance: 5,
          simQuery: query,
        })
      },
      { wrapper }
    )
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_001)
    })
    expect(fetchQuote).toHaveBeenCalledTimes(1)
    expect(result.current.simData).toBe(original)
    expect(result.current.isSubmitReady).toBe(false)
    await act(async () => {
      await vi.advanceTimersByTimeAsync(998)
    })
    expect(result.current.simData).toBe(original)
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1)
    })
    expect(result.current.simData).toBeUndefined()
    expect(result.current.submitPayload).toBeUndefined()
    expect(fetchQuote).toHaveBeenCalledTimes(1)
    const replacement = quote(Date.now() + 20_000)
    await act(async () => {
      finish!(replacement)
      await vi.advanceTimersByTimeAsync(1)
    })
    expect(result.current.simData).toEqual(replacement)
    expect(result.current.isSubmitReady).toBe(true)
    expect(original.executionMetadata.expires_at_ms).toBe(initialTime + 2_000)
    unmount()
    client.clear()
  })

  it('rearms a browser timer that fires1ms early against the same original deadline (#1499)', async () => {
    const data = quote(initialTime + 2_000)
    const setTimer = window.setTimeout.bind(window)
    const early = vi
      .spyOn(window, 'setTimeout')
      .mockImplementationOnce((callback, delay) => setTimer(callback, Number(delay) - 1))
    const { result, unmount } = renderHook(() =>
      useSubmitAlignedSimQuote({
        rawInputAmount: '100',
        debouncedRawInputAmount: '100',
        slippageTolerance: 5,
        simQuery: { data, isLoading: false, isError: false, isFetching: false, isPlaceholderData: false },
      })
    )
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_999)
    })
    expect(result.current.simData).toBe(data)
    expect(result.current.isSubmitReady).toBe(true)
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1)
    })
    expect(result.current.simData).toBeUndefined()
    expect(result.current.isSubmitReady).toBe(false)
    expect(data.executionMetadata.expires_at_ms).toBe(initialTime + 2_000)
    unmount()
    early.mockRestore()
  })

  it('expires the receive snapshot independently when its own timer fires1ms early (#1499)', async () => {
    const data = { offerAmount: '100', quote: quote(initialTime + 2_000) }
    const setTimer = window.setTimeout.bind(window)
    const early = vi
      .spyOn(window, 'setTimeout')
      .mockImplementationOnce((callback, delay) => setTimer(callback, Number(delay) - 1))
    const { result, unmount } = renderHook(() => useLocalQuoteExpired(data))
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_999)
    })
    expect(result.current).toBe(false)
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1)
    })
    expect(result.current).toBe(true)
    expect(data.quote.executionMetadata.expires_at_ms).toBe(initialTime + 2_000)
    unmount()
    early.mockRestore()
  })

  it('bounds timer delay for malformed far-future local metadata without overflowing into a callback loop', () => {
    const timer = vi.spyOn(window, 'setTimeout')
    const data = quote(initialTime + 2_147_484_647)
    const { unmount } = renderHook(() => useLocalQuoteExpired(data))
    expect(timer).toHaveBeenCalledWith(expect.any(Function), 2_147_483_647)
    expect(data.executionMetadata.expires_at_ms).toBe(initialTime + 2_147_484_647)
    unmount()
    timer.mockRestore()
  })

  it('uses the receive quote original expiry and keeps refresh bounded without retrying known missing data', () => {
    const nested = { offerAmount: '100', quote: quote(initialTime + 2_000) }
    expect(localQuoteStaleTime({ state: { data: nested, dataUpdatedAt: initialTime } })).toBe(2_000)
    expect(localQuoteRefetchInterval({ state: { data: nested, fetchStatus: 'idle' } })).toBe(1_000)
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
