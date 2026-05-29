import {
  estimateLimitOrderPlaceSequenceUlunaFeesTotal,
  estimateMarketPairSwapSequenceUlunaFeesTotal,
  estimateNativeSwapUlunaFeesTotal,
  estimateProvideLiquidityCw20SequenceUlunaFeesTotal,
  estimateProvideLiquidityNativeWrapUlunaFeesTotal,
} from '@/services/terraclassic/transactions'
import { fromRawAmount } from '@/utils/formatAmount'
import { isDecimalAmountDraft } from '@/utils/decimalAmountInput'

/** Fee envelope selector for one-click Max ([GitLab #213](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/213)). */
export type MaxAmountContext =
  | 'swap_native'
  | 'swap_cw20'
  | 'limit_place'
  | 'market_swap'
  | 'provide_liquidity_native_side'
  | 'provide_liquidity_cw20'
  | 'book_leg'

export type NativeSwapMaxHints = {
  isDirectWrap: boolean
  needsWrapInput: boolean
  hopCount?: number
}

export type ComputeMaxSpendableHumanAmountInput = {
  balanceRaw: string
  decimals: number
  assetIsNativeUluna: boolean
  context: MaxAmountContext
  /** Required for `swap_native` reserve sizing. */
  nativeSwapHints?: NativeSwapMaxHints
  /** `provide_liquidity_native_side`: number of `wrap_deposit` msgs in the combined tx (1 or 2). */
  nativeWrapDepositCount?: 1 | 2
  /** `market_swap` / `swap_cw20` hybrid path uses hybrid swap gas envelope. */
  marketUsesHybrid?: boolean
  /** `book_leg`: cap to pay amount (raw micro-units) in addition to balance. */
  payAmountRaw?: string
  /** Limit place batch rung count (default 1). */
  limitPlaceRungCount?: number
}

export type ComputeMaxSpendableHumanAmountResult = {
  human: string
  spendableRaw: bigint
  cappedByGas: boolean
  reserveUluna: bigint
}

function tryParseBalanceRaw(balanceRaw: string): bigint | null {
  try {
    if (!balanceRaw || !/^\d+$/.test(balanceRaw)) return null
    return BigInt(balanceRaw)
  } catch {
    return null
  }
}

function minBigInt(a: bigint, b: bigint): bigint {
  return a < b ? a : b
}

/** Uluna fee reserve for native Max — derived only from `transactions.ts` / `terraGas.ts` helpers. */
export function maxAmountReserveUlunaForContext(
  context: MaxAmountContext,
  options: Pick<
    ComputeMaxSpendableHumanAmountInput,
    'nativeSwapHints' | 'nativeWrapDepositCount' | 'marketUsesHybrid' | 'limitPlaceRungCount'
  > = {}
): bigint {
  switch (context) {
    case 'swap_native': {
      const hints = options.nativeSwapHints ?? { isDirectWrap: false, needsWrapInput: true }
      return estimateNativeSwapUlunaFeesTotal(hints)
    }
    case 'provide_liquidity_native_side':
      return estimateProvideLiquidityNativeWrapUlunaFeesTotal(options.nativeWrapDepositCount ?? 1)
    case 'limit_place':
      return estimateLimitOrderPlaceSequenceUlunaFeesTotal(options.limitPlaceRungCount ?? 1)
    case 'market_swap':
      return estimateMarketPairSwapSequenceUlunaFeesTotal(!!options.marketUsesHybrid)
    case 'provide_liquidity_cw20':
      return estimateProvideLiquidityCw20SequenceUlunaFeesTotal()
    case 'swap_cw20':
    case 'book_leg':
      return 0n
    default:
      return 0n
  }
}

/**
 * Compute the human Max amount for retail amount fields.
 * Native `uluna` pay subtracts the fee envelope for the action; CW20 Max stays full balance.
 */
export function computeMaxSpendableHumanAmount(
  input: ComputeMaxSpendableHumanAmountInput
): ComputeMaxSpendableHumanAmountResult {
  const balance = tryParseBalanceRaw(input.balanceRaw) ?? 0n
  let spendableRaw = balance
  let reserveUluna = 0n

  if (input.context === 'book_leg') {
    if (input.payAmountRaw) {
      const payCap = tryParseBalanceRaw(input.payAmountRaw)
      if (payCap !== null) spendableRaw = minBigInt(spendableRaw, payCap)
    }
  } else if (input.assetIsNativeUluna) {
    reserveUluna = maxAmountReserveUlunaForContext(input.context, input)
    spendableRaw = balance > reserveUluna ? balance - reserveUluna : 0n
  }

  const human = fromRawAmount(spendableRaw.toString(), input.decimals)
  const cappedByGas = input.assetIsNativeUluna && reserveUluna > 0n && spendableRaw < balance

  if (!isDecimalAmountDraft(human)) {
    return { human: '0', spendableRaw: 0n, cappedByGas, reserveUluna }
  }

  return { human, spendableRaw, cappedByGas, reserveUluna }
}

/** True when Max should be disabled (loading, error, zero spendable). */
export function isMaxSpendableActionDisabled(
  balanceQuery: { isLoading: boolean; isError: boolean; data?: string },
  spendableRaw: bigint
): boolean {
  return (
    balanceQuery.isLoading ||
    balanceQuery.isError ||
    !balanceQuery.data ||
    balanceQuery.data === '0' ||
    spendableRaw === 0n
  )
}
