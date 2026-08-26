import { formatTokenAmount } from '@/utils/formatAmount'

/** Retail zap + pair-manage copy (GitLab #533 / #559 / #489 / #660). Labels stay ≤ ~5 words. */

export const ONE_SIDED_ADD_TITLE = 'Zap Add'
export const ONE_SIDED_WITHDRAW_TITLE = 'Zap Withdraw'
export const ONE_SIDED_TOKEN_LABEL = 'Token'
export const ONE_SIDED_AMOUNT_LABEL = 'Amount'
export const ONE_SIDED_LP_LABEL = 'LP'
export const ONE_SIDED_WITHDRAW_AS_LABEL = 'Withdraw as'

export const POOL_MANAGE_PROVIDE_LABEL = 'Provide Liquidity'
export const POOL_MANAGE_WITHDRAW_LABEL = 'Withdraw Liquidity'
export const POOL_MANAGE_ACTIONS_TESTID = 'pool-manage-actions'

export const ONE_SIDED_NO_ROUTE = 'No route.'
export const ONE_SIDED_DUST = 'Amount too small.'
export const ONE_SIDED_NO_HOLDINGS = 'No tokens in wallet.'
export const ONE_SIDED_NO_LP = 'No LP in wallet.'
export const ONE_SIDED_STALE_QUOTE = 'Quote updating…'
export const ONE_SIDED_WRAP_PAUSED = 'Wrapping is Temporarily Paused'

/** AC7 / T-Z12: pre-sign min-swap is human units, never a raw uint like `500571`. */
export function formatHumanMinSwapLine(minReturnRaw: string, askDecimals: number): string {
  return `min swap ${formatTokenAmount(minReturnRaw, askDecimals)}`
}

/** Pay line (already human) + human min-swap for Zap Add pre-sign. */
export function oneSidedAddPreSignAmountLines(
  payHuman: string,
  minReturnRaw: string,
  askDecimals: number
): [string, string] {
  return [`${payHuman} in`, formatHumanMinSwapLine(minReturnRaw, askDecimals)]
}
