import { describe, expect, it } from 'vitest'
import { deriveWrapRateLimitStatus, formatWrapRateLimitCountdown, parseCosmWasmTimestampSec } from '../wrapRateLimit'

describe('parseCosmWasmTimestampSec', () => {
  it('parses CosmWasm nanos string', () => {
    expect(parseCosmWasmTimestampSec('1786264111105763126')).toBe(1786264111)
  })

  it('parses seconds-scale string', () => {
    expect(parseCosmWasmTimestampSec('1786264111')).toBe(1786264111)
  })

  it('parses { seconds } object', () => {
    expect(parseCosmWasmTimestampSec({ seconds: '100', nanos: 1 })).toBe(100)
  })

  it('returns null for empty', () => {
    expect(parseCosmWasmTimestampSec(null)).toBeNull()
    expect(parseCosmWasmTimestampSec('')).toBeNull()
  })
})

describe('deriveWrapRateLimitStatus', () => {
  const base = {
    config: { max_amount_per_window: '1000', window_seconds: 3600 },
    current_window_start: null as string | null,
    amount_used: '0',
  }

  it('returns null when config missing', () => {
    expect(deriveWrapRateLimitStatus({ config: null, current_window_start: null, amount_used: '0' }, 1)).toBeNull()
  })

  it('full remaining when no window started', () => {
    const s = deriveWrapRateLimitStatus(base, 1_000)
    expect(s).toMatchObject({
      maxRaw: 1000n,
      remainingRaw: 1000n,
      usedRaw: 0n,
      secondsUntilReset: null,
      windowActive: false,
    })
  })

  it('computes remaining and countdown in active window', () => {
    const start = 1_000_000
    const s = deriveWrapRateLimitStatus(
      {
        ...base,
        current_window_start: String(start),
        amount_used: '250',
      },
      start + 100
    )
    expect(s?.remainingRaw).toBe(750n)
    expect(s?.usedRaw).toBe(250n)
    expect(s?.secondsUntilReset).toBe(3500)
    expect(s?.windowActive).toBe(true)
  })

  it('treats expired window as full remaining', () => {
    const start = 1_000_000
    const s = deriveWrapRateLimitStatus(
      {
        ...base,
        current_window_start: String(BigInt(start) * 1_000_000_000n),
        amount_used: '1000',
      },
      start + 3601
    )
    expect(s?.remainingRaw).toBe(1000n)
    expect(s?.usedRaw).toBe(0n)
    expect(s?.windowExpired).toBe(true)
    expect(s?.secondsUntilReset).toBe(0)
  })
})

describe('formatWrapRateLimitCountdown', () => {
  it('formats null / now / duration', () => {
    expect(formatWrapRateLimitCountdown(null)).toBeNull()
    expect(formatWrapRateLimitCountdown(0)).toBe('now')
    expect(formatWrapRateLimitCountdown(125)).toBe('2:05')
    expect(formatWrapRateLimitCountdown(3660)).toBe('1h 1m')
  })
})
