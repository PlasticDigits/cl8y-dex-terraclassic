import { tryParseBigInt } from '@/utils/decimalAmountInput'
import { toRawAmount } from '@/utils/formatAmount'

const BPS_SCALE = 10000n

/** Clamp slippage percent to [0, 100] and convert to basis points (floor-friendly integer). */
export function slippagePercentToBps(slippagePercent: number): number {
  if (!Number.isFinite(slippagePercent)) return 0
  return Math.min(10000, Math.max(0, Math.round(slippagePercent * 100)))
}

/**
 * Apply a basis-point discount with floor semantics (pool-favorable protection).
 * `(raw * (10000 - bps)) / 10000` — never rounds up.
 *
 * @returns floored raw string, or `null` when input is not a valid uint string
 */
export function applyBpsFloor(rawAmount: string, bps: number): string | null {
  const raw = tryParseBigInt(rawAmount)
  if (raw === null) return null
  const clampedBps = Math.min(10000, Math.max(0, Math.trunc(bps)))
  const factor = BPS_SCALE - BigInt(clampedBps)
  return ((raw * factor) / BPS_SCALE).toString()
}

/**
 * Minimum receive after slippage tolerance (percent, e.g. `1` = 1%).
 * Used for swap `min_received` / `max_spread` protection args.
 */
export function applySlippagePercentFloor(rawAmount: string, slippagePercent: number): string | null {
  return applyBpsFloor(rawAmount, slippagePercentToBps(slippagePercent))
}

/**
 * Minimum token A/B received on LP withdraw with slippage protection.
 * Mirrors `(reserve * lpShare / totalLp) * (1 - slippage%)` using BigInt floor math.
 */
export function withdrawMinAssetAmounts(
  lpRaw: string,
  totalLp: string,
  reserveA: string,
  reserveB: string,
  slippagePercent: number
): [string, string] | null {
  const lp = tryParseBigInt(lpRaw)
  const total = tryParseBigInt(totalLp)
  const resA = tryParseBigInt(reserveA)
  const resB = tryParseBigInt(reserveB)
  if (lp === null || total === null || resA === null || resB === null) return null
  if (total === 0n || lp === 0n) return null

  const factor = BPS_SCALE - BigInt(slippagePercentToBps(slippagePercent))
  const denom = total * BPS_SCALE
  const minA = (resA * lp * factor) / denom
  const minB = (resB * lp * factor) / denom
  return [minA.toString(), minB.toString()]
}

/** Compare raw LP burn amount to on-chain LP balance (both uint strings). */
export function isLpBurnExceedsBalance(lpAmountHuman: string, lpDecimals: number, lpBalanceRaw: string): boolean {
  if (!lpAmountHuman) return false
  const burnRaw = tryParseBigInt(toRawAmount(lpAmountHuman, lpDecimals))
  const balance = tryParseBigInt(lpBalanceRaw)
  if (burnRaw === null || balance === null || burnRaw === 0n) return false
  return burnRaw > balance
}
