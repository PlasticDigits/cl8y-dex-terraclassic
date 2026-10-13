import { act, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { getRouteSolveProgress } from '@/services/indexer/client'
import { useRouteSolveProgress } from './useRouteSolveProgress'

vi.mock('@/services/indexer/client', () => ({ getRouteSolveProgress: vi.fn() }))
const args = () => ({
  enabled: true,
  isFetching: true,
  tokenIn: 'uluna',
  tokenOut: 'terra1output',
  amountIn: '10000000000',
  poolOnly: true,
  maxSpread: '0.03',
  maxMakerFills: 8,
  trader: 'terra1wallet',
})
const progress = { stage: 'searching', done: 0, total: 5, label: 'Searching', cache_hit: true, updated_at_ms: 1000 }
async function tick(ms = 0) {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms)
  })
}

describe('in-flight exact native progress intent #1489', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(1000)
    vi.clearAllMocks()
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('preserves original native gross intent and every solve policy parameter', async () => {
    vi.mocked(getRouteSolveProgress).mockResolvedValue(progress)
    const { result, unmount } = renderHook(useRouteSolveProgress, { initialProps: args() })
    await tick()
    expect(getRouteSolveProgress).toHaveBeenCalledWith(
      'uluna',
      'terra1output',
      '10000000000',
      expect.objectContaining({ trader: 'terra1wallet', poolOnly: true, maxSpread: '0.03', maxMakerFills: 8 })
    )
    expect(result.current.progress).toEqual(progress)
    expect(result.current.fetchStartedAtMs).toBe(1000)
    unmount()
  })

  it('clears stale advisory state and aborts old poll when identity, amount or protection changes', async () => {
    let firstSignal: AbortSignal | undefined
    vi.mocked(getRouteSolveProgress).mockImplementation((_a, _b, _c, options) => {
      firstSignal = options?.signal
      return Promise.resolve(progress)
    })
    const input = args()
    const { result, rerender, unmount } = renderHook(useRouteSolveProgress, { initialProps: input })
    await tick()
    const oldSignal = firstSignal
    await tick(500)
    rerender({ ...input, tokenIn: 'terra1wrapped', amountIn: '9800000000', poolOnly: false, maxSpread: '0.05' })
    expect(oldSignal?.aborted).toBe(true)
    expect(result.current.progress).toBeNull()
    expect(result.current.fetchStartedAtMs).toBe(1500)
    await tick()
    expect(getRouteSolveProgress).toHaveBeenLastCalledWith(
      'terra1wrapped',
      'terra1output',
      '9800000000',
      expect.objectContaining({ poolOnly: false, maxSpread: '0.05' })
    )
    rerender({ ...input, enabled: false })
    expect(result.current.progress).toBeNull()
    const count = vi.mocked(getRouteSolveProgress).mock.calls.length
    await tick(10_000)
    expect(getRouteSolveProgress).toHaveBeenCalledTimes(count)
    unmount()
  })

  it('ignores a late old-intent response and stops when quoting settles', async () => {
    let resolveOld: ((value: typeof progress) => void) | undefined
    vi.mocked(getRouteSolveProgress).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveOld = resolve
        })
    )
    const input = args()
    const { result, rerender, unmount } = renderHook(useRouteSolveProgress, { initialProps: input })
    rerender({ ...input, isFetching: false })
    await act(async () => {
      resolveOld?.(progress)
    })
    expect(result.current.progress).toBeNull()
    expect(result.current.fetchStartedAtMs).toBeNull()
    await tick(10_000)
    expect(getRouteSolveProgress).toHaveBeenCalledTimes(1)
    unmount()
  })
})
