/**
 * `/ust1` acquire prefill from Swap/Trade Guide links (GitLab #678).
 *
 * Only `direction` + `amount` are read. Hostile / scientific / negative values are ignored.
 * Deposit `amount` is human vFDUSD and is clamped to remaining window UST1 capacity
 * before apply — never auto-submit.
 */

import { isPositiveDecimalAmount } from '@/utils/decimalAmountInput'
import { fromRawAmount, toRawAmount } from '@/utils/formatAmount'
import {
  isOracleStale,
  rollingRemainingUst1,
  type Ust1EffectiveSwapView,
  type Ust1WindowDirection,
} from '@/utils/ust1WindowGates'
import { depositVfdusdToUst1, vfdusdInForTargetUst1 } from '@/utils/ust1WindowMath'

export const UST1_ACQUIRE_PREFILL_DECIMALS = 6
/** Reject absurd query lengths before bigint work (A10 / T3). */
export const UST1_ACQUIRE_PREFILL_AMOUNT_MAX_CHARS = 24

export type Ust1AcquirePrefillParse = {
  direction: Ust1WindowDirection | null
  amountHuman: string | null
}

export function parseUst1AcquirePrefill(params: { get: (key: string) => string | null }): Ust1AcquirePrefillParse {
  const directionRaw = (params.get('direction') ?? '').trim().toLowerCase()
  const direction: Ust1WindowDirection | null =
    directionRaw === 'deposit' || directionRaw === 'withdraw' ? directionRaw : null
  const amountRaw = (params.get('amount') ?? '').trim()
  const amountHuman =
    amountRaw.length > 0 &&
    amountRaw.length <= UST1_ACQUIRE_PREFILL_AMOUNT_MAX_CHARS &&
    isPositiveDecimalAmount(amountRaw)
      ? amountRaw
      : null
  return { direction, amountHuman }
}

/**
 * Clamp a human vFDUSD deposit so UST1 notional ≤ min(per-tx, rolling remaining).
 * Returns `null` when the window view cannot be trusted (fail closed — do not apply).
 */
export function clampUst1DepositPrefillAmount(
  amountHuman: string,
  view: Ust1EffectiveSwapView,
  nowSec: number
): string | null {
  if (!isPositiveDecimalAmount(amountHuman)) return null
  if (view.paused || view.oracle.paused || isOracleStale(view, nowSec)) return null
  let rate: bigint
  try {
    rate = BigInt(view.oracle.rate)
  } catch {
    return null
  }
  if (rate === 0n) return null
  let payRaw: bigint
  try {
    payRaw = BigInt(toRawAmount(amountHuman, UST1_ACQUIRE_PREFILL_DECIMALS))
  } catch {
    return null
  }
  if (payRaw <= 0n) return null
  let notional: bigint
  try {
    notional = depositVfdusdToUst1(payRaw, rate, view.fee_bps)
  } catch {
    return null
  }
  const perTx = safeBigInt(view.per_tx_ust1_limit)
  if (perTx === null) return null
  const remaining = rollingRemainingUst1(view, nowSec)
  const cap = perTx < remaining ? perTx : remaining
  if (cap <= 0n) return null
  if (notional <= cap) return amountHuman
  const vfdusd = vfdusdInForTargetUst1(cap, rate, view.fee_bps)
  if (vfdusd === null || vfdusd <= 0n) return null
  return fromRawAmount(vfdusd.toString(), UST1_ACQUIRE_PREFILL_DECIMALS)
}

function safeBigInt(raw: string): bigint | null {
  try {
    return BigInt(raw)
  } catch {
    return null
  }
}
