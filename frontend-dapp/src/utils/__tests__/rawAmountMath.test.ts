import { describe, it, expect } from 'vitest'
import {
  applyBpsFloor,
  applySlippagePercentFloor,
  withdrawMinAssetAmounts,
  isLpBurnExceedsBalance,
  isPositiveRawAmount,
  slippagePercentToBps,
  spreadPercentFromRawSim,
} from '../rawAmountMath'

const TWO_53_PLUS_1 = '9007199254740993'

describe('isPositiveRawAmount', () => {
  it('accepts positive uint strings above 2^53', () => {
    expect(isPositiveRawAmount(TWO_53_PLUS_1)).toBe(true)
    expect(isPositiveRawAmount('1')).toBe(true)
  })

  it('rejects zero and invalid strings', () => {
    expect(isPositiveRawAmount('0')).toBe(false)
    expect(isPositiveRawAmount('')).toBe(false)
    expect(isPositiveRawAmount('abc')).toBe(false)
  })
})

describe('spreadPercentFromRawSim', () => {
  it('computes spread percent with two decimal places', () => {
    expect(spreadPercentFromRawSim('1000000', '3000', '100')).toBe('0.01')
  })

  it('handles raw amounts above Number.MAX_SAFE_INTEGER', () => {
    const spread = '891712726219358'
    const ret = '8917127262193583'
    const comm = '0'
    const total = BigInt(ret) + BigInt(comm) + BigInt(spread)
    const scaled = (BigInt(spread) * 10000n + total / 2n) / total
    const expected = `${scaled / 100n}.${(scaled % 100n).toString().padStart(2, '0')}`
    expect(spreadPercentFromRawSim(ret, comm, spread)).toBe(expected)
    expect(spreadPercentFromRawSim(ret, comm, spread)).toBe('9.09')
  })

  it('returns 0.00 when gross is zero', () => {
    expect(spreadPercentFromRawSim('0', '0', '0')).toBe('0.00')
  })
})

describe('slippagePercentToBps', () => {
  it('converts percent to bps', () => {
    expect(slippagePercentToBps(0)).toBe(0)
    expect(slippagePercentToBps(1)).toBe(100)
    expect(slippagePercentToBps(50)).toBe(5000)
  })

  it('clamps out-of-range values', () => {
    expect(slippagePercentToBps(-1)).toBe(0)
    expect(slippagePercentToBps(150)).toBe(10000)
  })
})

describe('applyBpsFloor', () => {
  it('returns same amount at 0 bps', () => {
    expect(applyBpsFloor('1000000', 0)).toBe('1000000')
    expect(applyBpsFloor(TWO_53_PLUS_1, 0)).toBe(TWO_53_PLUS_1)
  })

  it('applies 50% slippage with floor', () => {
    expect(applyBpsFloor('1000001', 5000)).toBe('500000')
    expect(applyBpsFloor('999999', 5000)).toBe('499999')
  })

  it('differs from broken float path above 2^53', () => {
    const raw = TWO_53_PLUS_1
    const expected = applyBpsFloor(raw, 100)!
    const broken = Math.floor(parseFloat(raw) * 0.99).toString()
    expect(broken).not.toBe(expected)
    expect(expected).toBe('8917127262193583')
  })

  it('returns null for invalid raw strings', () => {
    expect(applyBpsFloor('', 100)).toBe(null)
    expect(applyBpsFloor('abc', 100)).toBe(null)
    expect(applyBpsFloor('-1', 100)).toBe(null)
  })
})

describe('applySlippagePercentFloor', () => {
  it('matches 1% slippage on small amounts', () => {
    expect(applySlippagePercentFloor('1000000', 1)).toBe('990000')
  })

  it('returns full return at 0% slippage', () => {
    expect(applySlippagePercentFloor(TWO_53_PLUS_1, 0)).toBe(TWO_53_PLUS_1)
  })
})

describe('withdrawMinAssetAmounts', () => {
  it('computes proportional withdraw mins with 1% slippage', () => {
    const mins = withdrawMinAssetAmounts('1000000', '10000000', '5000000', '8000000', 1)
    expect(mins).toEqual(['495000', '792000'])
  })

  it('handles large reserve strings without float loss', () => {
    const lp = '1000000'
    const total = '10000000'
    const reserve = TWO_53_PLUS_1
    const mins = withdrawMinAssetAmounts(lp, total, reserve, reserve, 0)!
    const share = (BigInt(reserve) * BigInt(lp)) / BigInt(total)
    expect(mins[0]).toBe(share.toString())
    expect(mins[1]).toBe(share.toString())
  })

  it('returns null when pool total share is zero', () => {
    expect(withdrawMinAssetAmounts('1', '0', '100', '100', 1)).toBe(null)
  })
})

describe('isLpBurnExceedsBalance', () => {
  it('is false when burn equals balance', () => {
    expect(isLpBurnExceedsBalance('1', 6, '1000000')).toBe(false)
  })

  it('is true when burn exceeds balance by one unit', () => {
    expect(isLpBurnExceedsBalance('1.000001', 6, '1000000')).toBe(true)
  })

  it('is false for empty amount', () => {
    expect(isLpBurnExceedsBalance('', 6, '1000000')).toBe(false)
  })
})
