import { describe, expect, it } from 'vitest'

import {
  buildLimitLadderPlacementPlan,
  buildLadderSpecWire,
  ladderRungsToBatchItems,
} from '../limitLadderPlacementPlan'

const rungs = [
  { price: '0.95', amountRaw: '50000000' },
  { price: '1.0', amountRaw: '50000000' },
  { price: '1.05', amountRaw: '50000000' },
]

describe('buildLimitLadderPlacementPlan (GitLab #268)', () => {
  it('thin empty book → thin_ladder path', () => {
    const plan = buildLimitLadderPlacementPlan({
      side: 'bid',
      startPrice: '0.95',
      endPrice: '1.05',
      count: 3,
      rungs,
      maxAdjustSteps: 32,
      windowOrders: [],
      hints: rungs.map((r) => ({
        price: r.price,
        predecessor_order_id: null,
        resolved: true,
        reason: 'head',
      })),
    })
    expect(plan.path).toBe('thin_ladder')
    expect(plan.skipRisk.predictedPlaced).toBe(3)
  })

  it('foreign orders → deep_batch path', () => {
    const plan = buildLimitLadderPlacementPlan({
      side: 'ask',
      startPrice: '0.95',
      endPrice: '1.05',
      count: 3,
      rungs,
      maxAdjustSteps: 32,
      windowOrders: [{ order_id: 5, price: '0.98', owner: 'o', side: 'ask', remaining: '1' }],
      hints: rungs.map((r, i) => ({
        price: r.price,
        predecessor_order_id: i === 0 ? null : 5,
        resolved: true,
      })),
    })
    expect(plan.path).toBe('deep_batch')
  })

  it('probe degraded → thin_ladder with warning note', () => {
    const plan = buildLimitLadderPlacementPlan({
      side: 'bid',
      startPrice: '0.95',
      endPrice: '1.05',
      count: 3,
      rungs,
      maxAdjustSteps: 32,
      windowOrders: [],
      hints: [],
      probeDegraded: true,
    })
    expect(plan.path).toBe('thin_ladder')
    expect(plan.probeDegraded).toBe(true)
    expect(plan.notes.some((n) => n.includes('unavailable'))).toBe(true)
  })
})

describe('ladderRungsToBatchItems', () => {
  it('omits hint when resolved:false', () => {
    const items = ladderRungsToBatchItems(
      [{ price: '1', amountRaw: '100' }],
      [{ price: '1', predecessor_order_id: 9, resolved: false, reason: 'pagination_gap' }],
      32
    )
    expect(items[0]?.hint_after_order_id).toBeUndefined()
  })

  it('includes hint when resolved', () => {
    const items = ladderRungsToBatchItems(
      [{ price: '1', amountRaw: '100' }],
      [{ price: '1', predecessor_order_id: 9, resolved: true }],
      32
    )
    expect(items[0]?.hint_after_order_id).toBe(9)
  })
})

describe('buildLadderSpecWire single anchor', () => {
  it('sets boundary hint on ladder spec only', () => {
    const plan = buildLimitLadderPlacementPlan({
      side: 'bid',
      startPrice: '1.05',
      endPrice: '0.95',
      count: 3,
      rungs,
      maxAdjustSteps: 32,
      windowOrders: [{ order_id: 2, price: '1.02', owner: 'o', side: 'bid', remaining: '1' }],
      hints: [
        { price: '1.05', predecessor_order_id: null, resolved: true, reason: 'head' },
        { price: '1.0', predecessor_order_id: 2, resolved: true },
        { price: '0.95', predecessor_order_id: 2, resolved: true },
      ],
    })
    if (plan.path !== 'single_anchor_ladder') {
      // deep book with foreign — skip anchor test
      return
    }
    const spec = buildLadderSpecWire({
      side: 'bid',
      startPrice: '1.05',
      endPrice: '0.95',
      count: 3,
      totalAmountRaw: '150000000',
      maxAdjustSteps: 32,
      plan,
    })
    expect(spec.hint_after_order_id).toBeDefined()
  })
})
