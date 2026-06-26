import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import {
  TRADE_ONBOARDING_DISMISSED_KEY,
  readTradeOnboardingDismissed,
  writeTradeOnboardingDismissed,
} from '../tradeOnboarding'

describe('tradeOnboarding', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  afterEach(() => {
    window.localStorage.clear()
  })

  it('defaults to not dismissed', () => {
    expect(readTradeOnboardingDismissed()).toBe(false)
  })

  it('persists dismiss in localStorage', () => {
    writeTradeOnboardingDismissed(true)
    expect(window.localStorage.getItem(TRADE_ONBOARDING_DISMISSED_KEY)).toBe('1')
    expect(readTradeOnboardingDismissed()).toBe(true)
  })
})
