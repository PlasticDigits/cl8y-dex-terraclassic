import type { IndexerPair, IndexerTrade } from '@/types'
import {
  isLimitPriceDirectionInvalid,
  parsePositivePriceHuman,
  tradeToToken1PerToken0Human,
} from '@/utils/limitOrderPriceReference'

export type LimitOrderPricePlaceGateTone = 'none' | 'warning' | 'error'

export type LimitOrderPricePlaceGateResult = {
  /** When false, the place button must stay disabled (invalid limit vs reference). */
  canPlaceLimit: boolean
  userMessage: string | null
  tone: LimitOrderPricePlaceGateTone
  refToken1PerToken0: number | null
}

/**
 * Preflight for limit **price** vs latest tape print (GitLab #154). When there is no resolvable reference, placement is allowed.
 */
export function evaluateLimitOrderPricePlaceGate(
  side: 'bid' | 'ask',
  priceInput: string,
  latestTrade: IndexerTrade | undefined | null,
  pair: Pick<IndexerPair, 'asset_0' | 'asset_1'> | undefined | null
): LimitOrderPricePlaceGateResult {
  const none: LimitOrderPricePlaceGateResult = {
    canPlaceLimit: true,
    userMessage: null,
    tone: 'none',
    refToken1PerToken0: null,
  }

  const limit = parsePositivePriceHuman(priceInput)
  if (limit == null) return none
  if (!latestTrade || !pair) return none

  const ref = tradeToToken1PerToken0Human(latestTrade, pair)
  if (ref == null || !(ref > 0)) return none

  if (isLimitPriceDirectionInvalid(side, limit, ref)) {
    const hint =
      side === 'bid'
        ? 'Buy limits must be below the latest trade reference price for this pair.'
        : 'Sell limits must be above the latest trade reference price for this pair.'
    return {
      canPlaceLimit: false,
      userMessage: hint,
      tone: 'error',
      refToken1PerToken0: ref,
    }
  }

  return { ...none, refToken1PerToken0: ref }
}
