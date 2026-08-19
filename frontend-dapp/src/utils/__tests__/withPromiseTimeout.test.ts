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

  it('ignores a late resolve after timeout (GitLab #567)', async () => {
    vi.useFakeTimers()
    let resolveLate!: (value: number) => void
    const pending = new Promise<number>((resolve) => {
      resolveLate = resolve
    })
    const wrapped = withPromiseTimeout(pending, 1_000, 'timed out')
    const assertion = expect(wrapped).rejects.toThrow('timed out')
    await vi.advanceTimersByTimeAsync(1_000)
    await assertion
    resolveLate(99)
    await Promise.resolve()
    await expect(wrapped).rejects.toThrow('timed out')
    vi.useRealTimers()
  })
})
