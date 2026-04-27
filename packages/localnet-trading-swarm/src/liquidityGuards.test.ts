import { describe, it, expect } from 'vitest'
import {
  PAIR_MINIMUM_LIQUIDITY,
  poolReservesOk,
  provideAmountsReasonable,
  pickScaledProvideAmounts,
} from './liquidityGuards.js'
import type { PoolResponse } from './types.js'

const mkPool = (a: string, b: string): PoolResponse => ({
  assets: [
    { info: { token: { contract_addr: 'terra1aa' } }, amount: a },
    { info: { token: { contract_addr: 'terra1bb' } }, amount: b },
  ],
  total_share: '1000000000000',
})

describe('liquidityGuards', () => {
  it('exports PAIR_MINIMUM_LIQUIDITY matching dApp constant', () => {
    expect(PAIR_MINIMUM_LIQUIDITY).toBe(1000n)
  })

  it('poolReservesOk rejects thin pools', () => {
    expect(poolReservesOk(mkPool('1000', '1000'))).toBe(false)
    expect(poolReservesOk(mkPool('10000000000', '10000000000'))).toBe(true)
  })

  it('pickScaledProvideAmounts returns null when too small', () => {
    const p = mkPool('5000000', '5000000')
    expect(pickScaledProvideAmounts(p, 1n)).toBeNull()
  })

  it('provideAmountsReasonable', () => {
    expect(provideAmountsReasonable(6000000n, 6000000n)).toBe(true)
    expect(provideAmountsReasonable(100n, 100n)).toBe(false)
  })
})
