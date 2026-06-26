/** Dismissed state for the first-visit trade IA strip (GitLab #417). */
export const TRADE_ONBOARDING_DISMISSED_KEY = 'cl8y-dex-trade-onboarding-dismissed'

export function readTradeOnboardingDismissed(): boolean {
  if (typeof window === 'undefined') return false
  try {
    return window.localStorage.getItem(TRADE_ONBOARDING_DISMISSED_KEY) === '1'
  } catch {
    return false
  }
}

export function writeTradeOnboardingDismissed(dismissed: boolean): void {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(TRADE_ONBOARDING_DISMISSED_KEY, dismissed ? '1' : '0')
  } catch {
    // ignore quota / private mode
  }
}
