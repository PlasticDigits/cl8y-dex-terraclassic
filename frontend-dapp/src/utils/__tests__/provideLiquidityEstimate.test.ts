import { describe, it, expect } from 'vitest'
import {
  isqrt,
  PAIR_MINIMUM_LIQUIDITY,
  estimateProvideLiquidityUserLp,
  isProportionalAddAmounts,
} from '../provideLiquidityEstimate'
import type { PoolResponse } from '@/types'

const pool1m = (total: string, r0: string, r1: string): PoolResponse => ({
  assets: [
    { info: { token: { contract_addr: 'a' } }, amount: r0 },
    { info: { token: { contract_addr: 'b' } }, amount: r1 },
  ],
  total_share: total,
})

describe('isqrt', () => {
  it('matches small values', () => {
    expect(isqrt(0n)).toBe(0n)
    expect(isqrt(1n)).toBe(1n)
    expect(isqrt(2n)).toBe(1n)
    expect(isqrt(4n)).toBe(2n)
    expect(isqrt(15n)).toBe(3n)
    expect(isqrt(16n)).toBe(4n)
  })
})

describe('estimateProvideLiquidityUserLp', () => {
  it('first deposit: sqrt(1M * 1M) - minimum liquidity', () => {
    const pool = pool1m('0', '0', '0')
    const userLp = estimateProvideLiquidityUserLp('1000000', '1000000', pool)
    const lpTotal = isqrt(1_000_000n * 1_000_000n)
    expect(userLp).toBe(lpTotal - PAIR_MINIMUM_LIQUIDITY)
  })

  it('subsequent deposit: min of two terms', () => {
    const pool = pool1m('1000000', '1000000', '2000000')
    const u = estimateProvideLiquidityUserLp('500000', '500000', pool)
    const lpA = (500_000n * 1_000_000n) / 1_000_000n
    const lpB = (500_000n * 1_000_000n) / 2_000_000n
    expect(u).toBe(lpA < lpB ? lpA : lpB)
    expect(u).toBe(250_000n)
  })

  it('returns null for zero/empty', () => {
    const p = pool1m('1000', '1000', '1000')
    expect(estimateProvideLiquidityUserLp('', '1', p)).toBeNull()
    expect(estimateProvideLiquidityUserLp('0', '1', p)).toBeNull()
  })
})

describe('isProportionalAddAmounts', () => {
  it('is true on 1:1 pool with equal raw amounts', () => {
    const pool = pool1m('1000000', '1000000', '1000000')
    expect(isProportionalAddAmounts('1000', '1000', pool)).toBe(true)
  })

  it('is false when one side is in excess (same pool)', () => {
    const pool = pool1m('1000000', '1000000', '1000000')
    expect(isProportionalAddAmounts('2000', '1000', pool)).toBe(false)
  })
})
