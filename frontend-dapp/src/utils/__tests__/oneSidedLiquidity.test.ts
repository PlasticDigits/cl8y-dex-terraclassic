import { describe, expect, it } from 'vitest'
import { netAfterWrapMapperFee } from '@/services/terraclassic/wrapMapper'
import { applySlippagePercentFloor } from '@/utils/rawAmountMath'
import {
  PAIR_LP_CW20_DECIMALS,
  constantProductAmountOut,
  conservativeZapInProvide,
  conservativeZapOutExecution,
  effectivePoolFeeBps,
  nativeAfterZapUnwrap,
  resolveZapInputKind,
  applyRouteThenZap,
  wrapNetForZapSolver,
  zapInSplit,
  zapOutMinWantedCw20,
  zapOutSplit,
} from '../oneSidedLiquidity'

function pool(resA: bigint, resB: bigint) {
  return { resA, resB }
}

describe('oneSidedLiquidity zap-in (GitLab #533 T1–T6)', () => {
  it('T1 equal pool 30 bps: leftover matches post-swap ratio after trim', () => {
    const { resA, resB } = pool(1_000_000_000n, 1_000_000_000n)
    const amountIn = 100_000_000n
    const split = zapInSplit({ amountIn, reserveIn: resA, reserveOut: resB, feeBps: 30 })
    expect(split.status).toBe('ok')
    if (split.status !== 'ok') return
    expect(split.postReserveIn).toBe(resA + split.swapIn)
    const floorAsk = (split.provideIn * split.postReserveOut) / split.postReserveIn
    const floorOffer = (split.provideOut * split.postReserveIn) / split.postReserveOut
    expect(split.provideOut === floorAsk || split.provideIn === floorOffer).toBe(true)
    expect(split.provideIn + split.leftoverIn).toBe(amountIn - split.swapIn)
    expect(split.provideOut + split.leftoverOut).toBe(split.swapOut)
  })

  it('T1 skewed pool still ratio-correct after swap', () => {
    const split = zapInSplit({
      amountIn: 50_000_000n,
      reserveIn: 10_000_000_000n,
      reserveOut: 1_000_000_000n,
      feeBps: 30,
    })
    expect(split.status).toBe('ok')
    if (split.status !== 'ok') return
    const floorAsk = (split.provideIn * split.postReserveOut) / split.postReserveIn
    const floorOffer = (split.provideOut * split.postReserveIn) / split.postReserveOut
    expect(split.provideOut === floorAsk || split.provideIn === floorOffer).toBe(true)
  })

  it('T1 6- vs 18-decimal legs', () => {
    const split = zapInSplit({
      amountIn: 1_000_000n,
      reserveIn: 1_000_000_000n,
      reserveOut: 10n ** 24n,
      feeBps: 30,
    })
    expect(split.status).toBe('ok')
    if (split.status !== 'ok') return
    expect(split.provideOut).toBeGreaterThan(0n)
    expect(split.provideIn).toBeGreaterThan(0n)
  })

  it('T1 fee-discount > 0 uses a lower effective fee than 30 bps', () => {
    const params = {
      amountIn: 100_000_000n,
      reserveIn: 1_000_000_000n,
      reserveOut: 1_000_000_000n,
      feeBps: 30,
    }
    const full = zapInSplit(params)
    const discounted = zapInSplit({ ...params, feeBps: effectivePoolFeeBps(30, 10) })
    expect(full.status).toBe('ok')
    expect(discounted.status).toBe('ok')
    if (full.status !== 'ok' || discounted.status !== 'ok') return
    expect(discounted.swapOut).toBeGreaterThan(full.swapOut)
  })

  it('T2 wrap-fee-only net: 10_000 LUNC @ 200 bps → 9_800 into the solver', () => {
    const gross = 10_000_000_000n
    const net = wrapNetForZapSolver(gross, 200)
    expect(net).toBe(9_800_000_000n)
    expect(net).toBe(netAfterWrapMapperFee(gross, 200))
    expect(net).not.toBe(9_751_000_000n)
    const split = zapInSplit({
      amountIn: net,
      reserveIn: 100_000_000_000n,
      reserveOut: 100_000_000_000n,
      feeBps: 30,
    })
    expect(split.status).toBe('ok')
  })

  it('T5 empty reserves → unavailable (no divide-by-zero)', () => {
    expect(zapInSplit({ amountIn: 1_000_000n, reserveIn: 0n, reserveOut: 0n, feeBps: 30 }).reason).toBe('empty_pool')
    expect(zapInSplit({ amountIn: 1_000_000n, reserveIn: 0n, reserveOut: 5n, feeBps: 30 }).reason).toBe('empty_pool')
    expect(constantProductAmountOut(1n, 0n, 1n, 30)).toBe(0n)
  })

  it('T6 dust / 1 raw unit is unavailable; ok splits never donate both leftovers', () => {
    expect(zapInSplit({ amountIn: 1n, reserveIn: 1_000_000_000n, reserveOut: 1_000_000_000n, feeBps: 30 }).status).toBe(
      'unavailable'
    )
    const split = zapInSplit({
      amountIn: 10_000_000n,
      reserveIn: 1_000_000_000n,
      reserveOut: 1_000_000_000n,
      feeBps: 30,
    })
    expect(split.status).toBe('ok')
    if (split.status !== 'ok') return
    expect(split.provideIn > 0n && split.provideOut > 0n).toBe(true)
  })
})

describe('oneSidedLiquidity zap-out (GitLab #533 T3 T4 T8)', () => {
  it('T3 pro-rata + sell other side; min out after 0.5 / 1 / 5%', () => {
    const out = zapOutSplit({
      lpRaw: 1_000_000_000_000_000_000n,
      totalShare: 10_000_000_000_000_000_000n,
      reserveA: 1_000_000_000n,
      reserveB: 2_000_000_000n,
      wantSide: 'a',
      feeBps: 30,
    })
    expect(out.status).toBe('ok')
    if (out.status !== 'ok') return
    expect(out.withdrawnA).toBe(100_000_000n)
    expect(out.withdrawnB).toBe(200_000_000n)
    expect(out.swapIn).toBe(out.withdrawnB)
    expect(out.totalWantedCw20).toBe(out.withdrawnA + out.swapOut)
    for (const pct of [0.5, 1, 5]) {
      const min = zapOutMinWantedCw20(out.totalWantedCw20, pct)
      expect(min).toBe(BigInt(applySlippagePercentFloor(out.totalWantedCw20.toString(), pct)!))
      expect(min!).toBeLessThan(out.totalWantedCw20)
    }
  })

  it('T4 native unwrap: mapper fee then burn tax; router min is post-fee pre-tax (R3)', () => {
    const wrapped = 10_000_000n
    const quoted = nativeAfterZapUnwrap(wrapped, 51, { rate: '0.015', capUluna: 10n ** 18n })
    const afterFee = netAfterWrapMapperFee(wrapped, 51)
    expect(quoted.routerMinReceiveBase).toBe(afterFee)
    expect(quoted.receive).toBeLessThan(afterFee)
    expect(quoted.receive).toBe((afterFee * 985n) / 1000n)
  })

  it('T8 LP decimals constant is 18, not 6', () => {
    expect(PAIR_LP_CW20_DECIMALS).toBe(18)
    expect(PAIR_LP_CW20_DECIMALS).not.toBe(6)
  })
})

describe('resolveZapInputKind', () => {
  const clunc = 'terra1clunc00000000000000000000000000000000'
  const ust1 = 'terra1ust100000000000000000000000000000000'

  it('pair CW20 is a leg without wrap', () => {
    expect(resolveZapInputKind(clunc, clunc, ust1)).toEqual({
      kind: 'pair_leg',
      side: 'a',
      wrapFromNative: null,
    })
  })

  it('T7 off-pair token is not a wrap of either leg', () => {
    expect(resolveZapInputKind('terra1other0000000000000000000000000000000', clunc, ust1)).toEqual({
      kind: 'off_pair',
    })
  })

  it('T7 empty route → no_route; mocked route-out then zap', () => {
    const params = { reserveIn: 1_000_000_000n, reserveOut: 1_000_000_000n, feeBps: 30 }
    expect(applyRouteThenZap(null, params).reason).toBe('no_route')
    expect(applyRouteThenZap('50000000', params).status).toBe('ok')
  })
})

function cw20CheckedSub(balance: bigint, amount: bigint): bigint {
  if (amount > balance) {
    throw new Error(`Overflow: Cannot Sub with ${balance} and ${amount}`)
  }
  return balance - amount
}

describe('oneSidedLiquidity zap floors (GitLab #559 T-Z1–T-Z3 T-Z7 T-Z10 T-Z11)', () => {
  it('T-Z1 pair-leg 5% / 30 bps: provideAsk ≤ swapMinReturn; leftover offer conserved; ratio-trimmed', () => {
    const amountIn = 100_000_000n
    const split = zapInSplit({
      amountIn,
      reserveIn: 1_000_000_000n,
      reserveOut: 1_000_000_000n,
      feeBps: 30,
    })
    expect(split.status).toBe('ok')
    if (split.status !== 'ok') return
    const swapMin = BigInt(applySlippagePercentFloor(split.swapOut.toString(), 5)!)
    const exec = conservativeZapInProvide(split, swapMin)
    expect(exec).not.toBeNull()
    if (!exec) return
    expect(exec.provideOut).toBeLessThanOrEqual(swapMin)
    expect(exec.provideIn + exec.leftoverIn).toBe(amountIn - split.swapIn)
    const floorAsk = (exec.provideIn * exec.postReserveOut) / split.postReserveIn
    const floorOffer = (exec.provideOut * split.postReserveIn) / exec.postReserveOut
    expect(exec.provideOut === floorAsk || exec.provideIn === floorOffer).toBe(true)
    expect(exec.postReserveOut).toBe(split.postReserveOut + (split.swapOut - swapMin))
  })

  it('T-Z2 fill in (min_return, quote) must not Cannot Sub; pre-existing ask is not required', () => {
    const split = zapInSplit({
      amountIn: 200_000_000n,
      reserveIn: 1_000_000_000n,
      reserveOut: 1_000_000_000n,
      feeBps: 30,
    })
    expect(split.status).toBe('ok')
    if (split.status !== 'ok') return
    const swapMin = BigInt(applySlippagePercentFloor(split.swapOut.toString(), 5)!)
    expect(split.provideOut).toBeGreaterThan(swapMin)
    const fill = split.provideOut - 1n
    expect(fill).toBeGreaterThanOrEqual(swapMin)
    expect(() => cw20CheckedSub(fill, split.provideOut)).toThrow(/Cannot Sub/)
    const exec = conservativeZapInProvide(split, swapMin)
    expect(exec).not.toBeNull()
    if (!exec) return
    expect(exec.provideOut).toBeLessThanOrEqual(fill)
    expect(() => cw20CheckedSub(fill, exec.provideOut)).not.toThrow()
    expect(() => cw20CheckedSub(fill, exec.provideOut)).not.toThrow()
    const leftoverAsk = fill - exec.provideOut
    expect(leftoverAsk).toBeGreaterThanOrEqual(0n)
  })

  it('T-Z2 production-shaped 6-dec fill 525495 vs quote 526916 / min 500571', () => {
    const quoteOut = 526_916n
    const minReturn = 500_571n
    const fill = 525_495n
    expect(minReturn).toBeLessThan(quoteOut)
    expect(fill).toBeGreaterThan(minReturn)
    expect(fill).toBeLessThan(quoteOut)
    const split = zapInSplit({
      amountIn: 200_000_000n,
      reserveIn: 10_000_000_000n,
      reserveOut: 10_000_000_000n,
      feeBps: 30,
    })
    expect(split.status).toBe('ok')
    if (split.status !== 'ok') return
    const synthetic: typeof split = {
      ...split,
      swapOut: quoteOut,
      provideOut: quoteOut < split.provideOut ? quoteOut : split.provideOut,
    }
    if (synthetic.status !== 'ok') return
    const exec = conservativeZapInProvide(synthetic, minReturn)
    expect(exec).not.toBeNull()
    if (!exec) return
    expect(exec.provideOut).toBeLessThanOrEqual(minReturn)
    expect(() => cw20CheckedSub(fill, exec.provideOut)).not.toThrow()
    expect(() => cw20CheckedSub(fill, quoteOut)).toThrow(/Cannot Sub/)
  })

  it('T-Z3 0% slippage: floors collapse to quote; still ratio-trimmed', () => {
    const split = zapInSplit({
      amountIn: 100_000_000n,
      reserveIn: 1_000_000_000n,
      reserveOut: 1_000_000_000n,
      feeBps: 30,
    })
    expect(split.status).toBe('ok')
    if (split.status !== 'ok') return
    const swapMin = BigInt(applySlippagePercentFloor(split.swapOut.toString(), 0)!)
    expect(swapMin).toBe(split.swapOut)
    const exec = conservativeZapInProvide(split, swapMin)
    expect(exec).not.toBeNull()
    if (!exec) return
    expect(exec.provideOut).toBe(split.provideOut)
    expect(exec.provideIn).toBe(split.provideIn)
    expect(exec.postReserveOut).toBe(split.postReserveOut)
  })

  it('T-Z7 skewed pool with dust conservative LP is unavailable at the quote layer (see quote tests)', () => {
    const split = zapInSplit({
      amountIn: 100n,
      reserveIn: 10n ** 18n,
      reserveOut: 50n,
      feeBps: 30,
    })
    if (split.status !== 'ok') {
      expect(split.reason).toMatch(/dust|empty_pool/)
      return
    }
    const swapMin = BigInt(applySlippagePercentFloor(split.swapOut.toString(), 5) ?? '0')
    const exec = conservativeZapInProvide(split, swapMin)
    if (exec) {
      expect(exec.provideOut).toBeLessThanOrEqual(swapMin)
    }
  })

  it('T-Z10 zap-out swapAmount ≤ min_assets[sold]; leftover sold is ok', () => {
    const out = zapOutSplit({
      lpRaw: 1_000_000_000_000_000_000n,
      totalShare: 10_000_000_000_000_000_000n,
      reserveA: 1_000_000_000n,
      reserveB: 2_000_000_000n,
      wantSide: 'a',
      feeBps: 30,
    })
    expect(out.status).toBe('ok')
    if (out.status !== 'ok') return
    const minA = (out.withdrawnA * 95n) / 100n
    const minB = (out.withdrawnB * 95n) / 100n
    const exec = conservativeZapOutExecution({
      split: out,
      wantSide: 'a',
      minAssets: [minA.toString(), minB.toString()],
      slippagePercent: 5,
    })
    expect(exec).not.toBeNull()
    if (!exec) return
    expect(BigInt(exec.swapAmount)).toBeLessThanOrEqual(minB)
    expect(BigInt(exec.swapAmount)).toBeLessThanOrEqual(out.swapIn)
    expect(out.swapIn - BigInt(exec.swapAmount)).toBeGreaterThanOrEqual(0n)
  })

  it('T-Z11 zap-out unwrap ≤ wanted floor + swapMinReturn (never quoted totalWantedCw20 when that exceeds)', () => {
    const out = zapOutSplit({
      lpRaw: 1_000_000_000_000_000_000n,
      totalShare: 10_000_000_000_000_000_000n,
      reserveA: 1_000_000_000n,
      reserveB: 2_000_000_000n,
      wantSide: 'a',
      feeBps: 30,
    })
    expect(out.status).toBe('ok')
    if (out.status !== 'ok') return
    const minA = (out.withdrawnA * 95n) / 100n
    const minB = (out.withdrawnB * 95n) / 100n
    const exec = conservativeZapOutExecution({
      split: out,
      wantSide: 'a',
      minAssets: [minA.toString(), minB.toString()],
      slippagePercent: 5,
    })
    expect(exec).not.toBeNull()
    if (!exec) return
    const wantedFloor = out.withdrawnA < minA ? out.withdrawnA : minA
    expect(BigInt(exec.unwrapAmount)).toBe(wantedFloor + BigInt(exec.swapMinReturn))
    expect(BigInt(exec.unwrapAmount)).toBeLessThan(out.totalWantedCw20)
    expect(() => cw20CheckedSub(wantedFloor + BigInt(exec.swapMinReturn), BigInt(exec.unwrapAmount))).not.toThrow()
    expect(() => cw20CheckedSub(wantedFloor + BigInt(exec.swapMinReturn), out.totalWantedCw20)).toThrow(/Cannot Sub/)
  })
})
