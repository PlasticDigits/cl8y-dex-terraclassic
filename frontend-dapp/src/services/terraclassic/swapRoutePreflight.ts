import { getPair } from './factory'
import { simulateHybridSwap, simulateSwap } from './pair'
import type { SwapOperation } from './router'
import type { AssetInfo } from '@/types'
import {
  hybridSpreadCmpAndTotal,
  poolOnlySpreadCmpAndTotal,
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
 * Sequential per-hop pair `simulation` / `hybrid_simulation` to recover spread metrics omitted from
 * router `simulate_swap_operations` (which only returns the final amount).
 */
export async function preflightSwapRouteSpread(
  operations: SwapOperation[],
  offerAmount: string,
  maxSpreadDecimalStr: string
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

    if (ts.hybrid != null) {
      const sim = await simulateHybridSwap(pairAddr, ts.offer_asset_info, currentOffer, ts.hybrid)
      const { spreadCmp, totalGrossOut } = hybridSpreadCmpAndTotal(sim)
      if (spreadRatioStrictlyExceedsMax(spreadCmp, totalGrossOut, maxSpreadDecimalStr)) {
        anyExceeds = true
      }
      if (totalGrossOut > 0n && spreadCmp * worstPctDen > worstPctNum * totalGrossOut) {
        worstPctNum = spreadCmp
        worstPctDen = totalGrossOut
      }
      currentOffer = sim.return_amount
    } else {
      const sim = await simulateSwap(pairAddr, ts.offer_asset_info, currentOffer)
      const { spreadCmp, totalGrossOut } = poolOnlySpreadCmpAndTotal(sim)
      if (spreadRatioStrictlyExceedsMax(spreadCmp, totalGrossOut, maxSpreadDecimalStr)) {
        anyExceeds = true
      }
      if (totalGrossOut > 0n && spreadCmp * worstPctDen > worstPctNum * totalGrossOut) {
        worstPctNum = spreadCmp
        worstPctDen = totalGrossOut
      }
      currentOffer = sim.return_amount
    }
  }

  return {
    worstSpreadPercent: spreadPercentOfGross(worstPctNum, worstPctDen),
    anyHopExceedsMaxSpread: anyExceeds,
  }
}
