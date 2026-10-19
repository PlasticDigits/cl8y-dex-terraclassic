/** Retail swap guard: block submit above this unless Expert Mode is on (GitLab #293). */
export const SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT = 30

/** Show extreme-slippage warning at or above this threshold (GitLab #293). */
export const SWAP_EXTREME_SLIPPAGE_WARNING_PCT = 99

export function parseSlippagePercent(raw: string | undefined | null): number | null {
  if (raw == null || raw.trim() === '') return null
  if (!/^(?:0|[1-9]\d*)(?:\.\d+)?$/.test(raw.trim())) return null
  const n = Number(raw)
  return Number.isFinite(n) && n >= 0 && n <= 100 ? n : null
}

/** Symmetric slippage vs a reference output (matches indexer `symmetric_slippage_percent`). */
export function symmetricSlippagePercentFromRaw(actualOutRaw: string, expectedOutRaw: string): string | null {
  const actual = parseFloat(actualOutRaw)
  const expected = parseFloat(expectedOutRaw)
  if (!Number.isFinite(actual) || !Number.isFinite(expected) || actual <= 0 || expected <= 0) {
    return null
  }
  const lo = Math.min(actual, expected)
  const hi = Math.max(actual, expected)
  return ((1 - lo / hi) * 100).toFixed(2)
}

/** Warn when indexer `estimated_amount_out` disagrees with wallet sim by at least this much (GitLab #471). */
export const INDEXER_WALLET_AMOUNT_MISMATCH_WARN_PCT = 0.1

/** True when indexer and wallet receive amounts differ enough to surface a reconcile label. */
export function indexerWalletAmountMismatch(indexerAmountOut: string, walletAmountOut: string): boolean {
  const pct = symmetricSlippagePercentFromRaw(walletAmountOut, indexerAmountOut)
  if (pct == null) return false
  return parseFloat(pct) >= INDEXER_WALLET_AMOUNT_MISMATCH_WARN_PCT
}

/** Exact integer economics before display rounding; preserve all 5/10/30/99 transitions. */
export function preciseFairDeviationPercent(actualRaw: string, fairRaw: string): string | null {
  const raw = /^(?:[1-9]\d{0,38})$/
  if (!raw.test(actualRaw) || !raw.test(fairRaw)) return null
  const actual = BigInt(actualRaw),
    fair = BigInt(fairRaw)
  const max = (1n << 128n) - 1n
  if (actual > max || fair > max) return null
  const denominator = actual > fair ? actual : fair
  const numerator = (actual > fair ? actual - fair : fair - actual) * 100n
  const scale = 10n ** 18n
  const fixed = (numerator * scale) / denominator
  const fraction = (fixed % scale).toString().padStart(18, '0').replace(/0+$/, '').padEnd(2, '0')
  const result = `${fixed / scale}.${fraction}`
  const parsed = Number(result)
  // IEEE conversion must never collapse an exact crossing onto its threshold.
  for (const threshold of [5, 10, 30, 99]) {
    const comparison = numerator - BigInt(threshold) * denominator
    if (comparison > 0n && parsed <= threshold) return String(threshold + Number.EPSILON * threshold)
    if (comparison < 0n && parsed >= threshold) return String(threshold - Number.EPSILON * threshold)
  }
  return result
}

/** Prefer wallet receive vs indexer spot; fall back to indexer slippage when spot is missing. */
export function resolveRouteSlippagePercent(
  walletReturnRaw: string,
  spotAmountOutRaw: string | undefined | null,
  indexerSlippagePercent: string | undefined | null
): string | undefined {
  if (spotAmountOutRaw != null) {
    // A supplied malformed reference must never fall back to a lower server metric.
    return preciseFairDeviationPercent(walletReturnRaw, spotAmountOutRaw) ?? undefined
  }
  const idx = indexerSlippagePercent?.trim()
  return idx || undefined
}

/** Prefer indexer route-based slippage; fall back to hop spread price impact. */
export function resolveSwapExpectedSlippagePercent(
  routeSlippagePercent: string | undefined | null,
  hopSpreadPercent: string | null
): number | null {
  const route = parseSlippagePercent(routeSlippagePercent)
  if (route != null) return route
  if (hopSpreadPercent == null) return null
  const hop = parseFloat(hopSpreadPercent)
  return Number.isFinite(hop) ? hop : null
}

/** Display cannot collapse an exact risk transition onto its threshold. */
export function formatTradingPriceDifference(percent: number): string {
  const rounded = percent.toFixed(2)
  for (const threshold of [5, 10, 30, 99]) {
    if (Number(rounded) === threshold && percent !== threshold) return `${percent > threshold ? '>' : '<'}${threshold}`
  }
  return rounded
}

export type ImpactAsideOutsideDetails = { kind: 'route'; percent: string } | { kind: 'worst_hop'; percent: string }

/**
 * Percent shown outside collapsed trade details (#1360).
 * A valid route reference is the single aggregate at every risk level.
 * A worst-hop spread is a legacy fallback when the route reference is absent.
 */
export function impactAsideOutsideDetails(input: {
  routeSlippagePct: number | null
  worstHopPercent: string | null | undefined
}): ImpactAsideOutsideDetails | null {
  const route = input.routeSlippagePct
  if (route != null && Number.isFinite(route) && route >= 0) {
    return { kind: 'route', percent: route.toFixed(2) }
  }
  const hopRaw = input.worstHopPercent?.trim()
  const hop = parseSlippagePercent(hopRaw)
  if (!hopRaw || hop == null) return null
  return { kind: 'worst_hop', percent: hopRaw }
}

export function slippageSeverityClass(pct: number): string {
  if (pct >= SWAP_EXTREME_SLIPPAGE_WARNING_PCT) return 'text-red-500 font-bold'
  if (pct > SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT) return 'text-red-400 font-semibold'
  if (pct > 5) return 'text-amber-400'
  if (pct > 1) return 'text-amber-300'
  return 'text-green-400'
}
