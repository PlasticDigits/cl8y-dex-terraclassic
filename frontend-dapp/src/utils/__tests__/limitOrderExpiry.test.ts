import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  limitOrderExpiryFromPreset24h,
  limitOrderExpiryFromPreset7d,
  localDatetimeInputToUnixSeconds,
  parseRawExpiresUnixInput,
  unixSecondsToLocalDatetimeInputValue,
} from '../limitOrderExpiry'

describe('limitOrderExpiry', () => {
  const FIXED_MS = 1_704_067_200_000 // 2023-12-27T12:00:00.000Z — fixed clock in tests

  afterEach(() => {
    vi.useRealTimers()
  })

  it('24h preset matches legacy “type Unix seconds” for the same end time (fixed now)', () => {
    const nowSec = Math.floor(FIXED_MS / 1000)
    const fromPreset = limitOrderExpiryFromPreset24h(FIXED_MS)
    const legacyTyped = String(nowSec + 86_400)
    expect(fromPreset).toBe(Number(legacyTyped))
  })

  it('7d preset is 7 × 24h after now (seconds)', () => {
    const d = limitOrderExpiryFromPreset7d(FIXED_MS) - limitOrderExpiryFromPreset24h(FIXED_MS)
    expect(d).toBe(6 * 86_400)
  })

  it('round-trips local datetime input ↔ unix (local) for a fixed string', () => {
    vi.setSystemTime(FIXED_MS)
    // Use a string that ECMA parses as local time (no Z)
    const s = '2030-06-15T10:30'
    const sec = localDatetimeInputToUnixSeconds(s)
    expect(sec).not.toBeNull()
    if (sec == null) return
    const back = unixSecondsToLocalDatetimeInputValue(sec)
    const again = localDatetimeInputToUnixSeconds(back)
    expect(again).toBe(sec)
  })

  it('parseRawExpiresUnixInput accepts only non-negative safe integers as strings', () => {
    expect(parseRawExpiresUnixInput('')).toBeNull()
    expect(parseRawExpiresUnixInput(' 1704067200 ')).toBe(1_704_067_200)
    expect(parseRawExpiresUnixInput('0')).toBe(0)
    expect(parseRawExpiresUnixInput('1.5')).toBe('invalid')
    expect(parseRawExpiresUnixInput('nope')).toBe('invalid')
  })
})
