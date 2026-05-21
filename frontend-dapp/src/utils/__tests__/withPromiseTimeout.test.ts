import { describe, it, expect, vi } from 'vitest'
import { withPromiseTimeout } from '../withPromiseTimeout'

describe('withPromiseTimeout', () => {
  it('resolves when the promise settles before the deadline', async () => {
    await expect(withPromiseTimeout(Promise.resolve(42), 1000, 'timed out')).resolves.toBe(42)
  })

  it('rejects with the timeout message when the promise does not settle in time', async () => {
    vi.useFakeTimers()
    const pending = new Promise<number>(() => {})
    const wrapped = withPromiseTimeout(
      pending,
      30_000,
      'Could not broadcast the transaction. Check your connection and try again.'
    )
    const assertion = expect(wrapped).rejects.toThrow(
      'Could not broadcast the transaction. Check your connection and try again.'
    )
    await vi.advanceTimersByTimeAsync(30_000)
    await assertion
    vi.useRealTimers()
  })
})
