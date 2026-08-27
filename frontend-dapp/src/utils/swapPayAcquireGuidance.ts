/**
 * Swap / Trade market pay-shortfall + funded high-impact guidance (GitLab #678).
 *
 * Pure helper — no React, no LCD. Pages pass LCD `getTokenBalance` / `effective_swap`
 * results. Identity is contract address (or native denom), never a ticker string.
 */

import { isPositiveDecimalAmount } from '@/utils/decimalAmountInput'
import { fromRawAmount } from '@/utils/formatAmount'
import { isOracleStale, rollingRemainingUst1, type Ust1EffectiveSwapView } from '@/utils/ust1WindowGates'
import { vfdusdInForTargetUst1 } from '@/utils/ust1WindowMath'

/** Same 5% confirm-again / size-warning threshold as Swap today (#293 / #497). */
export const SWAP_FUNDED_HIGH_IMPACT_PCT = 5
/** Conservative reduce: typed pay / this divisor, never above spendable. */
export const SWAP_ACQUIRE_REDUCE_DIVISOR = 10n
export const UST1_PAY_DECIMALS_DEFAULT = 6

export const SWAP_ACQUIRE_GUIDE_UST1_PATH = '/ust1'
export const SWAP_ACQUIRE_GUIDE_WRAP_PATH = '/wrap'

export const SWAP_ACQUIRE_COPY = {
  disconnectedQuoteOnly: 'Quote only',
  insufficientGeneric: (symbol: string) => `You don't have enough ${symbol}.`,
  insufficientUst1Window: (payHuman: string, vfdusdHuman: string) =>
    `You don't have ${payHuman} UST1. Deposit about ${vfdusdHuman} vFDUSD on UST1 (window max this tx).`,
  insufficientUst1OverWindow: 'The UST1 window cannot mint this size.',
  insufficientWrap: (symbol: string) => `You don't have enough ${symbol}.`,
  highImpact: 'This size moves the pool. Try a smaller amount.',
  guideUst1: 'Get UST1',
  guideWrap: 'Wrap',
} as const

export type SwapPayAcquireKind =
  | 'ok'
  | 'disconnected_quote'
  | 'insufficient_generic'
  | 'insufficient_ust1_window'
  | 'insufficient_ust1_over_window'
  | 'insufficient_wrap'
  | 'high_impact'

export type SwapPayAcquireGuidance = {
  kind: SwapPayAcquireKind
  message: string | null
  guideHref: string | null
  guideLabel: string | null
  reduceToHuman: string | null
  suggestedVfdusdHuman: string | null
}

export type EvaluateSwapPayAcquireInput = {
  walletConnected: boolean
  hasPositivePay: boolean
  hasSettledQuote: boolean
  payAsset: string
  paySymbol: string
  payDecimals: number
  payRaw: bigint | null
  payBalanceRaw: bigint | null
  vfdusdBalanceRaw: bigint | null
  ust1TokenAddress: string
  windowEnabled: boolean
  windowView: Ust1EffectiveSwapView | null
  windowViewError: boolean
  wrapEnabled: boolean
  wrappedPayAssets: ReadonlySet<string>
  expectedSlippagePct: number | null
  nowSec: number
}

const OK: SwapPayAcquireGuidance = {
  kind: 'ok',
  message: null,
  guideHref: null,
  guideLabel: null,
  reduceToHuman: null,
  suggestedVfdusdHuman: null,
}

export function isUst1PayAsset(payAsset: string, ust1TokenAddress: string): boolean {
  const pay = payAsset.trim()
  const ust1 = ust1TokenAddress.trim()
  return pay.length > 0 && ust1.length > 0 && pay === ust1
}

export function buildUst1DepositHref(amountHuman: string | null): string {
  if (amountHuman && isPositiveDecimalAmount(amountHuman)) {
    return `${SWAP_ACQUIRE_GUIDE_UST1_PATH}?direction=deposit&amount=${encodeURIComponent(amountHuman)}`
  }
  return `${SWAP_ACQUIRE_GUIDE_UST1_PATH}?direction=deposit`
}

/** Same-origin Guide targets only (`/ust1` + safe query, or `/wrap`). */
export function isAllowedAcquireHref(href: string): boolean {
  if (href === SWAP_ACQUIRE_GUIDE_WRAP_PATH) return true
  if (href === SWAP_ACQUIRE_GUIDE_UST1_PATH || href.startsWith(`${SWAP_ACQUIRE_GUIDE_UST1_PATH}?`)) {
    if (href.includes('://') || href.includes('//') || /[<>]|javascript:/i.test(href)) return false
    return true
  }
  return false
}

export function formatAcquireHumanAmount(raw: bigint, decimals: number): string {
  const human = fromRawAmount(raw.toString(), decimals)
  const [intPart, frac] = human.split('.')
  const grouped = intPart.replace(/\B(?=(\d{3})+(?!\d))/g, ',')
  return frac ? `${grouped}.${frac}` : grouped
}

function result(
  kind: SwapPayAcquireKind,
  patch: Partial<Omit<SwapPayAcquireGuidance, 'kind'>>
): SwapPayAcquireGuidance {
  return {
    kind,
    message: patch.message ?? null,
    guideHref: patch.guideHref ?? null,
    guideLabel: patch.guideLabel ?? null,
    reduceToHuman: patch.reduceToHuman ?? null,
    suggestedVfdusdHuman: patch.suggestedVfdusdHuman ?? null,
  }
}

function windowViewUsable(input: EvaluateSwapPayAcquireInput): Ust1EffectiveSwapView | null {
  if (!input.windowEnabled || input.windowViewError || !input.windowView) return null
  const view = input.windowView
  if (view.paused || view.oracle.paused || isOracleStale(view, input.nowSec)) return null
  try {
    if (BigInt(view.oracle.rate) === 0n) return null
  } catch {
    return null
  }
  return view
}

function ust1ShortfallGuidance(input: EvaluateSwapPayAcquireInput): SwapPayAcquireGuidance {
  const view = windowViewUsable(input)
  if (!view || input.payRaw === null || input.payBalanceRaw === null) {
    return result('insufficient_generic', {
      message: SWAP_ACQUIRE_COPY.insufficientGeneric(input.paySymbol),
    })
  }
  const shortfall = input.payRaw - input.payBalanceRaw
  if (shortfall <= 0n) return OK
  let rate: bigint
  try {
    rate = BigInt(view.oracle.rate)
  } catch {
    return result('insufficient_generic', {
      message: SWAP_ACQUIRE_COPY.insufficientGeneric(input.paySymbol),
    })
  }
  const perTx = (() => {
    try {
      return BigInt(view.per_tx_ust1_limit)
    } catch {
      return null
    }
  })()
  if (perTx === null) {
    return result('insufficient_generic', {
      message: SWAP_ACQUIRE_COPY.insufficientGeneric(input.paySymbol),
    })
  }
  const remaining = rollingRemainingUst1(view, input.nowSec)
  const cap = perTx < remaining ? perTx : remaining
  const payHuman = formatAcquireHumanAmount(input.payRaw, input.payDecimals)

  if (cap <= 0n || shortfall > cap) {
    const cappedVfdusd = cap > 0n ? vfdusdInForTargetUst1(cap, rate, view.fee_bps) : null
    const amountHuman =
      cappedVfdusd !== null && cappedVfdusd > 0n
        ? fromRawAmount(cappedVfdusd.toString(), UST1_PAY_DECIMALS_DEFAULT)
        : null
    return result('insufficient_ust1_over_window', {
      message: SWAP_ACQUIRE_COPY.insufficientUst1OverWindow,
      guideHref: buildUst1DepositHref(amountHuman),
      guideLabel: SWAP_ACQUIRE_COPY.guideUst1,
      suggestedVfdusdHuman: amountHuman,
    })
  }

  const vfdusd = vfdusdInForTargetUst1(shortfall, rate, view.fee_bps)
  if (vfdusd === null) {
    return result('insufficient_generic', {
      message: SWAP_ACQUIRE_COPY.insufficientGeneric(input.paySymbol),
    })
  }
  const amountHuman = fromRawAmount(vfdusd.toString(), UST1_PAY_DECIMALS_DEFAULT)
  const vfdusdDisplay = formatAcquireHumanAmount(vfdusd, UST1_PAY_DECIMALS_DEFAULT)
  return result('insufficient_ust1_window', {
    message: SWAP_ACQUIRE_COPY.insufficientUst1Window(payHuman, vfdusdDisplay),
    guideHref: buildUst1DepositHref(amountHuman),
    guideLabel: SWAP_ACQUIRE_COPY.guideUst1,
    suggestedVfdusdHuman: amountHuman,
  })
}

function fundedReduceToHuman(input: EvaluateSwapPayAcquireInput): string | null {
  if (input.payRaw === null || input.payBalanceRaw === null) return null
  if (input.payRaw <= 0n || input.payBalanceRaw <= 0n) return null
  const tenth = input.payRaw / SWAP_ACQUIRE_REDUCE_DIVISOR
  const capped = tenth < input.payBalanceRaw ? tenth : input.payBalanceRaw
  if (capped <= 0n || capped >= input.payRaw) return null
  return fromRawAmount(capped.toString(), input.payDecimals)
}

export function evaluateSwapPayAcquireGuidance(input: EvaluateSwapPayAcquireInput): SwapPayAcquireGuidance {
  void input.vfdusdBalanceRaw
  void input.hasSettledQuote
  if (!input.hasPositivePay) return OK

  if (!input.walletConnected) {
    return result('disconnected_quote', { message: null })
  }

  const short = input.payRaw !== null && input.payBalanceRaw !== null && input.payRaw > input.payBalanceRaw

  if (short) {
    if (isUst1PayAsset(input.payAsset, input.ust1TokenAddress)) {
      return ust1ShortfallGuidance(input)
    }
    if (input.wrapEnabled && input.wrappedPayAssets.has(input.payAsset)) {
      return result('insufficient_wrap', {
        message: SWAP_ACQUIRE_COPY.insufficientWrap(input.paySymbol),
        guideHref: SWAP_ACQUIRE_GUIDE_WRAP_PATH,
        guideLabel: SWAP_ACQUIRE_COPY.guideWrap,
      })
    }
    return result('insufficient_generic', {
      message: SWAP_ACQUIRE_COPY.insufficientGeneric(input.paySymbol),
    })
  }

  const funded = input.payRaw !== null && input.payBalanceRaw !== null && input.payRaw <= input.payBalanceRaw
  if (funded && input.expectedSlippagePct != null && input.expectedSlippagePct > SWAP_FUNDED_HIGH_IMPACT_PCT) {
    return result('high_impact', {
      message: SWAP_ACQUIRE_COPY.highImpact,
      reduceToHuman: fundedReduceToHuman(input),
    })
  }

  return OK
}

export function acquireGuidanceShowsQuoteOnly(g: SwapPayAcquireGuidance, hasSettledQuote: boolean): boolean {
  return g.kind === 'disconnected_quote' && hasSettledQuote
}

export function acquireGuidanceBlocksSubmit(g: SwapPayAcquireGuidance): boolean {
  return (
    g.kind === 'insufficient_generic' ||
    g.kind === 'insufficient_ust1_window' ||
    g.kind === 'insufficient_ust1_over_window' ||
    g.kind === 'insufficient_wrap'
  )
}
