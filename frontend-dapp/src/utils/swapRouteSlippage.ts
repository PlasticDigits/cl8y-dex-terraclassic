/** Retail swap guard: block submit above this unless Expert Mode is on (GitLab #293). */
export const SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT = 30

/** Show extreme-slippage warning at or above this threshold (GitLab #293). */
export const SWAP_EXTREME_SLIPPAGE_WARNING_PCT = 99

export function parseSlippagePercent(raw: string | undefined | null): number | null {
  if (raw == null || raw.trim() === '') return null
  const n = parseFloat(raw)
  return Number.isFinite(n) ? n : null
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

export function slippageSeverityClass(pct: number): string {
  if (pct >= SWAP_EXTREME_SLIPPAGE_WARNING_PCT) return 'text-red-500 font-bold'
  if (pct > SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT) return 'text-red-400 font-semibold'
  if (pct > 5) return 'text-amber-400'
  if (pct > 1) return 'text-amber-300'
  return 'text-green-400'
}
