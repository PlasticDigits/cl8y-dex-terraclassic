import type { PoolResponse } from '@/types'
import { fromRawAmount, toRawAmount } from '@/utils/formatAmount'
import { amountForTargetNetAfterWrapMapperFee, netAfterWrapMapperFee } from '@/services/terraclassic/wrapMapper'
import { computeProportionalCounterpartRaw } from '@/utils/provideLiquidityEstimate'

export type ComputeProvideCounterpartHumanArgs = {
  editedSide: 'a' | 'b'
  editedHuman: string
  pool: Pick<PoolResponse, 'assets'> | null | undefined
  decimalsA: number
  decimalsB: number
  needsWrapA: boolean
  needsWrapB: boolean
  /**
   * @deprecated Burn tax does not apply to wrap_deposit mint (#512). Kept optional for call-site
   * compatibility; ignored when computing wrap nets.
   */
  taxParamsA?: unknown
  /**
   * @deprecated See `taxParamsA`.
   */
  taxParamsB?: unknown
  /** Wrap-mapper `fee_wrap_bps` only (0 when unset / unknown). GitLab #507 / #516. */
  wrapMapperFeeBps?: number
}

function isDraftAmount(human: string): boolean {
  const trimmed = human.trim()
  return !trimmed || trimmed === '.' || trimmed === '0'
}

function editedNetRaw(
  editedSide: 'a' | 'b',
  editedHuman: string,
  decimalsA: number,
  decimalsB: number,
  needsWrapA: boolean,
  needsWrapB: boolean,
  wrapMapperFeeBps = 0
): string | null {
  const decimals = editedSide === 'a' ? decimalsA : decimalsB
  const needsWrap = editedSide === 'a' ? needsWrapA : needsWrapB
  const raw = toRawAmount(editedHuman, decimals)
  if (raw === '0') return null
  if (needsWrap) {
    // wrap_deposit is untaxed MsgExecuteContract; net CW20 = after mapper fee only (#512).
    return netAfterWrapMapperFee(BigInt(raw), wrapMapperFeeBps).toString()
  }
  return raw
}

function counterpartHumanFromNetRaw(
  counterpartSide: 'a' | 'b',
  counterpartNetRaw: string,
  decimalsA: number,
  decimalsB: number,
  needsWrapA: boolean,
  needsWrapB: boolean,
  wrapMapperFeeBps = 0
): string | null {
  const decimals = counterpartSide === 'a' ? decimalsA : decimalsB
  const needsWrap = counterpartSide === 'a' ? needsWrapA : needsWrapB
  if (needsWrap) {
    const gross = amountForTargetNetAfterWrapMapperFee(BigInt(counterpartNetRaw), wrapMapperFeeBps)
    return fromRawAmount(gross.toString(), decimals)
  }
  return fromRawAmount(counterpartNetRaw, decimals)
}

/**
 * Human-string counterpart for provide liquidity auto-fill.
 * Ratio math uses post–wrap-fee amounts when native wrap is enabled
 * (same as `provideRawAdd*`; GitLab #507 / #512).
 */
export function computeProvideCounterpartHuman(args: ComputeProvideCounterpartHumanArgs): string | null {
  const { editedSide, editedHuman, pool, decimalsA, decimalsB, needsWrapA, needsWrapB, wrapMapperFeeBps = 0 } = args

  if (isDraftAmount(editedHuman) || !pool) return null

  const editedNet = editedNetRaw(
    editedSide,
    editedHuman,
    decimalsA,
    decimalsB,
    needsWrapA,
    needsWrapB,
    wrapMapperFeeBps
  )
  if (editedNet == null) return null

  const counterpartNet = computeProportionalCounterpartRaw(editedSide, editedNet, pool)
  if (counterpartNet == null || counterpartNet === '0') return null

  const counterpartSide = editedSide === 'a' ? 'b' : 'a'
  return counterpartHumanFromNetRaw(
    counterpartSide,
    counterpartNet,
    decimalsA,
    decimalsB,
    needsWrapA,
    needsWrapB,
    wrapMapperFeeBps
  )
}
