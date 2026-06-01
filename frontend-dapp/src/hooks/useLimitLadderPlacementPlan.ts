import { useQuery } from '@tanstack/react-query'

import { getPairLimitBookInsertHints, getPairLimitBookPage } from '@/services/indexer/client'
import type { LadderRungPreview } from '@/utils/limitOrderLadder'
import { ladderPriceWindowParams } from '@/utils/limitLadderDepth'
import { buildLimitLadderPlacementPlan, type LimitLadderPlacementPlan } from '@/utils/limitLadderPlacementPlan'

export interface UseLimitLadderPlacementPlanParams {
  pairAddress: string
  side: 'bid' | 'ask'
  startPrice: string
  endPrice: string
  count: number
  rungs: LadderRungPreview[]
  maxAdjustSteps: number
  expiresAt?: number | null
  enabled?: boolean
}

export function limitLadderPlacementPlanQueryKey(params: UseLimitLadderPlacementPlanParams): unknown[] {
  return [
    'limitLadderPlacementPlan',
    params.pairAddress,
    params.side,
    params.startPrice,
    params.endPrice,
    params.count,
    params.rungs.map((r) => `${r.price}:${r.amountRaw}`).join('|'),
    params.maxAdjustSteps,
  ]
}

async function fetchLadderPlacementPlan(params: UseLimitLadderPlacementPlanParams): Promise<LimitLadderPlacementPlan> {
  const prices = params.rungs.map((r) => r.price)
  const window = ladderPriceWindowParams(params.side, params.startPrice, params.endPrice)

  try {
    const [bookPage, hintsResponse] = await Promise.all([
      getPairLimitBookPage(params.pairAddress, params.side, {
        limit: 100,
        priceFrom: window.priceFrom,
        priceTo: window.priceTo,
      }),
      getPairLimitBookInsertHints(params.pairAddress, params.side, prices),
    ])

    return buildLimitLadderPlacementPlan({
      side: params.side,
      startPrice: params.startPrice,
      endPrice: params.endPrice,
      count: params.count,
      rungs: params.rungs,
      maxAdjustSteps: params.maxAdjustSteps,
      expiresAt: params.expiresAt,
      windowOrders: bookPage.orders,
      hints: hintsResponse.hints,
      probeDegraded: false,
    })
  } catch {
    return buildLimitLadderPlacementPlan({
      side: params.side,
      startPrice: params.startPrice,
      endPrice: params.endPrice,
      count: params.count,
      rungs: params.rungs,
      maxAdjustSteps: params.maxAdjustSteps,
      expiresAt: params.expiresAt,
      windowOrders: [],
      hints: prices.map((price) => ({
        price,
        predecessor_order_id: null,
        resolved: false,
        reason: 'pagination_gap',
      })),
      probeDegraded: true,
    })
  }
}

/**
 * Depth probe + batch hint resolution for ladder preflight (indexer only — GitLab #268).
 */
export function useLimitLadderPlacementPlan(params: UseLimitLadderPlacementPlanParams) {
  const enabled =
    params.enabled !== false &&
    Boolean(params.pairAddress) &&
    params.rungs.length >= 2 &&
    !params.rungs.some((r) => !r.price.trim())

  return useQuery({
    queryKey: limitLadderPlacementPlanQueryKey(params),
    queryFn: () => fetchLadderPlacementPlan(params),
    enabled,
    staleTime: 8_000,
    refetchOnWindowFocus: false,
  })
}
