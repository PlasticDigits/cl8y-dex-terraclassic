/**
 * Head-most rung index in book order — mirrors `ladder_boundary_rung_index` in
 * `dex-common::limit_placement` (GitLab #266).
 */
export function ladderBoundaryRungIndex(
  side: 'bid' | 'ask',
  startPrice: string,
  endPrice: string,
  count: number
): number {
  const start = Number.parseFloat(startPrice)
  const end = Number.parseFloat(endPrice)
  if (!Number.isFinite(start) || !Number.isFinite(end) || count < 1) return 0

  if (side === 'bid') {
    return start >= end ? 0 : count - 1
  }
  return start <= end ? 0 : count - 1
}
