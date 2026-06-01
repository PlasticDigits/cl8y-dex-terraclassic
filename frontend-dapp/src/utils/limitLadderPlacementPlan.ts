import type { IndexerLimitBookInsertHintItem } from '@/types'
import type { LimitOrderLadderSpecWire, LimitOrderPlacementItemWire } from '@/services/terraclassic/pair'
import type { LadderRungPreview } from '@/utils/limitOrderLadder'
import { computeAdaptiveMaxAdjustSteps } from '@/utils/limitLadderAdaptiveSteps'
import { supportsLadderSingleAnchor } from '@/utils/limitLadderContractFeatures'
import {
  analyzeLadderDepth,
  computeLadderSkipRisk,
  wireHintPredecessor,
  type IndexerShallowLimitOrderLike,
  type LadderDepthAnalysis,
  type LadderSkipRisk,
} from '@/utils/limitLadderDepth'
import { ladderBoundaryRungIndex } from '@/utils/limitLadderBoundary'
import { LIMIT_LADDER_THIN_BOOK_ORDER_THRESHOLD } from '@/utils/limitLadderAdaptiveSteps'

export type LimitLadderPlacementPath = 'thin_ladder' | 'single_anchor_ladder' | 'deep_batch'

export interface LimitLadderPlacementPlan {
  path: LimitLadderPlacementPath
  recommendedMaxSteps: number
  skipRisk: LadderSkipRisk
  depth: LadderDepthAnalysis
  /** Resolved hints aligned to rung prices (same order as rungs). */
  hints: IndexerLimitBookInsertHintItem[]
  /** When true, indexer probe failed — caller should warn and use conservative ladder path. */
  probeDegraded: boolean
  /** Human-readable placement notes for pre-submit summary. */
  notes: string[]
}

export interface BuildLimitLadderPlacementPlanInput {
  side: 'bid' | 'ask'
  startPrice: string
  endPrice: string
  count: number
  rungs: LadderRungPreview[]
  maxAdjustSteps: number
  expiresAt?: number | null
  windowOrders: IndexerShallowLimitOrderLike[]
  hints: IndexerLimitBookInsertHintItem[]
  probeDegraded?: boolean
}

export type { IndexerShallowLimitOrderLike } from '@/utils/limitLadderDepth'

function choosePlacementPath(
  skipRisk: LadderSkipRisk,
  depth: LadderDepthAnalysis,
  boundaryHint: IndexerLimitBookInsertHintItem | undefined,
  allHintsResolved: boolean
): LimitLadderPlacementPath {
  const thinBook = depth.windowOrderCount <= LIMIT_LADDER_THIN_BOOK_ORDER_THRESHOLD && allHintsResolved

  if (thinBook && skipRisk.score === 0) {
    return 'thin_ladder'
  }

  if (
    supportsLadderSingleAnchor() &&
    boundaryHint?.resolved &&
    skipRisk.score === 0 &&
    depth.foreignOrdersBetweenRungs === 0 &&
    depth.unresolvedHintCount === 0
  ) {
    return 'single_anchor_ladder'
  }

  if (skipRisk.needsHintedBatchPath || !allHintsResolved || depth.foreignOrdersBetweenRungs > 0) {
    return 'deep_batch'
  }

  return thinBook ? 'thin_ladder' : 'single_anchor_ladder'
}

export function buildLimitLadderPlacementPlan(input: BuildLimitLadderPlacementPlanInput): LimitLadderPlacementPlan {
  const depth = analyzeLadderDepth({
    side: input.side,
    rungs: input.rungs,
    startPrice: input.startPrice,
    endPrice: input.endPrice,
    count: input.count,
    windowOrders: input.windowOrders,
    hints: input.hints,
  })

  const recommendedMaxSteps = computeAdaptiveMaxAdjustSteps({
    localDepth: depth.windowOrderCount,
    headToBoundaryDistance: depth.headToBoundaryDistance,
    rungCount: input.count,
  })

  const effectiveSteps = input.maxAdjustSteps
  const skipRisk = computeLadderSkipRisk(depth, input.count, effectiveSteps)
  const allHintsResolved = input.hints.length > 0 && input.hints.every((h) => h.resolved)
  const boundaryIdx = ladderBoundaryRungIndex(input.side, input.startPrice, input.endPrice, input.count)
  const boundaryHint = input.hints[boundaryIdx]

  const path = input.probeDegraded
    ? 'thin_ladder'
    : choosePlacementPath(skipRisk, depth, boundaryHint, allHintsResolved)

  const notes: string[] = []
  if (input.probeDegraded) {
    notes.push('Book depth unavailable — using conservative ladder path.')
  } else if (path === 'deep_batch') {
    notes.push('Deep book — placing via hinted batch for reliable rung coverage.')
  } else if (path === 'single_anchor_ladder') {
    notes.push('Single boundary anchor — on-chain chaining fills interior rungs.')
  } else {
    notes.push('Thin book — cheap ladder path.')
  }
  if (skipRisk.predictedSkipped > 0) {
    notes.push(
      `Expected ~${skipRisk.predictedPlaced}/${input.count} rungs placed (${skipRisk.predictedSkipped} may skip).`
    )
  }
  if (depth.unresolvedHintCount > 0) {
    notes.push(`${depth.unresolvedHintCount} rung(s) lack indexer hints — reduced placement confidence.`)
  }

  return {
    path,
    recommendedMaxSteps,
    skipRisk,
    depth,
    hints: input.hints,
    probeDegraded: input.probeDegraded ?? false,
    notes,
  }
}

/** Build batch wire items with per-rung hints (omit when unresolved). */
export function ladderRungsToBatchItems(
  rungs: LadderRungPreview[],
  hints: IndexerLimitBookInsertHintItem[],
  maxAdjustSteps: number,
  expiresAt?: number | null
): LimitOrderPlacementItemWire[] {
  return rungs.map((rung, idx) => {
    const item: LimitOrderPlacementItemWire = {
      price: rung.price,
      amount: rung.amountRaw,
      max_adjust_steps: maxAdjustSteps,
      expires_at: expiresAt ?? undefined,
    }
    const hint = hints[idx]
    if (hint) {
      const predecessor = wireHintPredecessor(hint)
      if (predecessor != null) {
        item.hint_after_order_id = predecessor
      }
    }
    return item
  })
}

/** Build ladder spec wire for thin / single-anchor paths. */
export function buildLadderSpecWire(input: {
  side: 'bid' | 'ask'
  startPrice: string
  endPrice: string
  count: number
  totalAmountRaw: string
  maxAdjustSteps: number
  expiresAt?: number | null
  plan: LimitLadderPlacementPlan
}): LimitOrderLadderSpecWire {
  const spec: LimitOrderLadderSpecWire = {
    side: input.side,
    start_price: input.startPrice,
    end_price: input.endPrice,
    count: input.count,
    total_amount: input.totalAmountRaw,
    distribution: 'equal',
    max_adjust_steps: input.maxAdjustSteps,
    expires_at: input.expiresAt ?? undefined,
  }

  if (input.plan.path === 'single_anchor_ladder' && input.plan.hints.length > 0) {
    const boundaryIdx = ladderBoundaryRungIndex(input.side, input.startPrice, input.endPrice, input.count)
    const boundaryHint = input.plan.hints[boundaryIdx]
    const predecessor = boundaryHint ? wireHintPredecessor(boundaryHint) : undefined
    if (predecessor != null) {
      spec.hint_after_order_id = predecessor
    }
  }

  return spec
}
