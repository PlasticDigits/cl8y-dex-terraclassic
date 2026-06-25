/** Recent-trades (tape) panel expanded on `/trade` (GitLab #417). */
export const TRADE_TAPE_EXPANDED_KEY = 'cl8y-dex-trade-tape-expanded'

/** Wallet swap history disclosure on `/trade` (GitLab #417). */
export const TRADE_WALLET_HISTORY_EXPANDED_KEY = 'cl8y-dex-trade-wallet-history-expanded'

export function readTradePanelExpanded(storageKey: string, defaultExpanded = false): boolean {
  if (typeof window === 'undefined') return defaultExpanded
  try {
    const stored = window.localStorage.getItem(storageKey)
    if (stored === null) return defaultExpanded
    return stored === '1'
  } catch {
    return defaultExpanded
  }
}

export function writeTradePanelExpanded(storageKey: string, expanded: boolean): void {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(storageKey, expanded ? '1' : '0')
  } catch {
    // ignore quota / private mode
  }
}

/** Desktop resizable tape panel sizes (percent of vertical stack). */
export const TRADE_DESKTOP_TAPE_EXPANDED_SIZE = 42
export const TRADE_DESKTOP_TAPE_COLLAPSED_SIZE = 6
