import { afterEach, describe, expect, it, vi } from 'vitest'
import { RECEIVE_QUOTE_TIMEOUT_MS, withReceiveQuoteDeadline } from './receiveQuoteDeadline'

afterEach(() => vi.useRealTimers())

describe('finite receive quote deadline (#1376)', () => {
  it('aborts hung downstream work at the overall deadline', async () => {
    vi.useFakeTimers()
    let downstream!: AbortSignal
    const promise = withReceiveQuoteDeadline(undefined, async (signal) => {
      downstream = signal
      return new Promise<never>(() => {})
    })
    const assertion = expect(promise).rejects.toThrow('timed out')
    await vi.advanceTimersByTimeAsync(RECEIVE_QUOTE_TIMEOUT_MS)
    await assertion
    expect(downstream.aborted).toBe(true)
    expect(vi.getTimerCount()).toBe(0)
  })

  it('cancels a superseded intent immediately and clears its deadline', async () => {
    vi.useFakeTimers()
    const controller = new AbortController()
    let downstream!: AbortSignal
    const promise = withReceiveQuoteDeadline(controller.signal, async (signal) => {
      downstream = signal
      return new Promise<never>(() => {})
    })
    const assertion = expect(promise).rejects.toThrow('superseded')
    controller.abort(new Error('superseded'))
    await assertion
    expect(downstream.aborted).toBe(true)
    expect(vi.getTimerCount()).toBe(0)
  })
})
