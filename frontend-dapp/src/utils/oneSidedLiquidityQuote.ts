import { getRouteSolve } from '@/services/indexer/client'
import { quoteCw20ViaRouteSolve } from '@/utils/cw20RouteSolveQuote'
import { applySlippagePercentFloor } from '@/utils/rawAmountMath'
import { estimateProvideLiquidityUserLp } from '@/utils/provideLiquidityEstimate'
import {
  applyRouteThenZap,
  conservativeZapInProvide,
  effectivePoolFeeBps,
  isEmptyPoolReserves,
  resolveZapInputKind,
  wrapNetForZapSolver,
  zapSwapPriceImpactPercent,
  type ZapInSplitOk,
} from '@/utils/oneSidedLiquidity'
import { assetInfoLabel, type PairInfo, type PoolResponse } from '@/types'
import type { SwapOperation } from '@/services/terraclassic/router'

export const ONE_SIDED_EMPTY_POOL_ERROR = 'Empty pool. Use Provide Liquidity.'
export const ONE_SIDED_NO_ROUTE_ERROR = 'No route'
export const ONE_SIDED_DUST_ERROR = 'Amount too small'

export type OneSidedAddSnapshot = {
  key: string
  tokenId: string
  pairAddress: string
  pairLabel: string
  payRaw: string
  wrapGross: string | null
  wrapDenom: string | null
  offerCw20: string
  askCw20: string
  swapAmount: string
  swapMinReturn: string
  provideOffer: string
  provideAsk: string
  estimatedLp: string | null
  slippagePercent: number
  priceImpactPercent: number | null
  routeIn: {
    token: string
    amount: string
    operations: SwapOperation[]
    minReturn: string
    maxSpread: string
  } | null
  split: ZapInSplitOk
}

export type OneSidedAddQuote =
  | { status: 'ok'; snapshot: OneSidedAddSnapshot }
  | { status: 'unavailable'; disableReason: string }

function pairLegs(pair: PairInfo): [string, string] {
  return [assetInfoLabel(pair.asset_infos[0]), assetInfoLabel(pair.asset_infos[1])]
}

function estimateZapInLp(input: {
  pool: PoolResponse
  offerCw20: string
  provideOffer: string
  provideAsk: string
  postReserveIn: bigint
  postReserveOut: bigint
}): string | null {
  const leg0 = assetInfoLabel(input.pool.assets[0].info)
  const offerIs0 = input.offerCw20 === leg0
  const amountA = offerIs0 ? input.provideOffer : input.provideAsk
  const amountB = offerIs0 ? input.provideAsk : input.provideOffer
  const resA = offerIs0 ? input.postReserveIn : input.postReserveOut
  const resB = offerIs0 ? input.postReserveOut : input.postReserveIn
  const lp = estimateProvideLiquidityUserLp(amountA, amountB, {
    assets: [
      { info: input.pool.assets[0].info, amount: resA.toString() },
      { info: input.pool.assets[1].info, amount: resB.toString() },
    ],
    total_share: input.pool.total_share,
  })
  if (lp == null || lp <= 0n) return null
  return lp.toString()
}

export async function quoteOneSidedAdd(input: {
  tokenId: string
  pair: PairInfo
  pairLabel: string
  payRaw: string
  pool: PoolResponse
  feeBps: number
  discountBps: number
  wrapFeeBps: number | null
  slippagePercent: number
  maxSpreadStr: string
  trader?: string
}): Promise<OneSidedAddQuote> {
  const resA = BigInt(input.pool.assets[0].amount)
  const resB = BigInt(input.pool.assets[1].amount)
  if (isEmptyPoolReserves(resA, resB)) {
    return { status: 'unavailable', disableReason: ONE_SIDED_EMPTY_POOL_ERROR }
  }

  const [legA, legB] = pairLegs(input.pair)
  const kind = resolveZapInputKind(input.tokenId, legA, legB)
  const feeBps = effectivePoolFeeBps(input.feeBps, input.discountBps)

  let solverIn = input.payRaw
  let wrapGross: string | null = null
  let wrapDenom: string | null = null
  let offerCw20: string
  let askCw20: string
  let offerRes: bigint
  let askRes: bigint
  let routeIn: OneSidedAddSnapshot['routeIn'] = null

  if (kind.kind === 'pair_leg') {
    offerCw20 = kind.side === 'a' ? legA : legB
    askCw20 = kind.side === 'a' ? legB : legA
    offerRes = kind.side === 'a' ? resA : resB
    askRes = kind.side === 'a' ? resB : resA
    if (kind.wrapFromNative) {
      wrapDenom = kind.wrapFromNative
      wrapGross = input.payRaw
      if (input.wrapFeeBps == null) {
        return { status: 'unavailable', disableReason: 'Wrap config unavailable' }
      }
      solverIn = wrapNetForZapSolver(BigInt(input.payRaw), input.wrapFeeBps).toString()
    }
  } else {
    const targets: Array<{ offer: string; ask: string; offerRes: bigint; askRes: bigint }> = [
      { offer: legA, ask: legB, offerRes: resA, askRes: resB },
      { offer: legB, ask: legA, offerRes: resB, askRes: resA },
    ]
    let routed: {
      target: (typeof targets)[0]
      amount: string
      operations: SwapOperation[]
    } | null = null
    for (const target of targets) {
      try {
        const quoted = await quoteCw20ViaRouteSolve({
          fromToken: input.tokenId,
          toToken: target.offer,
          simRaw: input.payRaw,
          maxMakerFills: 1,
          slippageTolerancePercent: input.slippagePercent,
          maxSpreadStr: input.maxSpreadStr,
          quoteTrader: input.trader ? { trader: input.trader } : undefined,
        })
        if (!quoted || quoted.return_amount === '0') continue
        const idx = await getRouteSolve(input.tokenId, target.offer, input.payRaw, {
          poolOnly: true,
          maxMakerFills: 1,
          trader: input.trader,
        })
        if (!idx.router_operations?.length) continue
        routed = { target, amount: quoted.return_amount, operations: quoted.indexerOperations }
        break
      } catch {
        continue
      }
    }
    if (!routed) return { status: 'unavailable', disableReason: ONE_SIDED_NO_ROUTE_ERROR }
    offerCw20 = routed.target.offer
    askCw20 = routed.target.ask
    offerRes = routed.target.offerRes
    askRes = routed.target.askRes
    solverIn = routed.amount
    const minReturn = applySlippagePercentFloor(routed.amount, input.slippagePercent)
    if (!minReturn) return { status: 'unavailable', disableReason: ONE_SIDED_DUST_ERROR }
    routeIn = {
      token: input.tokenId,
      amount: input.payRaw,
      operations: routed.operations,
      minReturn,
      maxSpread: input.maxSpreadStr,
    }
    // T-Z5 / AC5: route `minimum_receive` floors the zap amountIn.
    solverIn = minReturn
  }

  const split = applyRouteThenZap(solverIn, { reserveIn: offerRes, reserveOut: askRes, feeBps })
  if (split.status !== 'ok') {
    if (split.reason === 'empty_pool') return { status: 'unavailable', disableReason: ONE_SIDED_EMPTY_POOL_ERROR }
    if (split.reason === 'no_route') return { status: 'unavailable', disableReason: ONE_SIDED_NO_ROUTE_ERROR }
    return { status: 'unavailable', disableReason: ONE_SIDED_DUST_ERROR }
  }

  const swapMinReturn = applySlippagePercentFloor(split.swapOut.toString(), input.slippagePercent)
  if (!swapMinReturn || swapMinReturn === '0') {
    return { status: 'unavailable', disableReason: ONE_SIDED_DUST_ERROR }
  }

  const execution = conservativeZapInProvide(split, BigInt(swapMinReturn))
  if (!execution) {
    return { status: 'unavailable', disableReason: ONE_SIDED_DUST_ERROR }
  }

  const estimatedLp = estimateZapInLp({
    pool: input.pool,
    offerCw20,
    provideOffer: execution.provideIn.toString(),
    provideAsk: execution.provideOut.toString(),
    postReserveIn: split.postReserveIn,
    postReserveOut: execution.postReserveOut,
  })
  if (!estimatedLp) {
    return { status: 'unavailable', disableReason: ONE_SIDED_DUST_ERROR }
  }

  const key = [
    input.tokenId,
    input.pair.contract_addr,
    input.payRaw,
    solverIn,
    split.swapIn.toString(),
    swapMinReturn,
    execution.provideIn.toString(),
    execution.provideOut.toString(),
    String(input.slippagePercent),
  ].join('|')

  return {
    status: 'ok',
    snapshot: {
      key,
      tokenId: input.tokenId,
      pairAddress: input.pair.contract_addr,
      pairLabel: input.pairLabel,
      payRaw: input.payRaw,
      wrapGross,
      wrapDenom,
      offerCw20,
      askCw20,
      swapAmount: split.swapIn.toString(),
      swapMinReturn,
      provideOffer: execution.provideIn.toString(),
      provideAsk: execution.provideOut.toString(),
      estimatedLp,
      slippagePercent: input.slippagePercent,
      priceImpactPercent: zapSwapPriceImpactPercent(split.swapIn, split.swapOut, offerRes, askRes),
      routeIn,
      split,
    },
  }
}
