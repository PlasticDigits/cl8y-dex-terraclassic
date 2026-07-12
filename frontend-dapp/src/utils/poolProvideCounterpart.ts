import type { PoolResponse } from '@/types'
import { fromRawAmount, toRawAmount } from '@/utils/formatAmount'
import {
  grossUlunaForTargetNet,
  netUlunaAfterTransferTax,
  type NativeTransferTaxParams,
} from '@/utils/nativeTransferTax'
import { computeProportionalCounterpartRaw } from '@/utils/provideLiquidityEstimate'

export type ComputeProvideCounterpartHumanArgs = {
  editedSide: 'a' | 'b'
  editedHuman: string
  pool: Pick<PoolResponse, 'assets'> | null | undefined
  decimalsA: number
  decimalsB: number
  needsWrapA: boolean
  needsWrapB: boolean
  taxParamsA?: NativeTransferTaxParams | null
  taxParamsB?: NativeTransferTaxParams | null
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
  taxParamsA?: NativeTransferTaxParams | null,
  taxParamsB?: NativeTransferTaxParams | null
): string | null {
  const decimals = editedSide === 'a' ? decimalsA : decimalsB
  const needsWrap = editedSide === 'a' ? needsWrapA : needsWrapB
  const taxParams = editedSide === 'a' ? taxParamsA : taxParamsB
  const raw = toRawAmount(editedHuman, decimals)
  if (raw === '0') return null
  if (needsWrap) {
    if (!taxParams) return null
    return netUlunaAfterTransferTax(BigInt(raw), taxParams).toString()
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
  taxParamsA?: NativeTransferTaxParams | null,
  taxParamsB?: NativeTransferTaxParams | null
): string | null {
  const decimals = counterpartSide === 'a' ? decimalsA : decimalsB
  const needsWrap = counterpartSide === 'a' ? needsWrapA : needsWrapB
  const taxParams = counterpartSide === 'a' ? taxParamsA : taxParamsB
  if (needsWrap) {
    if (!taxParams) return null
    const gross = grossUlunaForTargetNet(BigInt(counterpartNetRaw), taxParams)
    return fromRawAmount(gross.toString(), decimals)
  }
  return fromRawAmount(counterpartNetRaw, decimals)
}

/**
 * Human-string counterpart for provide liquidity auto-fill.
 * Ratio math uses net post-tax amounts when native wrap is enabled (same as `provideRawAdd*`).
 */
export function computeProvideCounterpartHuman({
  editedSide,
  editedHuman,
  pool,
  decimalsA,
  decimalsB,
  needsWrapA,
  needsWrapB,
  taxParamsA,
  taxParamsB,
}: ComputeProvideCounterpartHumanArgs): string | null {
  if (isDraftAmount(editedHuman) || !pool) return null

  const netRaw = editedNetRaw(
    editedSide,
    editedHuman,
    decimalsA,
    decimalsB,
    needsWrapA,
    needsWrapB,
    taxParamsA,
    taxParamsB
  )
  if (netRaw === null) return null

  const counterpartNetRaw = computeProportionalCounterpartRaw(editedSide, netRaw, pool)
  if (counterpartNetRaw === null) return null

  const counterpartSide = editedSide === 'a' ? 'b' : 'a'
  return counterpartHumanFromNetRaw(
    counterpartSide,
    counterpartNetRaw,
    decimalsA,
    decimalsB,
    needsWrapA,
    needsWrapB,
    taxParamsA,
    taxParamsB
  )
}
