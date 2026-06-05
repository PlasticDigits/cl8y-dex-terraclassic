import { describe, expect, it } from 'vitest'
import type { IndexerShallowLimitOrder } from '@/types'
import { flattenLimitBookPages, resolveLimitInsertHintAfter } from '../limitBookInsertHint'

function order(order_id: number, price: string): IndexerShallowLimitOrder {
  return { order_id, owner: 'terra1maker', side: 'bid', price, remaining: '1000' }
}

describe('resolveLimitInsertHintAfter', () => {
  const bids = [order(1, '1.0'), order(2, '0.9'), order(3, '0.8')]

  it('returns null on empty book', () => {
    expect(resolveLimitInsertHintAfter('bid', '0.5', [], { hasMore: false })).toBeNull()
  })

  it('returns null for head insert (bid better than head)', () => {
    expect(resolveLimitInsertHintAfter('bid', '1.05', bids, { hasMore: false })).toBeNull()
  })

  it('returns predecessor for mid-book bid', () => {
    expect(resolveLimitInsertHintAfter('bid', '0.95', bids, { hasMore: false })).toBe(1)
  })

  it('returns last same-price order for equal-price bid tail at level', () => {
    const level = [order(10, '1.0'), order(11, '1.0'), order(12, '0.9')]
    expect(resolveLimitInsertHintAfter('bid', '1.0', level, { hasMore: false })).toBe(11)
  })

  it('returns last loaded order for tail bid when book fully loaded', () => {
    expect(resolveLimitInsertHintAfter('bid', '0.75', bids, { hasMore: false })).toBe(3)
  })

  it('returns null for tail bid when pagination has more pages', () => {
    expect(resolveLimitInsertHintAfter('bid', '0.75', bids, { hasMore: true })).toBeNull()
  })

  it('returns null for head insert on ask side', () => {
    const asks = [order(1, '0.8'), order(2, '0.9'), order(3, '1.0')]
    expect(resolveLimitInsertHintAfter('ask', '0.75', asks, { hasMore: false })).toBeNull()
  })

  it('returns predecessor for mid-book ask', () => {
    const asks = [order(1, '0.8'), order(2, '0.9'), order(3, '1.0')]
    expect(resolveLimitInsertHintAfter('ask', '0.85', asks, { hasMore: false })).toBe(1)
  })

  it('returns null for invalid price', () => {
    expect(resolveLimitInsertHintAfter('bid', 'abc', bids, { hasMore: false })).toBeNull()
  })
})

describe('flattenLimitBookPages', () => {
  it('concatenates pages and tracks tail has_more', () => {
    const flat = flattenLimitBookPages([
      { orders: [order(1, '1')], has_more: true },
      { orders: [order(2, '0.9')], has_more: false },
    ])
    expect(flat.orders.map((o) => o.order_id)).toEqual([1, 2])
    expect(flat.hasMore).toBe(false)
  })

  it('ignores pages with missing orders (GitLab #327)', () => {
    const flat = flattenLimitBookPages([
      { orders: [order(1, '1')], has_more: true },
      { has_more: false } as { orders: IndexerShallowLimitOrder[]; has_more: boolean },
    ])
    expect(flat.orders.map((o) => o.order_id)).toEqual([1])
    expect(flat.hasMore).toBe(false)
  })
})
