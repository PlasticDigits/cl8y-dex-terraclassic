import { describe, expect, it } from 'vitest'
import {
  depositVfdusdToUst1,
  quoteDepositUst1Out,
  quoteWithdrawVfdusdOut,
  UST1_RATE_SCALE,
  withdrawGrossUst1ToVfdusd,
} from '@/utils/ust1WindowMath'

describe('ust1WindowMath (#506)', () => {
  it('INV-SWAP-001: 1:1 rate with 1% fee reduces UST1 out', () => {
    const out = depositVfdusdToUst1(10_000_000n, UST1_RATE_SCALE, 100)
    expect(out).toBe(9_900_000n)
    expect(quoteDepositUst1Out('10000000', UST1_RATE_SCALE.toString(), 100)).toBe('9900000')
  })

  it('INV-SWAP-002: reverse fee vectors match ust1-common', () => {
    expect(withdrawGrossUst1ToVfdusd(10_000_000n, UST1_RATE_SCALE, 0)).toBe(10_000_000n)
    expect(withdrawGrossUst1ToVfdusd(10_000_000n, UST1_RATE_SCALE, 50)).toBe(9_950_000n)
    expect(withdrawGrossUst1ToVfdusd(2_000_000n, UST1_RATE_SCALE * 2n, 0)).toBe(1_000_000n)
    expect(quoteWithdrawVfdusdOut('10000000', UST1_RATE_SCALE.toString(), 50)).toBe('9950000')
  })

  it('rejects zero rate on withdraw', () => {
    expect(() => withdrawGrossUst1ToVfdusd(1n, 0n, 0)).toThrow(/zero/i)
    expect(quoteWithdrawVfdusdOut('1', '0', 0)).toBeNull()
  })
})
