import { toRawAmount } from '@/utils/formatAmount'
import { makerPlacementFeeBps } from '@/utils/limitOrderFeeSummary'
import { parsePositivePriceHuman } from '@/utils/limitOrderPriceReference'

/**
 * Human-scale counter-asset if the resting limit fully fills at the typed price.
 * Mirrors placement escrow after maker fee (`floor(amount × maker_bps / 10000)`) and
 * on-chain fill math: bid → token0 = floor(remaining_token1 / price); ask → token1 = floor(remaining_token0 × price).
 * Returns a plain decimal for notional calculations; rendering owns the display preference.
 */
export function limitOrderExpectedReceiveHuman(params: {
  side: 'bid' | 'ask'
  escrowAmountHuman: string
  escrowDecimals: number
  priceHuman: string
  effectiveFeeBps: number | null
}): string | null {
  const price = parsePositivePriceHuman(params.priceHuman)
  if (price == null) return null

  let escrowRaw: bigint
  try {
    escrowRaw = BigInt(toRawAmount(params.escrowAmountHuman, params.escrowDecimals))
  } catch {
    return null
  }
  if (escrowRaw <= 0n) return null

  const makerBps = params.effectiveFeeBps != null ? makerPlacementFeeBps(params.effectiveFeeBps) : 0
  const makerFeeRaw = (escrowRaw * BigInt(makerBps)) / 10000n
  const remainingRaw = escrowRaw - makerFeeRaw
  if (remainingRaw <= 0n) return null

  const remainingHuman = Number(remainingRaw) / 10 ** params.escrowDecimals
  if (!Number.isFinite(remainingHuman) || remainingHuman <= 0) return null

  const receiveHuman =
    params.side === 'bid'
      ? Math.floor((remainingHuman / price) * 1e12) / 1e12
      : Math.floor(remainingHuman * price * 1e12) / 1e12

  if (!Number.isFinite(receiveHuman) || receiveHuman <= 0) return null
  return receiveHuman.toLocaleString('en-US', { useGrouping: false, maximumFractionDigits: 12 })
}
