import { previewRouteTax } from './routeTaxFunding'
import { assertRouteTaxPath } from './routeTaxPath'
import { assetInfoLabel } from '@/types'
import { getPair } from './factory'
import { simulateHybridSwap, type QuoteTraderOptions } from './pair'
import { poolOnlyHybridParams } from './poolOnlyHybrid'
import type { SwapOperation } from './router'
import type { AssetInfo, HybridSwapParams } from '@/types'
import {
  hybridBookRequiresSlippageFloor,
  hybridMaxSpreadRealizedLegs,
  hybridNoBeliefMaterialPoolReject,
  hybridSpreadCmpAndTotal,
  spreadPercentOfGross,
  spreadRatioStrictlyExceedsMax,
} from '@/utils/swapMaxSpread'
import { applySlippagePercentFloor } from '@/utils/rawAmountMath'

/** Mirror router v1's gross-ratio partition on its funded economic input. */
function fundedHybridForQuote(
  hybrid: HybridSwapParams | null | undefined,
  gross: string,
  economic: string,
  fundingVersion?: 1,
  ratioTemplate = false
): HybridSwapParams {
  if (!hybrid) return poolOnlyHybridParams(economic)
  const total = BigInt(hybrid.pool_input) + BigInt(hybrid.book_input)
  if (ratioTemplate) {
    if (total <= 0n) throw new Error('Hybrid ratio template must have a positive total')
    const book = (BigInt(economic) * BigInt(hybrid.book_input)) / total
    return { ...hybrid, book_input: book.toString(), pool_input: (BigInt(economic) - book).toString() }
  }
  if (fundingVersion !== 1) return hybrid
  if (total === BigInt(economic)) return hybrid
  if (total !== BigInt(gross) || BigInt(gross) === 0n) throw new Error('Hybrid swap split mismatch')
  const book = (BigInt(economic) * BigInt(hybrid.book_input)) / BigInt(gross)
  return { ...hybrid, book_input: book.toString(), pool_input: (BigInt(economic) - book).toString() }
}

export interface SwapRoutePreflightSpread {
  /** Max over hops of contract-style spread % (2 decimal places). */
  worstSpreadPercent: string
  /** True if any hop would fail the pair `assert_max_spread` check for this `maxSpread`. */
  anyHopExceedsMaxSpread: boolean
  /** True if any hop has a book leg but no `belief_price` / `min_return` floor (GitLab #334). */
  anyHopMissingSlippageFloor: boolean
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
    return { worstSpreadPercent: '0.00', anyHopExceedsMaxSpread: false, anyHopMissingSlippageFloor: false }
  }
  if (quoteTrader?.taxFundingVersion === 1) await assertRouteTaxPath(operations, quoteTrader.signal)

  let currentOffer = offerAmount
  let worstPctNum = 0n
  let worstPctDen = 1n
  let anyExceeds = false
  let anyMissingFloor = false

  for (const op of operations) {
    const ts = op.terra_swap
    const assetTuple: [AssetInfo, AssetInfo] = [ts.offer_asset_info, ts.ask_asset_info]
    quoteTrader?.signal?.throwIfAborted()
    const pairRow = await getPair(
      assetTuple,
      ...(quoteTrader?.signal ? ([quoteTrader.signal] as const) : ([] as const))
    )
    const pairAddr = pairRow.contract_addr

    let economicOffer = currentOffer
    if (quoteTrader?.taxFundingVersion === 1) {
      economicOffer = (
        await previewRouteTax({
          token: assetInfoLabel(ts.offer_asset_info),
          kind: 'sell',
          amount: currentOffer,
          trader: quoteTrader.trader,
          pair: pairRow.contract_addr,
          signal: quoteTrader.signal,
        })
      ).economic
    }
    const hybrid = fundedHybridForQuote(ts.hybrid, currentOffer, economicOffer, quoteTrader?.taxFundingVersion)
    const offerBn = BigInt(economicOffer)
    const poolIn = BigInt(hybrid.pool_input)
    const bookIn = BigInt(hybrid.book_input)
    if (hybridBookRequiresSlippageFloor(bookIn, null, ts.min_return)) {
      anyMissingFloor = true
    }
    const sim = await simulateHybridSwap(pairAddr, ts.offer_asset_info, economicOffer, hybrid, quoteTrader)
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
    currentOffer =
      quoteTrader?.taxFundingVersion === 1
        ? (
            await previewRouteTax({
              token: assetInfoLabel(ts.ask_asset_info),
              kind: 'buy',
              amount: sim.return_amount,
              trader: quoteTrader.trader,
              pair: pairRow.contract_addr,
              signal: quoteTrader.signal,
            })
          ).net
        : sim.return_amount
  }

  return {
    worstSpreadPercent: spreadPercentOfGross(worstPctNum, worstPctDen),
    anyHopExceedsMaxSpread: anyExceeds,
    anyHopMissingSlippageFloor: anyMissingFloor,
  }
}

/** Slippage floor from `hybrid_simulation` with the submit hybrid split (GitLab #334 direct single-hop). */
export async function computeDirectHybridMinReturn(
  pairAddress: string,
  offerAssetInfo: AssetInfo,
  offerAmount: string,
  hybrid: HybridSwapParams,
  slippagePercent: number,
  quoteTrader?: QuoteTraderOptions
): Promise<string | undefined> {
  if (BigInt(hybrid.book_input) === 0n) return undefined
  const sim = await simulateHybridSwap(pairAddress, offerAssetInfo, offerAmount, hybrid, quoteTrader)
  const computed = applySlippagePercentFloor(sim.return_amount, slippagePercent)
  return computed != null && !hybridBookRequiresSlippageFloor(BigInt(hybrid.book_input), null, computed)
    ? computed
    : undefined
}

/** Attach per-hop `min_return` from slippage on simulated hop output (GitLab #334). */
export async function enrichSwapOperationsWithHopMinReturns(
  operations: SwapOperation[],
  offerAmount: string,
  slippagePercent: number,
  quoteTrader?: QuoteTraderOptions,
  options?: { hybridRatioTemplate?: boolean }
): Promise<SwapOperation[]> {
  if (operations.length === 0) return operations
  if (quoteTrader?.taxFundingVersion === 1) await assertRouteTaxPath(operations, quoteTrader.signal)

  let currentOffer = offerAmount
  const out: SwapOperation[] = []

  for (const op of operations) {
    const ts = op.terra_swap
    let minReturn = options?.hybridRatioTemplate ? undefined : (ts.min_return ?? undefined)
    const assetTuple: [AssetInfo, AssetInfo] = [ts.offer_asset_info, ts.ask_asset_info]
    quoteTrader?.signal?.throwIfAborted()
    const pairRow = await getPair(
      assetTuple,
      ...(quoteTrader?.signal ? ([quoteTrader.signal] as const) : ([] as const))
    )
    const economicOffer =
      quoteTrader?.taxFundingVersion === 1
        ? (
            await previewRouteTax({
              token: assetInfoLabel(ts.offer_asset_info),
              kind: 'sell',
              amount: currentOffer,
              trader: quoteTrader.trader,
              pair: pairRow.contract_addr,
              signal: quoteTrader.signal,
            })
          ).economic
        : currentOffer
    const hybrid = fundedHybridForQuote(
      ts.hybrid,
      currentOffer,
      economicOffer,
      quoteTrader?.taxFundingVersion,
      options?.hybridRatioTemplate
    )
    const bookIn = BigInt(hybrid.book_input)
    const sim = await simulateHybridSwap(pairRow.contract_addr, ts.offer_asset_info, economicOffer, hybrid, quoteTrader)
    const netOutput =
      quoteTrader?.taxFundingVersion === 1
        ? (
            await previewRouteTax({
              token: assetInfoLabel(ts.ask_asset_info),
              kind: 'buy',
              amount: sim.return_amount,
              trader: quoteTrader.trader,
              pair: pairRow.contract_addr,
              signal: quoteTrader.signal,
            })
          ).net
        : sim.return_amount
    if (bookIn > 0n && hybridBookRequiresSlippageFloor(bookIn, null, minReturn)) {
      const computed = applySlippagePercentFloor(netOutput, slippagePercent)
      minReturn = computed != null && !hybridBookRequiresSlippageFloor(bookIn, null, computed) ? computed : undefined
    }
    out.push({
      terra_swap: {
        ...ts,
        hybrid: ts.hybrid ? (options?.hybridRatioTemplate ? hybrid : ts.hybrid) : undefined,
        min_return: minReturn,
      },
    })
    currentOffer = netOutput
  }

  return out
}
