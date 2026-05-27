import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT,
  LIMIT_ORDER_MAX_ADJUST_STEPS_PRESET_VALUES,
  clampLimitOrderMaxAdjustSteps,
  limitOrderExpiryFromPreset24h,
  limitOrderExpiryFromPreset7d,
  limitOrderMaxAdjustStepsForPresetTier,
  localDatetimeInputToUnixSeconds,
  parseRawExpiresUnixInput,
  resolveLimitOrderMaxAdjustStepsPresetTier,
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

describe('limitOrderExpiry max_adjust_steps presets (GitLab #204)', () => {
  it('maps preset tiers to documented step integers', () => {
    expect(limitOrderMaxAdjustStepsForPresetTier('low')).toBe(16)
    expect(limitOrderMaxAdjustStepsForPresetTier('medium')).toBe(32)
    expect(limitOrderMaxAdjustStepsForPresetTier('high')).toBe(128)
    expect(LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT).toBe(LIMIT_ORDER_MAX_ADJUST_STEPS_PRESET_VALUES.medium)
  })

  it('resolves active tier from on-chain step integer', () => {
    expect(resolveLimitOrderMaxAdjustStepsPresetTier(16)).toBe('low')
    expect(resolveLimitOrderMaxAdjustStepsPresetTier(32)).toBe('medium')
    expect(resolveLimitOrderMaxAdjustStepsPresetTier(128)).toBe('high')
    expect(resolveLimitOrderMaxAdjustStepsPresetTier(64)).toBe('custom')
    expect(resolveLimitOrderMaxAdjustStepsPresetTier(200)).toBe('custom')
  })

  it('clamps custom steps to 1…256 with safe fallback', () => {
    expect(clampLimitOrderMaxAdjustSteps(0)).toBe(1)
    expect(clampLimitOrderMaxAdjustSteps(300)).toBe(256)
    expect(clampLimitOrderMaxAdjustSteps(Number.NaN)).toBe(LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT)
    expect(clampLimitOrderMaxAdjustSteps(48)).toBe(48)
  })
})
