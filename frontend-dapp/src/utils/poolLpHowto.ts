/** How-to dismiss storage for the /pool LUNC liquidity how-to (GitLab #531 / #547). */

export const POOL_LP_HOWTO_HINT_DISMISSED_KEY = 'cl8y-dex-pool-lp-howto-hint-dismissed'

/** Whole how-to section (hint + details). `'1'` / `'0'` only — do not JSON.parse (A6). */
export const POOL_LP_HOWTO_SECTION_DISMISSED_KEY = 'cl8y-dex-pool-lp-howto-section-dismissed'

function readFlag(key: string): boolean {
  if (typeof window === 'undefined') return false
  try {
    return window.localStorage.getItem(key) === '1'
  } catch {
    return false
  }
}

function writeFlag(key: string, dismissed: boolean): void {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(key, dismissed ? '1' : '0')
  } catch {
    // ignore quota / private mode
  }
}

export function readPoolLpHowtoHintDismissed(): boolean {
  return readFlag(POOL_LP_HOWTO_HINT_DISMISSED_KEY)
}

export function writePoolLpHowtoHintDismissed(dismissed: boolean): void {
  writeFlag(POOL_LP_HOWTO_HINT_DISMISSED_KEY, dismissed)
}

export function readPoolLpHowtoSectionDismissed(): boolean {
  return readFlag(POOL_LP_HOWTO_SECTION_DISMISSED_KEY)
}

export function writePoolLpHowtoSectionDismissed(dismissed: boolean): void {
  writeFlag(POOL_LP_HOWTO_SECTION_DISMISSED_KEY, dismissed)
  if (dismissed) writePoolLpHowtoHintDismissed(true)
}

/** Hide hint + details after a section dismiss (or legacy hint-only key is ignored for the section). */
export function isPoolLpHowtoSectionHidden(): boolean {
  return readPoolLpHowtoSectionDismissed()
}
