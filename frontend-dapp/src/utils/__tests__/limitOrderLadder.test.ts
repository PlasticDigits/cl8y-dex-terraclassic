import { describe, expect, it } from 'vitest'

import { expandLimitLadder } from '../limitOrderLadder'

describe('expandLimitLadder', () => {
  it('splits total amount across rungs', () => {
    const rungs = expandLimitLadder(
      {
        side: 'bid',
        startPrice: '0.9',
        endPrice: '1.1',
        count: 5,
        totalAmountRaw: '1000',
        distribution: 'equal',
        maxAdjustSteps: 32,
      },
      20
    )
    expect(rungs).toHaveLength(5)
    const sum = rungs.reduce((a, r) => a + BigInt(r.amountRaw), 0n)
    expect(sum).toBe(1000n)
  })
})
