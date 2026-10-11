import { act, renderHook } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { getRouteSolveProgress } from '@/services/indexer/client'
import { IndexerApiError } from '@/utils/routeQuoteErrors'
import { useQuoteErrorRecovery } from './useQuoteErrorRecovery'

vi.mock('@/services/indexer/client', () => ({ getRouteSolveProgress: vi.fn() }))

const initialTime = 1_800_000_000_000
function setup() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } })
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  )
  return { client, wrapper }
}
const args = () => ({
  error: new IndexerApiError(422, 'protection', '1000000', initialTime + 19_000, 'first') as unknown,
  currentIntent: true,
  tokenIn: 'terra1from',
  tokenOut: 'terra1to',
  amount: '100000000',
  maxMakerFills: 8,
  maxSpread: '0.05',
  refetch: vi.fn(),
})
const progress = (generation: string) => ({
  stage: 'idle',
  done: 1,
  total: 1,
  label: 'Local route ready',
  cache_hit: false,
  updated_at_ms: Date.now(),
  quote_generation: generation,
})
async function tick(ms = 0) {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms)
  })
}

describe('rejected quote observation recovery (#1473)', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(initialTime)
    vi.clearAllMocks()
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('does not repeatedly solve deterministic errors and refreshes once for a new published generation', async () => {
    const input = args()
    let generation = 'first'
    vi.mocked(getRouteSolveProgress).mockImplementation(async () => progress(generation))
    const { client, wrapper } = setup()
    const { result, rerender, unmount } = renderHook((value) => useQuoteErrorRecovery(value), {
      initialProps: input,
      wrapper,
    })
    await tick()
    expect(result.current).toBe('1000000')
    await tick(10_000)
    expect(getRouteSolveProgress).toHaveBeenCalledTimes(2)
    expect(input.refetch).not.toHaveBeenCalled()
    generation = 'second'
    await tick(10_000)
    expect(input.refetch).toHaveBeenCalledTimes(1)
    expect(result.current).toBeUndefined() // Original estimate expired; progress does not renew it.
    await tick(10_000)
    expect(input.refetch).toHaveBeenCalledTimes(1)
    rerender({ ...input, error: new IndexerApiError(422, 'protection', undefined, undefined, 'second') })
    await tick(10_000)
    expect(input.refetch).toHaveBeenCalledTimes(1)
    unmount()
    client.clear()
  })

  it('clears estimate and cancels observation watching immediately when intent changes', async () => {
    const input = args()
    vi.mocked(getRouteSolveProgress).mockResolvedValue(progress('first'))
    const { client, wrapper } = setup()
    const { result, rerender, unmount } = renderHook((value) => useQuoteErrorRecovery(value), {
      initialProps: input,
      wrapper,
    })
    await tick()
    expect(result.current).toBe('1000000')
    rerender({ ...input, currentIntent: false, amount: '10000000' })
    expect(result.current).toBeUndefined()
    await tick(30_000)
    expect(getRouteSolveProgress).toHaveBeenCalledTimes(1)
    expect(input.refetch).not.toHaveBeenCalled()
    unmount()
    client.clear()
  })

  it('does not treat unknown/transport errors or malformed progress as a generation recovery signal', async () => {
    const input = args()
    vi.mocked(getRouteSolveProgress).mockResolvedValue(progress('private.example/actor'))
    const { client, wrapper } = setup()
    const { rerender, unmount } = renderHook((value) => useQuoteErrorRecovery(value), { initialProps: input, wrapper })
    await tick()
    expect(input.refetch).not.toHaveBeenCalled()
    rerender({ ...input, error: new Error('Failed to fetch') })
    await tick(30_000)
    expect(getRouteSolveProgress).toHaveBeenCalledTimes(1)
    expect(input.refetch).not.toHaveBeenCalled()
    unmount()
    client.clear()
  })
  it.each(['missing_observations', 'stale_observations'] as const)(
    'watches %s without an error generation and solves once for each valid publication',
    async (reason) => {
      const input = { ...args(), error: new IndexerApiError(503, reason) }
      let generation = 'first'
      vi.mocked(getRouteSolveProgress).mockImplementation(async () => progress(generation))
      const { client, wrapper } = setup()
      const { unmount } = renderHook(() => useQuoteErrorRecovery(input), { wrapper })
      await tick(1)
      expect(input.refetch).toHaveBeenCalledTimes(1)
      await tick(40_000)
      expect(input.refetch).toHaveBeenCalledTimes(1)
      generation = 'second'
      await tick(10_000)
      expect(input.refetch).toHaveBeenCalledTimes(2)
      generation = 'first'
      await tick(10_000)
      expect(input.refetch).toHaveBeenCalledTimes(2)
      unmount()
      client.clear()
    }
  )
  it('ignores and aborts late progress for a cancelled missing-price intent', async () => {
    const input = { ...args(), error: new IndexerApiError(503, 'missing_observations') }
    let finish!: (value: ReturnType<typeof progress>) => void
    let signal: AbortSignal | undefined
    vi.mocked(getRouteSolveProgress).mockImplementation((_a, _b, _c, options) => {
      signal = options?.signal
      return new Promise((resolve) => {
        finish = resolve
      })
    })
    const { client, wrapper } = setup()
    const { rerender, unmount } = renderHook(useQuoteErrorRecovery, { initialProps: input, wrapper })
    await tick()
    rerender({ ...input, currentIntent: false, amount: '2' })
    expect(signal?.aborted).toBe(true)
    finish(progress('first'))
    await tick(30_000)
    expect(input.refetch).not.toHaveBeenCalled()
    expect(getRouteSolveProgress).toHaveBeenCalledTimes(1)
    unmount()
    client.clear()
  })
  it('bounds recent generation suppression while never repeating unchanged current publications', async () => {
    const input = { ...args(), error: new IndexerApiError(503, 'missing_observations') }
    let generation = 'publication-0'
    vi.mocked(getRouteSolveProgress).mockImplementation(async () => progress(generation))
    const { client, wrapper } = setup()
    const { unmount } = renderHook(() => useQuoteErrorRecovery(input), { wrapper })
    await tick(1)
    for (let i = 1; i <= 65; i++) {
      generation = `publication-${i}`
      await tick(10_000)
    }
    expect(input.refetch).toHaveBeenCalledTimes(66)
    await tick(30_000)
    expect(input.refetch).toHaveBeenCalledTimes(66)
    generation = 'publication-2' // Still in the64-publication history.
    await tick(10_000)
    expect(input.refetch).toHaveBeenCalledTimes(66)
    generation = 'publication-0' // Evicted after64 newer publications; this is a changed signal.
    await tick(10_000)
    expect(input.refetch).toHaveBeenCalledTimes(67)
    await tick(30_000)
    expect(input.refetch).toHaveBeenCalledTimes(67)
    unmount()
    client.clear()
  })
  it('resets recovery ownership through a successful intermediate UI intent, including return toA', async () => {
    const input = { ...args(), error: new IndexerApiError(503, 'missing_observations') as unknown, intentKey: 'A' }
    vi.mocked(getRouteSolveProgress).mockResolvedValue(progress('publication'))
    const { client, wrapper } = setup()
    const { rerender, unmount } = renderHook(useQuoteErrorRecovery, { initialProps: input, wrapper })
    await tick(1)
    expect(input.refetch).toHaveBeenCalledTimes(1)
    rerender({ ...input, intentKey: 'B', error: null })
    await tick(1)
    rerender(input)
    await tick(1)
    expect(input.refetch).toHaveBeenCalledTimes(2)
    await tick(30_000)
    expect(input.refetch).toHaveBeenCalledTimes(2)
    unmount()
    client.clear()
  })
  it.each(['missing_observations', 'stale_observations', 'accounting_unavailable'] as const)(
    'recovers typed503%s only after a fresh generation',
    async (reason) => {
      const input = {
        ...args(),
        error: new IndexerApiError(503, reason, undefined, undefined, 'first'),
        exactOutput: true,
      }
      let generation = 'first'
      vi.mocked(getRouteSolveProgress).mockImplementation(async () => progress(generation))
      const { client, wrapper } = setup()
      const { unmount } = renderHook(() => useQuoteErrorRecovery(input), { wrapper })
      await tick()
      await tick(10_000)
      expect(input.refetch).not.toHaveBeenCalled()
      generation = 'second'
      await tick(10_000)
      await tick(1)
      expect(input.refetch).toHaveBeenCalledTimes(1)
      expect(getRouteSolveProgress).toHaveBeenLastCalledWith(
        'terra1from',
        'terra1to',
        '100000000',
        expect.objectContaining({ exactOutput: true })
      )
      await tick(10_000)
      expect(input.refetch).toHaveBeenCalledTimes(1)
      unmount()
      client.clear()
    }
  )
})
