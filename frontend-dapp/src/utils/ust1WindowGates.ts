/**
 * Submit gates for `/ust1` oracle window UI (GitLab #506).
 * All pause / stale / limit checks derive from on-chain `effective_swap` fields.
 */

import { depositVfdusdToUst1, withdrawGrossUst1ToVfdusd } from '@/utils/ust1WindowMath'

export type Ust1WindowDirection = 'deposit' | 'withdraw'

export type Ust1EffectiveSwapView = {
  fee_bps: number
  per_tx_ust1_limit: string
  rolling_24h_ust1_limit: string
  paused: boolean
  rolling_window_start_sec: number
  rolling_volume_ust1: string
  max_oracle_age_sec: number
  oracle: {
    rate: string
    last_update_sec: number
    paused: boolean
  }
}

export type Ust1SubmitBlockReason =
  | 'window_unavailable'
  | 'wallet_disconnected'
  | 'amount_empty'
  | 'amount_invalid'
  | 'window_paused'
  | 'oracle_paused'
  | 'oracle_stale'
  | 'zero_output'
  | 'per_tx_limit'
  | 'rolling_limit'
  | 'insufficient_balance'
  | 'quote_loading'
  | 'quote_error'
  | 'submitting'

const ROLLING_WINDOW_SECS = 86_400

/** Effective UST1 notional already used in the current rolling window (resets after 24h). */
export function effectiveRollingVolumeUst1(
  view: Pick<Ust1EffectiveSwapView, 'rolling_window_start_sec' | 'rolling_volume_ust1'>,
  nowSec: number
): bigint {
  const start = view.rolling_window_start_sec
  if (start === 0 || nowSec >= start + ROLLING_WINDOW_SECS) return 0n
  try {
    return BigInt(view.rolling_volume_ust1)
  } catch {
    return 0n
  }
}

export function rollingRemainingUst1(view: Ust1EffectiveSwapView, nowSec: number): bigint {
  const limit = BigInt(view.rolling_24h_ust1_limit)
  const used = effectiveRollingVolumeUst1(view, nowSec)
  return limit > used ? limit - used : 0n
}

/**
 * Oracle age check using wall-clock seconds (LCD block time not required for UI gate).
 * Chain still enforces with `env.block.time`.
 */
export function isOracleStale(
  view: Pick<Ust1EffectiveSwapView, 'max_oracle_age_sec' | 'oracle'>,
  nowSec: number
): boolean {
  const last = view.oracle.last_update_sec
  if (last === 0) return true
  return nowSec - last > view.max_oracle_age_sec
}

export function ust1NotionalForAmount(
  direction: Ust1WindowDirection,
  amountRaw: bigint,
  rate: bigint,
  feeBps: number
): { ust1Notional: bigint; receiveRaw: bigint } {
  if (direction === 'deposit') {
    const receiveRaw = depositVfdusdToUst1(amountRaw, rate, feeBps)
    return { ust1Notional: receiveRaw, receiveRaw }
  }
  const receiveRaw = withdrawGrossUst1ToVfdusd(amountRaw, rate, feeBps)
  return { ust1Notional: amountRaw, receiveRaw }
}

export type EvaluateUst1SubmitGateInput = {
  windowEnabled: boolean
  walletConnected: boolean
  direction: Ust1WindowDirection
  amountRaw: bigint | null
  amountDraftEmpty: boolean
  balanceRaw: bigint | null
  view: Ust1EffectiveSwapView | undefined
  viewLoading: boolean
  viewError: boolean
  submitting: boolean
  nowSec: number
}

export type EvaluateUst1SubmitGateResult = {
  canSubmit: boolean
  reason: Ust1SubmitBlockReason | null
  receiveRaw: bigint | null
  ust1Notional: bigint | null
  ctaLabel: string
  statusMessage: string | null
}

export function ust1CtaLabel(direction: Ust1WindowDirection, reason: Ust1SubmitBlockReason | null): string {
  if (reason === 'wallet_disconnected') return 'Connect wallet'
  if (reason === 'window_paused') return 'Window paused'
  if (reason === 'oracle_paused') return 'Oracle paused'
  if (reason === 'oracle_stale') return 'Oracle stale'
  if (reason === 'per_tx_limit') return 'Over per-tx limit'
  if (reason === 'rolling_limit') return 'Over 24h limit'
  if (reason === 'insufficient_balance') return 'Insufficient balance'
  if (reason === 'zero_output') return 'Amount too small'
  if (reason === 'submitting') return direction === 'deposit' ? 'Depositing…' : 'Withdrawing…'
  if (reason === 'quote_loading') return 'Loading…'
  if (reason === 'quote_error' || reason === 'window_unavailable') return 'Unavailable'
  if (reason === 'amount_empty' || reason === 'amount_invalid') {
    return direction === 'deposit' ? 'Deposit' : 'Withdraw'
  }
  return direction === 'deposit' ? 'Deposit' : 'Withdraw'
}

export function ust1StatusMessage(reason: Ust1SubmitBlockReason | null): string | null {
  switch (reason) {
    case 'window_paused':
      return 'Window is paused.'
    case 'oracle_paused':
      return 'Oracle is paused.'
    case 'oracle_stale':
      return 'Oracle rate is stale.'
    case 'per_tx_limit':
      return 'Amount exceeds the per-transaction UST1 limit.'
    case 'rolling_limit':
      return 'Amount exceeds remaining 24h UST1 capacity.'
    case 'insufficient_balance':
      return 'Balance is too low for this amount.'
    case 'zero_output':
      return 'Output would be zero after fees.'
    case 'window_unavailable':
      return 'UST1 window is not configured.'
    case 'quote_error':
      return 'Could not load window quote.'
    default:
      return null
  }
}

export function evaluateUst1SubmitGate(input: EvaluateUst1SubmitGateInput): EvaluateUst1SubmitGateResult {
  const blocked = (reason: Ust1SubmitBlockReason): EvaluateUst1SubmitGateResult => ({
    canSubmit: false,
    reason,
    receiveRaw: null,
    ust1Notional: null,
    ctaLabel: ust1CtaLabel(input.direction, reason),
    statusMessage: ust1StatusMessage(reason),
  })

  if (!input.windowEnabled) return blocked('window_unavailable')
  if (input.viewLoading) return blocked('quote_loading')
  if (input.viewError || !input.view) return blocked('quote_error')
  if (input.view.paused) return blocked('window_paused')
  if (input.view.oracle.paused) return blocked('oracle_paused')
  if (isOracleStale(input.view, input.nowSec)) return blocked('oracle_stale')
  if (!input.walletConnected) return blocked('wallet_disconnected')
  if (input.submitting) return blocked('submitting')
  if (input.amountDraftEmpty) return blocked('amount_empty')
  if (input.amountRaw === null || input.amountRaw <= 0n) return blocked('amount_invalid')

  let rate: bigint
  try {
    rate = BigInt(input.view.oracle.rate)
  } catch {
    return blocked('quote_error')
  }
  if (rate === 0n) return blocked('oracle_stale')

  let ust1Notional: bigint
  let receiveRaw: bigint
  try {
    ;({ ust1Notional, receiveRaw } = ust1NotionalForAmount(input.direction, input.amountRaw, rate, input.view.fee_bps))
  } catch {
    return blocked('quote_error')
  }

  if (receiveRaw <= 0n) return blocked('zero_output')

  const perTx = BigInt(input.view.per_tx_ust1_limit)
  if (ust1Notional > perTx) return blocked('per_tx_limit')

  const remaining = rollingRemainingUst1(input.view, input.nowSec)
  if (ust1Notional > remaining) return blocked('rolling_limit')

  if (input.balanceRaw !== null && input.amountRaw > input.balanceRaw) {
    return blocked('insufficient_balance')
  }

  return {
    canSubmit: true,
    reason: null,
    receiveRaw,
    ust1Notional,
    ctaLabel: ust1CtaLabel(input.direction, null),
    statusMessage: null,
  }
}
