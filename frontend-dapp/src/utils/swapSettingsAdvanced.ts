export const SWAP_SETTINGS_ADVANCED_OPEN_KEY = 'cl8y-dex-swap-settings-advanced-open'

export function readSwapSettingsAdvancedOpen(): boolean {
  if (typeof window === 'undefined') return false
  try {
    return window.localStorage.getItem(SWAP_SETTINGS_ADVANCED_OPEN_KEY) === '1'
  } catch {
    return false
  }
}

export function writeSwapSettingsAdvancedOpen(open: boolean): void {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(SWAP_SETTINGS_ADVANCED_OPEN_KEY, open ? '1' : '0')
  } catch {
    // ignore quota / private mode
  }
}
