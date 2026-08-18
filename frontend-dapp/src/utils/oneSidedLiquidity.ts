/**
 * One-sided pool zap math (GitLab #533 / Z533-4, Z533-5, Z533-10; #559 / Z559).
 *
 * Pair swap adds the **full** offer to the input reserve, takes commission from
 * **gross output**, and sends commission to the treasury (reserves drop by gross).
 * Mirrors `pool_net_output_for_input` in
 * `smartcontracts/contracts/pair/src/hybrid_reverse.rs`.
 *
 * Retail zap-in swaps a slice so leftover + net ask match the **post-swap**
 * reserve ratio, then trims so leftover stays in the wallet (never donate).
 *
 * **Z559-1:** quoted `swapOut` / `provideOut` may be optimistic. Execution
 * `provideAsk` follows `swapMinReturn` (and zap-out follows withdraw `min_assets`),
 * then re-trims to the conservative post-swap ratio. Do not TransferFrom a
 * quoted ask the swap is allowed to miss.
 */

import { applySlippagePercentFloor, estimateWithdrawAssetAmounts } from '@/utils/rawAmountMath'
import { getNativeEquivalent, getWrappedEquivalent, isNativeDenom } from '@/types'
import { netAfterWrapMapperFee } from '@/services/terraclassic/wrapMapper'
import { netUlunaAfterTransferTax, type NativeTransferTaxParams } from '@/utils/nativeTransferTax'

/** LP CW20 `decimals` on the pair share token — not the UI leftover `6` (Z533-10). */
export const PAIR_LP_CW20_DECIMALS = 18
/** Alias used in skills / verify grep. */
export const LP_CW20_DECIMALS = PAIR_LP_CW20_DECIMALS

export const FEE_DENOM_BPS = 10_000n

export type ZapUnavailableReason = 'empty_pool' | 'dust' | 'invalid' | 'no_route'

export type ZapInSplitOk = {
  status: 'ok'
  swapIn: bigint
  /** Net ask the user receives (after commission). */
  swapOut: bigint
  /** Gross ask leaving the pool (net + commission). */
  swapGross: bigint
  provideIn: bigint
  provideOut: bigint
  leftoverIn: bigint
  leftoverOut: bigint
  postReserveIn: bigint
  postReserveOut: bigint
}

export type ZapUnavailable = {
  status: 'unavailable'
  reason: ZapUnavailableReason
}

export type ZapInSplit = ZapInSplitOk | ZapUnavailable

export type ZapOutOk = {
  status: 'ok'
  withdrawnA: bigint
  withdrawnB: bigint
  swapIn: bigint
  swapOut: bigint
  totalWantedCw20: bigint
}

export type ZapOutSplit = ZapOutOk | ZapUnavailable

export type ZapPairSide = 'a' | 'b'

export type ZapInputKind = { kind: 'pair_leg'; side: ZapPairSide; wrapFromNative: string | null } | { kind: 'off_pair' }

export function effectivePoolFeeBps(feeBps: number, discountBps = 0): number {
  const fee = Number.isFinite(feeBps) ? Math.floor(feeBps) : 0
  const discount = Number.isFinite(discountBps) ? Math.max(0, Math.floor(discountBps)) : 0
  return Math.max(0, fee - discount)
}

function ceilDiv(n: bigint, d: bigint): bigint {
  if (d <= 0n) throw new Error('ceilDiv: bad denominator')
  if (n <= 0n) return 0n
  const q = n / d
  return q * d < n ? q + 1n : q
}

export type PoolNetSim = {
  net: bigint
  gross: bigint
  commission: bigint
  newReserveIn: bigint
  newReserveOut: bigint
}

/**
 * Pair pool-leg net ask for a given offer (output-side fee + ceil k/x).
 */
export function poolNetOutputForInput(
  offer: bigint,
  reserveIn: bigint,
  reserveOut: bigint,
  feeBps: number
): PoolNetSim | null {
  if (offer <= 0n || reserveIn <= 0n || reserveOut <= 0n) return null
  const newReserveIn = reserveIn + offer
  const k = reserveIn * reserveOut
  const newReserveOut = ceilDiv(k, newReserveIn)
  if (newReserveOut >= reserveOut) return null
  const gross = reserveOut - newReserveOut
  const bps = BigInt(Math.max(0, Math.min(10_000, Math.floor(feeBps))))
  const commission = (gross * bps) / FEE_DENOM_BPS
  const net = gross - commission
  if (net <= 0n) return null
  return { net, gross, commission, newReserveIn, newReserveOut }
}

/** Net user output; 0 when the pool cannot fill. */
export function constantProductAmountOut(offer: bigint, reserveIn: bigint, reserveOut: bigint, feeBps: number): bigint {
  return poolNetOutputForInput(offer, reserveIn, reserveOut, feeBps)?.net ?? 0n
}

export type TrimmedProvide = {
  provideIn: bigint
  provideOut: bigint
  leftoverIn: bigint
  leftoverOut: bigint
}

/**
 * Ratio-trim leftover offer + received ask to `reserveOutAfter / reserveInAfter`.
 * Excess stays in the wallet (Z533-4 — never donate).
 */
export function trimProvideToRatio(
  remainingIn: bigint,
  swapOut: bigint,
  reserveInAfter: bigint,
  reserveOutAfter: bigint
): TrimmedProvide | null {
  if (remainingIn <= 0n || swapOut <= 0n || reserveInAfter <= 0n || reserveOutAfter <= 0n) return null
  const idealOut = (remainingIn * reserveOutAfter) / reserveInAfter
  if (swapOut > idealOut) {
    return {
      provideIn: remainingIn,
      provideOut: idealOut,
      leftoverIn: 0n,
      leftoverOut: swapOut - idealOut,
    }
  }
  const idealIn = (swapOut * reserveInAfter) / reserveOutAfter
  if (idealIn <= 0n || idealIn > remainingIn) return null
  const provideOut = (idealIn * reserveOutAfter) / reserveInAfter
  if (provideOut <= 0n || provideOut > swapOut) return null
  return {
    provideIn: idealIn,
    provideOut,
    leftoverIn: remainingIn - idealIn,
    leftoverOut: swapOut - provideOut,
  }
}

/**
 * Zap-in split. Empty or one-sided reserves → `empty_pool`.
 * Provide amounts are ratio-trimmed so retail never donates (Z533-4).
 */
export function zapInSplit(params: {
  amountIn: bigint
  reserveIn: bigint
  reserveOut: bigint
  feeBps: number
}): ZapInSplit {
  const { amountIn, reserveIn, reserveOut, feeBps } = params
  if (amountIn <= 0n) return { status: 'unavailable', reason: 'invalid' }
  if (reserveIn <= 0n || reserveOut <= 0n) return { status: 'unavailable', reason: 'empty_pool' }

  const maxSwap = amountIn - 1n
  if (maxSwap < 1n) return { status: 'unavailable', reason: 'dust' }

  let lo = 1n
  let hi = maxSwap
  const best: { v: { s: bigint; sim: PoolNetSim; remaining: bigint; abs: bigint } | null } = { v: null }

  const consider = (s: bigint) => {
    const remaining = amountIn - s
    const sim = poolNetOutputForInput(s, reserveIn, reserveOut, feeBps)
    if (!sim || remaining <= 0n) return
    const cmp = remaining * sim.newReserveOut - sim.net * sim.newReserveIn
    const abs = cmp < 0n ? -cmp : cmp
    const cur = best.v
    if (!cur || abs < cur.abs || (abs === cur.abs && s < cur.s)) {
      best.v = { s, sim, remaining, abs }
    }
    return cmp
  }

  while (lo <= hi) {
    const s = (lo + hi) / 2n
    const cmp = consider(s)
    if (cmp == null) {
      hi = s - 1n
      continue
    }
    if (cmp === 0n) break
    if (cmp > 0n) lo = s + 1n
    else hi = s - 1n
  }

  const found = best.v
  if (found) {
    const start = found.s > 64n ? found.s - 64n : 1n
    const end = found.s + 64n > maxSwap ? maxSwap : found.s + 64n
    for (let s = start; s <= end; s++) consider(s)
  }

  const chosen = best.v
  if (!chosen) return { status: 'unavailable', reason: 'dust' }

  const trimmed = trimProvideToRatio(
    chosen.remaining,
    chosen.sim.net,
    chosen.sim.newReserveIn,
    chosen.sim.newReserveOut
  )
  if (!trimmed || trimmed.provideIn <= 0n || trimmed.provideOut <= 0n) {
    return { status: 'unavailable', reason: 'dust' }
  }

  return {
    status: 'ok',
    swapIn: chosen.s,
    swapOut: chosen.sim.net,
    swapGross: chosen.sim.gross,
    postReserveIn: chosen.sim.newReserveIn,
    postReserveOut: chosen.sim.newReserveOut,
    ...trimmed,
  }
}

export type ConservativeZapInProvide = TrimmedProvide & {
  /** Ask reserve after a worse-than-quote fill (shortfall stayed in the pool). */
  postReserveOut: bigint
}

/**
 * Z559-1 / Z559-2: size provide to the swap floor, then re-trim to the
 * conservative post-swap ratio.
 *
 * Worst fill the swap may produce is `swapMinReturn`. `provideOut` cannot exceed
 * that (user may have **zero** pre-existing ask — A-Z2). If less ask left the
 * pool, `postReserveOut` is higher than the quoted reserve; offer shrinks and
 * leftover stays in the wallet (Z533-4).
 */
export function conservativeZapInProvide(split: ZapInSplitOk, swapMinReturn: bigint): ConservativeZapInProvide | null {
  const remainingIn = split.provideIn + split.leftoverIn
  const conservativeOut = swapMinReturn < split.swapOut ? swapMinReturn : split.swapOut
  if (conservativeOut <= 0n || remainingIn <= 0n) return null
  const extraAskInPool = split.swapOut - conservativeOut
  const postReserveOut = split.postReserveOut + extraAskInPool
  const trimmed = trimProvideToRatio(remainingIn, conservativeOut, split.postReserveIn, postReserveOut)
  if (!trimmed || trimmed.provideIn <= 0n || trimmed.provideOut <= 0n) return null
  if (trimmed.provideOut > conservativeOut) return null
  return { ...trimmed, postReserveOut }
}

export type ConservativeZapOutExecution = {
  swapAmount: string
  swapMinReturn: string
  unwrapAmount: string
}

/**
 * Z559-3: zap-out execution follows withdraw/swap floors, not optimistic quotes.
 *
 * - `swapAmount ≤ min_assets[sold]` (may be less than pro-rata `swapIn`)
 * - unwrap send ≤ `min(withdrawn wanted, min_assets[wanted]) + swapMinReturn`
 *
 * Leftover sold or wanted CW20 stays in the wallet (Z533-8).
 */
export function conservativeZapOutExecution(input: {
  split: ZapOutOk
  wantSide: ZapPairSide
  minAssets: [string, string]
  slippagePercent: number
}): ConservativeZapOutExecution | null {
  let minA: bigint
  let minB: bigint
  try {
    minA = BigInt(input.minAssets[0])
    minB = BigInt(input.minAssets[1])
  } catch {
    return null
  }
  const soldMin = input.wantSide === 'a' ? minB : minA
  const wantedMin = input.wantSide === 'a' ? minA : minB
  const withdrawnWanted = input.wantSide === 'a' ? input.split.withdrawnA : input.split.withdrawnB
  const wantedFloor = withdrawnWanted < wantedMin ? withdrawnWanted : wantedMin
  if (wantedFloor <= 0n || soldMin <= 0n) return null

  const swapAmount = input.split.swapIn < soldMin ? input.split.swapIn : soldMin
  if (swapAmount <= 0n) return null

  let swapOutForAmount = input.split.swapOut
  if (swapAmount < input.split.swapIn && input.split.swapIn > 0n) {
    swapOutForAmount = (input.split.swapOut * swapAmount) / input.split.swapIn
  }
  const swapMin = zapOutSwapMinReturn(swapOutForAmount, input.slippagePercent)
  if (swapMin == null || swapMin <= 0n) return null

  const unwrapAmount = wantedFloor + swapMin
  if (unwrapAmount <= 0n) return null
  return {
    swapAmount: swapAmount.toString(),
    swapMinReturn: swapMin.toString(),
    unwrapAmount: unwrapAmount.toString(),
  }
}

/**
 * Zap-out: pro-rata withdraw, then sell the other side against **post-withdraw** reserves.
 */
export function zapOutSplit(params: {
  lpRaw: bigint
  totalShare: bigint
  reserveA: bigint
  reserveB: bigint
  wantSide: ZapPairSide
  feeBps: number
}): ZapOutSplit {
  const { lpRaw, totalShare, reserveA, reserveB, wantSide, feeBps } = params
  if (lpRaw <= 0n || totalShare <= 0n) return { status: 'unavailable', reason: 'invalid' }
  if (reserveA <= 0n || reserveB <= 0n) return { status: 'unavailable', reason: 'empty_pool' }
  if (lpRaw > totalShare) return { status: 'unavailable', reason: 'invalid' }

  const withdrawn = estimateWithdrawAssetAmounts(
    lpRaw.toString(),
    totalShare.toString(),
    reserveA.toString(),
    reserveB.toString()
  )
  if (!withdrawn) return { status: 'unavailable', reason: 'dust' }
  const withdrawnA = BigInt(withdrawn[0])
  const withdrawnB = BigInt(withdrawn[1])
  if (withdrawnA <= 0n || withdrawnB <= 0n) return { status: 'unavailable', reason: 'dust' }

  const reserveAAfter = reserveA - withdrawnA
  const reserveBAfter = reserveB - withdrawnB
  if (reserveAAfter <= 0n || reserveBAfter <= 0n) return { status: 'unavailable', reason: 'dust' }

  if (wantSide === 'a') {
    const swapOut = constantProductAmountOut(withdrawnB, reserveBAfter, reserveAAfter, feeBps)
    if (swapOut <= 0n) return { status: 'unavailable', reason: 'dust' }
    return {
      status: 'ok',
      withdrawnA,
      withdrawnB,
      swapIn: withdrawnB,
      swapOut,
      totalWantedCw20: withdrawnA + swapOut,
    }
  }

  const swapOut = constantProductAmountOut(withdrawnA, reserveAAfter, reserveBAfter, feeBps)
  if (swapOut <= 0n) return { status: 'unavailable', reason: 'dust' }
  return {
    status: 'ok',
    withdrawnA,
    withdrawnB,
    swapIn: withdrawnA,
    swapOut,
    totalWantedCw20: withdrawnB + swapOut,
  }
}

export function zapOutMinWantedCw20(totalWantedCw20: bigint, slippagePercent: number): bigint | null {
  const floored = applySlippagePercentFloor(totalWantedCw20.toString(), slippagePercent)
  if (floored == null) return null
  const n = BigInt(floored)
  return n > 0n ? n : null
}

export function zapOutSwapMinReturn(swapOut: bigint, slippagePercent: number): bigint | null {
  const floored = applySlippagePercentFloor(swapOut.toString(), slippagePercent)
  if (floored == null) return null
  const n = BigInt(floored)
  return n > 0n ? n : null
}

/** Wrap-fee-only net (W8) — never apply Classic burn tax to wrap_deposit. */
export function wrapNetForZapSolver(grossNative: bigint, wrapFeeBps: number): bigint {
  return netAfterWrapMapperFee(grossNative, wrapFeeBps)
}

/**
 * Native receive after unwrap: mapper `fee_unwrap_bps` then InstantWithdraw burn tax (W9).
 * `routerMinReceiveBase` is post-fee pre-tax (R3) when a router `unwrap_output` hop is used.
 */
export function nativeAfterZapUnwrap(
  wrappedAmount: bigint,
  unwrapFeeBps: number,
  tax: NativeTransferTaxParams
): { receive: bigint; routerMinReceiveBase: bigint } {
  const afterFee = netAfterWrapMapperFee(wrappedAmount, unwrapFeeBps)
  return {
    routerMinReceiveBase: afterFee,
    receive: netUlunaAfterTransferTax(afterFee, tax),
  }
}

export function resolveZapInputKind(tokenId: string, asset0: string, asset1: string): ZapInputKind {
  const id = tokenId.trim()
  const a = asset0.trim()
  const b = asset1.trim()
  if (id === a) return { kind: 'pair_leg', side: 'a', wrapFromNative: null }
  if (id === b) return { kind: 'pair_leg', side: 'b', wrapFromNative: null }
  const wrapped = isNativeDenom(id) ? getWrappedEquivalent(id) : null
  if (wrapped && wrapped === a) return { kind: 'pair_leg', side: 'a', wrapFromNative: id }
  if (wrapped && wrapped === b) return { kind: 'pair_leg', side: 'b', wrapFromNative: id }
  return { kind: 'off_pair' }
}

export function resolveZapOutputKind(tokenId: string, asset0: string, asset1: string): ZapInputKind {
  return resolveZapInputKind(tokenId, asset0, asset1)
}

/** Off-pair input must route-in before zap (T7 / AC10). */
export function classifyZapToken(tokenId: string, legs: [string, string]): ZapInputKind | { kind: 'needs_route' } {
  const resolved = resolveZapInputKind(tokenId, legs[0], legs[1])
  if (resolved.kind === 'off_pair') return { kind: 'needs_route' }
  return resolved
}

export function nativeUnwrapDenomForPairLeg(pairLegCw20: string): string | null {
  return getNativeEquivalent(pairLegCw20)
}

/** T7: empty indexer route disables; a positive route-out is zapped as a pair-leg. */
export function applyRouteThenZap(
  routeOutRaw: string | null | undefined,
  params: { reserveIn: bigint; reserveOut: bigint; feeBps: number }
): ZapInSplit {
  if (routeOutRaw == null || routeOutRaw === '' || routeOutRaw === '0') {
    return { status: 'unavailable', reason: 'no_route' }
  }
  let amountIn: bigint
  try {
    amountIn = BigInt(routeOutRaw)
  } catch {
    return { status: 'unavailable', reason: 'invalid' }
  }
  return zapInSplit({ amountIn, ...params })
}

export function isEmptyPoolReserves(reserveA: bigint, reserveB: bigint): boolean {
  return reserveA <= 0n || reserveB <= 0n
}

export function zapSwapPriceImpactPercent(
  swapIn: bigint,
  swapOut: bigint,
  reserveIn: bigint,
  reserveOut: bigint
): number | null {
  if (swapIn <= 0n || reserveIn <= 0n || reserveOut <= 0n) return null
  const spot = (swapIn * reserveOut) / reserveIn
  if (spot <= 0n) return null
  const lo = swapOut < spot ? swapOut : spot
  const hi = swapOut > spot ? swapOut : spot
  if (hi <= 0n) return null
  return Number((1 - Number(lo) / Number(hi)) * 100)
}
