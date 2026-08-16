/** First-visit hint dismiss for the /pool LUNC liquidity how-to (GitLab #531). */

export const POOL_LP_HOWTO_HINT_DISMISSED_KEY = 'cl8y-dex-pool-lp-howto-hint-dismissed'

export function readPoolLpHowtoHintDismissed(): boolean {
  if (typeof window === 'undefined') return false
  try {
    return window.localStorage.getItem(POOL_LP_HOWTO_HINT_DISMISSED_KEY) === '1'
  } catch {
    return false
  }
}

export function writePoolLpHowtoHintDismissed(dismissed: boolean): void {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(POOL_LP_HOWTO_HINT_DISMISSED_KEY, dismissed ? '1' : '0')
  } catch {
    // ignore quota / private mode
  }
}
