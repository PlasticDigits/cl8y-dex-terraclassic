import { getPair } from './factory'
import { simulateHybridSwap, type QuoteTraderOptions } from './pair'
import { poolOnlyHybridParams } from './poolOnlyHybrid'
import type { SwapOperation } from './router'
import type { AssetInfo } from '@/types'
import {
  hybridMaxSpreadRealizedLegs,
  hybridNoBeliefMaterialPoolReject,
  hybridSpreadCmpAndTotal,
  spreadPercentOfGross,
  spreadRatioStrictlyExceedsMax,
} from '@/utils/swapMaxSpread'

export interface SwapRoutePreflightSpread {
  /** Max over hops of contract-style spread % (2 decimal places). */
  worstSpreadPercent: string
  /** True if any hop would fail the pair `assert_max_spread` check for this `maxSpread`. */
  anyHopExceedsMaxSpread: boolean
}

/**
 * Sequential per-hop pair `hybrid_simulation` to recover spread metrics omitted from
 * router `simulate_swap_operations` (which only returns the final amount).
 */
export async function preflightSwapRouteSpread(
  operations: SwapOperation[],
  offerAmount: string,
  maxSpreadDecimalStr: string,
  quoteTrader?: QuoteTraderOptions
): Promise<SwapRoutePreflightSpread> {
  if (operations.length === 0) {
    return { worstSpreadPercent: '0.00', anyHopExceedsMaxSpread: false }
  }

  let currentOffer = offerAmount
  let worstPctNum = 0n
  let worstPctDen = 1n
  let anyExceeds = false

  for (const op of operations) {
    const ts = op.terra_swap
    const assetTuple: [AssetInfo, AssetInfo] = [ts.offer_asset_info, ts.ask_asset_info]
    const pairRow = await getPair(assetTuple)
    const pairAddr = pairRow.contract_addr

    const hybrid = ts.hybrid != null ? ts.hybrid : poolOnlyHybridParams(currentOffer)
    const offerBn = BigInt(currentOffer)
    const poolIn = BigInt(hybrid.pool_input)
    const bookIn = BigInt(hybrid.book_input)
    const declaredMaterialReject = hybridNoBeliefMaterialPoolReject(offerBn, poolIn, bookIn, 1n)
    if (declaredMaterialReject?.kind === 'insufficient_pool_leg') {
      anyExceeds = true
      break
    }
    const sim = await simulateHybridSwap(pairAddr, ts.offer_asset_info, currentOffer, hybrid, quoteTrader)
    const materialRejectAfterSim = hybridNoBeliefMaterialPoolReject(
      offerBn,
      poolIn,
      bookIn,
      BigInt(sim.pool_return_amount)
    )
    if (materialRejectAfterSim != null) {
      anyExceeds = true
    }
    const bookNet = BigInt(sim.book_return_amount)
    const bookComm = BigInt((sim as { book_commission_amount?: string }).book_commission_amount ?? '0')
    const offerConsumed = BigInt(
      (sim as { limit_book_offer_consumed?: string }).limit_book_offer_consumed ??
        (bookNet === 0n && bookComm === 0n ? '0' : hybrid.book_input)
    )
    const realized = hybridMaxSpreadRealizedLegs(poolIn, bookIn, offerConsumed)
    const legs = realized.bookInput > 0n && realized.poolInput > 0n ? realized : undefined
    const { spreadCmp, totalGrossOut } = hybridSpreadCmpAndTotal(sim, legs)
    if (spreadRatioStrictlyExceedsMax(spreadCmp, totalGrossOut, maxSpreadDecimalStr)) {
      anyExceeds = true
    }
    if (totalGrossOut > 0n && spreadCmp * worstPctDen > worstPctNum * totalGrossOut) {
      worstPctNum = spreadCmp
      worstPctDen = totalGrossOut
    }
    currentOffer = sim.return_amount
  }

  return {
    worstSpreadPercent: spreadPercentOfGross(worstPctNum, worstPctDen),
    anyHopExceedsMaxSpread: anyExceeds,
  }
}
