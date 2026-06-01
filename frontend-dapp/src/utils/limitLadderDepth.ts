import type { IndexerLimitBookInsertHintItem } from '@/types'
import type { LadderRungPreview } from '@/utils/limitOrderLadder'
import { comparePositiveDecimalStrings } from '@/utils/limitOrderNonCrossing'
import { ladderBoundaryRungIndex } from '@/utils/limitLadderBoundary'

export interface LadderPriceWindowParams {
  priceFrom: string
  priceTo: string
}

/** Map ladder band to indexer `price_from` / `price_to` (GitLab #267). */
export function ladderPriceWindowParams(
  side: 'bid' | 'ask',
  startPrice: string,
  endPrice: string
): LadderPriceWindowParams {
  const cmp = comparePositiveDecimalStrings(startPrice.trim(), endPrice.trim())
  if (cmp == null) {
    return { priceFrom: startPrice.trim(), priceTo: endPrice.trim() }
  }
  if (side === 'bid') {
    return cmp === 'gt' || cmp === 'eq'
      ? { priceFrom: startPrice.trim(), priceTo: endPrice.trim() }
      : { priceFrom: endPrice.trim(), priceTo: startPrice.trim() }
  }
  return cmp === 'lt' || cmp === 'eq'
    ? { priceFrom: startPrice.trim(), priceTo: endPrice.trim() }
    : { priceFrom: endPrice.trim(), priceTo: startPrice.trim() }
}

export interface LadderDepthAnalysis {
  /** Resting orders in the probed window (may include foreign makers). */
  windowOrderCount: number
  /** Orders whose price falls strictly between consecutive rung prices. */
  foreignOrdersBetweenRungs: number
  /** Orders from book head until the boundary rung slot (walk budget proxy). */
  headToBoundaryDistance: number
  /** Unresolved hint count from indexer batch resolver. */
  unresolvedHintCount: number
}

/** Minimal order row for depth math (indexer `limit-book` orders). */
export interface IndexerShallowLimitOrderLike {
  order_id: number
  price: string
  owner?: string
  side?: string
  remaining?: string
}

function rungPricesSet(rungs: LadderRungPreview[]): Set<string> {
  return new Set(rungs.map((r) => r.price.trim()))
}

function isStrictlyBetween(price: string, low: string, high: string, side: 'bid' | 'ask'): boolean {
  const cLow = comparePositiveDecimalStrings(price, low)
  const cHigh = comparePositiveDecimalStrings(price, high)
  if (cLow == null || cHigh == null) return false
  if (side === 'bid') {
    return (cLow === 'lt' && cHigh === 'gt') || cLow === 'eq' || cHigh === 'eq'
  }
  return (cLow === 'gt' && cHigh === 'lt') || cLow === 'eq' || cHigh === 'eq'
}

/** Count foreign orders interleaved between ladder rung prices. */
export function countForeignOrdersBetweenRungs(
  side: 'bid' | 'ask',
  rungs: LadderRungPreview[],
  windowOrders: IndexerShallowLimitOrderLike[]
): number {
  if (rungs.length < 2 || windowOrders.length === 0) return 0
  const rungPrices = rungPricesSet(rungs)
  let foreign = 0
  for (const order of windowOrders) {
    if (rungPrices.has(order.price.trim())) continue
    for (let i = 0; i < rungs.length - 1; i++) {
      const a = rungs[i]!.price
      const b = rungs[i + 1]!.price
      const lo = comparePositiveDecimalStrings(a, b) === 'lt' ? a : b
      const hi = comparePositiveDecimalStrings(a, b) === 'lt' ? b : a
      if (isStrictlyBetween(order.price, lo, hi, side)) {
        foreign += 1
        break
      }
    }
  }
  return foreign
}

/** Distance from head to boundary rung insert slot (order count proxy). */
export function computeHeadToBoundaryDistance(
  side: 'bid' | 'ask',
  rungs: LadderRungPreview[],
  startPrice: string,
  endPrice: string,
  count: number,
  windowOrders: IndexerShallowLimitOrderLike[]
): number {
  if (windowOrders.length === 0) return 0
  const boundaryIdx = ladderBoundaryRungIndex(side, startPrice, endPrice, count)
  const boundaryPrice = rungs[boundaryIdx]?.price
  if (!boundaryPrice) return windowOrders.length

  let distance = 0
  for (const order of windowOrders) {
    const cmp = comparePositiveDecimalStrings(boundaryPrice, order.price.trim())
    if (cmp == null) break
    if (side === 'bid') {
      if (cmp === 'gt') return distance
      distance += 1
      continue
    }
    if (cmp === 'lt') return distance
    distance += 1
  }
  return distance
}

export function analyzeLadderDepth(input: {
  side: 'bid' | 'ask'
  rungs: LadderRungPreview[]
  startPrice: string
  endPrice: string
  count: number
  windowOrders: IndexerShallowLimitOrderLike[]
  hints: IndexerLimitBookInsertHintItem[]
}): LadderDepthAnalysis {
  const foreignOrdersBetweenRungs = countForeignOrdersBetweenRungs(input.side, input.rungs, input.windowOrders)
  const headToBoundaryDistance = computeHeadToBoundaryDistance(
    input.side,
    input.rungs,
    input.startPrice,
    input.endPrice,
    input.count,
    input.windowOrders
  )
  const unresolvedHintCount = input.hints.filter((h) => !h.resolved).length
  return {
    windowOrderCount: input.windowOrders.length,
    foreignOrdersBetweenRungs,
    headToBoundaryDistance,
    unresolvedHintCount,
  }
}

export interface LadderSkipRisk {
  score: number
  predictedPlaced: number
  predictedSkipped: number
  /** True when UI should prefer hinted batch path over cheap ladder. */
  needsHintedBatchPath: boolean
}

/**
 * Skip-risk score = foreign orders between rungs + head→boundary gap beyond step budget.
 * Unresolved hints add to predicted skips (GitLab #268).
 */
export function computeLadderSkipRisk(
  analysis: LadderDepthAnalysis,
  rungCount: number,
  maxAdjustSteps: number
): LadderSkipRisk {
  const headGapPenalty = Math.max(0, analysis.headToBoundaryDistance - maxAdjustSteps)
  const score = analysis.foreignOrdersBetweenRungs + headGapPenalty + analysis.unresolvedHintCount
  const predictedSkipped = Math.min(
    rungCount,
    analysis.unresolvedHintCount +
      (headGapPenalty > 0 ? 1 : 0) +
      (analysis.foreignOrdersBetweenRungs > 0 ? Math.min(analysis.foreignOrdersBetweenRungs, rungCount - 1) : 0)
  )
  const predictedPlaced = Math.max(0, rungCount - predictedSkipped)
  const needsHintedBatchPath =
    score > 0 ||
    analysis.windowOrderCount > 0 ||
    analysis.unresolvedHintCount > 0 ||
    analysis.foreignOrdersBetweenRungs > 0

  return { score, predictedPlaced, predictedSkipped, needsHintedBatchPath }
}

/** Map indexer hint to wire predecessor — omit when unresolved (invariant L14). */
export function wireHintPredecessor(hint: IndexerLimitBookInsertHintItem): number | undefined {
  if (!hint.resolved) return undefined
  if (hint.predecessor_order_id == null) return undefined
  return hint.predecessor_order_id
}
