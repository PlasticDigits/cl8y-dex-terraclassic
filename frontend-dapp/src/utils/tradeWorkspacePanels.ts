/** Recent-trades (tape) panel expanded on `/trade` (GitLab #417). */
export const TRADE_TAPE_EXPANDED_KEY = 'cl8y-dex-trade-tape-expanded'

/** Wallet swap history disclosure on `/trade` (GitLab #417). */
export const TRADE_WALLET_HISTORY_EXPANDED_KEY = 'cl8y-dex-trade-wallet-history-expanded'

/** Desktop order-book column visible on `/trade` (GitLab #561). Default visible. */
export const TRADE_BOOK_VISIBLE_KEY = 'cl8y-dex-trade-book-visible'

/** Desktop order-ticket column visible on `/trade` (GitLab #561). Default visible. */
export const TRADE_TICKET_VISIBLE_KEY = 'cl8y-dex-trade-ticket-visible'

/**
 * Boolean workspace flags persist as `'1'` / `'0'` only (GitLab #417 / #561).
 * Missing, corrupt, or non-boolean strings fall back to `defaultValue`.
 * localStorage throws (quota / private mode) also fall back — no crash.
 */
export function readTradePanelFlag(storageKey: string, defaultValue: boolean): boolean {
  if (typeof window === 'undefined') return defaultValue
  try {
    const stored = window.localStorage.getItem(storageKey)
    if (stored === '1') return true
    if (stored === '0') return false
    return defaultValue
  } catch {
    return defaultValue
  }
}

export function writeTradePanelFlag(storageKey: string, value: boolean): void {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(storageKey, value ? '1' : '0')
  } catch {
    // ignore quota / private mode
  }
}

export function readTradePanelExpanded(storageKey: string, defaultExpanded = false): boolean {
  return readTradePanelFlag(storageKey, defaultExpanded)
}

export function writeTradePanelExpanded(storageKey: string, expanded: boolean): void {
  writeTradePanelFlag(storageKey, expanded)
}

export function readTradePanelVisible(storageKey: string, defaultVisible = true): boolean {
  return readTradePanelFlag(storageKey, defaultVisible)
}

export function writeTradePanelVisible(storageKey: string, visible: boolean): void {
  writeTradePanelFlag(storageKey, visible)
}

/** Side columns stay ~1fr; the chart is 2.2fr by default and absorbs a hidden side (2.2 + 1 = 3.2). */
const TRADE_DESKTOP_SIDE_TRACK = 'minmax(13rem, 1fr)'
const TRADE_DESKTOP_CHART_TRACK = 'minmax(0, 2.2fr)'
const TRADE_DESKTOP_CHART_EXPANDED_TRACK = 'minmax(0, 3.2fr)'

/**
 * Desktop CSS grid columns so the chart absorbs width when a side panel hides (L561-5–L561-7).
 * The remaining side panel must not grow into the vacated track.
 * Book and ticket stay mounted (`hidden` + `inert`); they must not occupy a grid track.
 */
export function tradeDesktopGridTemplateColumns(bookVisible: boolean, ticketVisible: boolean): string {
  if (bookVisible && ticketVisible) {
    return `${TRADE_DESKTOP_SIDE_TRACK} ${TRADE_DESKTOP_CHART_TRACK} ${TRADE_DESKTOP_SIDE_TRACK}`
  }
  if (bookVisible) {
    return `${TRADE_DESKTOP_SIDE_TRACK} ${TRADE_DESKTOP_CHART_EXPANDED_TRACK}`
  }
  if (ticketVisible) {
    return `${TRADE_DESKTOP_CHART_EXPANDED_TRACK} ${TRADE_DESKTOP_SIDE_TRACK}`
  }
  return 'minmax(0, 1fr)'
}

export function tradeDesktopChartGridColumn(bookVisible: boolean): number {
  return bookVisible ? 2 : 1
}

export function tradeDesktopTicketGridColumn(bookVisible: boolean): number {
  return bookVisible ? 3 : 2
}
