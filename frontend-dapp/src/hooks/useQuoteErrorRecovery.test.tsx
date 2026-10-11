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
  error: new IndexerApiError(422, 'protection', '1000000', initialTime + 19_000, 'first'),
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
    rerender({ ...input, error: new IndexerApiError(503, 'stale_observations') })
    await tick(30_000)
    expect(getRouteSolveProgress).toHaveBeenCalledTimes(1)
    expect(input.refetch).not.toHaveBeenCalled()
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
