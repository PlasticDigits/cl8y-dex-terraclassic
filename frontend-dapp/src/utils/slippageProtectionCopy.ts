/** Retail label for user-configured on-chain `max_spread` guard (GitLab #412). */
export const SLIPPAGE_PROTECTION_LABEL = 'Slippage protection'

/** Footnote in Settings — on-chain parameter name. */
export const SLIPPAGE_PROTECTION_ON_CHAIN_FOOTNOTE = 'Sent on-chain as max_spread (price impact cap per hop).'

/**
 * Default retail slippage / route-impact protection percent (GitLab #497).
 * Passed on-chain as `max_spread = percent / 100` (e.g. 5 → `"0.05"`).
 * Also drives the insufficient-liquidity / route-impact warning copy.
 */
export const DEFAULT_SLIPPAGE_TOLERANCE_PERCENT = 5

/**
 * Preset buttons in Swap Settings and Trade market (includes the default).
 * Tighter options remain available; custom input covers 0.01–50%.
 * The three chips are one aligned group — label / Custom must not wrap as siblings (#528).
 */
export const SLIPPAGE_TOLERANCE_PRESETS_PERCENT = [0.5, 1.0, 5.0] as const

/** On-chain `max_spread` string from a retail percent (5 → `"0.05"`). */
export function maxSpreadFromSlippagePercent(percent: number): string {
  return (percent / 100).toString()
}

/**
 * Swap Custom field: digits + one `.` only. Strips `e`, hex, punctuation, and
 * unicode digits so raw user text cannot reach the chain (GitLab #528 A4).
 */
export function sanitizeSlippageCustomInput(raw: string): string {
  return raw.replace(/[^\d.]/g, '').replace(/(\.\d*)\./g, '$1')
}

/**
 * Persist target for a sanitized Custom string. `null` means do not write the
 * store (empty, NaN, or below 0.01). Values above 50 clamp to 50.
 */
export function persistSlippageCustomInput(sanitized: string): number | null {
  if (sanitized === '') return null
  const parsed = parseFloat(sanitized)
  if (Number.isNaN(parsed)) return null
  if (parsed > 50) return 50
  if (parsed < 0.01) return null
  return parsed
}

export function isSlippageCustomOutOfRange(sanitized: string): boolean {
  if (sanitized === '') return false
  const parsed = parseFloat(sanitized)
  return Number.isNaN(parsed) || parsed < 0.01 || parsed > 50
}

/**
 * Show “High slippage protection increases front-running risk” when tolerance
 * is strictly above this percent (default equals the threshold — no warn at 5%).
 */
export const HIGH_SLIPPAGE_PROTECTION_WARN_PERCENT = 5

/** Route execution-quality metric (#293) — distinct from tolerance. */
export const ROUTE_EXECUTION_SLIPPAGE_LABEL = 'Expected slippage'

export const ROUTE_EXECUTION_SLIPPAGE_TOOLTIP =
  'Execution-quality estimate vs fair cross-rate token prices. This is not your slippage protection setting in Settings.'

export const TRANSACTION_DEADLINE_LABEL = 'Transaction deadline'

/** Format seconds for retail deadline display (e.g. 300 → "5 min"). */
export function formatTransactionDeadline(seconds: number): string {
  if (seconds % 60 === 0 && seconds >= 60) {
    const minutes = seconds / 60
    return minutes === 1 ? '1 min' : `${minutes} min`
  }
  return seconds === 1 ? '1 sec' : `${seconds} sec`
}
