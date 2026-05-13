import { isLimitPriceDirectionInvalid, parsePositivePriceHuman } from '@/utils/limitOrderPriceReference'

export type LimitOrderPricePlaceGateTone = 'none' | 'warning' | 'error'

export type LimitOrderPricePlaceGateResult = {
  /** When false, the place button must stay disabled (invalid limit vs reference). */
  canPlaceLimit: boolean
  userMessage: string | null
  tone: LimitOrderPricePlaceGateTone
  refToken1PerToken0: number | null
}

export type LimitOrderPricePlaceGateContext = {
  /** Pool / LCD reference is still loading while tape is unavailable. */
  refResolutionLoading?: boolean
  /** Pool query failed (LCD / network) while tape is unavailable. */
  refResolutionError?: boolean
}

const MSG_LOADING =
  'Resolving reference price from the AMM pool (indexer trade tape unavailable). Try again in a moment.'
const MSG_LCD_ERROR =
  'Cannot validate limit price: on-chain pool query failed. Check wallet / LCD connectivity or wait for the indexer.'
const MSG_NO_REF =
  'Cannot validate limit price: no indexed trade and no usable AMM pool reference (empty pool or unknown token decimals). Wait for the indexer or add tokens to the registry.'

/**
 * Preflight for limit **price** vs reference (GitLab #154 + #166).
 * Callers resolve `refToken1PerToken0` from indexed last trade and/or on-chain `pool` via `resolveLimitOrderPriceRef`.
 * When the user typed a positive limit and no reference is available, placement is **blocked** (no silent bypass).
 */
export function evaluateLimitOrderPricePlaceGate(
  side: 'bid' | 'ask',
  priceInput: string,
  refToken1PerToken0: number | null | undefined,
  ctx?: LimitOrderPricePlaceGateContext
): LimitOrderPricePlaceGateResult {
  const none: LimitOrderPricePlaceGateResult = {
    canPlaceLimit: true,
    userMessage: null,
    tone: 'none',
    refToken1PerToken0: null,
  }

  const limit = parsePositivePriceHuman(priceInput)
  if (limit == null) return none

  if (ctx?.refResolutionLoading) {
    return {
      canPlaceLimit: false,
      userMessage: MSG_LOADING,
      tone: 'warning',
      refToken1PerToken0: null,
    }
  }

  if (ctx?.refResolutionError) {
    return {
      canPlaceLimit: false,
      userMessage: MSG_LCD_ERROR,
      tone: 'error',
      refToken1PerToken0: null,
    }
  }

  const ref = refToken1PerToken0
  if (ref == null || !(ref > 0) || !Number.isFinite(ref)) {
    return {
      canPlaceLimit: false,
      userMessage: MSG_NO_REF,
      tone: 'error',
      refToken1PerToken0: null,
    }
  }

  if (isLimitPriceDirectionInvalid(side, limit, ref)) {
    const hint =
      side === 'bid'
        ? 'Buy limits must be below the reference price for this pair (indexed tape or AMM pool spot).'
        : 'Sell limits must be above the reference price for this pair (indexed tape or AMM pool spot).'
    return {
      canPlaceLimit: false,
      userMessage: hint,
      tone: 'error',
      refToken1PerToken0: ref,
    }
  }

  return { ...none, refToken1PerToken0: ref }
}
