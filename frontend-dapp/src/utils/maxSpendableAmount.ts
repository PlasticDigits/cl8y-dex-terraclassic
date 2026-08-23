import {
  estimateLimitOrderPlaceSequenceUlunaFeesTotal,
  estimateMarketPairSwapSequenceUlunaFeesTotal,
  estimateNativeSwapUlunaFeesTotal,
  estimateProvideLiquidityCw20SequenceUlunaFeesTotal,
  estimateProvideLiquidityNativeWrapUlunaFeesTotal,
  estimateZapInUlunaFeesTotal,
  estimateZapOutUlunaFeesTotal,
} from '@/services/terraclassic/transactions'
import { fromRawAmount } from '@/utils/formatAmount'
import { applyExtraDebitSellCap } from '@/utils/taxPreviewMaxSpend'
import { isDecimalAmountDraft } from '@/utils/decimalAmountInput'

/** Fee envelope selector for one-click Max ([GitLab #213](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/213)). */
export type MaxAmountContext =
  | 'swap_native'
  | 'swap_cw20'
  | 'limit_place'
  | 'market_swap'
  | 'provide_liquidity_native_side'
  | 'provide_liquidity_cw20'
  | 'zap_in'
  | 'zap_out'
  | 'book_leg'

export type NativeSwapMaxHints = {
  isDirectWrap: boolean
  needsWrapInput: boolean
  /** Router `unwrap_output` sub-message (CW20→native, GitLab #343). */
  needsUnwrapOutput?: boolean
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
  /** `zap_in`: wrap deposits (0/1) and optional route-in hops. */
  zapInHints?: { wrapDeposits?: 0 | 1; routeHops?: number }
  /** `zap_out`: include unwrap mapper send. */
  zapOutUnwrap?: boolean
  /** `market_swap` / `swap_cw20` hybrid path uses hybrid swap gas envelope. */
  marketUsesHybrid?: boolean
  /** `book_leg`: cap to pay amount (raw micro-units) in addition to balance. */
  payAmountRaw?: string
  /** Limit place batch rung count (default 1). */
  limitPlaceRungCount?: number
  /** Community tax extra-debit sell (#593). Caps declared so debit ≤ spendable. */
  extraDebitSellBps?: number | null
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
    | 'nativeSwapHints'
    | 'nativeWrapDepositCount'
    | 'marketUsesHybrid'
    | 'limitPlaceRungCount'
    | 'zapInHints'
    | 'zapOutUnwrap'
  > = {}
): bigint {
  switch (context) {
    case 'swap_native': {
      const hints = options.nativeSwapHints ?? { isDirectWrap: false, needsWrapInput: true }
      return estimateNativeSwapUlunaFeesTotal(hints)
    }
    case 'provide_liquidity_native_side':
      return estimateProvideLiquidityNativeWrapUlunaFeesTotal(options.nativeWrapDepositCount ?? 1)
    case 'zap_in':
      return estimateZapInUlunaFeesTotal(options.zapInHints ?? { wrapDeposits: 1, routeHops: 0 })
    case 'zap_out':
      return estimateZapOutUlunaFeesTotal({ unwrap: options.zapOutUnwrap === true })
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

  if (input.extraDebitSellBps != null && input.extraDebitSellBps > 0) {
    spendableRaw = applyExtraDebitSellCap(spendableRaw, input.extraDebitSellBps)
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
