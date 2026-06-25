/**
 * Pair fee math for limit-order **maker placement** copy (GitLab #157).
 * On-chain: `effective_fee_bps / 2` at placement (`orderbook::maker_placement_fee_bps`).
 */

/** Integer fee after tier discount, matching pair `effective_fee_bps` composition. */
export function effectiveSwapFeeBps(pairFeeBps: number, discountBps: number | null | undefined): number {
  const fee = Math.floor(Number(pairFeeBps))
  const disc = Math.floor(Number(discountBps ?? 0))
  if (!Number.isFinite(fee) || fee < 0) return 0
  if (!Number.isFinite(disc) || disc <= 0) return Math.min(fee, 10000)
  const clampedDisc = Math.min(Math.max(disc, 0), 10000)
  return Math.floor((fee * (10000 - clampedDisc)) / 10000)
}

/** Maker-side bps charged once from escrow when the resting order is placed (`floor(effective/2)`). */
export function makerPlacementFeeBps(effectiveFeeBps: number): number {
  const e = Math.floor(Number(effectiveFeeBps))
  if (!Number.isFinite(e) || e < 0) return 0
  return Math.floor(e / 2)
}

/** Human percent label for fee bps (1 bps = 0.01%). */
export function bpsToPercentLabel(bps: number): string {
  const n = Number(bps)
  if (!Number.isFinite(n) || n < 0) return '—'
  const pct = n / 100
  if (pct === 0) return '0%'
  if (pct >= 1) return `${pct.toFixed(2)}%`
  if (pct >= 0.1) return `${pct.toFixed(2)}%`
  return `${pct.toFixed(3)}%`
}
