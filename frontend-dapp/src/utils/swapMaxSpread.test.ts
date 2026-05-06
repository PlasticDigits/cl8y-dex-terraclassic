import { describe, it, expect } from 'vitest'
import {
  hybridSpreadCmpAndTotal,
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

  it('formats spread percent with two decimals', () => {
    expect(spreadPercentOfGross(1n, 100n)).toBe('1.00')
    expect(spreadPercentOfGross(1n, 3n)).toBe('33.33')
  })
})
