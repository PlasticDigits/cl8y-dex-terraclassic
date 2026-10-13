import { act, renderHook } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { getRouteSolveProgress } from '@/services/indexer/client'
import { localQuote } from '@/test/localRouteFixtures'
import { useRouteQuotePromotion } from './useRouteQuotePromotion'

vi.mock('@/services/indexer/client', () => ({ getRouteSolveProgress: vi.fn() }))
const initialTime = 1_800_000_000_000
const progress = (best = '1000000', generation = 'fixture1', complete = false) => ({
  stage: 'deeper',
  done: 1,
  total: 5,
  label: 'Searching',
  cache_hit: true,
  updated_at_ms: Date.now(),
  quote_generation: generation,
  best_amount_out: best,
  search_complete: complete,
})
function args() {
  const q = localQuote()
  return {
    enabled: true,
    intentKey: 'native:10000000000',
    tokenIn: 'uluna',
    tokenOut: q.token_out,
    amount: '10000000000',
    poolOnly: true,
    maxMakerFills: 8,
    maxSpread: '0.03',
    quote: { return_amount: q.estimated_amount_out!, executionMetadata: q.execution_metadata! },
    refetch: vi.fn(),
  }
}
function setup() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } })
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  )
  return { client, wrapper }
}
async function tick(ms = 1) {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms)
  })
}

describe('bounded exact-intent promotion observation #1489', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(initialTime)
    vi.clearAllMocks()
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('polls native gross identity and actual selected mode/protection; adopts only higher same-cohort publication', async () => {
    const input = args()
    let publication = progress()
    vi.mocked(getRouteSolveProgress).mockImplementation(async () => publication)
    const { client, wrapper } = setup()
    const { rerender, unmount } = renderHook(useRouteQuotePromotion, { initialProps: input, wrapper })
    await tick()
    expect(getRouteSolveProgress).toHaveBeenLastCalledWith(
      'uluna',
      input.tokenOut,
      '10000000000',
      expect.objectContaining({ poolOnly: true, maxSpread: '0.03', maxMakerFills: 8 })
    )
    expect(input.refetch).not.toHaveBeenCalled()
    publication = progress('1334678')
    await tick(1000)
    expect(input.refetch).toHaveBeenCalledTimes(1)
    await tick(2000)
    expect(input.refetch).toHaveBeenCalledTimes(1)
    rerender({ ...input, quote: { ...input.quote, return_amount: '1334678' } })
    await tick(1000)
    expect(input.refetch).toHaveBeenCalledTimes(1)
    publication = progress('1400000', 'fixture1', true)
    await tick(1000)
    expect(input.refetch).toHaveBeenCalledTimes(2)
    const polls = vi.mocked(getRouteSolveProgress).mock.calls.length
    await tick(3000)
    expect(getRouteSolveProgress).toHaveBeenCalledTimes(polls)
    unmount()
    client.clear()
  })

  it.each(['999999', '0', '-1', '1.2', '1e6', '9'.repeat(40)])(
    'ignores lower or malformed publication %s',
    async (best) => {
      const input = args()
      vi.mocked(getRouteSolveProgress).mockResolvedValue(progress(best))
      const { client, wrapper } = setup()
      const { unmount } = renderHook(useRouteQuotePromotion, { initialProps: input, wrapper })
      await tick()
      expect(input.refetch).not.toHaveBeenCalled()
      unmount()
      client.clear()
    }
  )

  it('never promotes a changed generation or renews original expiry', async () => {
    const input = args()
    vi.mocked(getRouteSolveProgress).mockResolvedValue(progress('2000000', 'changed'))
    const { client, wrapper } = setup()
    const { unmount } = renderHook(useRouteQuotePromotion, { initialProps: input, wrapper })
    await tick(11_000)
    expect(input.refetch).not.toHaveBeenCalled()
    const polls = vi.mocked(getRouteSolveProgress).mock.calls.length
    await tick(20_000)
    expect(getRouteSolveProgress).toHaveBeenCalledTimes(polls)
    unmount()
    client.clear()
  })

  it('stops and aborts immediately for review/signing, declared mode or obsolete intent', async () => {
    const input = args()
    let signal: AbortSignal | undefined
    vi.mocked(getRouteSolveProgress).mockImplementation((_a, _b, _c, options) => {
      signal = options?.signal
      return new Promise(() => {})
    })
    const { client, wrapper } = setup()
    const { rerender, unmount } = renderHook(useRouteQuotePromotion, { initialProps: input, wrapper })
    await tick()
    rerender({ ...input, enabled: false, intentKey: 'edited', amount: '20000000000' })
    expect(signal?.aborted).toBe(true)
    await tick(10_000)
    expect(getRouteSolveProgress).toHaveBeenCalledTimes(1)
    expect(input.refetch).not.toHaveBeenCalled()
    unmount()
    client.clear()
  })

  it('ends bounded observation after a failure without treating progress as executable output', async () => {
    const input = args()
    vi.mocked(getRouteSolveProgress).mockRejectedValue(new Error('offline'))
    const { client, wrapper } = setup()
    const { unmount } = renderHook(useRouteQuotePromotion, { initialProps: input, wrapper })
    await tick(5000)
    expect(getRouteSolveProgress).toHaveBeenCalledTimes(1)
    expect(input.refetch).not.toHaveBeenCalled()
    unmount()
    client.clear()
  })
})
