import { formatNum, formatPairPrice } from './formatAmount'

/** Cap compact USD / spot strings so adversarial JSON cannot lock the tab (GitLab #548 **A4**). */
const OVERVIEW_DISPLAY_CAP = 24

function parseFiniteUsd(raw: string | null | undefined): number | null {
  if (raw == null || raw === '') return null
  if (typeof raw !== 'string') return null
  if (raw.length > 64) return null
  if (/[<>]/.test(raw)) return null
  const n = Number.parseFloat(raw)
  if (!Number.isFinite(n)) return null
  return n
}

/**
 * Compact USD volume for Charts overview, pair 24h stats, trader leaderboard, and trader profile.
 *
 * - Unpriced / missing / invalid → `—` (never `$0` when trades > 0).
 * - Idle (`trades === 0`) → `$0`.
 * - Does **not** read raw mixed-unit `total_volume` / `total_volume_24h`.
 */
export function formatIndexedVolumeUsd(usd: string | null | undefined, trades: number): string {
  const n = parseFiniteUsd(usd)
  if (n == null) return '—'
  if (n < 0) return '—'
  if (n === 0) return trades > 0 ? '—' : '$0'
  const compact = formatNum(n, 4)
  const out = `$${compact}`
  if (out.length > OVERVIEW_DISPLAY_CAP) return '—'
  return out
}

/**
 * `/pool` Vol + pair-search badge: compact USD or **—**.
 * Idle `"0"`, missing, unpriced, and hostile strings all render as em-dash (never `$0`, never quote-token).
 */
export function formatPairListVolumeUsd(usd: string | null | undefined): string {
  return formatIndexedVolumeUsd(usd, 1)
}

/**
 * Charts overview 24h volume: `$` + compact human USD.
 *
 * - Unpriced / missing / invalid → `—` (never `$0` when trades > 0).
 * - Idle DEX (`total_trades_24h === 0`) → `$0`.
 * - Does **not** read `total_volume_24h` (raw mixed integers stay API-only).
 */
export function formatChartsOverviewVolumeUsd(usd: string | null | undefined, trades: number): string {
  return formatIndexedVolumeUsd(usd, trades)
}

/** USTC/USD spot from overview — `$` + price formatter, never compact `T` (#548 **C4**). */
export function formatChartsOverviewUstcUsd(raw: string | null | undefined): string {
  const n = parseFiniteUsd(raw)
  if (n == null || n <= 0) return '—'
  const human = formatPairPrice(n, 6)
  if (human === '0') return '—'
  const out = `$${human}`
  if (out.length > OVERVIEW_DISPLAY_CAP) return '—'
  if (/[TBMK]$/.test(human)) return '—'
  return out
}

export function formatChartsOverviewCount(n: number | null | undefined): string {
  if (n == null || !Number.isFinite(n) || n < 0) return '—'
  return Math.trunc(n).toLocaleString('en-US')
}
