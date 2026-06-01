import { describe, expect, it } from 'vitest'

import type { IndexerLimitBookInsertHintItem } from '@/types'
import {
  analyzeLadderDepth,
  computeLadderSkipRisk,
  countForeignOrdersBetweenRungs,
  ladderPriceWindowParams,
  wireHintPredecessor,
} from '../limitLadderDepth'

describe('ladderPriceWindowParams', () => {
  it('bid band: price_from is high, price_to is low', () => {
    expect(ladderPriceWindowParams('bid', '0.95', '1.05')).toEqual({
      priceFrom: '1.05',
      priceTo: '0.95',
    })
  })

  it('ask band: price_from is low, price_to is high', () => {
    expect(ladderPriceWindowParams('ask', '0.95', '1.05')).toEqual({
      priceFrom: '0.95',
      priceTo: '1.05',
    })
  })
})

describe('wireHintPredecessor (L14)', () => {
  it('returns undefined when unresolved', () => {
    const hint: IndexerLimitBookInsertHintItem = {
      price: '1',
      predecessor_order_id: 42,
      resolved: false,
      reason: 'pagination_gap',
    }
    expect(wireHintPredecessor(hint)).toBeUndefined()
  })

  it('returns id when resolved', () => {
    const hint: IndexerLimitBookInsertHintItem = {
      price: '1',
      predecessor_order_id: 7,
      resolved: true,
    }
    expect(wireHintPredecessor(hint)).toBe(7)
  })

  it('omits head insert (null predecessor)', () => {
    const hint: IndexerLimitBookInsertHintItem = {
      price: '2',
      predecessor_order_id: null,
      resolved: true,
      reason: 'head',
    }
    expect(wireHintPredecessor(hint)).toBeUndefined()
  })
})

describe('countForeignOrdersBetweenRungs', () => {
  const rungs = [
    { price: '1.0', amountRaw: '100' },
    { price: '1.1', amountRaw: '100' },
  ]

  it('counts order between rung prices', () => {
    const foreign = countForeignOrdersBetweenRungs('ask', rungs, [
      { order_id: 1, owner: 'a', side: 'ask', price: '1.05', remaining: '1' },
    ])
    expect(foreign).toBe(1)
  })
})

describe('computeLadderSkipRisk', () => {
  it('flags deep batch path when foreign orders present', () => {
    const depth = analyzeLadderDepth({
      side: 'bid',
      rungs: [
        { price: '0.95', amountRaw: '50' },
        { price: '1.0', amountRaw: '50' },
      ],
      startPrice: '0.95',
      endPrice: '1.0',
      count: 2,
      windowOrders: [{ order_id: 9, owner: 'x', side: 'bid', price: '0.97', remaining: '1' }],
      hints: [
        { price: '0.95', predecessor_order_id: null, resolved: true, reason: 'head' },
        { price: '1.0', predecessor_order_id: 9, resolved: true },
      ],
    })
    const risk = computeLadderSkipRisk(depth, 2, 32)
    expect(risk.needsHintedBatchPath).toBe(true)
  })
})
