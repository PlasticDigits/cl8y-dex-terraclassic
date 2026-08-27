/**
 * Shared trailing-window copy for Charts / Protocol / Pool volume (GitLab #576).
 *
 * Indexer 24h / 7d / 30d stats are `Utc::now() − N`, not UTC calendar buckets.
 * Visible labels stay ≤ ~5 words (#489). Depth lives in `title` / `aria-label`.
 * Strings are static repo constants — never interpolate overview JSON (A1).
 */

/** Visible Charts overview + pair 24h USD volume label. */
export const TRAILING_24H_VOLUME_LABEL = 'Last 24h Vol (USD)'

/** Visible Charts overview 24h trades label. */
export const TRAILING_24H_TRADES_LABEL = 'Last 24h Trades'

/** Pair 24h Stats primary volume — same trailing wording as overview (W2). */
export const TRAILING_PAIR_VOL_USD_LABEL = TRAILING_24H_VOLUME_LABEL

export const PROTOCOL_VOLUME_24H_LABEL = 'Last 24h vol'
export const PROTOCOL_VOLUME_7D_LABEL = 'Last 7d vol'
export const PROTOCOL_VOLUME_30D_LABEL = 'Last 30d vol'
export const PROTOCOL_TRADES_24H_LABEL = 'Last 24h trades'

export const PROTOCOL_FEES_24H_LABEL = 'Last 24h fees'
export const PROTOCOL_FEES_7D_LABEL = 'Last 7d fees'
export const PROTOCOL_FEES_30D_LABEL = 'Last 30d fees'
export const PROTOCOL_FEES_24H_CHG_LABEL = '24h fee change'
export const PROTOCOL_FEES_7D_CHG_LABEL = '7d fee change'
export const PROTOCOL_FEES_30D_CHG_LABEL = '30d fee change'

/** Charts pair-list SORT option; value stays `volume_24h`. */
export const CHARTS_PAIR_SORT_VOLUME_LABEL = 'Last 24h volume'

/** Pool table Vol header stays one word; title discloses the window (U6). */
export const POOL_VOL_HEADER_LABEL = 'Vol'

export const TRAILING_24H_VOLUME_TITLE = 'Priced swaps in the last 24 hours, not a midnight reset.'

export const TRAILING_24H_TRADES_TITLE = 'Swap count in the last 24 hours, not a midnight reset.'

export const TRAILING_7D_VOLUME_TITLE = 'Priced swaps in the last 7 days, not a calendar-week reset.'

export const TRAILING_30D_VOLUME_TITLE = 'Priced swaps in the last 30 days, not a calendar-month reset.'

export const POOL_VOL_HEADER_TITLE = 'Quote-side volume in the last 24 hours, not a midnight reset.'

export const TRAILING_24H_FEES_TITLE = 'Treasury fees in the last 24 hours, not a midnight reset.'
export const TRAILING_7D_FEES_TITLE = 'Treasury fees in the last 7 days, not a calendar-week reset.'
export const TRAILING_30D_FEES_TITLE = 'Treasury fees in the last 30 days, not a calendar-month reset.'
export const TRAILING_24H_FEES_CHG_TITLE = 'Change vs the last 24 hours prior window. Em-dash until both windows fill.'
export const TRAILING_7D_FEES_CHG_TITLE = 'Change vs the last 7 days prior window. Em-dash until both windows fill.'
export const TRAILING_30D_FEES_CHG_TITLE = 'Change vs the last 30 days prior window. Em-dash until both windows fill.'

export const TRAILING_24H_VOLUME_CHG_TITLE =
  'Change vs the last 24 hours prior window. Em-dash until both windows fill.'
export const TRAILING_7D_VOLUME_CHG_TITLE = 'Change vs the last 7 days prior window. Em-dash until both windows fill.'
export const TRAILING_30D_VOLUME_CHG_TITLE = 'Change vs the last 30 days prior window. Em-dash until both windows fill.'

export const TRAILING_LIQUIDITY_24H_TITLE = 'Change vs the last 24 hours snapshot. Em-dash until the window fills.'
/** Unused on Total liquidity after #677; 30d Δ% stays on GET /overview. */
export const TRAILING_LIQUIDITY_30D_TITLE = 'Change vs the last 30 days snapshot. Em-dash until the window fills.'

export const PROTOCOL_VOLUME_DAILY_LABEL = 'UTC volume'
export const PROTOCOL_VOLUME_DAILY_7D_LABEL = '7d'
export const PROTOCOL_VOLUME_DAILY_30D_LABEL = '30d'
export const PROTOCOL_VOLUME_GRAIN_HOURLY_LABEL = 'Hourly'
export const PROTOCOL_VOLUME_GRAIN_DAILY_LABEL = 'Daily'
export const PROTOCOL_VOLUME_GRAIN_MONTHLY_LABEL = 'Monthly'
export const PROTOCOL_VOLUME_DAILY_TITLE = 'UTC calendar volume. Not the trailing 24h, 7d, or 30d tiles.'
export const PROTOCOL_VOLUME_GRAIN_SUBTITLE = {
  hourly: 'UTC hour',
  daily: 'UTC calendar day',
  monthly: 'UTC calendar month',
} as const

/** Visible labels that must stay ≤ ~5 words (#489 / W3). */
export const TRAILING_WINDOW_VISIBLE_LABELS = [
  TRAILING_24H_VOLUME_LABEL,
  TRAILING_24H_TRADES_LABEL,
  PROTOCOL_VOLUME_24H_LABEL,
  PROTOCOL_VOLUME_7D_LABEL,
  PROTOCOL_VOLUME_30D_LABEL,
  PROTOCOL_TRADES_24H_LABEL,
  PROTOCOL_FEES_24H_LABEL,
  PROTOCOL_FEES_7D_LABEL,
  PROTOCOL_FEES_30D_LABEL,
  PROTOCOL_FEES_24H_CHG_LABEL,
  PROTOCOL_FEES_7D_CHG_LABEL,
  PROTOCOL_FEES_30D_CHG_LABEL,
  PROTOCOL_VOLUME_DAILY_LABEL,
  PROTOCOL_VOLUME_GRAIN_HOURLY_LABEL,
  PROTOCOL_VOLUME_GRAIN_DAILY_LABEL,
  PROTOCOL_VOLUME_GRAIN_MONTHLY_LABEL,
  CHARTS_PAIR_SORT_VOLUME_LABEL,
  POOL_VOL_HEADER_LABEL,
] as const

/** Progressive-disclosure titles (static; no JSON, URLs, or settlement claims). */
export const TRAILING_WINDOW_TITLES = [
  TRAILING_24H_VOLUME_TITLE,
  TRAILING_24H_TRADES_TITLE,
  TRAILING_7D_VOLUME_TITLE,
  TRAILING_30D_VOLUME_TITLE,
  POOL_VOL_HEADER_TITLE,
  TRAILING_24H_FEES_TITLE,
  TRAILING_7D_FEES_TITLE,
  TRAILING_30D_FEES_TITLE,
  TRAILING_24H_FEES_CHG_TITLE,
  TRAILING_7D_FEES_CHG_TITLE,
  TRAILING_30D_FEES_CHG_TITLE,
  TRAILING_24H_VOLUME_CHG_TITLE,
  TRAILING_7D_VOLUME_CHG_TITLE,
  TRAILING_30D_VOLUME_CHG_TITLE,
  TRAILING_LIQUIDITY_24H_TITLE,
  TRAILING_LIQUIDITY_30D_TITLE,
] as const

export function trailingWindowLabelWordCount(label: string): number {
  return label.trim().split(/\s+/).filter(Boolean).length
}

/** Compose StatBox value `aria-label` so screen readers hear the window without hover. */
export function composeStatAriaLabel(title: string, value: string): string {
  return `${title} ${value}`
}
