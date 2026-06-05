import { describe, it, expect } from 'vitest'
import {
  hybridMaxSpreadRealizedLegs,
  hybridNoBeliefMaterialPoolReject,
  hybridSpreadCmpAndTotal,
  minPoolInputForBookHybrid,
  parseDecimalStringToScaled,
  poolOnlySpreadCmpAndTotal,
  spreadPercentOfGross,
  spreadRatioStrictlyExceedsMax,
} from './swapMaxSpread'

describe('swapMaxSpread', () => {
  it('parses decimal strings to scaled rationals', () => {
    expect(parseDecimalStringToScaled('0.01')).toEqual({ mantissa: 1n, scalePow: 100n })
    expect(parseDecimalStringToScaled('1')).toEqual({ mantissa: 1n, scalePow: 1n })
    expect(parseDecimalStringToScaled('0.5')).toEqual({ mantissa: 5n, scalePow: 10n })
  })

  it('detects ratio strictly greater than max spread (contract parity)', () => {
    expect(spreadRatioStrictlyExceedsMax(1n, 100n, '0.01')).toBe(false)
    expect(spreadRatioStrictlyExceedsMax(2n, 100n, '0.01')).toBe(true)
  })

  it('computes pool-only spread_cmp capped by pool gross', () => {
    const { spreadCmp, totalGrossOut } = poolOnlySpreadCmpAndTotal({
      return_amount: '90',
      commission_amount: '10',
      spread_amount: '500',
    })
    expect(totalGrossOut).toBe(100n)
    expect(spreadCmp).toBe(100n)
  })

  it('computes hybrid spread_cmp and total gross including book leg', () => {
    const { spreadCmp, totalGrossOut } = hybridSpreadCmpAndTotal({
      spread_amount: '5',
      commission_amount: '1',
      pool_return_amount: '9',
      book_return_amount: '20',
    })
    expect(spreadCmp).toBe(5n)
    expect(totalGrossOut).toBe(30n)
  })

  it('folds book shortfall into hybrid spread_cmp (#273)', () => {
    const { spreadCmp } = hybridSpreadCmpAndTotal(
      {
        spread_amount: '1',
        commission_amount: '3',
        pool_return_amount: '997',
        book_return_amount: '4985',
      },
      { poolInput: 1000n, bookInput: 10000n }
    )
    expect(spreadCmp).toBeGreaterThan(4000n)
  })

  it('folds book shortfall when book net is zero (#273)', () => {
    const { spreadCmp } = hybridSpreadCmpAndTotal(
      {
        spread_amount: '1',
        commission_amount: '3',
        pool_return_amount: '1097',
        book_return_amount: '0',
      },
      { poolInput: 1100n, bookInput: 9900n }
    )
    expect(spreadCmp).toBeGreaterThan(4900n)
  })

  it('uses realized legs for max-spread shortfall (#273)', () => {
    expect(hybridMaxSpreadRealizedLegs(1000n, 10000n, 0n)).toEqual({
      poolInput: 11000n,
      bookInput: 0n,
    })
    expect(hybridMaxSpreadRealizedLegs(1000n, 10000n, 10000n)).toEqual({
      poolInput: 1000n,
      bookInput: 10000n,
    })
    expect(hybridMaxSpreadRealizedLegs(0n, 100000n, 60000n)).toEqual({
      poolInput: 40000n,
      bookInput: 60000n,
    })
    expect(hybridMaxSpreadRealizedLegs(1000n, 10000n, 5000n)).toEqual({
      poolInput: 6000n,
      bookInput: 5000n,
    })
  })

  it('requires at least 10% pool leg when book leg is set (#307)', () => {
    expect(minPoolInputForBookHybrid(11000n)).toBe(1100n)
    expect(hybridNoBeliefMaterialPoolReject(11000n, 1000n, 10000n, 997n)).toMatchObject({
      kind: 'insufficient_pool_leg',
    })
    expect(hybridNoBeliefMaterialPoolReject(10000n, 6000n, 4000n, 5947n)).toBeNull()
  })

  it('formats spread percent with two decimals', () => {
    expect(spreadPercentOfGross(1n, 100n)).toBe('1.00')
    expect(spreadPercentOfGross(1n, 3n)).toBe('33.33')
  })
})
