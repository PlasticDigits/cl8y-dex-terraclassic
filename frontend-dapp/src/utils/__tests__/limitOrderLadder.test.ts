import { describe, expect, it } from 'vitest'

import { expandLimitLadder, LimitLadderError, sumLadderAmountsRaw } from '../limitOrderLadder'

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
    expect(sumLadderAmountsRaw(rungs)).toBe('1000')
  })

  it('sums equal rungs as integer not string concat (GitLab #233)', () => {
    const rungs = expandLimitLadder(
      {
        side: 'bid',
        startPrice: '0.95',
        endPrice: '1.05',
        count: 5,
        totalAmountRaw: '100000000',
        distribution: 'equal',
        maxAdjustSteps: 32,
      },
      20
    )
    expect(rungs.every((r) => r.amountRaw === '20000000')).toBe(true)
    expect(sumLadderAmountsRaw(rungs)).toBe('100000000')
    expect(sumLadderAmountsRaw(rungs)).not.toBe('02000000020000000200000002000000020000000')
  })

  it('throws named min-size when any equal-split rung is below 10 raw (Forgejo #1219)', () => {
    const spec = {
      side: 'bid' as const,
      startPrice: '0.95',
      endPrice: '1.05',
      count: 3,
      totalAmountRaw: '1',
      distribution: 'equal' as const,
      maxAdjustSteps: 32,
    }
    expect(() => expandLimitLadder(spec, 20)).toThrow(LimitLadderError)
    expect(() => expandLimitLadder(spec, 20)).toThrow('Minimum size is 10 units')
    expect(() => expandLimitLadder({ ...spec, totalAmountRaw: '29' }, 20)).toThrow('Minimum size is 10 units')
  })

  it('expands a healthy 5-rung ladder of 10 raw each', () => {
    const rungs = expandLimitLadder(
      {
        side: 'bid',
        startPrice: '0.95',
        endPrice: '1.05',
        count: 5,
        totalAmountRaw: '50',
        distribution: 'equal',
        maxAdjustSteps: 32,
      },
      20
    )
    expect(rungs).toHaveLength(5)
    expect(rungs.every((r) => r.amountRaw === '10')).toBe(true)
    expect(sumLadderAmountsRaw(rungs)).toBe('50')
  })

  it('keeps descending human prices (JS already used end-start)', () => {
    const rungs = expandLimitLadder(
      {
        side: 'ask',
        startPrice: '3',
        endPrice: '1',
        count: 3,
        totalAmountRaw: '30',
        distribution: 'equal',
        maxAdjustSteps: 32,
      },
      20
    )
    expect(rungs.map((r) => r.price)).toEqual(['3', '2', '1'])
  })
})
