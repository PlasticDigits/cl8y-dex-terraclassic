import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  raceWithAbortAndTimeout,
  WALLETCONNECT_CANCELLED_MESSAGE,
  WALLETCONNECT_TIMEOUT_MESSAGE,
} from '../walletConnectSession'

describe('raceWithAbortAndTimeout (GitLab #554)', () => {
  afterEach(() => {
    vi.clearAllTimers()
    vi.useRealTimers()
  })

  it('rejects with timeout and calls onTimeout', async () => {
    vi.useFakeTimers()
    const onTimeout = vi.fn()
    const assertion = expect(
      raceWithAbortAndTimeout(new Promise(() => undefined), {
        timeoutMs: 50,
        signal: new AbortController().signal,
        onTimeout,
      })
    ).rejects.toThrow(WALLETCONNECT_TIMEOUT_MESSAGE)
    await vi.advanceTimersByTimeAsync(50)
    await assertion
    expect(onTimeout).toHaveBeenCalledTimes(1)
  })

  it('rejects with cancelled when aborted', async () => {
    const abort = new AbortController()
    const pending = raceWithAbortAndTimeout(new Promise(() => undefined), {
      timeoutMs: 10_000,
      signal: abort.signal,
    })
    abort.abort()
    await expect(pending).rejects.toThrow(WALLETCONNECT_CANCELLED_MESSAGE)
  })
})
